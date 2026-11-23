//! Zap — keyboard-first Database IDE for the terminal (explorer · SQL · results).

mod app;
mod host;

use anyhow::Result;
use std::path::PathBuf;

fn print_version() {
    println!("zap {}", env!("CARGO_PKG_VERSION"));
}

fn print_help() {
    println!(
        "zap {} — keyboard-first Database IDE for the terminal\n\n\
         Usage:\n  \
         zap [WORKSPACE]\n  \
         zap --version | -V\n  \
         zap --help | -h\n\n\
         WORKSPACE defaults to the current directory.",
        env!("CARGO_PKG_VERSION")
    );
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let first = args.next();
    match first.as_deref() {
        Some("--version") | Some("-V") => {
            print_version();
            return Ok(());
        }
        Some("--help") | Some("-h") => {
            print_help();
            return Ok(());
        }
        _ => {}
    }

    if let Some(dir) = ide_core::cache_dir() {
        let _ = std::fs::create_dir_all(&dir);
        let file = std::fs::File::create(dir.join("zap.log"))?;
        tracing_subscriber::fmt()
            .with_writer(file)
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .init();
    }

    let root = first
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    app::run(root).await
}
