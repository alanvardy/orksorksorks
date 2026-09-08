# Implementation Plan

## Overview

When the config file can't be read, embed the resolved path and how it was resolved into the `"io"` error's `message` text. No `Error` struct fields, no JSON schema change, `source` stays `"io"`.

---

## Phase 1: `ConfigPathSource` type + resolution-source tracking

### Changes

#### 1. Add `ConfigPathSource` enum and `Display` impl
**File**: `src/config_dir.rs`
**Action**: modify

Add the enum after the `FILE_NAME` constant (before `resolve_config_dir`):

```rust
/// How the config file path was resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigPathSource {
    /// User passed `--config <path>`.
    ExplicitFlag,
    /// Resolved from the `XDG_CONFIG_HOME` environment variable.
    XdgConfigHome,
    /// Resolved from `$HOME/.config`.
    HomeDotConfig,
    /// Resolved from `%APPDATA%` (Windows).
    AppData,
}

impl std::fmt::Display for ConfigPathSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExplicitFlag => write!(f, "specified via --config"),
            Self::XdgConfigHome => write!(f, "resolved from XDG_CONFIG_HOME"),
            Self::HomeDotConfig => write!(f, "resolved from HOME/.config"),
            Self::AppData => write!(f, "resolved from %APPDATA%"),
        }
    }
}
```

#### 2. Update `resolve_config_dir` to return `(PathBuf, ConfigPathSource)`
**File**: `src/config_dir.rs`
**Action**: modify

Change signature from `-> Result<PathBuf, Error>` to `-> Result<(PathBuf, ConfigPathSource), Error>` and tag each return arm:

```rust
fn resolve_config_dir() -> Result<(PathBuf, ConfigPathSource), Error> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        let dir = PathBuf::from(xdg);
        if dir.is_absolute() {
            return Ok((dir, ConfigPathSource::XdgConfigHome));
        }
    }

    if cfg!(windows) {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return Ok((PathBuf::from(appdata), ConfigPathSource::AppData));
        }
    } else if let Some(home) = std::env::var_os("HOME") {
        return Ok((PathBuf::from(home).join(".config"), ConfigPathSource::HomeDotConfig));
    }

    Err(Error::new(
        "config-dir",
        "could not determine a config directory: set XDG_CONFIG_HOME or HOME",
    ))
}
```

#### 3. Update `config_file_path` to return `(PathBuf, ConfigPathSource)`
**File**: `src/config_dir.rs`
**Action**: modify

Change signature from `-> Result<PathBuf, Error>` to `-> Result<(PathBuf, ConfigPathSource), Error>`:

```rust
pub fn config_file_path(explicit: Option<&Path>) -> Result<(PathBuf, ConfigPathSource), Error> {
    match explicit {
        Some(path) => Ok((path.to_path_buf(), ConfigPathSource::ExplicitFlag)),
        None => resolve_config_dir().map(|(dir, source)| (dir.join(FILE_NAME), source)),
    }
}
```

#### 4. Update `select_command` to destructure the new tuple
**File**: `src/commands/mod.rs`
**Action**: modify

Two lines in `select_command` (lines 69-70 and 75-76):

```rust
// Init arm (line 69-70):
let (path, _) = crate::config_dir::config_file_path(config.as_deref())?;
init_command(&path)

// Step arm (line 75-76):
let (path, _) = crate::config_dir::config_file_path(config.as_deref())?;
step_command(&path)
```

#### 5. Update `config_dir.rs` unit tests to assert source
**File**: `src/config_dir.rs` (in `#[cfg(test)] mod tests`)
**Action**: modify

Update each test to destructure the tuple and assert the source variant:

- `absolute_xdg_is_used` — add `let (path, source) = config_file_path(None).unwrap();` and `assert_eq!(source, ConfigPathSource::XdgConfigHome);`
- `unset_xdg_falls_back_to_home_dot_config` — assert `ConfigPathSource::HomeDotConfig`
- `empty_xdg_falls_back_to_home` — assert `ConfigPathSource::HomeDotConfig`
- `relative_xdg_falls_back_to_home` — assert `ConfigPathSource::HomeDotConfig`
- `explicit_path_passthrough` — `let (path, source) = config_file_path(Some(p)).unwrap();` and `assert_eq!(source, ConfigPathSource::ExplicitFlag);`
- `unresolved_home_yields_config_dir_error` — unchanged (still `unwrap_err()`)

Add a new test for `Display`:

```rust
#[test]
fn config_path_source_display_pins_phrases() {
    assert_eq!(ConfigPathSource::ExplicitFlag.to_string(), "specified via --config");
    assert_eq!(ConfigPathSource::XdgConfigHome.to_string(), "resolved from XDG_CONFIG_HOME");
    assert_eq!(ConfigPathSource::HomeDotConfig.to_string(), "resolved from HOME/.config");
    assert_eq!(ConfigPathSource::AppData.to_string(), "resolved from %APPDATA%");
}
```

### Verification
#### Automated
- [x] `cargo nextest run config_dir::` passes
- [x] `scripts/test.sh` passes

#### Manual
- [ ] `cargo build` compiles cleanly

---

## Phase 2: Enriched `"io"` error in `read_config`

### Changes

#### 1. Update `read_config` signature and catch the io error
**File**: `src/config.rs`
**Action**: modify

Add `use crate::config_dir::ConfigPathSource;` at the top of the file (after the existing `use crate::errors::Error;`).

Change `read_config` from:

```rust
pub fn read_config(path: &std::path::Path) -> Result<Config, Error> {
    let contents = std::fs::read_to_string(path)?;
    let config = toml::from_str(&contents)?;
    Ok(config)
}
```

To:

```rust
pub fn read_config(path: &std::path::Path, source: ConfigPathSource) -> Result<Config, Error> {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            return Err(Error::new(
                "io",
                &format!(
                    "could not read config file at {} ({}): {}",
                    path.display(),
                    source,
                    e
                ),
            ));
        }
    };
    let config = toml::from_str(&contents)?;
    Ok(config)
}
```

#### 2. Update `step_command` to accept and thread `ConfigPathSource`
**File**: `src/commands/mod.rs`
**Action**: modify

Change signature from `fn step_command(path: &std::path::Path) -> Result<String, Error>` to:

```rust
fn step_command(path: &std::path::Path, source: crate::config_dir::ConfigPathSource) -> Result<String, Error> {
    let cfg = crate::config::read_config(path, source)?;
    let cwd = std::env::current_dir()?;
    let artifact_dir = artifact_dir_path(&cwd, &git::current_branch()?);
    determine_step(&cfg, &artifact_dir)
}
```

#### 3. Update `select_command` step arm to pass source through
**File**: `src/commands/mod.rs`
**Action**: modify

Change the step arm destructure from `(path, _)` to `(path, source)` and pass source to `step_command`:

```rust
Commands::Step { config } => {
    let (path, source) = crate::config_dir::config_file_path(config.as_deref())?;
    step_command(&path, source)
}
```

The init arm stays `let (path, _) = …` (init never reads config).

#### 4. Update `read_config` unit tests
**File**: `src/config.rs` (in `#[cfg(test)] mod tests`)
**Action**: modify

- `read_config_loads_steps_from_disk` — add `ConfigPathSource::ExplicitFlag` as second arg to `read_config(&path, …)`.
- `read_config_missing_file_tags_io` — add source, extend assertion:

```rust
#[test]
fn read_config_missing_file_tags_io() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nope.toml");
    let err = read_config(&path, ConfigPathSource::ExplicitFlag).unwrap_err();
    assert_eq!(err.source, "io");
    assert!(
        err.message.contains(&path.display().to_string()),
        "message should contain path: {}",
        err.message
    );
    assert!(
        err.message.contains("specified via --config"),
        "message should contain resolution: {}",
        err.message
    );
}
```

- `read_config_malformed_toml_tags_toml_de` — add `ConfigPathSource::XdgConfigHome` as second arg (source irrelevant for TOML error path, but the signature requires it).

- Add a new test for XdgConfigHome source in the message:

```rust
#[test]
fn read_config_io_error_contains_xdg_source() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("orksorksorks.toml");
    let err = read_config(&path, ConfigPathSource::XdgConfigHome).unwrap_err();
    assert_eq!(err.source, "io");
    assert!(err.message.contains("resolved from XDG_CONFIG_HOME"));
    assert!(err.message.contains(&path.display().to_string()));
}
```

#### 5. Update `select_command_routes_step` test
**File**: `src/commands/mod.rs` (line ~458-471)
**Action**: modify

The test uses `config: Some(PathBuf::from("definitely-missing-config-file.toml"))` — this now goes through `ExplicitFlag`. The `assert_eq!(err.source, "io")` assertion stays green unchanged. No source assertion needed per the structure (the test only asserts routing dispatched to `step_command`).

### Verification
#### Automated
- [x] `cargo nextest run config::` passes
- [x] `cargo nextest run commands::` passes
- [x] `scripts/test.sh` passes

#### Manual
- [ ] `cargo run -- step --config /tmp/nope.toml` stderr contains `/tmp/nope.toml` and `"specified via --config"`
- [ ] `XDG_CONFIG_HOME=/bad/dir cargo run -- step` stderr contains `/bad/dir/orksorksorks/orksorksorks.toml` and `"resolved from XDG_CONFIG_HOME"`

---

## Phase 3: End-to-end output verification

### Changes

No source changes — this phase is tests only.

#### 1. Add integration tests for path-in-error output
**File**: `tests/step.rs`
**Action**: modify

Add three new tests at the end of the file (after the existing `step_without_flag_ignores_cwd_config` test):

```rust
/// Text-mode error with explicit --config includes the path and resolution.
#[test]
fn step_with_missing_config_shows_path_in_stderr() {
    let dir = init_git_repo();
    let missing = dir.path().join("nope.toml");

    let output = Command::cargo_bin("orksorksorks")
        .unwrap()
        .args(["step", "--config", "nope.toml"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&missing.display().to_string()),
        "stderr should contain path: {stderr}"
    );
    assert!(
        stderr.contains("specified via --config"),
        "stderr should contain resolution: {stderr}"
    );
    assert!(!stderr.contains('\x1b'), "stderr should have no ANSI: {stderr}");
}

/// Text-mode error with XDG_CONFIG_HOME includes the resolved path and resolution.
#[test]
fn step_with_missing_xdg_config_shows_path_in_stderr() {
    let dir = init_git_repo();
    let xdg = tempfile::tempdir().unwrap();
    let config_dir = xdg.path().join("cfg");
    std::fs::create_dir_all(&config_dir).unwrap();
    // Config dir exists but no orksorksorks.toml inside it

    let output = Command::cargo_bin("orksorksorks")
        .unwrap()
        .arg("step")
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let expected_path = config_dir.join("orksorksorks.toml");
    assert!(
        stderr.contains(&expected_path.display().to_string()),
        "stderr should contain path: {stderr}"
    );
    assert!(
        stderr.contains("resolved from XDG_CONFIG_HOME"),
        "stderr should contain resolution: {stderr}"
    );
    assert!(!stderr.contains('\x1b'), "stderr should have no ANSI: {stderr}");
}

/// JSON error output includes the path in message, no ANSI, and no envelope shape change.
#[test]
fn step_with_missing_config_json_contains_path_in_message() {
    let dir = init_git_repo();

    let output = Command::cargo_bin("orksorksorks")
        .unwrap()
        .args(["step", "--config", "nope.toml", "-j"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let msg = json["error"]["message"].as_str().unwrap();
    let src = json["error"]["source"].as_str().unwrap();

    assert_eq!(src, "io");
    assert!(
        msg.contains("nope.toml"),
        "message should contain path: {msg}"
    );
    assert!(
        msg.contains("specified via --config"),
        "message should contain resolution: {msg}"
    );
    assert!(!stdout.contains('\x1b'), "stdout should have no ANSI: {stdout}");
}
```

### Verification
#### Automated
- [x] `cargo nextest run` (full test suite) passes
- [x] `scripts/test.sh` passes (final gate — fmt, check, clippy `-D warnings`, nextest, forbidden-strings)

#### Manual
- [ ] `cargo run -- step --config /tmp/nope.toml` — stderr shows path + "specified via --config"
- [ ] `XDG_CONFIG_HOME=/bad/dir cargo run -- step` — stderr shows path + "resolved from XDG_CONFIG_HOME"
- [ ] `cargo run -- step --config /tmp/nope.toml -j` — JSON stdout has path in `error.message`, no `\x1b`

---

## Testing Checkpoints

Resume points if context resets — the listed command must be green before advancing:

1. **After Stage 1**: `scripts/test.sh` — `ConfigPathSource` exists, resolution tests assert correct source per branch.
2. **After Stage 2**: `scripts/test.sh` — `read_config` error `message` contains path + resolution phrase, `source == "io"`.
3. **After Stage 3**: `scripts/test.sh` — integration stderr/JSON assertions pass, no ANSI, no JSON-shape change.