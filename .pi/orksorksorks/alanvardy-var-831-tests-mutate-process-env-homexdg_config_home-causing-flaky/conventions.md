# Conventions — orksorksorks (edtion 2024, binary-only Rust crate)

## Canonical commands

- **Full local gate**: `./scripts/test.sh` (scripts/test.sh:4-19), in order:
  1. `cargo fmt --all` (`:5`)
  2. `cargo check` (`:8`)
  3. `cargo clippy --tests -- -D warnings` (`:11`)
  4. `cargo nextest run --no-tests pass` (`:14`) — **no `--profile`** locally
  5. forbidden-strings gate: `rg -i -g '*.rs' 'TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:'` — must match nothing, else exit 1 (`:17-19`)
- **Install**: `./scripts/install.sh` → `cargo install --path . --locked` (scripts/install.sh:6)
- **CI (PR to main)** — `.github/workflows/ci-pr.yml` fans out to:
  - `.github/workflows/_reusable-lint.yml:15,17,19,22`: `cargo check --locked --all-features`; `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features --locked -- -D warnings`; forbidden-strings rg.
  - `.github/workflows/_reusable-test.yml:23`: `cargo nextest run --profile ci --all-features --no-tests pass`.
  - Coverage upload only when `upload-coverage: true` (main-only): `cargo llvm-cov nextest --profile ci --all-features --lcov --output-path lcov.info` (`:26`) → `codecov/codecov-action@v5` (`:29`) → `codecov/test-results-action@v1` on `target/nextest/ci/junit.xml` (`:36`).
- **Toolchain**: rust-toolchain.toml pins `channel = "1.98.1"`, clippy + rustfmt components. Locally installed runner: `cargo-nextest 0.9.143`.
- No Makefile at repo root.

## `.config/nextest.toml` (all 10 lines)
- `[profile.ci]`: `retries = 2`, `fail-fast = false`, `slow-timeout = { period = "60s" }`.
- `[profile.ci.junit]`: `path = "target/nextest/ci/junit.xml"`, `store-success-output = false`, `store-failure-output = true`.
- Local `scripts/test.sh` runs the **default** profile (retries 0), so the `ci` profile only applies in CI. Only profile-CI coverage runs go through `cargo llvm-cov nextest` with `--profile ci`.

## Test-suite inventory

### Unit tests — in-module `#[cfg(test)] mod tests`, run via `cargo nextest run`
| File | Coverage | Platform/environment gating |
|---|---|---|
| `src/config_dir.rs:48-114` (6 tests) | `config_file_path`/`resolve_config_dir`: XDG absolute, unset/empty/relative fallback, HOME path, explicit passthrough, missing-HOME error tag | None — but **mutates process env** (see research Q2); no `cfg!(windows)` test for `APPDATA` |
| `src/commands/mod.rs:171-397` (~30 tests) | `select_command` routing (init, step), clap parsing (short/long `-c`/`--config`, kebab rejection), `artifact_dir_path` composition (trailing slash, slash normalization), `determine_step` priority/default rules | `select_command_routes_init` (`:177-190`) sets `XDG_CONFIG_HOME` (never unset); no platform gating |
| `src/git.rs:40-116` (5 tests) | `current_branch_in` (real `git` subprocesses against disposable repos, `:48-68`), detached-HEAD error, `parse_branch_output` purity | **Requires `git` on PATH**; repos built via `std::process::Command`, hermetic vs ambient checkout (CI leaves detached HEAD) |
| `src/errors.rs:67-139` (7 tests) | Display text, all three `From` tags, JSON round-trip, PartialEq, `std::error::Error` impl | None |
| `src/config.rs:46-127` (7 tests) | default serialization, round-trip, missing steps, `read_config` happy + `"io"` + `"toml::de"` tags | None; uses `tempfile::tempdir()` |
| `src/format.rs:27-49` (4 tests) | plain-string color helpers under `cfg!(test)` | `cfg!(test)` compile-time (ANSI stripped, `:8`) |

### Integration tests — `tests/*.rs`, spawn the real built binary
Each file is its own test binary; under nextest each test = one process. Pattern: `assert_cmd::Command::cargo_bin("orksorksorks").unwrap()` + `.current_dir(<temp git repo>)` + optional `.env(...)`/`.env_remove(...)`.

| File | Coverage |
|---|---|
| `tests/branch.rs` (3) | `branch` plain + `-j` JSON, failure outside repo |
| `tests/artifact_directory.rs` (3) | composed path + trailing slash + `-j`, failure outside repo; `std::fs::canonicalize` to match binary's cwd view (`:38-45`) |
| `tests/init_creates_file.rs` (6) | default TOML content, success message, read-only-dir failure + `"io"` in `-j` error envelope, `--config` override ignores hostile `XDG_CONFIG_HOME`, missing HOME+XDG → `"config-dir"` tag (`env_remove` at `:115-116`) |
| `tests/json_output.rs` (2) | `init -j` valid JSON `"data"`; `init` plain text, no ANSI |
| `tests/step.rs` (7) | step resolution, `-j`, no-artifact failure, default step, real-beats-default, config-dir (`--config` omitted) read via `XDG_CONFIG_HOME`, cwd config ignored |

Assertion/helper dev-deps: `assert_cmd` 2.2.2 (`Cargo.toml:20`), `predicates` 3.1.4 (`Cargo.toml:21`), `pretty_assertions` 1.4.1 (`Cargo.toml:22`); fixture crate `tempfile` 3.27.0 (`Cargo.toml:15`).

## Build/verify gotchas
- **Binary-only crate** (no `[lib]`; file is 25 lines, `[package]` at `Cargo.toml:1`): integration tests cannot import crate types — `tests/init_creates_file.rs:20-24` asserts raw serialized TOML instead. Unit tests live in-module only.
- **Env-mutation discipline**: `config_dir` tests and `select_command_routes_init` mutate env with `unsafe { std::env::set_var/remove_var }`; only `absolute_xdg_is_used` restores (src/config_dir.rs:54→57). Harmless under nextest process-per-test; would be order-dependent under libtest/`serial_test`.
- **Color**: never assert ANSI — helpers strip it under `cfg!(test)` (`src/format.rs:8-14`); assertions use plain substrings.
- **JSON errors print to stdout** (`src/main.rs:46-47`), exit code 1 (`:77`); integration tests read the error envelope from stdout.
- **Tempdirs must be deletable at drop**: read-only-dir tests deliberately restore permissions before returning so `tempfile` can clean up (tests/init_creates_file.rs:40-44, 59-63).
- **Git subprocess tests need real `git`** and use `-c user.name/-c user.email` config for commits (src/git.rs:61-64, tests/artifact_directory.rs:25-28); never rely on the ambient checkout (detached HEAD in CI).
- **Forbidden strings** are gate-breaking in both local (`scripts/test.sh:17-19`) and CI lint (`.github/workflows/_reusable-lint.yml:22`): `TODO:`, `todo:`, `FIXME`, `fixme`, `dbg!`, `DEBUG:`, `FIXTURE:`.
- Filtered/partial runs should use `cargo nextest run --no-tests pass <filter>` (pass keeps exit 0 when nothing matches).