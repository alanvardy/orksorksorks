# Implementation Plan

## Overview

Relocate the `init` write from the CWD to a per-user config directory: `--config/-c <PATH>` passes an explicit path through unchanged; otherwise the target is `<config dir>/orksorksorks.toml` where the config dir resolves from `$XDG_CONFIG_HOME` (absolute only) → `$HOME/.config` (Unix/macOS) → `%APPDATA%` (Windows). `init` auto-creates the parent, reports the resolved path in the success message, and fails with `source: "config-dir"` when no directory is resolvable. `src/main.rs`'s error envelope, JSON shape, and exit codes stay stable; no new dependencies.

> **Deviation from `structure.md`:** Layer 4 lists `tests/json_output.rs` as "unchanged — regression", but that file actually breaks under Design Decisons 7 (new success message) and 8 (`XDG_CONFIG_HOME` test override): its two message assertions pin the old bare-filename string, and both tests spawn `init` with inherited env, so they would truncate-write the developer's real `$HOME/.config/orksorksorks.toml`. This plan adds the necessary fix to `json_output.rs` (tempdir-scoped `XDG_CONFIG_HOME` + updated message assertion). All other phases follow `structure.md` unchanged.

---

## Phase 1: Config-directory resolution

### Changes

#### 1. New resolution module
**File**: `src/config_dir.rs`
**Action**: create

```rust
//! Per-user config directory resolution for the `init` write target.

use std::path::{Path, PathBuf};

use crate::errors::Error;

/// The config filename written by `init`, joined under the resolved directory.
const FILE_NAME: &str = "orksorksorks.toml";

/// Resolve the full target path for `init`.
///
/// `Some(path)` passes an explicit override through unchanged. `None` falls
/// back to `<config dir>/orksorksorks.toml`, where the config dir is resolved
/// from `$XDG_CONFIG_HOME` (absolute only), then `$HOME/.config` (Unix) or
/// `%APPDATA%` (Windows).
pub fn config_file_path(explicit: Option<&Path>) -> Result<PathBuf, Error> {
    match explicit {
        Some(path) => Ok(path.to_path_buf()),
        None => resolve_config_dir().map(|dir| dir.join(FILE_NAME)),
    }
}

/// Resolve the per-user config directory from the environment.
fn resolve_config_dir() -> Result<PathBuf, Error> {
    // XDG wins on every platform, but only when set to an absolute path.
    // Unset, empty, and relative values all mean "fall back" (XDG spec).
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        let dir = PathBuf::from(xdg);
        if dir.is_absolute() {
            return Ok(dir);
        }
    }

    if cfg!(windows) {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return Ok(PathBuf::from(appdata));
        }
    } else if let Some(home) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(home).join(".config"));
    }

    Err(Error::new(
        "config-dir",
        "could not determine a config directory: set XDG_CONFIG_HOME or HOME",
    ))
}
```

**Tests** — appended in-module `#[cfg(test)] mod tests` (uses `super::*`, so it reaches the private `resolve_config_dir` and the `FILE_NAME` const). `std::env::set_var`/`remove_var` are `unsafe` on rustc 1.98; safe because nextest runs each test in its own process. No `#[cfg(target_os = ...)]` gating, per `structure.md` (no Windows CI).

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_xdg_is_used() {
        unsafe { std::env::set_var("XDG_CONFIG_HOME", "/tmp/ork-cfg") }
        let path = config_file_path(None).unwrap();
        assert_eq!(path, std::path::Path::new("/tmp/ork-cfg").join(FILE_NAME));
        unsafe { std::env::remove_var("XDG_CONFIG_HOME") }
    }

    #[test]
    fn unset_xdg_falls_back_to_home_dot_config() {
        unsafe {
            std::env::remove_var("XDG_CONFIG_HOME");
            std::env::set_var("HOME", "/home/ork");
        }
        let path = config_file_path(None).unwrap();
        assert_eq!(path, std::path::Path::new("/home/ork/.config").join(FILE_NAME));
    }

    #[test]
    fn empty_xdg_falls_back_to_home() {
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", "");
            std::env::set_var("HOME", "/home/ork");
        }
        let path = config_file_path(None).unwrap();
        assert_eq!(path, std::path::Path::new("/home/ork/.config").join(FILE_NAME));
    }

    #[test]
    fn relative_xdg_falls_back_to_home() {
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", "relative/dir");
            std::env::set_var("HOME", "/home/ork");
        }
        let path = config_file_path(None).unwrap();
        assert_eq!(path, std::path::Path::new("/home/ork/.config").join(FILE_NAME));
    }

    #[test]
    fn explicit_path_passthrough() {
        let p = std::path::Path::new("/custom/override.toml");
        assert_eq!(config_file_path(Some(p)).unwrap(), p.to_path_buf());
    }

    #[test]
    fn unresolved_home_yields_config_dir_error() {
        unsafe {
            std::env::remove_var("HOME");
            std::env::remove_var("XDG_CONFIG_HOME");
        }
        let err = config_file_path(None).unwrap_err();
        assert_eq!(err.source, "config-dir");
    }
}
```

#### 2. Register the module
**File**: `src/main.rs`
**Action**: modify — add `mod config_dir;` in the module block (alphabetical order):

```rust
mod commands;
mod config;
mod config_dir;
mod errors;
mod format;
```

No other `src/main.rs` change: the error envelope, JSON rendering, and exit paths are untouched.

### Verification
#### Automated
- [x] `cargo nextest run config_dir` passes (the six unit tests above)
- [x] `./scripts/test.sh` passes (fmt → check → clippy `-D warnings` → nextest → forbidden-string gate)

#### Manual
- [ ] `cargo test --bin orksorksorks config_dir -- --nocapture` shows all six tests green (sanity, optional — nextest is the gate)

---

## Phase 2: CLI surface — `--config/-c` flag

### Changes

#### 1. `Init` gains a config field; routing arm updated (value ignored yet)
**File**: `src/commands/mod.rs`
**Action**: modify

Add `use std::path::PathBuf;` to the top imports, then change the enum and match arm:

```rust
/// Top-level subcommands.
#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// (i) Create a new orksorksorks.toml file
    Init {
        /// Write to this path instead of the default config directory
        #[arg(short = 'c', long, value_parser = clap::value_parser!(PathBuf))]
        config: Option<PathBuf>,
    },
}

/// Route a parsed CLI to its handler and return a success message or error.
pub fn select_command(cli: &Cli) -> Result<String, Error> {
    match &cli.command {
        Commands::Init { config: _config } => init_command(),
    }
}
```

`init_command()` is unchanged in this phase (still writes to CWD). The `_config` binding suppresses the unused-variable warning under `-D warnings`; it is renamed to `config` and consumed in Phase 3.

#### 2. Update/add parse tests
**File**: `src/commands/mod.rs` (in-module `mod tests`)
**Action**: modify — update the existing `select_command_routes_init` construction for the new variant shape, and add three parse tests.

```rust
#[test]
fn select_command_routes_init() {
    let cli = Cli {
        json: false,
        command: Commands::Init { config: None },
    };
    let result = select_command(&cli).unwrap();
    // Under cfg!(test) color is stripped
    assert_eq!(result, "✓ Created orksorksorks.toml");
}

#[test]
fn cli_try_parse_accepts_config_short() {
    use clap::Parser;
    let cli = Cli::try_parse_from(["orksorksorks", "init", "-c", "/tmp/x.toml"]).unwrap();
    match cli.command {
        Commands::Init { config } => {
            assert_eq!(config, Some(PathBuf::from("/tmp/x.toml")))
        }
    }
}

#[test]
fn cli_try_parse_accepts_config_long() {
    use clap::Parser;
    let cli = Cli::try_parse_from(["orksorksorks", "init", "--config", "/tmp/x.toml"]).unwrap();
    match cli.command {
        Commands::Init { config } => {
            assert_eq!(config, Some(PathBuf::from("/tmp/x.toml")))
        }
    }
}

#[test]
fn cli_try_parse_init_without_config() {
    use clap::Parser;
    let cli = Cli::try_parse_from(["orksorksorks", "init"]).unwrap();
    match cli.command {
        Commands::Init { config } => assert_eq!(config, None),
    }
}
```

Existing `cli_try_parse_rejects_no_subcommand`, `cli_try_parse_accepts_init`, and `cli_command_debug_assert` are unchanged and still pass.

### Verification
#### Automated
- [x] `cargo nextest run cli_try_parse` passes
- [x] `./scripts/test.sh` passes (Phase 1 tests still green beneath it)

#### Manual
- [ ] `cargo run -- init -c /tmp/x.toml` still succeeds (flag parsed; write still lands in CWD this phase)
- [ ] `cargo run -- init --help` shows the `-c, --config <CONFIG>` option

---

## Phase 3: Handler — resolve, create parent, report resolved path

### Changes

#### 1. `select_command` resolves and routes the concrete path
**File**: `src/commands/mod.rs`
**Action**: modify

```rust
pub fn select_command(cli: &Cli) -> Result<String, Error> {
    match &cli.command {
        Commands::Init { config } => {
            let path = crate::config_dir::config_file_path(config.as_deref())?;
            init_command(&path)
        }
    }
}
```

#### 2. `init_command` takes the resolved path, creates the parent, reports the path
**File**: `src/commands/mod.rs`
**Action**: modify

```rust
/// Create a new `orksorksorks.toml` file with default configuration.
fn init_command(path: &std::path::Path) -> Result<String, Error> {
    use std::io::Write;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let config = crate::config::Config::default();
    let toml_str = toml::to_string(&config)?;
    let mut file = std::fs::File::create(path)?;
    file.write_all(toml_str.as_bytes())?;
    file.flush()?;
    file.sync_all()?;
    Ok(crate::format::green_string(&format!(
        "✓ Created {}",
        path.display()
    )))
}
```

`create_dir_all` and `File::create` both map through the existing `From<std::io::Error>` (`source: "io"`) — no new error plumbing. Overwrite semantics (`File::create` O_TRUNC, no existence check) are unchanged.

#### 3. Rewrite the in-process routing test to a tempdir (no runner-CWD write)
**File**: `src/commands/mod.rs` (in-module `mod tests`)
**Action**: modify — replace `select_command_routes_init` written in Phase 2.

```rust
#[test]
fn select_command_routes_init() {
    let temp = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", temp.path());
    }
    let cli = Cli {
        json: false,
        command: Commands::Init { config: None },
    };
    let result = select_command(&cli).unwrap();
    let expected_path = temp.path().join("orksorksorks.toml");
    assert!(expected_path.exists(), "config file not created");
    // Under cfg!(test) color is stripped
    assert_eq!(result, format!("✓ Created {}", expected_path.display()));
}
```

#### 4. Repoint the four CWD-coupled integration tests
**File**: `tests/init_creates_file.rs`
**Action**: modify — set `XDG_CONFIG_HOME` to a fresh subdirectory of `tempdir()` instead of redirecting the child CWD; drop all `current_dir` calls.

```rust
#[test]
fn init_creates_orksorksorks_toml_with_default_content() {
    let temp = tempfile::tempdir().unwrap();
    let config_dir = temp.path().join("config");
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.env("XDG_CONFIG_HOME", &config_dir);

    cmd.arg("init").assert().success();

    let config_path = config_dir.join("orksorksorks.toml");
    assert!(config_path.exists(), "config file not created");

    let content = fs::read_to_string(&config_path).unwrap();
    // The crate is binary-only (no lib target), so integration tests can't
    // import `Config`; assert the exact serialization of the default value.
    let expected = "version = \"0.1.0\"\n";
    assert_eq!(content, expected);
}

#[test]
fn init_returns_success_message() {
    let temp = tempfile::tempdir().unwrap();
    let config_dir = temp.path().join("config");
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.env("XDG_CONFIG_HOME", &config_dir);

    let resolved = config_dir.join("orksorksorks.toml");
    cmd.arg("init")
        .assert()
        .success()
        // Under cfg!(test) the child still applies color, so assert the path
        // substring that survives the ANSI-free stdout.
        .stdout(predicate::str::contains(format!(
            "✓ Created {}",
            resolved.display()
        )));
}

#[test]
fn init_in_readonly_dir_returns_error_exit_code() {
    let temp = tempfile::tempdir().unwrap();
    let config_dir = temp.path().join("config");
    fs::create_dir(&config_dir).unwrap();
    let mut perms = fs::metadata(&config_dir).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&config_dir, perms).unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.env("XDG_CONFIG_HOME", &config_dir);

    cmd.arg("init").assert().failure();

    // Restore writable so tempfile can clean up
    let mut perms = fs::metadata(&config_dir).unwrap().permissions();
    perms.set_readonly(false);
    fs::set_permissions(&config_dir, perms).unwrap();
}

#[test]
fn init_json_error_in_readonly_dir_shows_source_io() {
    let temp = tempfile::tempdir().unwrap();
    let config_dir = temp.path().join("config");
    fs::create_dir(&config_dir).unwrap();
    let mut perms = fs::metadata(&config_dir).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&config_dir, perms).unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.env("XDG_CONFIG_HOME", &config_dir);

    let output = cmd.args(["init", "-j"]).output().unwrap();
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""source":"io""#), "stdout: {stdout}");

    // Restore
    let mut perms = fs::metadata(&config_dir).unwrap().permissions();
    perms.set_readonly(false);
    fs::set_permissions(&config_dir, perms).unwrap();
}
```

Note: `create_dir_all` on an already-existing readonly directory returns Ok (AlreadyExists is not an error), so the readonly failure still surfaces at `File::create` as `PermissionDenied` → `"io"`. The default-content test doubles as proof that `create_dir_all` auto-creates `config_dir`.

### Verification
#### Automated
- [x] `cargo nextest run init_` passes (integration tests + `select_command_routes_init`)
- [x] `./scripts/test.sh` passes

#### Manual
- [ ] `XDG_CONFIG_HOME=/tmp/x cargo run -- init` prints `✓ Created /tmp/x/orksorksorks.toml` and creates `/tmp/x/` (auto-create)
- [ ] `cat /tmp/x/orksorksorks.toml` shows `version = "0.1.0"`

---

## Phase 4: Integration hardening — override, unresolvable home, regression

### Changes

#### 1. New override + unresolvable-home integration tests
**File**: `tests/init_creates_file.rs`
**Action**: modify — add two tests.

```rust
#[test]
fn init_with_config_flag_writes_to_given_path() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("custom.toml");
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    // Hostile default env — must be ignored when --config is passed.
    let hostile = temp.path().join("never-used");
    cmd.env("XDG_CONFIG_HOME", &hostile);
    cmd.arg("init").arg("--config").arg(&target);

    cmd.assert().success();

    // File lands exactly at the override path, default dir not consulted.
    assert!(target.exists(), "override target not created");
    assert!(!hostile.exists(), "default dir should not be created");
    let content = fs::read_to_string(&target).unwrap();
    assert_eq!(content, "version = \"0.1.0\"\n");
}

#[test]
fn init_without_home_fails_with_config_dir_tag() {
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.env_remove("HOME");
    cmd.env_remove("XDG_CONFIG_HOME");

    let output = cmd.args(["init", "-j"]).output().unwrap();
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(r#""source":"config-dir""#),
        "stdout: {stdout}"
    );
}
```

#### 2. Fix `json_output.rs` (deviation — was "unchanged" in the outline)
**File**: `tests/json_output.rs`
**Action**: modify — set a tempdir-scoped `XDG_CONFIG_HOME` (so the tests no longer truncate-write the real `$HOME/.config/orksorksorks.toml`) and update both message assertions to the resolved-path form. Also runs each command once (the original `.output()` + `.assert()` double-ran the binary).

```rust
use assert_cmd::Command;

#[test]
fn init_json_returns_valid_json_with_data_field() {
    let temp = tempfile::tempdir().unwrap();
    let config_dir = temp.path().join("config");
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.env("XDG_CONFIG_HOME", &config_dir);

    let output = cmd.args(["init", "-j"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""data""#), "stdout: {stdout}");
    let resolved = config_dir.join("orksorksorks.toml");
    assert!(
        stdout.contains(&format!("✓ Created {}", resolved.display())),
        "stdout: {stdout}"
    );
    // Valid JSON
    let _: serde_json::Value = serde_json::from_str(&stdout).unwrap();
}

#[test]
fn init_no_json_prints_plain_text() {
    let temp = tempfile::tempdir().unwrap();
    let config_dir = temp.path().join("config");
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.env("XDG_CONFIG_HOME", &config_dir);

    let output = cmd.arg("init").output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let resolved = config_dir.join("orksorksorks.toml");
    assert!(
        stdout.contains(&format!("✓ Created {}", resolved.display())),
        "stdout: {stdout}"
    );
    assert!(
        !stdout.contains('\x1b'),
        "stdout should have no ANSI: {stdout}"
    );
}
```

### Verification
#### Automated
- [ ] `cargo nextest run init_with` passes (override + config-dir-tag tests)
- [ ] `cargo nextest run json_output` passes (JSON validity + ANSI-free regression)
- [ ] `./scripts/test.sh` passes end-to-end

#### Manual
- [ ] `cargo run -- init --config /tmp/foo.toml` then `cat /tmp/foo.toml` shows `version = "0.1.0"`
- [ ] `env -u HOME -u XDG_CONFIG_HOME cargo run -- init -j` prints a JSON error containing `"source":"config-dir"` and exits non-zero

---

## Cross-cutting notes

- **No `src/errors.rs` change**: `Error::new("config-dir", …)` uses the existing constructor; both render paths (text `Display`, JSON `output_json`) handle the new tag automatically. The tag is only *produced* in `src/config_dir.rs` and *asserted* in Phase 4.
- **Only cross-layer coupling** is the success-message string `✓ Created <path>` (produced in Phase 3, asserted in Phases 3–4). If it changes, update all four assertions in the same change.
- **Committed repo-root `orksorksorks.toml`** becomes orphaned by the default (nothing reads it; it is still exercised only by `tests/json_output.rs`'s overwrite path, which now redirects to a tempdir). Out of scope here — flagged as a follow-up decision (delete vs. keep).
- **No new dependencies**: all resolution is `std::env` + `std::fs`; `clap::value_parser!(PathBuf)` uses the existing clap derive feature.