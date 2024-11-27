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

        if let Ok(gba_file_mtime) = gba_file_mtime {
            if elf_file_mtime < gba_file_mtime {
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

        agb_gbafix::write_gba_file(
            &elf_content,
            GbaHeader {
                start_code: [b'a', b'b', b'c', b'd'],
                game_title: [
                    b'a', b'b', b'c', b'd', b'e', b'f', b'g', b'h', b'i', b'j', b'k', b'l',
                ],
                maker_code: [b'A', b'G'],
                game_code: [b'a', b'b', b'c', b'd'],
                software_version: 1,
            },
            agb_gbafix::PaddingBehaviour::DoNotPad,
            args.include_debug_info,
            &mut buf_writer,
        )
        .context("Failed to fix gba file header")?;

        buf_writer.flush()?;

        Ok(output_file_name)
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
