# Research Findings

Crate: `orksorksorks` — edtion 2024, binary-only (25-line `Cargo.toml` opens with `[package]` at [Cargo.toml:1]; no `[lib]` target). 6 modules (`src/main.rs:8-13`), 5 integration-test files under `tests/`.

## Q1: Config path resolution end-to-end (`src/config_dir.rs`)

### Findings
- `FILE_NAME = "orksorksorks.toml"` (`src/config_dir.rs:8`).
- `config_file_path(explicit: Option<&Path>) -> Result<PathBuf, Error>` (`src/config_dir.rs:16-22`): `Some(path)` passes the explicit override through unchanged; `None` → `resolve_config_dir().map(|dir| dir.join(FILE_NAME))` (`:20`).
- `resolve_config_dir()` (`src/config_dir.rs:24-46`) — resolution order, every env read:
  1. `std::env::var_os("XDG_CONFIG_HOME")` (`:27`) — consulted on **every platform**. Returned as-is **only if `PathBuf::from(xdg).is_absolute()`** (`:28-30`). Unset (`None`), empty, and relative values all fall through.
  2. `cfg!(windows)` (`:33`): `std::env::var_os("APPDATA")` (`:35-37`) → returned **raw** — no absolute check, no `.join`, no fall-through on empty/relative.
  3. Unix (else branch, `:38-40`): `std::env::var_os("HOME")` → `PathBuf::from(home).join(".config")` — no absolute check.
  4. All exhausted → `Err(Error::new("config-dir", "could not determine a config directory: set XDG_CONFIG_HOME or HOME"))` (`:42-45`).
- The `"config-dir"` tag is a **hand-constructed** `Error::new` (`src/errors.rs:18-23`), not a `From` conversion — unlike `"io"`/`"toml::ser"`/`"toml::de"`.
- Consumers: `select_command` — `Init { config }` arm at `src/commands/mod.rs:69-70`, `Step { config }` arm at `:75-76`, both via `config_file_path(config.as_deref())?`. So the same resolution is the `init` write target and the `step` read target; `--config`/`-c` bypasses it entirely.
- Doc comment (`:10-15`) states XDG wins on every platform; the `APPDATA` asymmetry (no absolute check) is not called out there.
- `init_command` (`src/commands/mod.rs:82-98`) then creates parent dirs, writes default config, flushes and `sync_all()`s.

## Q2: Process-env mutation in unit tests

### Findings — `config_dir::tests` (`src/config_dir.rs:48-114`)
| Test (`:line`) | Mutations | Restored after body? |
|---|---|---|
| `absolute_xdg_is_used` (`:53-58`) | `set_var("XDG_CONFIG_HOME", "/tmp/ork-cfg")` (`:54`) | ✅ removed at `:57` |
| `unset_xdg_falls_back_to_home_dot_config` (`:61-71`) | `remove_var("XDG_CONFIG_HOME")`, `set_var("HOME", "/home/ork")` (`:63-64`) | ❌ never |
| `empty_xdg_falls_back_to_home` (`:74-84`) | `set_var("XDG_CONFIG_HOME", "")`, `set_var("HOME", "/home/ork")` (`:76-77`) | ❌ never |
| `relative_xdg_falls_back_to_home` (`:87-97`) | `set_var("XDG_CONFIG_HOME", "relative/dir")`, `set_var("HOME", "/home/ork")` (`:89-90`) | ❌ never |
| `explicit_path_passthrough` (`:100-103`) | none | — |
| `unresolved_home_yields_config_dir_error` (`:106-113`) | `remove_var("HOME")`, `remove_var("XDG_CONFIG_HOME")` (`:108-109`) | ❌ never |

- Only one of six tests restores; the other four leave `HOME=/home/ork` and/or `XDG_CONFIG_HOME` unset for the remainder of their test process. No RAII guard, no `temp_env`-style restore, no serialization attribute anywhere.
- Fixtures: none of the `config_dir` tests use `tempdir` — they mutate env directly against hardcoded paths (`/tmp/ork-cfg`, `/home/ork`).

### Findings — `select_command_routes_init` (`src/commands/mod.rs:177-190`)
- `let temp = tempfile::tempdir().unwrap()` (`:178`) — the only fixture: a real tempdir.
- `unsafe { std::env::set_var("XDG_CONFIG_HOME", temp.path()) }` (`:180`) — **never removed**; the tempdir also receives the created `orksorksorks.toml` (asserted at `:184-185`) and is auto-cleaned by `tempfile` on drop.
- No other test in `mod.rs` mutates env; `select_command_routes_step` (`:458-473`) routes via an explicit missing `--config` path and expects the `"io"` tag.

### Findings — mid-suite state under nextest
- Under nextest's **process-per-test** model (see Q5) each test runs in a freshly spawned OS process, so the mutations above die with the test process: nothing leaks across tests, and no restore is actually required for *correctness under nextest*. The un-restored mutations only matter to a shared-process harness (libtest threads or `serial_test::serial`), where the ambient env progressively accumulates (`HOME=/home/ork`, `XDG_CONFIG_HOME` unset, then later `XDG_CONFIG_HOME=<tempdir>` if `select_command_routes_init` ran) and test order becomes load-bearing.

## Q3: Test execution, `scripts/test.sh`, CI, nextest config

### Findings
- Gate `scripts/test.sh` (scripts/test.sh:5-19): `cargo fmt --all` → `cargo check` → `cargo clippy --tests -- -D warnings` → `cargo nextest run --no-tests pass` → grep gate for `TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:` (stderr/repo-wide, `:17-19`). Note: local gate runs nextest **without** `--profile ci`.
- CI `ci-pr.yml` (on PRs to main) → `_reusable-lint.yml` (`.github/workflows/_reusable-lint.yml:15,17,19,22`: `cargo check --locked --all-features` `:15`; `cargo fmt --all -- --check` `:17`; `cargo clippy --all-targets --all-features --locked -- -D warnings` `:19`; forbidden-strings rg `:22`) and `_reusable-test.yml` (`.github/workflows/_reusable-test.yml:13-36`): `cargo nextest run --profile ci --all-features --no-tests pass` (`:23`); optional (main-only input `upload-coverage`) `cargo llvm-cov nextest --profile ci --all-features --lcov --output-path lcov.info` (`:26`) → Codecov upload (`:29`), junit upload from `target/nextest/ci/junit.xml` (`:36`).
- `.config/nextest.toml` (all 10 lines): `[profile.ci]` → `retries = 2`, `fail-fast = false`, `slow-timeout = { period = "60s" }`; `[profile.ci.junit]` → path `target/nextest/ci/junit.xml`, `store-success-output = false`, `store-failure-output = true`. Locally installed nextest `cargo-nextest 0.9.143`; `--profile` `:483`, `--retries` `:233`, `--no-tests` `:293` in its `run --help`.
- `--no-tests pass`: exit 0 when zero tests match (avoids failing when a filter matches nothing).
- **Unit-level isolation: yes — nextest is process-per-test.** Every `#[test]` runs in its own OS process (nexte.st/docs/design/why-process-per-test/, how-it-works); libtest's thread-per-test shared-process model is explicitly not used. This is the decisive environment-isolation mechanism for the Q2 mutations.
- Real-subprocess patterns already present:
  - Production code spawns `git`: `current_branch_in` (`src/git.rs:17-25`, `std::process::Command::new("git").args(["branch","--show-current"]).current_dir(dir).output()` at `:18-21`).
  - `git.rs` unit tests spawn real git to build disposable repos: `init_git_branch` helper (`src/git.rs:48-68`) runs `git init`/`checkout -b`/`add`/`commit`; `current_branch_in_detached_head_errors` additionally runs `git checkout --detach` (`:84-88`).
  - Integration tests `tests/*.rs` spawn the **real built binary**: `assert_cmd::Command::cargo_bin("orksorksorks").unwrap()` (tests/artifact_directory.rs:44, tests/branch.rs:37, tests/init_creates_file.rs:26, tests/json_output.rs:15, tests/step.rs:50), with `.current_dir(temp.path())` and per-test env via `.env("XDG_CONFIG_HOME", ...)` (tests/init_creates_file.rs:27) / `.env_remove("HOME")` (tests/init_creates_file.rs:115). These repos are hermetic because CI checks out detached HEAD (tests/artifact_directory.rs:7-10).

## Q4: Dependency-injection conventions

### Findings
- The established pattern: **public fn takes explicit state; a thin wrapper reads ambient env; unit tests call the explicit-state variant**.
  - `git::current_branch()` (`src/git.rs:8-10`) = wrapper: `current_branch_in(&std::env::current_dir()?)` — the `?` maps io failures to `"io"`.
  - `git::current_branch_in(dir)` (`src/git.rs:17-25`) = explicit dir; unit tests point it at disposable repos (`:73-94`).
  - `parse_branch_output(exit_ok, stdout, stderr)` (`src/git.rs:28-36`) = pure decision fn, unit-tested with literals (`:95-116`).
  - `artifact_dir_path(cwd, branch)` (`src/commands/mod.rs:105-112`) = pure string composition (slash→hyphen normalize, trailing-slash contract); wrapper `artifact_directory_command` (`:120-131`) reads `std::env::current_dir()?` (`:121`) + `git::current_branch()?`; unit tests call it with literal path/branch (`:274-301`).
  - `determine_step(config, artifact_dir)` (`src/commands/mod.rs:132-161`) = pure given `Config` + dir string; wrapper `step_command(path)` (`:163-170`) reads cwd (`:165`) and branch from git; unit tests construct `Config`/`Step` literals + `tempfile::tempdir()` (`:302-474`).
  - `config_file_path(explicit: Option<&Path>)` (`src/config_dir.rs:16-22`) = injection via `Option`; the env-reading core is private (`resolve_config_dir` `:24`) and only reachable through tests passing `None`.
- Production cwd reads are always wrapped with `?` so io errors ride `From<std::io::Error>` → `"io"` (`src/errors.rs:39-46`).
- Binary-only layout: no `[lib]` in the 25-line `Cargo.toml` (`Cargo.toml:1`); unit tests are in-module `#[cfg(test)] mod tests` (src/config_dir.rs:48, src/commands/mod.rs:171, src/git.rs:40, src/errors.rs:67, src/config.rs:46, src/format.rs:27); integration tests cannot import crate internals — tests/init_creates_file.rs:20-24 explicitly asserts the serialized default TOML instead of importing `Config`.

## Q5: Facilities for isolating/sharing process-level state

### Findings
- nextest execution model: **each test runs in its own separate OS process**; env/cwd mutations by a test are invisible to every other test and die with the test process (nexte.st/docs/design/why-process-per-test/). The runner spawns test processes from the ambient env — each test starts with the runner's environment snapshot, so un-restored `set_var`/`remove_var` calls are harmless cross-test under nextest.
- Therefore `serial_test::serial` (in-process per-key mutex; crates.io `serial_test`) is a documented **no-op under nextest** — its lock lives in one process, and each nextest test is alone in its process. Cross-process serialization requires `serial_test::file_serial` (file-lock based) or nextest `[test-groups]` entries with `max-threads` (nexte.st/docs/configuration/test-groups/). None are used in this repo.
- Other knobs available in the installed `cargo-nextest 0.9.143`: `-j/--test-threads` (help `:226`), `--retries` (`:233`), `--fail-fast` (`:252`), `-P/--profile` (`:483`), `--no-tests` (`:293`); config-side profiles/overrides/test-groups in `.config/nextest.toml`; JUnit output knobs (`store-*-output`). libtest-side: `--test-threads`/`RUST_TEST_THREADS` (irrelevant under nextest).
- Fixture crates in use: `tempfile` 3.27.0 (`Cargo.toml:15`) — `tempfile::tempdir()` auto-cleans on drop, which is why the read-only-dir tests restore permissions before returning (tests/init_creates_file.rs:40-44, 59-63); `assert_cmd` 2.2.2 (`Cargo.toml:20`) — `Command::cargo_bin` builds the debug binary and spawns it as a child process with `.env()`/`.env_remove()` control; `pretty_assertions`/`predicates` (`Cargo.toml:21-22`) are test-only assertion helpers.
- Limitation surfaced by the search: process creation cost per test, and any *shared* state must move to disk or out-of-process — the repo already follows that (repos/tempdirs on disk, env overrides to children).

## Q6: Every env/cwd read + the central `Error` type

### Findings — `std::env::` reads in `src/`
- Runtime reads: `src/config_dir.rs:27` (`var_os("XDG_CONFIG_HOME")`), `:35` (`var_os("APPDATA")`, Windows-only under `cfg!(windows)` at `:33`), `:38` (`var_os("HOME")`, Unix-only); `src/commands/mod.rs:121` (`current_dir()`, artifact_directory), `:165` (`current_dir()`, step); `src/git.rs:9` (`current_dir()`, branch). That is the complete set — nothing else reads env at runtime.
- Compile-time `env!()` consts (`src/commands/mod.rs:7-17`): `CARGO_PKG_NAME/AUTHORS/DESCRIPTION/VERSION` + `BUILD_TARGET/PROFILE/TIMESTAMP` — the last three are injected by `build.rs:6-15` via `cargo:rustc-env` from cargo `TARGET`/`PROFILE` env and chrono time.
- Test-only mutations: `src/config_dir.rs:54,57,63-64,76-77,89-90,108-109` and `src/commands/mod.rs:180` (Q2 table).
- Which tests exercise each read: `XDG_CONFIG_HOME` — config_dir unit tests (`:53-113`), `select_command_routes_init` (`src/commands/mod.rs:177-190`), integration tests via `cmd.env` (tests/init_creates_file.rs:27/66, tests/json_output.rs:16, tests/step.rs:189-190) and `env_remove` (tests/init_creates_file.rs:115-116); `HOME` — config_dir tests `:61-97`/`:106-113`, integration `init_without_home_fails_with_config_dir_tag` (tests/init_creates_file.rs:111-125); `APPDATA` — **no test coverage anywhere**; `current_dir()` — only covered through the binary path (`cmd.current_dir(...)`) in integration tests; unit tests always use the explicit-state variants instead.

### Findings — `Error` type (`src/errors.rs`)
- `struct Error { message: String, source: String }` (`src/errors.rs:10-13`), derives `Debug, Clone, PartialEq, Eq, Serialize` (`:9`).
- `Error::new(source, message)` (`src/errors.rs:18-23`); `impl fmt::Display` (`src/errors.rs:26-35`) renders `"Error from {yellow source}:\n{red message}"` (`:30-33`); `impl std::error::Error` (`:37`).
- `From` conversions: `std::io::Error` → `"io"` (`src/errors.rs:39-46`), `toml::ser::Error` → `"toml::ser"` (`src/errors.rs:48-55`), `toml::de::Error` → `"toml::de"` (`src/errors.rs:57-64`). Consumed via `?` at: `src/git.rs:9`, `src/config.rs:40-41`, `src/commands/mod.rs:69,75` (config-dir Result), `src/commands/mod.rs:83-93` (init I/O + `toml::to_string`).
- All source tags in the tree: `"io"`, `"toml::ser"`, `"toml::de"`, `"config-dir"` (`src/config_dir.rs:43`), `"git"` (`src/git.rs:32,38`), `"step"` (`src/commands/mod.rs:158`).
- Rendering (`src/main.rs`): text mode — Ok → stdout via `println!` (`:30`), Err → stderr with `\n\n` prefix (`:33`); JSON mode — Ok → `serde_json::json!({"data": ...})` + stdout (`:42-43`), Err → `{"error": {"message", "source"}}` + stdout (`:46-47`), **both printed to stdout**. Exit code: `std::process::exit(1)` on Err (`:77`). ANSI stripped under `cfg!(test)` (`src/format.rs:8-14`) — the single color chokepoint.
- `Error` unit tests cover display, each `From`, JSON round-trip, PartialEq, and std::error::Error impl (`src/errors.rs:67-139`).

## Cross-Cutting Observations
- **Process-per-test is the load-bearing isolation fact of this repo**: nextest (pinned toolchain `rust-toolchain.toml`, `channel = "1.98.1"`) gives every unit test its own process, so the un-restored env mutations in Q2 are currently harmless — but the tests are written as if they shared a process, which only holds under libtest-length assumptions and has no restore discipline.
- The codebase consistently separates **pure/composed functions with explicit state** from **thin env-reading wrappers** (Q4); unit tests target the pure layer, integration tests target the binary.
- `tests/` files exercise the whole binary through spawned processes with per-process env/cwd overrides — the same isolation boundary nextest gives unit tests, applied externally (assert_cmd).
- Color is a compile-time concern (`cfg!(test)`, `src/format.rs:8`); JSON errors carry `source` as a structured field while text errors rely on ANSI.
- No Makefile at repo root; the project gate is `scripts/test.sh`, mirrored by the two reusable CI workflows.

## Open Areas
- `APPDATA` branch (`src/config_dir.rs:35-37`) has no test coverage (Windows-only; no `cfg!(windows)` tests exist) — its raw-return-no-absolute-check behavior is untested.
- The exact semantics of `tempfile::tempdir()` cleanup-on-drop failure modes (beyond the tests' deliberate permission-restore comments, `tests/init_creates_file.rs:40-44`) are not documented in-tree.
- nextest `slow-timeout` (`period = "60s"`) and the junit `store-failure-output` settings in `.config/nextest.toml` are configured but never validated by repo tests; the local `scripts/test.sh` doesn't use the `ci` profile, so those keys only bite in CI.