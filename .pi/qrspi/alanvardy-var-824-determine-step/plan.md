# Implementation Plan

## Overview

Add a `step` subcommand that reads a `[[steps]]` array-of-tables from TOML config, derives the artifact directory from cwd + git branch, and returns the name of the last (reverse-iterated) step whose `trigger_artifact` exists — failing through the central `Error` type, output layer (`src/main.rs`) untouched.

Phases mirror `structure.md` Stages 1–5. No new dependencies; no change to `src/main.rs`, `Display`, bell/exit-code policy, or `init` bytes.

---

## Phase 1: Error foundation — `From<toml::de::Error>`

### Changes

#### 1. Deserialization error mapping
**File**: `src/errors.rs`
**Action**: modify

Add a third `From` impl, immediately after the existing `From<toml::ser::Error>` block (mirrors it exactly):

```rust
impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Self {
            source: "toml::de".to_string(),
            message: e.to_string(),
        }
    }
}
```

#### 2. Unit test
**File**: `src/errors.rs` (inside the `#[cfg(test)] mod tests` block)
**Action**: modify

```rust
#[test]
fn from_toml_de_error_tags_toml_de() {
    #[derive(serde::Deserialize)]
    struct NeedsAField {
        required: String,
    }
    let toml_err = toml::from_str::<NeedsAField>("missing = \"nope\"").unwrap_err();
    let err = Error::from(toml_err);
    assert_eq!(err.source, "toml::de");
}
```

### Verification
#### Automated
- [x] `cargo test from_toml_de_error` passes
- [x] `./scripts/test.sh` green

#### Manual
- [ ] Confirm no change to existing `From<io::Error>` / `From<toml::ser::Error>` tests (`from_io_error_tags_io`, `from_toml_ser_error_tags_toml_ser`).

---

## Phase 2: Config schema — `steps: Vec<Step>`

### Changes

#### 1. `Step` struct + `steps` field + `Default`
**File**: `src/config.rs`
**Action**: modify

Add `Step` and extend `Config` (derives match `Config`'s existing set):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Step {
    /// Human-readable name returned by the `step` subcommand.
    pub name: String,
    /// Filename whose presence at the artifact directory marks this step.
    pub trigger_artifact: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    /// Config format version.
    pub version: String,
    /// Ordered steps; the current step is the last whose artifact exists.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<Step>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: "0.1.0".to_string(),
            steps: Vec::new(),
        }
    }
}
```

The `#[serde(default, skip_serializing_if = "Vec::is_empty")]` is load-bearing: a `Config::default()` still serializes to exactly `version = "0.1.0"\n`, keeping `tests/init_creates_file.rs:22-25` green (no test edit needed).

#### 2. Unit tests
**File**: `src/config.rs` (inside the `#[cfg(test)] mod tests` block)
**Action**: modify

```rust
#[test]
fn steps_round_trip() {
    let config = Config {
        version: "0.1.0".to_string(),
        steps: vec![
            Step { name: "one".to_string(), trigger_artifact: "a.txt".to_string() },
            Step { name: "two".to_string(), trigger_artifact: "b.txt".to_string() },
        ],
    };
    let serialized = toml::to_string(&config).unwrap();
    let deserialized: Config = toml::from_str(&serialized).unwrap();
    assert_eq!(config, deserialized);
}

#[test]
fn missing_steps_deserializes_to_empty_vec() {
    let config: Config = toml::from_str("version = \"0.1.0\"\n").unwrap();
    assert!(config.steps.is_empty());
}
```

### Verification
#### Automated
- [x] `cargo test steps_round_trip` and `cargo test missing_steps_deserializes_to_empty_vec` pass
- [x] `cargo test default_config_serializes_to_expected_toml` still passes (pins no-`steps` output)
- [x] `./scripts/test.sh` green — `tests/init_creates_file.rs:22-25` (exact `version = "0.1.0"\n` bytes) untouched

#### Manual
- [ ] `cargo run -- init` in a scratch dir writes `version = "0.1.0"\n` only (no `[[steps]]` line in the file).

---

## Phase 3: Config data-access — `read_config`

### Changes

#### 1. Disk read + deserialize
**File**: `src/config.rs`
**Action**: modify

Add `use crate::errors::Error;` to the top imports, then:

```rust
pub fn read_config(path: &std::path::Path) -> Result<Config, Error> {
    let contents = std::fs::read_to_string(path)?;
    let config = toml::from_str(&contents)?;
    Ok(config)
}
```

`read_to_string` errors map to `"io"` via the existing `From<std::io::Error>`; `toml::from_str` errors map to `"toml::de"` via Phase 1's new impl.

#### 2. Unit tests
**File**: `src/config.rs` (inside `#[cfg(test)] mod tests`)
**Action**: modify

```rust
#[test]
fn read_config_loads_steps_from_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("orksorksorks.toml");
    std::fs::write(
        &path,
        "version = \"0.1.0\"\n[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\n",
    )
    .unwrap();
    let config = read_config(&path).unwrap();
    assert_eq!(config.steps.len(), 1);
    assert_eq!(config.steps[0].name, "one");
    assert_eq!(config.steps[0].trigger_artifact, "a.txt");
}

#[test]
fn read_config_missing_file_tags_io() {
    let dir = tempfile::tempdir().unwrap();
    let err = read_config(&dir.path().join("nope.toml")).unwrap_err();
    assert_eq!(err.source, "io");
}

#[test]
fn read_config_malformed_toml_tags_toml_de() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("orksorksorks.toml");
    // Missing the required `name` field on the step
    std::fs::write(&path, "version = \"0.1.0\"\n[[steps]]\ntrigger_artifact = \"a.txt\"\n").unwrap();
    let err = read_config(&path).unwrap_err();
    assert_eq!(err.source, "toml::de");
}
```

### Verification
#### Automated
- [x] `cargo test read_config` passes (all three tests)
- [x] `./scripts/test.sh` green

#### Manual
- [ ] n/a — fully covered by unit tests and the Phase 5 end-to-end path.

---

## Phase 4: Determine-step logic — `determine_step`

### Changes

#### 1. Business rule
**File**: `src/commands/mod.rs`
**Action**: modify

Add `use crate::config::Config;` to the top imports, then:

```rust
/// Reverse-iterate steps and return the name of the first whose
/// trigger artifact exists at `artifact_dir`. `artifact_dir` must end in a
/// trailing slash — the same string-composition convention as `artifact_dir_path`.
fn determine_step(config: &Config, artifact_dir: &str) -> Result<String, Error> {
    for step in config.steps.iter().rev() {
        let path = format!("{artifact_dir}{}", step.trigger_artifact);
        if std::path::Path::new(&path).try_exists()? {
            return Ok(step.name.clone());
        }
    }
    Err(Error::new(
        "step",
        &format!("{artifact_dir}: no trigger artifact matched"),
    ))
}
```

`try_exists()?` returns `Ok(false)` for absent (keep looping) and `Err(io)` for permission-denied (rides `From<std::io::Error>` → `"io"`).

#### 2. Unit tests
**File**: `src/commands/mod.rs` (inside `#[cfg(test)] mod tests`)
**Action**: modify

Add `use crate::config::{Config, Step};` inside the tests module, then:

```rust
#[test]
fn determine_step_returns_step_with_present_artifact() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("first.txt"), "").unwrap();
    let config = Config {
        version: "0.1.0".to_string(),
        steps: vec![
            Step { name: "one".to_string(), trigger_artifact: "first.txt".to_string() },
            Step { name: "two".to_string(), trigger_artifact: "second.txt".to_string() },
        ],
    };
    let artifact_dir = format!("{}/", dir.path().display());
    assert_eq!(determine_step(&config, &artifact_dir).unwrap(), "one");
}

#[test]
fn determine_step_prefers_last_step_in_reverse() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("first.txt"), "").unwrap();
    std::fs::write(dir.path().join("second.txt"), "").unwrap();
    let config = Config {
        version: "0.1.0".to_string(),
        steps: vec![
            Step { name: "one".to_string(), trigger_artifact: "first.txt".to_string() },
            Step { name: "two".to_string(), trigger_artifact: "second.txt".to_string() },
        ],
    };
    let artifact_dir = format!("{}/", dir.path().display());
    assert_eq!(determine_step(&config, &artifact_dir).unwrap(), "two");
}

#[test]
fn determine_step_no_match_errors_with_step_tag() {
    let dir = tempfile::tempdir().unwrap();
    let config = Config {
        version: "0.1.0".to_string(),
        steps: vec![Step { name: "one".to_string(), trigger_artifact: "first.txt".to_string() }],
    };
    let artifact_dir = format!("{}/", dir.path().display());
    let err = determine_step(&config, &artifact_dir).unwrap_err();
    assert_eq!(err.source, "step");
    assert!(err.message.contains("no trigger artifact matched"), "{}", err.message);
}
```

### Verification
#### Automated
- [x] `cargo test determine_step` passes (all three tests)
- [x] `./scripts/test.sh` green

#### Manual
- [ ] n/a — covered by unit tests; the `prefers_last_step_in_reverse` test documents reverse priority (the "ordered progression" interpretation).

---

## Phase 5: CLI transport — `step` command

### Changes

#### 1. New `Commands::Step` variant
**File**: `src/commands/mod.rs`
**Action**: modify

Add `use std::path::PathBuf;` to the top imports, then extend the enum:

```rust
    /// Determine the current step from present trigger artifacts
    Step {
        /// Path to the TOML config file (relative paths resolve against the CWD)
        #[arg(long, value_name = "CONFIG", default_value = "orksorksorks.toml")]
        config: PathBuf,
    },
```

#### 2. Dispatch arm
**File**: `src/commands/mod.rs` (`select_command`)
**Action**: modify

```rust
        Commands::Step { config } => step_command(config.clone()),
```

#### 3. Handler
**File**: `src/commands/mod.rs`
**Action**: modify

```rust
fn step_command(config: std::path::PathBuf) -> Result<String, Error> {
    let cfg = crate::config::read_config(&config)?;
    let cwd = std::env::current_dir()?;
    let artifact_dir = artifact_dir_path(&cwd, &current_branch()?);
    determine_step(&cfg, &artifact_dir)
}
```

Note: `read_config` runs before git resolution so a missing/unreadable config deterministically fails with `"io"` (matters for the routing test below). The default relative `orksorksorks.toml` resolves against the process CWD via `read_to_string` — no explicit join, preserving the codebase's no-`Path::join`-in-production convention.

#### 4. Unit tests
**File**: `src/commands/mod.rs` (inside `#[cfg(test)] mod tests`)
**Action**: modify

```rust
#[test]
fn cli_try_parse_accepts_step() {
    use clap::Parser;
    let result = Cli::try_parse_from(["orksorksorks", "step"]);
    assert!(result.is_ok());
}

#[test]
fn cli_try_parse_accepts_step_with_custom_config() {
    use clap::Parser;
    let cli = Cli::try_parse_from(["orksorksorks", "step", "--config", "custom.toml"]).unwrap();
    match cli.command {
        Commands::Step { config } => {
            assert_eq!(config, std::path::PathBuf::from("custom.toml"));
        }
        _ => panic!("expected Commands::Step"),
    }
}

#[test]
fn select_command_routes_step() {
    let cli = Cli {
        json: false,
        command: Commands::Step {
            config: std::path::PathBuf::from("definitely-missing-config-file.toml"),
        },
    };
    // step_command reads the (missing) config first → "io", proving the
    // arm dispatched to step_command (and not, e.g., branch/artifact_directory).
    let err = select_command(&cli).unwrap_err();
    assert_eq!(err.source, "io");
}
```

#### 5. Integration tests
**File**: `tests/step.rs`
**Action**: create

```rust
use assert_cmd::Command;

const BRANCH: &str = "main";

/// `git init -b <name>` yields an unborn branch, and modern git (>= 2.22)
/// still reports its name from `git branch --show-current` — verified on
/// git 2.50.1. No commit is needed, so no user.name/user.email are required.
fn init_git_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let status = std::process::Command::new("git")
        .args(["init", "-b", BRANCH])
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success(), "git init failed");
    dir
}

fn write_config(dir: &std::path::Path) {
    std::fs::write(
        dir.join("orksorksorks.toml"),
        concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\n",
            "name = \"one\"\n",
            "trigger_artifact = \"first.txt\"\n",
            "[[steps]]\n",
            "name = \"two\"\n",
            "trigger_artifact = \"second.txt\"\n",
        ),
    )
    .unwrap();
}

fn artifact_dir(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join(".pi").join("orksorksorks").join(BRANCH)
}

#[test]
fn step_prints_name_of_step_with_present_artifact() {
    let dir = init_git_repo();
    write_config(dir.path());
    let artifact_dir = artifact_dir(dir.path());
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("second.txt"), "").unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd.arg("step").current_dir(dir.path()).output().unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim_end_matches('\x07').trim_end(), "two");
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn step_json_returns_valid_json_with_no_ansi() {
    let dir = init_git_repo();
    write_config(dir.path());
    let artifact_dir = artifact_dir(dir.path());
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("second.txt"), "").unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd
        .args(["step", "-j"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["data"].as_str(), Some("two"));
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn step_no_artifacts_present_fails() {
    let dir = init_git_repo();
    write_config(dir.path());

    Command::cargo_bin("orksorksorks").unwrap()
        .arg("step")
        .current_dir(dir.path())
        .assert()
        .failure();
}
```

### Verification
#### Automated
- [ ] `cargo test cli_try_parse_accepts_step` and `cargo test cli_try_parse_accepts_step_with_custom_config` pass
- [ ] `cargo test select_command_routes_step` passes
- [ ] `cargo test step_` passes (the three integration tests)
- [ ] `./scripts/test.sh` green (fmt → check → clippy → nextest → forbidden-strings)

#### Manual
- [ ] In a scratch git repo: `git init -b main`, write an `orksorksorks.toml` with two `[[steps]]` and create `.pi/orksorksorks/main/second.txt`, then `cargo run -- step` prints `two` with no ANSI.
- [ ] `cargo run -- step -j` prints `{"data":"two"}` (the integration test asserts `v["data"]`; a white-space-tolerant parse).
- [ ] With no trigger artifacts present, `cargo run -- step` exits 1 (no-match error).

---

## Cross-cutting notes (from `structure.md`)

- **Reverse priority** = last-forward step whose artifact exists; documented by `determine_step_prefers_last_step_in_reverse` and `step_prints_name_of_step_with_present_artifact`.
- **Output layer untouched** — no change to `src/main.rs`, `Display`, or bell/exit policy. The Phase 5 no-match test asserts `.failure()` only; the JSON error envelope body remains unit-covered only (`src/errors.rs` serialize test).
- **Config read lives in `src/config.rs`** (data-access), not in the handler, so it is unit-testable before CLI wiring exists.
- **No new dependencies**; `tempfile` (already a runtime dep) is used test-side only.