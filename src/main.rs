use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{fs, io};

use clap::Parser;
use log::{debug, error, trace};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[arg(required = true)]
    paths: Vec<String>,

    #[command(flatten)]
    verbosity: clap_verbosity_flag::Verbosity,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    env_logger::builder()
        .filter_level(cli.verbosity.into())
        .init();

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
                    error!("  I/O error for '{}': {}", path, io_error);
                }
                PathValidationError::NotADirectory { path } => {
                    error!("  Not a directory: {}", path.display())
                }
                PathValidationError::ShouldNotTargetRoot { path } => {
                    error!("  Should not target root directory: {}", path.display())
                }
            }
        }
        return ExitCode::FAILURE;
    }

    for path in paths {
        match env_collect(path) {
            Ok(result) => {
                println!("{}: {} env vars", result.path.display(), result.count);
            }
            Err(err) => {
                error!(
                    "Failed to collect env vars from {}: {}",
                    err.path.display(),
                    err.message
                );
                return ExitCode::FAILURE;
            }
        }
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

struct EnvCollectResult {
    path: PathBuf,
    count: usize,
}

struct EnvCollectError {
    path: PathBuf,
    message: String,
}

fn env_collect(path: PathBuf) -> Result<EnvCollectResult, EnvCollectError> {
    debug!("collecting env vars from {}", path.display());
    let files = env_find_files(path.clone()).map_err(|e| EnvCollectError {
        path: e.path.clone(),
        message: e.message,
    })?;
    Ok(EnvCollectResult {
        path,
        count: files.len(),
    })
}

struct EnvFindFilesError {
    path: PathBuf,
    message: String,
}

impl EnvFindFilesError {
    fn from_io(path: &Path, io_error: io::Error) -> Self {
        Self {
            path: path.to_path_buf(),
            message: io_error.to_string(),
        }
    }
}

fn env_find_files(dir: PathBuf) -> Result<Vec<PathBuf>, EnvFindFilesError> {
    let mut files = vec![];

    let read_dir =
        fs::read_dir(dir.clone()).map_err(|io_error| EnvFindFilesError::from_io(&dir, io_error))?;

    for entry in read_dir {
        let entry = entry.map_err(|io_error| EnvFindFilesError::from_io(&dir, io_error))?;
        let path = entry.path();
        if path.is_file() && env_is_env_file(&path) {
            trace!("Found env file: {}", path.display());
            files.push(path);
            continue;
        }
        if path.is_dir() && !env_is_dir_ignored(&path) {
            files.extend(env_find_files(path)?);
        }
    }

    files.sort();
    Ok(files)
}

fn env_is_env_file(path: &Path) -> bool {
    match path.file_name().and_then(|name| name.to_str()) {
        None => false,
        Some(name) => env_is_env_filename(name),
    }
}

fn env_is_env_filename(name: &str) -> bool {
    // dotenv
    if name.starts_with(".env") || name.starts_with(".env.") {
        return true;
    }

    // direnv
    if name.starts_with(".envrc") || name.starts_with(".envrc.") {
        return true;
    }

    // spring
    if name.starts_with("application")
        && (name.ends_with("yml") || name.ends_with("yaml") || name.ends_with("properties"))
    {
        return true;
    }

    // android
    if name.ends_with(".jks") {
        return true;
    }
    if name == "keystore.properties" {
        return true;
    }

    // flutter
    if name.starts_with("main") && name.ends_with(".dart") {
        return true;
    }

    false
}

fn env_is_dir_ignored(path: &Path) -> bool {
    match path.file_name().and_then(|name| name.to_str()) {
        None => false,
        Some(name) => env_is_dirname_ignored(name),
    }
}

fn env_is_dirname_ignored(dirname: &str) -> bool {
    // any hidden directory
    if dirname.starts_with('.') {
        return true;
    }

    // python
    if matches!(dirname, "__pycache__" | "venv") {
        return true;
    }

    // nodejs
    if matches!(dirname, "node_modules") {
        return true;
    }

    false
}
