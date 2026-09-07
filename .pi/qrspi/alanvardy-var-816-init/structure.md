# Structure Outline

## Approach

Horizontal, bottom-up: build the convention spine from the ground up, each layer fully tested before the next. The dependency chain is Build infra → Foundation modules → CLI framework + output layer → Init command. Every stage ships its tests alongside the code; `./scripts/test.sh` is the final gate.

## Stage 1: Build Infrastructure

Prove the full build pipeline before any real code lands. Cargo.toml with all dependencies, build.rs, toolchain, CI workflows, nextest config, codecov.yml, and `scripts/test.sh` with the lock prologue.

**Files**: `Cargo.toml`, `build.rs`, `rust-toolchain.toml`, `.config/nextest.toml`, `codecov.yml`, `.github/workflows/ci-pr.yml`, `.github/workflows/_reusable-lint.yml`, `.github/workflows/_reusable-test.yml`, `scripts/test.sh`, `src/main.rs` (minimal `#[tokio::main] async fn main() { println!("Hello, world!"); }`)

**Key changes**:
- `Cargo.toml` — all `[dependencies]` and `[dev-dependencies]` from Decision 6, edition 2024, package metadata (`authors = ["Alan Vardy <alan@vardy.cc>"]`, license MIT, homepage/repository)
- `build.rs` — `cargo:rustc-env=BUILD_TARGET`/`BUILD_PROFILE` from `env::var`, `BUILD_TIMESTAMP` from `chrono::Utc::now().to_rfc3339()`
- `rust-toolchain.toml` — `channel = "1.97.1"`, `components = ["clippy", "rustfmt"]`
- `scripts/test.sh` — lock prologue (macOS `lockf` / Linux `flock` on `~/.cache/pi/test-gate.lock`) → `cargo fmt --all` → `cargo check` → `cargo clippy --tests -- -D warnings` → `cargo nextest run` → `rg` TODO/FIXME/dbg gate
- `codecov.yml` — ignore `src/main.rs` only (empty ignore list, per Decision 5); project target auto/10%, patch 50%
- `src/main.rs` — `#![warn(missing_docs)]`, `#[tokio::main]`

**Tests**: None written (cargo nextest runs zero tests vacuously).
**Verify**: `cargo build` succeeds; `cargo fmt --all`, `cargo check`, `cargo clippy --tests -- -D warnings` all pass on empty crate.

---

## Stage 2: Foundation Modules — errors.rs + format.rs

Ship the error and output-formatting conventions. Every subsequent stage imports these.

**Files**: `src/errors.rs` (new), `src/format.rs` (new), `src/main.rs` (add `mod errors; mod format;`)

**Key changes**:
- `pub struct Error { pub message: String, pub source: String }` — `#[derive(Debug, Clone, PartialEq, Eq, Serialize)]`
- `impl fmt::Display for Error` — `"Error from {yellow source}:\n{red message}"`
- `impl std::error::Error for Error {}` — empty
- `impl From<std::io::Error> for Error` — `("io", e.to_string())`
- `impl From<toml::ser::Error> for Error` — `("toml::ser", e.to_string())`
- `pub fn Error::new(source: &str, message: &str) -> Self` — primary constructor
- `pub fn green_string(s: &str) -> String` (and red_string, yellow_string, cyan_string) — `colored` crate wrappers
- `fn apply_color(...)` — strips ANSI under `cfg!(test)`

**Tests**: `src/errors.rs` — unit tests: Display strings (verify colored output), `From<io::Error>` tag, `From<toml::ser::Error>` tag, `Serialize` round-trip (assert JSON `{"message":...,"source":...}`), `PartialEq`. `src/format.rs` — unit tests: all color helpers assert plain strings under `cfg!(test)` (no ANSI).
**Verify**: `cargo nextest run` (only Stage 2 tests green), `cargo clippy --tests -- -D warnings` passes.

---

## Stage 3: CLI Framework + Output Layer

Ship the CLI skeleton and typed output layer. `orksorksorks init` parses correctly and routes through `CommandResult` — but the handler is a no-op stub returning `✓`.

**Files**: `src/main.rs` (expand — CommandResult, output layer, run_command, verify_cmd), `src/commands/mod.rs` (new — Cli, Commands, select_command, init stub), `tests/json_output.rs` (new)

**Key changes**:
- `pub struct Cli` — `#[derive(Parser, Clone)]`, `#[command(name = NAME, author = AUTHOR, version = LONG_VERSION, about = ABOUT)]` with `env!` constants + `BUILD_*` from build.rs, `json: bool` (`#[arg(short = 'j', long, global = true, default_value_t = false)]`), `command: Commands` (`#[command(subcommand)]`)
- `pub enum Commands` — `#[derive(Subcommand, Debug, Clone)]`, `Init` variant: `/// (i) Create a new orksorksorks.toml file`
- `pub fn select_command(cli: &Cli) -> Result<String, Error>` — two-level match; `Commands::Init` → `init_command()`
- `fn init_command() -> Result<String, Error>` — stub: `Ok(format::green_string("✓ init stub"))`
- `pub struct CommandResult { pub result: Result<String, Error>, pub bell_success: bool, pub bell_failure: bool, pub json: bool }`
- `fn output_result(cr: &CommandResult)` — dispatches to `output_text` / `output_json`
- `fn output_text(result: &Result<String, Error>)` — success → `println!("{}", data)` + `\x07`; error → `eprintln!("\n\n{e}")` + bell
- `fn output_json(result: &Result<String, Error>)` — `{"data": …}` / `{"error": {"message": …, "source": …}}`
- `fn run_command(cli: Cli)` — `select_command` → `CommandResult` → `output_result`; select errors force bells true + exit 1
- `#[test] fn verify_cmd()` — `Cli::try_parse().err(); Cli::command().debug_assert();`

**Tests**: `tests/json_output.rs` — integration test: `init -j` returns valid JSON with `data` field containing `✓`; `init` (no `-j`) prints plain text to stdout with no ANSI. `src/main.rs` — `verify_cmd` (clap parse smoke test). `src/commands/mod.rs` — unit test: `select_command` routes `Init` variant to the handler.
**Verify**: `cargo nextest run` (all unit + integration tests green), `cargo clippy --tests -- -D warnings` passes.

---

## Stage 4: Init Command

Replace the stub with the real handler: serialize default `Config` to TOML, write `orksorksorks.toml`, return ✓.

**Files**: `src/config.rs` (new), `src/commands/mod.rs` (replace `init_command` stub), `src/main.rs` (add `mod config;`), `tests/init_creates_file.rs` (new), `tests/json_output.rs` (update expected string)

**Key changes**:
- `pub struct Config` — `#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]`, starts with `version: String` field (default `"0.1.0"`)
- `impl Default for Config` — constructor
- Full `fn init_command() -> Result<String, Error>`:
  - `toml::to_string(&Config::default())?` → maps to `Error` via `From<toml::ser::Error>`
  - `std::fs::File::create("orksorksorks.toml")?` → `write_all` → `flush` → `sync_all`
  - Returns `Ok(format::green_string("✓ Created orksorksorks.toml"))`

**Tests**: `tests/init_creates_file.rs` — integration test: `assert_cmd` runs `init` in `tempfile::TempDir`, asserts exit 0 + `orksorksorks.toml` exists + content matches `toml::to_string(&Config::default()).unwrap()`. `src/config.rs` — unit test: `Config::default()` serializes to expected TOML, round-trip `Serialize`→`Deserialize`. Error-path integration test: `init` in read-only dir returns non-zero + JSON error envelope with `"source": "io"`. Update `tests/json_output.rs` expected string to `"✓ Created orksorksorks.toml"`.
**Verify**: `./scripts/test.sh` passes end-to-end (lock → fmt → check → clippy → nextest → TODO gate).

## Testing Checkpoints

| After Stage | Must Pass |
|---|---|
| Stage 1 | `cargo build`, `cargo fmt --all`, `cargo check`, `cargo clippy --tests -- -D warnings` |
| Stage 2 | `cargo nextest run` (errors + format unit tests), `cargo clippy --tests -- -D warnings` |
| Stage 3 | `cargo nextest run` (all unit + integration tests), `cargo clippy --tests -- -D warnings` |
| Stage 4 | `./scripts/test.sh` (full gate: lock → fmt → check → clippy → nextest → TODO rg) |