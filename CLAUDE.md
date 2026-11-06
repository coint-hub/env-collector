# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`env-collector` is a Rust CLI that collects a developer workspace's local configuration (`.env` and similar files) into an archive. Given one or more project directories, it finds the matching files in each and writes one `<dirname>_<YYYYmmdd-HHMMSS>.tar.zst` per directory into the current working directory.

The archives contain real secrets. `*.tar` and `*.tar.zst` are gitignored for that reason; when trying the tool by hand, run it from a scratch directory rather than the repository root.

## Commands

The toolchain (rustc, cargo, clippy, rustfmt, rust-analyzer) is pinned in `shell.nix`. It loads automatically through direnv (`.envrc` is `use nix`, kept out of git by the global ignore file), or manually with `nix-shell`.

```sh
cargo build
cargo run -- <dir>...            # -v / -q adjust log verbosity
cargo test                       # all tests
cargo test --test cli <name>     # a single end-to-end test, by name substring
cargo clippy --all-targets       # must stay warning-free
cargo fmt
```

## Architecture

Everything lives in `src/main.rs`; it is a binary crate with no library target. `main` runs a three-stage pipeline:

1. **Validate** (`path_validate`): every argument is made absolute and must be an existing directory other than the filesystem root. All failures are collected and reported together before exiting, rather than stopping at the first.
2. **Find** (`env_find_files`): a recursive walk. `env_is_env_filename` decides which file names count (dotenv, direnv, Spring `application*`, Android keystores, Flutter `main*.dart`); `env_is_dirname_ignored` decides which directories are not entered (hidden directories, `__pycache__`, `venv`, `node_modules`). To support a new ecosystem, extend these two functions.
3. **Archive** (`env_build_tar`): files are streamed through `tar::Builder` into a `zstd::Encoder`. Entries are stored relative to the target's *parent*, so the archive unpacks into a directory named after the project.

Conventions that span the file:

- Functions are namespaced by prefix: `path_*` for argument validation, `env_*` for collection.
- Each stage has its own small error struct carrying a `path` and a `message`, converted by hand with `map_err`. There is no `anyhow`/`thiserror`.
- Results go to stdout with `println!`; diagnostics go through the `log` macros (stderr, via `env_logger`). The default level is **warn**, not the `clap-verbosity-flag` default of error.

## Behaviour that must be preserved

- **Symlinks are never followed.** The walk uses `DirEntry::file_type`, not `Path::is_file`/`is_dir` (which follow links and previously caused cyclic links to be archived dozens of times). Each skipped symlink is logged with `warn!`, which is why the default log level is warn.
- **The archive is created with mode `0400`, at creation time** (`env_create_tar_file`), so it is never briefly readable by other users. It uses `create_new`, so an existing archive is never overwritten.
- **The code must keep compiling on Windows.** Unix-only APIs (the mode bits) are behind `#[cfg(unix)]`; on Windows the archive keeps default permissions.
- The zstd encoder is finished explicitly after `tar::Builder::into_inner`; dropping it instead leaves a truncated stream.

## Tests

`tests/cli.rs` holds end-to-end tests that run the compiled binary (`CARGO_BIN_EXE_env-collector`) against real directory trees built under `CARGO_TARGET_TMPDIR`, then inspect stderr, the archive's entries, name and permission bits. The file is `#![cfg(unix)]` because it creates symlinks and checks modes. The `Workspace` helper gives each test its own `proj/` (the target) and `out/` (the working directory the archive lands in); name the workspace after the test so parallel tests do not collide.

Because there is no library target, integration tests can only observe behaviour through the binary. Unit tests for the pure helpers (`env_is_env_filename`, `env_is_dirname_ignored`) would go in a `#[cfg(test)]` module inside `src/main.rs`; none exist yet.

## Known open edges

Verified against the current code and not yet addressed:

- Two targets with the same directory name in one run collide on the archive name (same name, same second) and the second fails with "File exists".
- A target such as `..` fails at the archive stage ("Root path has no valid file name") because paths are made absolute but not canonicalized.
- File name matching is prefix/suffix based and over-matches (`.envoy.json`, `applicationyml`, `maintenance.dart`).
- A directory with no matches still produces an empty archive and exit code 0.
- The summary line says "env vars" but the number is a count of files.
