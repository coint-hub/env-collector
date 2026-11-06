//! End-to-end tests: run the compiled binary against real directory trees.
//!
//! Unix only, because they create symlinks and inspect permission bits.
#![cfg(unix)]

use std::fs::{self, File};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A scratch directory holding one project to collect from and one
/// directory to run the binary in.
struct Workspace {
    root: PathBuf,
}

impl Workspace {
    fn new(name: &str) -> Self {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("out")).unwrap();
        fs::create_dir_all(root.join("proj")).unwrap();
        Self { root }
    }

    fn project(&self) -> PathBuf {
        self.root.join("proj")
    }

    fn collect(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_env-collector"))
            .arg(self.project())
            .current_dir(self.root.join("out"))
            .output()
            .unwrap()
    }

    fn archive(&self) -> PathBuf {
        let mut archives: Vec<PathBuf> = fs::read_dir(self.root.join("out"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(archives.len(), 1, "expected exactly one archive");
        archives.remove(0)
    }

    fn archive_entries(&self) -> Vec<String> {
        let decoder = zstd::Decoder::new(File::open(self.archive()).unwrap()).unwrap();
        let mut entries: Vec<String> = tar::Archive::new(decoder)
            .entries()
            .unwrap()
            .map(|entry| entry.unwrap().path().unwrap().display().to_string())
            .collect();
        entries.sort();
        entries
    }
}

#[test]
fn symlinked_directory_is_not_followed() {
    let workspace = Workspace::new("symlinked_directory_is_not_followed");
    let project = workspace.project();
    fs::write(project.join(".env"), "A=1\n").unwrap();
    fs::create_dir(project.join("inner")).unwrap();
    symlink(&project, project.join("inner").join("back")).unwrap();

    let output = workspace.collect();

    assert!(output.status.success());
    assert_eq!(workspace.archive_entries(), ["proj/.env"]);
}

#[test]
fn symlinked_env_file_is_not_collected() {
    let workspace = Workspace::new("symlinked_env_file_is_not_collected");
    let outside = workspace.root.join("outside.env");
    fs::write(&outside, "A=1\n").unwrap();
    symlink(&outside, workspace.project().join(".env")).unwrap();

    let output = workspace.collect();

    assert!(output.status.success());
    assert_eq!(workspace.archive_entries(), Vec::<String>::new());
}

#[test]
fn skipped_symlink_is_warned_about_by_default() {
    let workspace = Workspace::new("skipped_symlink_is_warned_about_by_default");
    let link = workspace.project().join("linked");
    symlink(workspace.root.join("out"), &link).unwrap();

    let output = workspace.collect();

    let stderr = String::from_utf8(output.stderr).unwrap();
    let warning = stderr
        .lines()
        .find(|line| line.contains("WARN"))
        .unwrap_or_else(|| panic!("no warning in stderr: {stderr:?}"));
    assert!(
        warning.contains(link.to_str().unwrap()),
        "warning does not name the symlink: {warning}"
    );
}

#[test]
fn archive_is_read_only_for_its_owner() {
    let workspace = Workspace::new("archive_is_read_only_for_its_owner");
    fs::write(workspace.project().join(".env"), "A=1\n").unwrap();

    let output = workspace.collect();

    assert!(output.status.success());
    let mode = fs::metadata(workspace.archive())
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o400, "mode was {:o}", mode & 0o777);
}
