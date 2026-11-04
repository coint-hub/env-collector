use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[arg(required = true)]
    paths: Vec<String>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let mut paths = Vec::new();
    let mut errors = Vec::new();
    for result in cli.paths.iter().map(|path| path_validate(path)) {
        match result {
            Ok(path) => paths.push(path),
            Err(err) => errors.push(err),
        }
    }
    if !errors.is_empty() {
        eprintln!("Failed to validate {} paths:", errors.len());
        for err in errors {
            match err {
                PathValidationError::Io { path, io_error } => {
                    eprintln!("  I/O error for '{}': {}", path, io_error);
                }
                PathValidationError::NotADirectory { path } => {
                    eprintln!("  Not a directory: {}", path.display())
                }
                PathValidationError::ShouldNotTargetRoot { path } => {
                    eprintln!("  Should not target root directory: {}", path.display())
                }
            }
        }
        return ExitCode::FAILURE;
    }

    for path in paths {
        println!("TODO: {}", path.display());
    }

    ExitCode::SUCCESS
}

#[derive(Debug)]
enum PathValidationError {
    Io { path: String, io_error: io::Error },
    NotADirectory { path: PathBuf },
    ShouldNotTargetRoot { path: PathBuf },
}

fn path_validate(raw_path: &str) -> Result<PathBuf, PathValidationError> {
    let path = std::path::absolute(raw_path).map_err(|io_error| PathValidationError::Io {
        path: raw_path.to_string(),
        io_error,
    })?;

    if !path.is_dir() {
        return Err(PathValidationError::NotADirectory { path });
    }
    if path.parent().is_none() {
        return Err(PathValidationError::ShouldNotTargetRoot { path });
    }

    Ok(path)
}
