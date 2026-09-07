# Implementation Plan

## Overview

Ship the full convention spine and the `orksorksorks init` command: typed errors, colored output, JSON envelope, TOML config serialization, build infra, CI workflows, and a passing `./scripts/test.sh` gate.

## Phase 1: Build Infrastructure

Prove the full build pipeline before any real code lands.

### Changes

#### 1. Cargo.toml — populate [dependencies] and [dev-dependencies]
**File**: `Cargo.toml`
**Action**: modify

Replace the empty `[dependencies]` with:
```toml
[package]
name = "orksorksorks"
version = "0.1.0"
edition = "2024"
authors = ["Alan Vardy <alan@vardy.cc>"]
license = "MIT"
homepage = "https://github.com/alanvardy/orksorksorks"
repository = "https://github.com/alanvardy/orksorksorks"

[dependencies]
clap = { version = "4.6.1", features = ["derive"] }
tokio = { version = "1.52.3", features = ["full"] }
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.150"
colored = "3.1.1"
toml = "0.8.23"
tempfile = "3.27.0"

[dev-dependencies]
assert_cmd = "2.2.2"
predicates = "3.1.4"
pretty_assertions = "1.4.1"

[build-dependencies]
chrono = "0.4.42"
```

#### 2. build.rs — emit BUILD_* env vars
**File**: `build.rs`
**Action**: create

```rust
use std::env;

fn main() {
    println!("cargo:rustc-env=BUILD_TARGET={}", env::var("TARGET").unwrap());
    println!("cargo:rustc-env=BUILD_PROFILE={}", env::var("PROFILE").unwrap());
    println!(
        "cargo:rustc-env=BUILD_TIMESTAMP={}",
        chrono::Utc::now().to_rfc3339()
    );
}
```

#### 3. rust-toolchain.toml — pin 1.97.1 with clippy + rustfmt
**File**: `rust-toolchain.toml`
**Action**: create

```toml
[toolchain]
channel = "1.97.1"
components = ["clippy", "rustfmt"]
```

#### 4. .config/nextest.toml — CI profile
**File**: `.config/nextest.toml`
**Action**: create

```toml
[profile.ci]
retries = 2
fail-fast = false
slow-timeout = { period = "60s" }

[profile.ci.junit]
path = "target/nextest/ci/junit.xml"
store-success-output = false
store-failure-output = true
```

#### 5. scripts/test.sh — full gate with process lock
**File**: `scripts/test.sh`
**Action**: create

```bash
#!/usr/bin/env bash
set -euo pipefail

# Process lock — one test-suite run at a time across all repos
LOCKFILE="$HOME/.cache/pi/test-gate.lock"
mkdir -p "$(dirname "$LOCKFILE")"

if [[ "$(uname)" == "Darwin" ]]; then
    exec /usr/bin/lockf -t 0 "$LOCKFILE" /bin/sh -c "
        exec 3>>'$LOCKFILE'
        flock() { true; }  # no-op, lockf already holds
        $(declare -f _run_gates)
        _run_gates
    " || exit 1
else
    exec {lockfd}>"$LOCKFILE"
    flock -w 0 "$lockfd" || exit 1
fi

_run_gates() {
    echo "=== fmt ==="
    cargo fmt --all
    echo "=== check ==="
    cargo check
    echo "=== clippy ==="
    cargo clippy --tests -- -D warnings
    echo "=== nextest ==="
    cargo nextest run
    echo "=== TODO/FIXME/dbg gate ==="
    ! rg -i -s -g '*.rs' 'TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:'
}
```

**Note on lock**: The script uses a platform-aware wrapper. On macOS it shells through `lockf`. On Linux it uses `flock`. The inner `_run_gates` function is the same for both paths. This avoids the `lockf`-only approach from api/vardy breaking on Linux CI runners.

#### 6. codecov.yml — ignore main.rs only, auto/10% project, 50% patch
**File**: `codecov.yml`
**Action**: create

```yaml
ignore:
  - "src/main.rs"

coverage:
  status:
    project:
      default:
        target: auto
        threshold: 10%
    patch:
      default:
        target: 50%
```

#### 7. .github/workflows/_reusable-lint.yml — lint workflow
**File**: `.github/workflows/_reusable-lint.yml`
**Action**: create

```yaml
name: Lint

on:
  workflow_call:

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - name: cargo check
        run: cargo check --locked --all-features
      - name: cargo fmt
        run: cargo fmt --all -- --check
      - name: cargo clippy
        run: cargo clippy --all-targets --all-features --locked -- -D warnings
      - name: forbidden strings
        run: |
          ! rg -i -s -g '*.rs' 'TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:'
```

#### 8. .github/workflows/_reusable-test.yml — test + coverage workflow
**File**: `.github/workflows/_reusable-test.yml`
**Action**: create

```yaml
name: Test

on:
  workflow_call:
    inputs:
      upload-coverage:
        required: false
        type: boolean
        default: false

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - uses: taiki-e/install-action@v2
        with:
          tool: cargo-nextest,cargo-llvm-cov
      - name: cargo nextest run
        run: cargo nextest run --profile ci --all-features
      - name: coverage (main only)
        if: inputs.upload-coverage
        run: cargo llvm-cov nextest --profile ci --all-features --lcov --output-path lcov.info
      - name: upload coverage
        if: inputs.upload-coverage
        uses: codecov/codecov-action@v5
        with:
          files: lcov.info
      - name: upload test results
        if: inputs.upload-coverage
        uses: codecov/test-results-action@v1
        with:
          files: target/nextest/ci/junit.xml
```

#### 9. .github/workflows/ci-pr.yml — PR workflow calling reusables
**File**: `.github/workflows/ci-pr.yml`
**Action**: create

```yaml
name: CI (PR)

on:
  pull_request:
    branches: [main]

jobs:
  lint:
    uses: ./.github/workflows/_reusable-lint.yml
  test:
    uses: ./.github/workflows/_reusable-test.yml
```

#### 10. src/main.rs — minimal async entrypoint with missing_docs
**File**: `src/main.rs`
**Action**: modify (replace entirely)

```rust
#![warn(missing_docs)]

#[tokio::main]
async fn main() {
    println!("Hello, world!");
}
```

### Verification

#### Automated
- [x] `cargo build` succeeds
- [x] `cargo fmt --all` passes (no diff)
- [x] `cargo check` passes
- [x] `cargo clippy --tests -- -D warnings` passes

#### Manual
- [ ] `cargo run` prints "Hello, world!"
- [ ] `.github/workflows/` YAML is well-formed (view in editor)

---

## Phase 2: Foundation Modules — errors.rs + format.rs

Ship the error and output-formatting conventions. Every subsequent stage imports these.

### Changes

#### 1. src/errors.rs — Error struct, Display, Serialize, From impls
**File**: `src/errors.rs`
**Action**: create

```rust
use colored::Colorize;
use serde::Serialize;
use std::fmt;

/// The central error type for orksworksorks.
///
/// `source` is a lowercase tag (e.g. `"io"`, `"toml::ser"`) and `message`
/// is the human-readable description.  `Display` owns all coloring —
/// callers must not pre-apply ANSI codes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Error {
    pub message: String,
    pub source: String,
}

impl Error {
    /// Primary constructor.  Prefer `&format!(...)` for the message to
    /// avoid an unnecessary allocation.
    pub fn new(source: &str, message: &str) -> Self {
        Self {
            source: source.to_string(),
            message: message.to_string(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Error from {}:\n{}",
            crate::format::yellow_string(&self.source),
            crate::format::red_string(&self.message),
        )
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self {
            source: "io".to_string(),
            message: e.to_string(),
        }
    }
}

impl From<toml::ser::Error> for Error {
    fn from(e: toml::ser::Error) -> Self {
        Self {
            source: "toml::ser".to_string(),
            message: e.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn display_includes_source_and_message() {
        let err = Error::new("test_tag", "something broke");
        let displayed = format!("{err}");
        // Under cfg!(test) ANSI is stripped; check plain substrings
        assert!(displayed.contains("Error from test_tag"), "{displayed}");
        assert!(displayed.contains("something broke"), "{displayed}");
    }

    #[test]
    fn from_io_error_tags_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
        let err = Error::from(io_err);
        assert_eq!(err.source, "io");
        assert!(err.message.contains("no such file"));
    }

    #[test]
    fn from_toml_ser_error_tags_toml_ser() {
        // toml::ser::Error is hard to construct; use a struct that fails
        #[derive(serde::Serialize)]
        struct AlwaysFail;
        impl serde::Serialize for AlwaysFail {
            fn serialize<S: serde::Serializer>(&self, _s: S) -> Result<S::Ok, S::Error> {
                Err(serde::ser::Error::custom("boom"))
            }
        }
        let toml_err = toml::to_string(&AlwaysFail).unwrap_err();
        let err = Error::from(toml_err);
        assert_eq!(err.source, "toml::ser");
    }

    #[test]
    fn serialize_round_trip() {
        let err = Error::new("io", "permission denied");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains(r#""message":"permission denied""#));
        assert!(json.contains(r#""source":"io""#));
    }

    #[test]
    fn partial_eq() {
        let a = Error::new("x", "y");
        let b = Error::new("x", "y");
        let c = Error::new("x", "z");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn error_trait_impl() {
        let err = Error::new("t", "m");
        // Just prove the trait is usable
        let _: &dyn std::error::Error = &err;
    }
}
```

#### 2. src/format.rs — color helpers with cfg!(test) stripping
**File**: `src/format.rs`
**Action**: create

```rust
use colored::{Color, Colorize};

/// Apply color if not running under test, otherwise return the string
/// unchanged.  This is the single chokepoint for ANSI control — every
/// color helper routes through it.
fn apply_color(s: &str, color: Color) -> String {
    if cfg!(test) {
        s.to_string()
    } else {
        s.color(color).to_string()
    }
}

pub fn green_string(s: &str) -> String {
    apply_color(s, Color::Green)
}

pub fn red_string(s: &str) -> String {
    apply_color(s, Color::Red)
}

pub fn yellow_string(s: &str) -> String {
    apply_color(s, Color::Yellow)
}

pub fn cyan_string(s: &str) -> String {
    apply_color(s, Color::Cyan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn green_string_is_plain() {
        assert_eq!(green_string("✓"), "✓");
    }

    #[test]
    fn red_string_is_plain() {
        assert_eq!(red_string("error"), "error");
    }

    #[test]
    fn yellow_string_is_plain() {
        assert_eq!(yellow_string("io"), "io");
    }

    #[test]
    fn cyan_string_is_plain() {
        assert_eq!(cyan_string("info"), "info");
    }

    #[test]
    fn all_helpers_strip_ansi_under_test() {
        // cfg!(test) is true here; all should return plain strings
        for input in &["hello", "✓", "error text"] {
            assert!(!green_string(input).contains('\x1b'));
            assert!(!red_string(input).contains('\x1b'));
            assert!(!yellow_string(input).contains('\x1b'));
            assert!(!cyan_string(input).contains('\x1b'));
        }
    }
}
```

#### 3. src/main.rs — declare modules
**File**: `src/main.rs`
**Action**: modify

Add `mod errors; mod format;` after the `#![warn(missing_docs)]` line but before `#[tokio::main]`:
```rust
#![warn(missing_docs)]

mod errors;
mod format;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
}
```

### Verification

#### Automated
- [x] `cargo nextest run` passes (only Phase 2 tests green)
- [x] `cargo clippy --tests -- -D warnings` passes

#### Manual
- [ ] Run a quick test outside `cfg!(test)` to see colors: `cargo run` then temporarily remove the cfg gate — confirmation only; restore after

---

## Phase 3: CLI Framework + Output Layer

Ship the CLI skeleton and typed output layer. `orksworksorks init` parses correctly and routes through `CommandResult` — but the handler is a no-op stub returning `✓`.

### Changes

#### 1. src/commands/mod.rs — Cli, Commands, select_command, init stub
**File**: `src/commands/mod.rs`
**Action**: create

```rust
use clap::{Parser, Subcommand};

use crate::errors::Error;

const NAME: &str = env!("CARGO_PKG_NAME");
const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");
const LONG_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "\nbuild target: ",
    env!("BUILD_TARGET"),
    "\nbuild profile: ",
    env!("BUILD_PROFILE"),
    "\nbuild timestamp: ",
    env!("BUILD_TIMESTAMP"),
);

/// CLI argument root.
#[derive(Parser, Clone)]
#[command(
    name = NAME,
    author = AUTHOR,
    version = LONG_VERSION,
    about = ABOUT,
    long_about = None
)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    /// Output results as JSON
    #[arg(short = 'j', long, global = true, default_value_t = false)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Commands,
}

/// Top-level subcommands.
#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// (i) Create a new orksworksorks.toml file
    Init,
}

/// Route a parsed CLI to its handler and return a success message or error.
pub fn select_command(cli: &Cli) -> Result<String, Error> {
    match &cli.command {
        Commands::Init => init_command(),
    }
}

/// Stub — replaced in Phase 4 with the real handler.
fn init_command() -> Result<String, Error> {
    Ok(crate::format::green_string("✓ init stub"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn select_command_routes_init() {
        let cli = Cli {
            json: false,
            command: Commands::Init,
        };
        let result = select_command(&cli).unwrap();
        // Under cfg!(test) color is stripped
        assert_eq!(result, "✓ init stub");
    }

    #[test]
    fn cli_try_parse_rejects_no_subcommand() {
        use clap::Parser;
        let result = Cli::try_parse_from(["orksorksorks"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_try_parse_accepts_init() {
        use clap::Parser;
        let result = Cli::try_parse_from(["orksorksorks", "init"]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_command_debug_assert() {
        Cli::command().debug_assert();
    }
}
```

#### 2. src/main.rs — replace with CommandResult, output layer, run_command
**File**: `src/main.rs`
**Action**: modify (replace entirely)

```rust
#![warn(missing_docs)]

mod commands;
mod errors;
mod format;

use clap::Parser;
use errors::Error;

/// The result of running a command, ready for the output layer.
pub struct CommandResult {
    pub result: Result<String, Error>,
    pub bell_success: bool,
    pub bell_failure: bool,
    pub json: bool,
}

/// Print the command result as text (stdout on success, stderr on error).
fn output_text(result: &Result<String, Error>) {
    match result {
        Ok(data) => {
            println!("{data}");
            // Terminal bell on success
            print!("\x07");
        }
        Err(e) => {
            eprintln!("\n\n{e}");
            // Terminal bell on failure
            print!("\x07");
        }
    }
}

/// Print the command result as a JSON envelope.
fn output_json(result: &Result<String, Error>) {
    match result {
        Ok(data) => {
            let json = serde_json::json!({"data": data});
            println!("{json}");
        }
        Err(e) => {
            let json = serde_json::json!({"error": {"message": e.message, "source": e.source}});
            println!("{json}");
        }
    }
}

/// Dispatch to text or JSON output based on the command result's json flag.
fn output_result(cr: &CommandResult) {
    if cr.json {
        output_json(&cr.result);
    } else {
        output_text(&cr.result);
    }
}

/// Parse CLI, run the command, and output the result.
fn run_command(cli: Cli) {
    let json = cli.json;
    let result = commands::select_command(&cli);
    let cr = match result {
        Ok(data) => CommandResult {
            result: Ok(data),
            bell_success: true,
            bell_failure: false,
            json,
        },
        Err(e) => CommandResult {
            result: Err(e),
            bell_success: false,
            bell_failure: true,
            json,
        },
    };
    output_result(&cr);
    if cr.result.is_err() {
        std::process::exit(1);
    }
}

#[tokio::main]
async fn main() {
    let cli = commands::Cli::parse();
    run_command(cli);
}
```

#### 3. tests/json_output.rs — integration test for JSON output
**File**: `tests/json_output.rs`
**Action**: create

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn init_json_returns_valid_json_with_data_field() {
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd.args(["init", "-j"]).output().unwrap();

    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""data""#), "stdout: {stdout}");
    assert!(stdout.contains("✓ init stub"), "stdout: {stdout}");
    // Valid JSON
    let _: serde_json::Value = serde_json::from_str(&stdout).unwrap();
}

#[test]
fn init_no_json_prints_plain_text() {
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd.arg("init").output().unwrap();

    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("✓ init stub"), "stdout: {stdout}");
    assert!(!stdout.contains('\x1b'), "stdout should have no ANSI: {stdout}");
}
```

### Verification

#### Automated
- [x] `cargo nextest run` passes (all unit + integration tests green)
- [x] `cargo clippy --tests -- -D warnings` passes

#### Manual
- [ ] `cargo run -- init` prints "✓ init stub" to stdout
- [ ] `cargo run -- init -j` prints a JSON object with `{"data":"✓ init stub"}`
- [ ] `cargo run -- --version` prints version info including build target/profile/timestamp

---

## Phase 4: Init Command

Replace the stub with the real handler: serialize default `Config` to TOML, write `orksorksorks.toml`, return ✓.

### Changes

#### 1. src/config.rs — Config struct with Default, Serialize, Deserialize
**File**: `src/config.rs`
**Action**: create

```rust
use serde::{Deserialize, Serialize};

/// Application configuration, serialized as TOML in `orksorksorks.toml`.
///
/// The `version` field tracks the config format version so future
/// migrations can detect and upgrade older files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    /// Config format version.
    pub version: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: "0.1.0".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn default_config_serializes_to_expected_toml() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("version"), "{toml_str}");
        assert!(toml_str.contains(r#""0.1.0""#), "{toml_str}");
    }

    #[test]
    fn config_round_trip_serialize_deserialize() {
        let config = Config::default();
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }
}
```

#### 2. src/main.rs — add mod config
**File**: `src/main.rs`
**Action**: modify

Insert `mod config;` after `mod commands;`:
```rust
mod commands;
mod config;
mod errors;
mod format;
```

#### 3. src/commands/mod.rs — replace init_command stub
**File**: `src/commands/mod.rs`
**Action**: modify

Replace the stub `init_command()` function:
```rust
/// Create a new `orksorksorks.toml` file with default configuration.
fn init_command() -> Result<String, Error> {
    use std::io::Write;

    let config = crate::config::Config::default();
    let toml_str = toml::to_string(&config)?;
    let mut file = std::fs::File::create("orksorksorks.toml")?;
    file.write_all(toml_str.as_bytes())?;
    file.flush()?;
    file.sync_all()?;
    Ok(crate::format::green_string("✓ Created orksorksorks.toml"))
}
```

Also update the unit test that asserts on the stub string. In the `#[cfg(test)] mod tests` block, change:
```rust
    #[test]
    fn select_command_routes_init() {
        let cli = Cli {
            json: false,
            command: Commands::Init,
        };
        let result = select_command(&cli).unwrap();
        // Under cfg!(test) color is stripped
        assert_eq!(result, "✓ Created orksorksorks.toml");
    }
```

#### 4. tests/init_creates_file.rs — integration test for file creation
**File**: `tests/init_creates_file.rs`
**Action**: create

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

#[test]
fn init_creates_orksorksorks_toml_with_default_content() {
    let temp = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.current_dir(temp.path());

    cmd.arg("init").assert().success();

    let config_path = temp.path().join("orksorksorks.toml");
    assert!(config_path.exists(), "config file not created");

    let content = fs::read_to_string(&config_path).unwrap();
    let expected = toml::to_string(&orksorksorks::config::Config::default()).unwrap();
    assert_eq!(content, expected);
}

#[test]
fn init_returns_success_message() {
    let temp = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.current_dir(temp.path());

    cmd.arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("✓ Created orksorksorks.toml"));
}

#[test]
fn init_in_readonly_dir_returns_error_exit_code() {
    let temp = tempfile::tempdir().unwrap();
    let mut perms = fs::metadata(temp.path()).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(temp.path(), perms).unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.current_dir(temp.path());

    cmd.arg("init").assert().failure();

    // Restore writable so tempfile can clean up
    let mut perms = fs::metadata(temp.path()).unwrap().permissions();
    perms.set_readonly(false);
    fs::set_permissions(temp.path(), perms).unwrap();
}

#[test]
fn init_json_error_in_readonly_dir_shows_source_io() {
    let temp = tempfile::tempdir().unwrap();
    let mut perms = fs::metadata(temp.path()).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(temp.path(), perms).unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.current_dir(temp.path());

    let output = cmd.args(["init", "-j"]).output().unwrap();
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""source":"io""#), "stdout: {stdout}");

    // Restore
    let mut perms = fs::metadata(temp.path()).unwrap().permissions();
    perms.set_readonly(false);
    fs::set_permissions(temp.path(), perms).unwrap();
}
```

#### 5. tests/json_output.rs — update expected string
**File**: `tests/json_output.rs`
**Action**: modify

Replace `"✓ init stub"` with `"✓ Created orksorksorks.toml"` in both tests:
- `init_json_returns_valid_json_with_data_field`: `assert!(stdout.contains("✓ Created orksorksorks.toml"), ...)`
- `init_no_json_prints_plain_text`: `assert!(stdout.contains("✓ Created orksorksorks.toml"), ...)`

#### 6. src/main.rs — make Config accessible to integration tests
**File**: `src/main.rs`
**Action**: modify

Integration tests in `tests/init_creates_file.rs` need to call `orksorksorks::config::Config::default()` to compute the expected TOML. This requires `config` to be `pub mod`:
```rust
pub mod commands;
pub mod config;
mod errors;
mod format;
```

(Errors and format stay `mod` — integration tests don't need them.)

### Verification

#### Automated
- [ ] `./scripts/test.sh` passes end-to-end (lock → fmt → check → clippy → nextest → TODO gate)

#### Manual
- [ ] `cargo run -- init` creates `orksorksorks.toml` in CWD, prints green ✓
- [ ] `cargo run -- init -j` prints JSON with `{"data":"✓ Created orksorksorks.toml"}`
- [ ] `cat orksorksorks.toml` shows `version = "0.1.0"`
- [ ] Running `cargo run -- init` again overwrites the file (no error)
- [ ] `cargo run -- init` in a read-only directory exits non-zero with red error
