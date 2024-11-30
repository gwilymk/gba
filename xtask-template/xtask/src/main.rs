use std::{
    env,
    path::{Path, PathBuf},
};

use build::BuildArgs;
use clap::{Args, Parser, Subcommand};
use xshell::Shell;

const GAME_FOLDER: &str = "my_game";

#[derive(Parser, Debug)]
#[command(name = "cargo xtask")]
struct XTaskCli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Build {
        #[command(flatten)]
        args: CliBuildArgs,
    },
}

#[derive(Debug, Args, Clone, Default)]
#[command(flatten_help = true)]
struct CliBuildArgs {
    #[arg(long, short)]
    release: bool,
    #[arg(long, short('d'))]
    include_debug: bool,
}

impl Command {
    fn execute(&self) -> anyhow::Result<()> {
        let mut sh = Shell::new()?;

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let root_game_folder = Path::new(manifest_dir).join("..").canonicalize()?;

        let target_dir = if let Ok(target) = env::var("CARGO_TARGET_DIR") {
            PathBuf::from(&target)
        } else {
            root_game_folder.join("target")
        };

        sh.set_var("CARGO_TARGET_DIR", &target_dir);

        let game_folder = Path::new(manifest_dir).join("..").join(GAME_FOLDER);
        sh.change_dir(&game_folder);

        let cargo_metadata = cargo_metadata::MetadataCommand::new()
            .manifest_path(game_folder.join("Cargo.toml"))
            .no_deps()
            .exec()?;

        match self {
            Command::Build { args } => {
                let build_args = BuildArgs::from_cli_build_args(args, &cargo_metadata, &target_dir);

                let output_file = build::build(&mut sh, &build_args)?;
                println!("Built gba file to {}", output_file.display());
            }
        }

        Ok(())
    }
}

mod build {
    use std::{
        fs::{self, File},
        io::{BufWriter, Write},
        path::{Path, PathBuf},
        time::SystemTime,
    };

    use agb_gbafix::GbaHeader;
    use anyhow::Context;
    use serde::Deserialize;
    use xshell::{cmd, Shell};

    use crate::CliBuildArgs;

    pub struct BuildArgs {
        release: bool,
        include_debug_info: bool,
        cargo_metadata: cargo_metadata::Metadata,
        target_dir: PathBuf,
    }

    impl BuildArgs {
        pub fn from_cli_build_args(
            args: &CliBuildArgs,
            cargo_metadata: &cargo_metadata::Metadata,
            target_dir: &Path,
        ) -> Self {
            Self {
                release: args.release,
                include_debug_info: args.include_debug,
                cargo_metadata: cargo_metadata.clone(),
                target_dir: target_dir.to_path_buf(),
            }
        }
    }

    // Returns the path to the resulting .gba file
    pub fn build(sh: &mut Shell, args: &BuildArgs) -> anyhow::Result<PathBuf> {
        let release_arg = if args.release {
            Some("--release")
        } else {
            None
        };

        cmd!(sh, "cargo build {release_arg...}").run()?;

        let extension = if cfg!(windows) { ".elf" } else { "" };
        let folder = if args.release { "release" } else { "debug" };

        let elf_file = args
            .target_dir
            .join("thumbv4t-none-eabi")
            .join(folder)
            .join(&args.cargo_metadata.packages[0].name)
            .with_extension(extension);
        let output_file_name = elf_file.with_extension("gba");

        let elf_file_mtime = get_mtime(&elf_file)?;
        let gba_file_mtime = get_mtime(&output_file_name);
        let cargo_file_mtime = get_mtime(&sh.current_dir().join("Cargo.toml"))?;

        if let Ok(gba_file_mtime) = gba_file_mtime {
            if elf_file_mtime < gba_file_mtime && cargo_file_mtime < gba_file_mtime {
                println!("Skipping generating gba file as unchanged");
                return Ok(output_file_name);
            }
        }

        let elf_content = fs::read(&elf_file)
            .with_context(|| format!("Failed to read from {}", elf_file.display()))?;

        let output = File::create(&output_file_name).with_context(|| {
            format!(
                "Failed to open file {} for writing",
                output_file_name.display()
            )
        })?;
        let mut buf_writer = BufWriter::new(output);

        let header = gba_header_from_metadata(&args.cargo_metadata.packages[0])?;

        agb_gbafix::write_gba_file(
            &elf_content,
            header,
            agb_gbafix::PaddingBehaviour::DoNotPad,
            args.include_debug_info,
            &mut buf_writer,
        )
        .context("Failed to fix gba file header")?;

        buf_writer.flush()?;

        Ok(output_file_name)
    }

    fn gba_header_from_metadata(package: &cargo_metadata::Package) -> anyhow::Result<GbaHeader> {
        #[derive(Deserialize, Default, Debug)]
        struct AgbMetadata {
            start_code: Option<String>,
            game_title: Option<String>,
            maker_code: Option<String>,
            game_code: Option<String>,
            software_version: Option<u8>,
        }
        let agb_metadata: AgbMetadata = package
            .metadata
            .get("agb")
            .map(|metadata| {
                serde_json::from_value(metadata.clone()).context("Failed to parse metadata")
            })
            .unwrap_or_else(|| Ok(Default::default()))?;

        let mut start_code = [0; 4];
        start_code.copy_from_slice(agb_metadata.start_code.unwrap_or_default().as_bytes());
        let mut game_title = [0; 12];
        game_title.copy_from_slice(agb_metadata.game_title.unwrap_or_default().as_bytes());
        let mut maker_code = [0; 2];
        maker_code.copy_from_slice(agb_metadata.maker_code.unwrap_or_default().as_bytes());
        let mut game_code = [0; 4];
        game_code.copy_from_slice(agb_metadata.game_code.unwrap_or_default().as_bytes());
        let software_version = agb_metadata.software_version.unwrap_or_default();

        let header = GbaHeader {
            start_code,
            game_title,
            maker_code,
            game_code,
            software_version,
        };

        Ok(header)
    }

    fn get_mtime(file: &Path) -> Result<SystemTime, anyhow::Error> {
        fs::metadata(file)
            .with_context(|| format!("failed to get metadata for {}", file.display()))?
            .modified()
            .with_context(|| format!("Failed to get modification time for {}", file.display()))
    }
}

impl Default for Command {
    fn default() -> Self {
        Command::Build {
            args: Default::default(),
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args = XTaskCli::parse();

    let command = args.command.unwrap_or_default();

    command.execute()?;

    Ok(())
}
