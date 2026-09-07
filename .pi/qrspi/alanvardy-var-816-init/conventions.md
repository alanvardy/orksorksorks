# Conventions Appendix

Reference roots: `~/dev/tod` (CLI, primary), `~/dev/api`, `~/dev/vardy` (web APIs). Current project (`orkworksorks`) has no conventions of its own — these are the author's patterns as documented from the reference repos. Paths below are relative to the given repo root (`tod/…` = `~/dev/tod/…`).

## Canonical commands

### Gate order — this is the author's test suite entrypoint
`tod/scripts/test.sh:4-16` runs, in order:
1. `cargo fmt --all` (writes, not `--check`) — test.sh:5
2. `cargo check` — test.sh:7
3. `cargo clippy --tests -- -D warnings` — test.sh:9
4. `cargo nextest run` — test.sh:11
5. rg "FORGOTTEN TODOS" gate: `rg -i -s -g '*.rs' 'TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:'` (fails on any hit) — test.sh:12-14
6. `./scripts/testcfg_clean.sh` (deletes `tests/*.testcfg`, testcfg_clean.sh:8-10) — test.sh:16

api/vardy `scripts/test.sh` prepend a **process lock** (one test-suite-run-at-a-time, shared across repos):
`/usr/bin/lockf` on `$HOME/.cache/pi/test-gate.lock` (api scripts/test.sh:1-13, vardy scripts/test.sh:1-13), then `cargo fmt --all` → `cargo sqlx prepare -- --tests` (api) → `cargo check --all-targets`/`--tests` → (vardy only) `./scripts/build-css.sh` + `git diff --exit-code -- static/site.css` → `cargo clippy --all-targets --all-features --locked -- -D warnings` → `cargo nextest run` → forbidden-strings rg (api :15-27, vardy :14-22).

### CI gates (parity with local)
- `cargo fmt --all -- --check` — tod `.github/workflows/_reusable-lint.yml:43`, api/vardy `ci.yml`
- `cargo clippy --all-targets --all-features --locked -- -D warnings` — `_reusable-lint.yml:57`, api/vardy `ci.yml`
- `cargo check --locked --all-features` — tod only, `_reusable-lint.yml:29`
- `cargo nextest run --profile ci --all-features` — `_reusable-test.yml:64`; coverage on main: `cargo llvm-cov nextest --profile ci --all-features --lcov --output-path lcov.info` (`:67`)
- Codecov upload of `lcov.info` + `target/nextest/ci/junit.xml` (report_type test_results) — `_reusable-test.yml:71-87`
- rg gate for TODO/FIXME/dbg — `_reusable-lint.yml:66-68`; api/vardy via `scripts/lint_string.sh`
- api CI env: `CARGO_TERM_COLOR=always`, `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_PROFILE_RELEASE_LTO=true`, `CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1`, `RUSTFLAGS="-C link-arg=-fuse-ld=mold"`, `SQLX_OFFLINE=true`

### Toolchain
- `rust-toolchain.toml`: `[toolchain] channel = "1.97.1", components = ["clippy", "rustfmt"]` (tod/vardy); api pins 1.98.0. CI uses `dtolnay/rust-toolchain@stable` (deviation from local pin).
- **No** rustfmt.toml / clippy.toml / .cargo/config.toml in any project — defaults everywhere.
- Crate-root lint: `#![warn(missing_docs)]` in `tod/src/main.rs:4` only (CLI convention: doc comments are the help text, so missing_docs enforces it).
- edition 2024, caret-version deps (tod: full 3-part `"1.0.150"`; api/vardy: loose `"0.5"`/`"1"`), no `[workspace]`/`[lib]`/`[[bin]]`, `crates/tod-e2e` is standalone (own Cargo.lock; build with `--manifest-path crates/tod-e2e/Cargo.toml`).

## Test-suite inventory

### tod — integration tests (`tod/tests/`, drive compiled binary via `assert_cmd::Command::cargo_bin`)
| File | Covers | Platform gating |
|---|---|---|
| `tests/config_create.rs` | 13 tests: `auth token` create/update, `config check/about`, `config reset --force` delete paths | none |
| `tests/config_open.rs` | 3 tests: editor round-trip via `VISUAL`/`EDITOR` = no-op (`noop_editor()` :4; Windows uses `cmd.exe /C rem`) | none |
| `tests/json_output.rs` | 14 tests: `-j/--json` contract, error JSON shape, no-ANSI, jq pipe; `json_*_returns…` variants | 3 tests `#[ignore = "requires live Todoist API or mockito server"]` (:66,92,118) |
| `tests/responses/*.json` | 19 API-response fixtures consumed by `src/test/responses.rs` | — |

### tod — inline `#[cfg(test)]` unit modules (selected; full set ~35 modules)
| Module | Tests | Notes |
|---|---|---|
| `src/main.rs:144-149` | 1 | `verify_cmd`: `Cli::try_parse().err(); Cli::command().debug_assert();` — the clap smoke test |
| `src/format.rs:129` | 17 | text/date formatting |
| `src/input.rs:310` | 20 | natural-language parsing |
| `src/time.rs:16,254` | 19 | fixed clock via `src/test_time.rs` `FixedTimeProvider` |
| `src/tasks/mod.rs:1107` (`:2269` proptests) | 11 + proptest | serde via `serde_test`, mockito API, property round-trips |
| `src/lists.rs:322` | many `#[tokio::test]` | mockito-heavy; `test_import_creates_14_tasks` :332 |
| `src/errors.rs:192` | 7 | Display string, `From` tags |
| `src/commands/mod.rs:677` | 18 | dispatch/selection |
| `src/config/file.rs:248` | 3 | write/read via `tempfile::Builder` (:95) + mockito (:281) |
| `src/tasks/format.rs:213`, `src/priority.rs:57`, `src/labels.rs:68`(+proptest :127), `src/projects.rs:732`, `src/update.rs:138`, `src/regexes.rs:24`, `src/shell.rs:100` (assert_cmd+gag, stdout capture :268,273), `src/cargo.rs:63` (mockito crates.io) | small | `pretty_assertions` in most |
| `src/test/mod.rs` | fixtures | `#[cfg(test)] pub mod fixtures; pub mod responses;` — shared builders |

### tod-e2e live-API crate
- `crates/tod-e2e/tests/e2e_todoist.rs` — 20 `#[test]`, subprocess-driven: `tod()` sets `DISABLE_SPINNER=1` (:31-35), `tod_binary_path()` honors `TOD_E2E_TOD_BIN` or `cargo build --manifest-path` (:41-66); per-run fresh config in `tempfile::TempDir`; fixtures `e2e_todoist_fixtures/TOD_DEV_CI_DYNAMIC.csv`, `TOD_DEV_CI_STATIC_READ.csv`. Requires `TOD_E2E_TOKEN` + static/dynamic projects. Run only via `e2e_todoist.yml` (workflow_dispatch) — not part of `scripts/test.sh`.

### api / vardy
- `src/test/arkitect.rs` — architectural-rules test w/ `rust_arkitect` (dev-dep `api/Cargo.toml:48`, `vardy/Cargo.toml:26`; pulls `toml` 0.8.23 transitively). api/vardy have ~88 `#[cfg(test)]` sites in src + `scripts/test.sh` lockf gate.

## Build/verify gotchas
- **One test-suite run at a time**: api/vardy serialize via `lockf` on `~/.cache/pi/test-gate.lock` (api scripts/test.sh:1-13). tod's scripts/test.sh does **not** lock.
- `cargo nextest run` is the runner everywhere (not `cargo test`); CI profile via `.config/nextest.toml` (tod: retries=2, fail-fast=false, slow-timeout 60s).
- `Cargo.lock` committed (tod/api/vardy have lockfiles); CI uses `--locked`; `cargo sqlx prepare -- --tests` gate only in sqlx repos (api/vardy).
- Coverage requires `cargo-llvm-cov` + `nextest`, installed via `taiki-e/install-action` in CI; coverage runs on main only.
- Codecov ignore lists are per-repo `codecov.yml` (tod ignores errors/input/format/debug/main, `target: auto, threshold: 10%`, patch 50%).
- `build.rs` emits `BUILD_TARGET`/`BUILD_PROFILE`/`BUILD_TIMESTAMP` (`tod/build.rs:11-17`) consumed as env! in `LONG_VERSION` — a build that strips build.rs changes --version/about output.
- ANSI colors are stripped under `cfg!(test)` (`tod/src/format.rs:8-13`) — don't assert raw ANSI in unit tests; integration tests assert text output contains no escape codes (`tests/json_output.rs:298`).
- `Error::new(source, message)` requires borrows for `format!` results (`&format!(...)`, `tod/src/errors.rs:181-190`).
- Success strings are returned as data: config `save()` returns `Ok(format::green_string("✓"))` (`tod/src/config/file.rs:54`) and the ✓ is printed by the generic output layer — do not print success separately in handlers.