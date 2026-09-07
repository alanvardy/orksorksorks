# Design Discussion

## Current State

The project is a stub: `src/main.rs:1` prints `"Hello, world!"`, `Cargo.toml:6` has empty `[dependencies]`, and no `Cargo.lock`, tests, CI, or config exists (`research.md` Q0). The only Rust conventions available are the author's sibling project `tod` — a mature clap-based CLI with a hand-rolled error type, `✓`-success output, JSON serialization, tokio::fs, nextest, and CI gates (`research.md` cross-cutting observations).

## Desired End State

`orksorksorks init` is a working subcommand that:

1. Creates an `orksorksorks.toml` file in the current directory, serialized from a default `Config` struct via the `toml` crate
2. Prints a green `✓` confirmation message via the established output layer
3. Supports `-j/--json` for structured output
4. Errors are typed (`Error { message, source }`), colored, and JSON-serializable
5. The project is structurally complete: all modules, `build.rs`, `scripts/test.sh`, CI workflows, `rust-toolchain.toml`, `.config/nextest.toml`, and `codecov.yml` are in place
6. At least one integration test (assert_cmd) and one unit test (verify_cmd) pass
7. `./scripts/test.sh` passes (fmt → check → clippy → nextest → TODO gate)

## Patterns to Follow

Every pattern is from `tod` (the author's primary CLI reference):

### CLI structure (`tod/src/commands/mod.rs`)
- `Cli` struct: `#[derive(Parser, Clone)]`, `#[command(name, author, version, about)]` with `env!` constants (`:47-73`)
- `Commands` enum: `#[derive(Subcommand, Debug, Clone)]`, newtype-tuple variants, `#[clap(alias = "x")]` single-char aliases, doc comments starting with `/// (x) description` (`:75-124`)
- Dispatch: `select_command` match on `Commands` → per-group dispatcher → handler returns `Result<String, Error>` (`:141-158`)
- `-j/--json` global flag: `#[arg(short = 'j', long, global = true, default_value_t = false)]` (`:62`)
- `verify_cmd()` test: `Cli::try_parse().err(); Cli::command().debug_assert();` (`tod/src/main.rs:144-149`)

### Error handling (`tod/src/errors.rs`)
- `Error { message: String, source: String }` with `#[derive(Debug, Clone, PartialEq, Eq, Serialize)]` (`:24-27`)
- `Display` owns coloring: `"Error from {yellow source}:\n{red message}"` (`:29-39`)
- `From<io::Error>` → `("io", e.to_string())` (`:41-48`)
- Constructor: `Error::new(source: &str, message: &str)` with `&format!(...)` borrows (`:181-190`)
- `impl std::error::Error for Error {}` is empty — no `source()` chain (`:179`)

### Output (`tod/src/main.rs`)
- `CommandResult { result, bell_success, bell_failure, json }` (`:50-55`)
- `output_result` → `output_text` (stdout `println!`+bell on success, stderr on error) / `output_json` (`{"data": …}` / `{"error": {message, source}}`) (`:78-124`)
- `run_command` is the chokepoint: `unwrap_or_else` → err with bells true (`:126-136`)
- `✓` is returned as data (`Ok(format::green_string("✓"))`, `tod/src/config/file.rs:54`) — never printed separately in handlers

### Format/color (`tod/src/format.rs`)
- `colored` crate, helpers: `green_string`, `red_string`, `cyan_string`, `yellow_string`, etc. (`:18-53`)
- `cfg!(test)` strips ANSI (`:8-13`) — unit tests assert plain strings, integration tests (`tests/json_output.rs:298`) assert no escape codes

### File I/O (`tod/src/config/file.rs`)
- `touch_file`: `create_dir_all(parent)` + `File::create` (`:24-31`)
- `save`: serialize → `OpenOptions` write/read/truncate → `write_all` → `flush()` → `sync_all()` → return `Ok(✓)` (`:33-56`)

### Build metadata (`tod/build.rs`)
- Emit `BUILD_TARGET`/`BUILD_PROFILE`/`BUILD_TIMESTAMP` via `cargo:rustc-env=` (`:1-19`)

### Testing & CI (`tod/scripts/test.sh:4-16`, conventions.md)
- Gate order: `cargo fmt --all` → `cargo check` → `cargo clippy --tests -- -D warnings` → `cargo nextest run` → `rg TODO/FIXME/dbg!` gate → `testcfg_clean.sh`
- Integration tests via `assert_cmd::Command::cargo_bin` + `predicates`
- `tempfile` as runtime dependency (not dev-only) for test/prod config paths
- `pretty_assertions` in unit test modules

### Patterns NOT to follow
- `tod` has no `rustfmt.toml` / `clippy.toml` — we'll match that (rustfmt defaults)
- `tod` lacks the `lockf` process gate in `scripts/test.sh` (api/vardy have it) — we'll add it since this is a greenfield project and api/vardy show it's the author's preferred pattern, but **platform-aware**: `lockf` on macOS, `flock` on Linux (see Decision 8)
- `tod`'s `Cargo.toml` uses full 3-part carets (`"1.0.150"`) — we'll match that for consistency

## Design Decisions

1. **Scaffold scope — Full convention spine**: Replicate `tod`'s entire layout in one pass. `errors.rs`, `format.rs`, `commands/mod.rs`, `build.rs`, `scripts/test.sh`, `.github/workflows/`, `rust-toolchain.toml`, `.config/nextest.toml`, and `codecov.yml` all ship with this feature. `init` is the first real command; by the time the second subcommand lands, the project is already a complete CLI skeleton. This is the highest initial cost but the cheapest path for every feature that follows.

2. **Config format — TOML with serialized default struct**: Add the `toml` crate and serialize a `Config` struct on `init`. The file is named `orksorksorks.toml` and the author's projects use TOML for toolchain/nextest config — this is the natural choice for a CLI config file. The `Config` struct starts minimal (an empty struct or a `version` field) and grows as features land. Serialization establishes the load/save pattern from day one.

3. **Async runtime — tokio now**: Add `tokio` with `#[tokio::main]` and use `tokio::fs`. Matches `tod/src/main.rs:57` exactly; every future subcommand that touches the network or parallel I/O inherits the right runtime. Adding tokio later would require migrating the entrypoint — a breaking change to the template.

4. **Error type — Full `Error` struct now**: Ship `Error { message, source }` with `Display` (colored), `Serialize`, `impl std::error::Error`, and `From<io::Error>` + `From<toml::ser::Error>`. This is the error convention the task explicitly calls out as part of the feature's scope ("establish … error handling … conventions"). No thiserror/anyhow — the author never uses them (`research.md` Q3).

5. **Output layer — Full `CommandResult` + JSON envelope**: Ship `-j/--json` as a global flag, `CommandResult` struct, `output_result`/`output_text`/`output_json` functions, and the `✓`-as-returned-data convention. The task says "establish … output … conventions" — this is the output convention. A `-j` flag with no structured data to emit is still correct: `{"data": "✓ Created orksorksorks.toml"}` is valid JSON output.

6. **Dependency versions — Pin full 3-part carets**: Match `tod`'s convention (`research.md` Q5): `clap = { version = "4.6.1", features = ["derive"] }`, `tokio = { version = "1.52.3", features = ["full"] }`, `serde = { version = "1.0.228", features = ["derive"] }`, `serde_json = "1.0.150"`, `colored = "3.1.1"`, `toml = "0.8.23"`, `tempfile = "3.27.0"` (runtime). Dev-deps: `assert_cmd = "2.2.2"`, `predicates = "3.1.4"`, `pretty_assertions = "1.4.1"`.

7. **Toolchain — Pin 1.97.1**: Match `tod` and `vardy`'s channel (`research.md` Q5). `tod/rust-toolchain.toml:3-5` — `channel = "1.97.1"`, `components = ["clippy", "rustfmt"]`. No `rustfmt.toml` or `clippy.toml`.

8. **Test gate — process lock on `~/.cache/pi/test-gate.lock`**: Match api/vardy's convention (`conventions.md` "Canonical commands") — `scripts/test.sh` opens with a lock prologue to serialize test-suite runs. `tod` lacks this, but api/vardy show it's the author's preferred pattern, and a greenfield project should adopt the newer convention. **Platform switch**: detect the host and use `/usr/bin/lockf` on macOS (the api/vardy pattern) and `flock` (util-linux) on Linux — so CI on a Linux runner gets a native lock rather than a missing-binary failure.

## What We're NOT Doing

- **No config file loading/reading** — `init` creates, it does not read. Config loading (`Config::load`) ships when the first subcommand that needs it (e.g., `status`) lands.
- **No auth, no API client, no HTTP** — `init` is purely local. No `reqwest`, no oauth, no `src/todoist/` or equivalent.
- **No `src/config/projects.rs` or multi-file config** — the `Config` struct is a single flat file. No project-scoped config, no `Config::projects`, no glob patterns.
- **No `--force` / `--path` flags** — `init` always creates in `$PWD/orksorksorks.toml`. Flags like `--force` (overwrite) or `--path` (custom location) are out of scope.
- **No spinner, no progress bars, no interactive prompts** — `init` is a one-shot command.
- **No `crates/` workspace or e2e crate** — the project is a single binary crate. An e2e crate follows the same pattern as `tod/crates/tod-e2e` when a live API exists.
- **No `tokio` features beyond `full`** — match `tod/Cargo.toml:28`; revisit when size matters.
- **No `#[ignore]` live-API tests** — there is no API yet.

## Open Risks

1. **`toml` crate is new to the author's dependency graph** — zero direct usage in any `~/dev/` project (`research.md` Q4). The `toml::to_string` API is stable and well-known, but the author has no established pattern for TOML serialization. If `toml` serialization causes friction, fall back to a hand-written header string (Option C from Q2) without adding the dep.

2. **Scope is large for a single feature** — full convention spine means ~15 files created/modified. If the user prefers to land this incrementally, the CI/workflow files can be deferred to a follow-up PR without breaking the `init` command.

3. **`lockf`/`flock` platform split is untested in the author's repos** — api/vardy only ever run `lockf` on macOS; no reference repo has a Linux `flock` path. The `scripts/test.sh` platform switch (`uname`-based) is our own addition, so the Linux branch should be smoke-tested before CI relies on it.

4. **`tokio` `full` feature is heavy** — `tod` uses `features = ["full"]` (`tod/Cargo.toml:28`). For a command that only does file I/O, this is overkill. Acceptable for convention consistency; can be pared down later.

5. **`codecov.yml` ignore list needs thought** — `tod` ignores `errors.rs, input.rs, format.rs, debug.rs, main.rs` (`tod/codecov.yml`). We're building `errors.rs` and `format.rs` now — they should NOT be ignored initially since they have no other coverage. The ignore list should start empty or with only `main.rs` and grow as coverage stabilizes.