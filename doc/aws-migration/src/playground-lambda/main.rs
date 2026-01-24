use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde::Deserialize;
use std::process::Command;
use tempfile::TempDir;

#[derive(Deserialize)]
struct BuildRequest {
    code: String,
}

async fn build_handler(event: Request) -> Result<Response<Body>, Error> {
    let code: BuildRequest = serde_json::from_slice(event.body())?;

    // Create temp build directory
    let build_dir = TempDir::new_in("/tmp")?;

    // Copy cached target directory for faster builds
    let status = Command::new("cp")
        .args([
            "-r",
            "/var/task/cached-target",
            &build_dir.path().join("target").to_string_lossy(),
        ])
        .status()?;

    if !status.success() {
        return Ok(Response::builder()
            .status(500)
            .body(Body::Text("Failed to setup build environment".into()))?);
    }

    // Copy template Cargo.toml
    std::fs::copy(
        "/var/task/template/Cargo.toml",
        build_dir.path().join("Cargo.toml"),
    )?;
    std::fs::create_dir_all(build_dir.path().join("src"))?;

    // Write user code
    std::fs::write(build_dir.path().join("src/main.rs"), &code.code)?;

    // Run cargo build with timeout handled by Lambda itself (90s)
    let output = Command::new("cargo")
        .current_dir(&build_dir)
        .args(["build", "--release"])
        .env("CARGO_HOME", "/root/.cargo")
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Ok(Response::builder()
            .status(400)
            .header("content-type", "text/plain")
            .body(Body::Text(stderr.into_owned()))?);
    }

    // Find the built binary
    let binary_path = build_dir
        .path()
        .join("target/thumbv4t-none-eabi/release/agb-template");

    // Fix ROM header with agb-gbafix
    let gba_path = build_dir.path().join("output.gba");
    let fix_output = Command::new("agb-gbafix")
        .args([
            &binary_path.to_string_lossy(),
            "-o",
            &gba_path.to_string_lossy(),
        ])
        .output()?;

    if !fix_output.status.success() {
        return Ok(Response::builder()
            .status(500)
            .body(Body::Text("Failed to fix ROM header".into()))?);
    }

    // Compress with gzip
    let rom_data = std::fs::read(&gba_path)?;
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    std::io::Write::write_all(&mut encoder, &rom_data)?;
    let compressed = encoder.finish()?;

    Ok(Response::builder()
        .status(200)
        .header("content-type", "application/gzip")
        .body(Body::Binary(compressed))?)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();

    run(service_fn(build_handler)).await
}
