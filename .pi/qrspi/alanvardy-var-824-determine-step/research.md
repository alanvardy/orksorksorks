# Research Findings

Crate: `orksorksorks` — binary-only Rust CLI (edition 2024, `rust-toolchain.toml` pins channel `1.98.1`). No lib target. Modules: `src/main.rs`, `src/commands/mod.rs`, `src/config.rs`, `src/errors.rs`, `src/format.rs`. Line numbers verified against working tree.

## Q1: Config story end-to-end

### Findings
- `Config` is a single-field struct: `#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)] pub struct Config { pub version: String }` (`src/config.rs:8-11`). Doc comment says it serializes to TOML in `orksorksorks.toml` and `version` tracks format version for future migration (`src/config.rs:3-6`).
- Defaulting is a **manual** `impl Default` — `version: "0.1.0"` (`src/config.rs:13-19`). Serialization is `toml::to_string(&config) -> String`; deserialization is `toml::from_str` (`toml = "1.1.5"`, `Cargo.toml:17`).
- **Production path is write-only.** The sole consumer is `init_command` (`src/commands/mod.rs:61-71`): `Config::default()` → `toml::to_string(&config)?` (L65) → `std::fs::File::create("orksorksorks.toml")?` + `write_all`/`flush`/`sync_all` (L66-69) → returns green `"✓ Created orksorksorks.toml"` (L70). File is created/truncated, fsynced, and **never read back by any command**.
- The only `toml::from_str` in the tree is the unit test round-trip at `src/config.rs:38`; production has no disk-read config path.
- **No `Vec` / array-of-tables / `Option` fields exist anywhere.** Full struct inventory: `Config{version: String}` (`src/config.rs:8-10`); `Error{message, source: String}` (`src/errors.rs:10-12`); `CommandResult{result: Result<String,Error>, bell_success, bell_failure, json: bool}` (`src/main.rs:17-26`); `Cli{json: bool, command: Commands}` (`src/commands/mod.rs:28-35`); `Commands` enum has zero-payload unit variants only (`src/commands/mod.rs:39-49`). There is therefore no serde shape for `Vec`/`Option` in use.
- Integration test pins exact on-disk bytes: `assert_eq!(content, "version = \"0.1.0\"\n")` (`tests/init_creates_file.rs:22-25`).
- Connection: `main` (`src/main.rs:93`) → `run_command` (L68-89) → `select_command` (`src/commands/mod.rs:52-58`) → `init_command`; serde errors map to `Error{source:"toml::ser"}` via `From<toml::ser::Error>` (`src/errors.rs:48-55`), I/O errors to `source:"io"` (`src/errors.rs:39-46`).

## Q2: CLI arg surface and dispatch

### Findings
- Whole CLI declared in `src/commands/mod.rs` via clap derive. `Cli` (`src/commands/mod.rs:28-35`): `#[derive(Parser, Clone)]` (L20), `#[command(name, author, version, about, long_about = None)]` (L21-27) with `NAME/AUTHOR/ABOUT` from `env!("CARGO_PKG_*")` (L5-7) and `LONG_VERSION` a `concat!` of version + `BUILD_TARGET/PROFILE/TIMESTAMP` (L9-17, injected by `build.rs`), plus `#[command(arg_required_else_help = true)]` (L28).
- Fields: `json: bool` (L31) with `#[arg(short = 'j', long, global = true, default_value_t = false)]` (L30) — the **only flag**, global (usable before/after any subcommand); `command: Commands` (L34) with `#[command(subcommand)]` (L33).
- `Commands` enum (`src/commands/mod.rs:39-49`): three **zero-payload unit variants** — `Init` (L41), `Branch` (L44), `ArtifactDirectory` (L48) with `#[command(name = "artifact_directory")]` (L47) overriding clap's kebab-case default. **No value-taking options and no positional args exist** — the only `#[arg(...)]` in `src/` is L30. Tests lock this: `cli_try_parse_rejects_no_subcommand` and `cli_try_parse_rejects_kebab_case_artifact_directory` (`src/commands/mod.rs:139, 186` areas).
- Dispatch: `select_command` is a plain exhaustive `match` calling zero-arg handlers and returning their `Result<String, Error>` directly (`src/commands/mod.rs:52-58`).
- **Handler contract:** every handler is `fn ...() -> Result<String, Error>`; `Ok(String)` is the full text the output layer prints (must not pre-color — `Display`/format helpers own ANSI), `Err(Error)` is the typed error. Error propagation via `?` + `From` impls.
- Output layer (`src/main.rs`): `Cli::parse()` in `main` (L93) — clap's `parse` (not `try_parse`) exits the process itself on parse failure (clap_builder 4.6.6 `Error::exit()`; exit 2 for usage/error, 0 for help/version — never reaches `run_command`). `run_command` (L68-89) captures `cli.json`, calls `select_command`, wraps in `CommandResult{result, bell_success, bell_failure, json}` (L17-26, built L71-84), calls `output_result` (L59-65, routes on `cr.json` only), then `std::process::exit(1)` if error (L86-87).
- Text mode (`output_text`, `src/main.rs:29-42`): `Ok` → `println!("{data}")` to stdout (L32) + bell `print!("\x07")` (L34); `Err` → `eprintln!("\n\n{e}")` to stderr (L37) + bell (L39). JSON mode (`output_json`, L45-56): `Ok` → `println!` of `{"data": data}` on stdout (L48-49); `Err` → stdout `{"error": {"message": e.message, "source": e.source}}` (L52-53), no bells, no stderr.
- `bell_success`/`bell_failure` are assigned (L74-75, 80-81) but **never read** — the bells are hardcoded in `output_text`; JSON emits none.
- **Typed path args in clap 4.6.6 (pinned `Cargo.toml:11`):** the derive emits `value_parser!(<inner_type>)` when no explicit parser is given, and `std::path::PathBuf` has a dedicated `ValueParserFactory` → `ValueParser::path_buf()` (clap_builder 4.6.6 `value_parser.rs:2311-2315`), with `Box<std::path::Path>` supported via a map parser (2317-2322). `PathBufValueParser` (`value_parser.rs:995-1033`) converts via `PathBuf::from` and **does not check existence** — only failure mode is an empty value. Capability is unused today; paths are built at runtime inside handlers (`src/commands/mod.rs:120`, L78-84).

## Q3: Artifact directory path

### Findings
- Path contract (`artifact_dir_path`, `src/commands/mod.rs:78-84`): pure function `(cwd: &std::path::Path, branch: &str) -> String` composing `format!("{}/.pi/orksorksorks/{}/", cwd.display(), branch.replace('/', "-"))` (L80-82). Components:
  - `cwd` via `cwd.display()` (L81) — no trailing-slash stripping.
  - `.pi/orksorksorks` hardcoded literal (L80); not configurable, not in `Config`.
  - Branch normalization: every `/` → `-` (L82), so `feature/x` → `feature-x` single segment.
  - Trailing `/` is a literal part of the contract (doc L76-78).
  - Pure: no git, no I/O, no error path (doc L74-76).
- Command (`artifact_directory_command`, `src/commands/mod.rs:118-121`): `let cwd = std::env::current_dir()?;` (L120) — process cwd from std, **not** `$PWD` env — then `Ok(artifact_dir_path(&cwd, &current_branch()?))` (L121). Doc notes "plain (no directory creation)" (L117-118).
- Branch sourced by spawning git: `std::process::Command::new("git").args(["branch", "--show-current"]).output()?` (`src/commands/mod.rs:91-94`), stdout/stderr via `String::from_utf8_lossy` (L95-96), then pure `parse_branch_output` (L100-109): non-zero exit → `Err(Error::new("git", stderr.trim()))` (L102-103); empty trimmed stdout → `Err(Error::new("git", "not on a branch (detached HEAD)"))` (L106-107); else `Ok(trimmed)` (L109). Spawn failure (git not on PATH) rides `From<std::io::Error>` → `"io"`; git-level failures → `"git"` (doc L89-90). No env-var or config override for branch.
- Test mirror: `expected_artifact_directory()` (`tests/artifact_directory.rs:3-14`) re-derives cwd via `std::env::current_dir()` and branch via the same `git branch --show-current` subprocess, then composes the **same literal** `format!("{}/.pi/orksorksorks/{}/", ...)` with `branch.replace('/', "-")` — an independent re-implementation of the same contract, not a copy of the helper.
- Unit coverage of the pure function: `src/commands/mod.rs:200-230` (trailing slash, plainness/no `\x1b`, `feature/x`, `a/b/c`, slashless branch). Handler-level test recomputes cwd+branch independently (`src/commands/mod.rs:270-277`).
- Output/exit integration: success text = path + bell (trimmed in test via `trim_end_matches('\x07')`, `tests/artifact_directory.rs:25`); `-j` → `{"data": path}` parsed as JSON (L34-38); outside a git repo → `.failure()` (L43-48), exercising `exit(1)` (`src/main.rs:86-87`).

## Q4: Filesystem operations

### Findings
- **Process cwd obtained in exactly one production site:** `std::env::current_dir()?` at `src/commands/mod.rs:120` (`artifact_directory_command`). `init` writes to a bare relative string instead (below). Tests re-derive cwd in `tests/artifact_directory.rs:4` and `src/commands/mod.rs:273`.
- **No production file-existence checks.** Grep for `exists|try_exists|metadata|is_file|is_dir` over `src/` yields nothing. The only `exists()` call in the repo is a test assertion `tests/init_creates_file.rs:19`; `fs::metadata` appears only in tests (`tests/init_creates_file.rs:43,53,61,75`).
- Production file operations are **write-only** and all inside `init_command` (`src/commands/mod.rs:66-69`): `File::create("orksorksorks.toml")` (unconditional create/truncate, no pre-check), `write_all`, `flush`, `sync_all`. Errors → `"io"` tag via `From<std::io::Error>` (`src/errors.rs:39-46`).
- **Path joining is string formatting, not `Path::join`:** `artifact_dir_path` uses `format!` with `cwd.display()` (`src/commands/mod.rs:79-83`); `init` uses literal `"orksorksorks.toml"` (L66). The only `Path::join` in the tree is test-side: `temp.path().join("orksorksorks.toml")` (`tests/init_creates_file.rs:18`).
- **Test sandbox pattern:** `tempfile::tempdir()` (runtime dep `Cargo.toml:15` but used only in tests) + `assert_cmd::Command::cargo_bin("orksorksorks")` + `cmd.current_dir(temp.path())` — `tests/init_creates_file.rs:12-14,30-32,42-48,60-66`; `tests/artifact_directory.rs:45-47`; `tests/branch.rs:40-42`. Read-only-dir tests flip `fs::metadata(...).permissions().set_readonly(true)`, run, restore with `set_readonly(false)` so tempfile can clean up (L42-55, 59-77), with file-level `#![allow(clippy::permissions_set_readonly_false)]` (`tests/init_creates_file.rs:4`).
- **`tokio` enables only `#[tokio::main]`:** declared `tokio = { version = "1.53.1", features = ["full"] }` (`Cargo.toml:16`); the only usage is the annotation on `main` (`src/main.rs:91-92`). Everything below is synchronous std (`fs::File`, `process::Command`, `env::current_dir`); no `.await`, no async fs I/O anywhere.

## Q5: Errors

### Findings
- `Error` struct: `#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct Error { pub message: String, pub source: String }` (`src/errors.rs:9-13`). `source` is a lowercase origin tag (doc L3-8); `Display` owns all coloring.
- One public constructor: `Error::new(source, message)` (`src/errors.rs:18-23`). No crate code pre-formats messages.
- **Exactly two `From` impls** (grep `From<` finds nothing else in the crate):
  - `From<std::io::Error> for Error` → `source: "io"`, message = `e.to_string()` (`src/errors.rs:39-46`)
  - `From<toml::ser::Error> for Error` → `source: "toml::ser"` (`src/errors.rs:48-55`)
- `?`-operator sites riding these: `toml::to_string` (mod.rs:65); `File::create`/write/flush/sync (mod.rs:66-69); git spawn (mod.rs:93); `current_dir` (mod.rs:120).
- Third tag created directly, no conversion: `"git"` via `Error::new` in `parse_branch_output` (mod.rs:102, 106). Three tags in existence: `"io"`, `"toml::ser"`, `"git"`.
- **Missing:** `From<toml::de::Error>` does not exist. The only TOML read is the test `toml::from_str(&serialized).unwrap()` (`src/config.rs:38`) — it unwraps directly; a parse failure cannot be represented as crate `Error`. No `serde_json`/`TryFrom` conversions either.
- `impl fmt::Display for Error` renders `Error from {yellow source}:\n{red message}` (`src/errors.rs:26-35`), coloring via `crate::format::yellow_string`/`red_string` (L28-32) through the single `apply_color` chokepoint which strips ANSI under `cfg!(test)` (`src/format.rs:6-12`). `impl std::error::Error for Error {}` is an empty marker (L37), proven by test `error_trait_impl` (`src/errors.rs:109-115`).
- Surface through `main.rs`: errors travel only inside `Result<String, Error>` via `run_command` → `CommandResult` → `output_result`. Text: stderr `"\n\n" + Display` + bell (`src/main.rs:37-40`). JSON: stdout `{"error":{"message","source"}}` hand-built field-by-field from public fields — the derived `Serialize` (used/tested at `src/errors.rs:91-98`) is **not** used for the envelope (`src/main.rs:52-53`). Exit 1 both modes (L86-87).
- Note: no integration test asserts the JSON error envelope or stderr text; unit tests cover tag mapping (`src/errors.rs:69-89`).

## Q6: Test gate

### Findings (see conventions.md for the full inventory)
- **Tier 1 — inline `#[cfg(test)]` units** (33 tests): `src/commands/mod.rs:124-280` (routing, clap parsing via `Cli::try_parse_from`, pure-function assertions incl. `parse_branch_output` error tags); `src/config.rs:22-41` (default serialization, round-trip); `src/errors.rs:57-116` (Display, both From mappings, JSON round-trip, PartialEq, dyn-Error); `src/format.rs:27-52` (color helpers plain under test).
- **Tier 2 — integration spawning the real binary** (12 tests, dev-deps `assert_cmd 2.2.2`, `predicates 3.1.4`, `pretty_assertions 1.4.1`, `Cargo.toml:20-22`): `tests/init_creates_file.rs` (4), `tests/json_output.rs` (2), `tests/branch.rs` (3), `tests/artifact_directory.rs` (3).
- Gate: `scripts/test.sh` (fmt, check, clippy, nextest, forbidden-strings rg) — verified lines below. CI reusable workflows (`ci-pr.yml` → `_reusable-lint.yml`, `_reusable-test.yml`) use `--locked`, clippy `-D warnings`, nextest `--profile ci`, optional coverage via `cargo llvm-cov` + codecov; `codecov.yml` ignores `src/main.rs`.
- **Conventions:** color is neutralized under test via `cfg!(test)` in `apply_color` (`src/format.rs:6-12`); integration assertions trim exactly one trailing bell before comparing (`tests/branch.rs:20`, `tests/artifact_directory.rs:25`); no-ANSI assertion is universal (`tests/*.rs` various); exact on-disk bytes pinned inline; no fixtures/snapshots/golden files exist.

## Cross-Cutting Observations

- **One contract, two layers:** handlers return `Result<String, Error>` (text-first, error as data); `main.rs` is the only place text/JSON/bell/exit-code policy lives (`output_text`/`output_json`/`output_result`, `src/main.rs:29-65`). Handlers never emit JSON or bells themselves.
- **Everything is synchronous std** except the `#[tokio::main]` shell; `tokio full` is declared but effectively unused beyond the macro.
- **Path composition is string-based**, never `Path::join`, and the trailing slash is an explicit contract; the test suite re-implements the same format string independently rather than importing it.
- **Config is a write-only artifact today** — `init` emits a TOML file that nothing reads; no deserialization error path exists (`From<toml::de::Error>` absent).
- **Dead-ish fields:** `CommandResult.bell_success`/`bell_failure` are set but never read; bells are hardcoded in `output_text`.
- **No CLI value/positional args exist;** clap's typed `PathBuf` support is available in the pinned version but unused.
- **Style markers:** edition-2024 std `Result` with `Ok`/`Err`/`unwrap`/`unwrap_err`/`is_ok`/`is_err` + `?` with `From` conversions; `pretty_assertions::assert_eq!`/`assert_ne!` in tests; `crate::format::*_string` for all user-facing colored output; `#![warn(missing_docs)]` on `src/main.rs:6`.

## Open Areas

- The exact clap-derived help/error text, usage strings, and version output were asserted in source analysis only (clap_builder 4.6.6 `error/mod.rs:221-249`); no test pins them.
- No integration test covers the JSON error envelope `{"error":{...}}`, stderr text-mode error formatting, or detached-HEAD output end-to-end.
- An older planning doc (branch `alanvardy-var-816-init` research.md) described a `Cli` with `verbose`, `Option<PathBuf>`, `Option<u64>` fields that **do not exist** in current code — do not assume them.
- The artifact-path trailing-slash contract's rationale is documented only via a cross-reference to `task.md` (not read here); tests treat it as normative (`src/commands/mod.rs:76-78`, `tests/artifact_directory.rs:25`).
- Whether `tokio`'s fs/net/time/sync features are intended for a near-term async path is unknown; today only `#[tokio::main]` is exercised.