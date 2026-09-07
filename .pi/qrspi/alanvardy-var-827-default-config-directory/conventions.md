# Conventions — factual appendix for Design / Structure / Plan

Crate: `orksorksorks` 0.1.0 (`Cargo.toml:2`), edtion 2024 (`Cargo.toml:3`), binary-only. Toolchain: `rust-toolchain.toml` channel `1.98.1`, components `["clippy", "rustfmt"]`.

## Canonical commands

Local gate — `scripts/test.sh` (order matters, `set -euo pipefail` at `:2`):
1. `cargo fmt --all` (`scripts/test.sh:6`)
2. `cargo check` (`:9`)
3. `cargo clippy --tests -- -D warnings` (`:12`)
4. `cargo nextest run --no-tests pass` (`:15`)
5. Forbidden-string gate: `rg -i 'TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:' -g '*.rs'` fails the run if matched (`:18-20`)

CI (`.github/workflows/ci-pr.yml:9,11` → reusable workflows):
- Lint `.github/workflows/_reusable-lint.yml`: `cargo check --locked --all-features` (`:15`), `cargo fmt --all -- --check` (`:17`), `cargo clippy --all-targets --all-features --locked -- -D warnings` (`:19`)
- Test `.github/workflows/_reusable-test.yml`: installs `cargo-nextest,cargo-llvm-cov` via taiki-e (`:19-21`), `cargo nextest run --profile ci --all-features --no-tests pass` (`:23`), `cargo llvm-cov nextest --profile ci --all-features --lcov --output-path lcov.info` (`:26`), codecov upload (`:29`), junit upload from `target/nextest/ci/junit.xml` (`:34-36`)

Install — `scripts/install.sh`: `cargo install --path . --locked` (`:6`); install location is never computed, only echoed (`:8`).

Nextest profile — `.config/nextest.toml`: `[profile.ci]` retries = 2, fail-fast = false, slow-timeout 60s (`:2-4`); junit to `target/nextest/ci/junit.xml` (`:6-8`).

## Build metadata

`build.rs` (build-dependency `chrono`, `Cargo.toml:23`) emits `cargo:rustc-env=BUILD_TARGET={TARGET}` / `BUILD_PROFILE={PROFILE}` / `BUILD_TIMESTAMP={now}` (`build.rs:5-13`). These feed the `env!` constants in `src/commands/mod.rs:5-16` (`NAME`, `AUTHOR`, `ABOUT` from `CARGO_PKG_*`; `LONG_VERSION` concatenates version + build metadata), used by the clap root-command attrs (`mod.rs:20-26`).

## Test-suite inventory

Integration tests (`tests/*.rs` — compiled as separate binaries; **cannot import crate items** because there is no `[lib]`/`src/lib.rs`; comment at `tests/init_creates_file.rs:22-23`):

| File | Test (line) | Covers | Platform gating |
|---|---|---|---|
| `tests/init_creates_file.rs:11` | `init_creates_orksorksorks_toml_with_default_content` | file created at CWD; **exact** content `version = "0.1.0"\n` (`:24-25`) | none |
| `tests/init_creates_file.rs:29` | `init_returns_success_message` | stdout contains `✓ Created orksorksorks.toml` via `predicate::str::contains` (`:37`) | none |
| `tests/init_creates_file.rs:41` | `init_in_readonly_dir_returns_error_exit_code` | readonly tempdir → `.failure()` (`:50`) | none |
| `tests/init_creates_file.rs:59` | `init_json_error_in_readonly_dir_shows_source_io` | JSON stdout contains `"source":"io"` (`:72`); exit non-success (`:69`) | none |
| `tests/json_output.rs:4` | `init_json_returns_valid_json_with_data_field` | `-j` → valid JSON envelope with `"data"` (parsed via `serde_json::from_str`, `:17`) | none |
| `tests/json_output.rs:21` | `init_no_json_prints_plain_text` | plain text has no `\x1b` ANSI (`:33`) | none |

In-module unit tests (`#[cfg(test)] mod tests`, `use super::*` — can reach private internals):

| File (mod) | Test (line) | Covers |
|---|---|---|
| `src/commands/mod.rs:64` | `select_command_routes_init` (`:70`) — **calls `init_command()` in-process, writes `orksorksorks.toml` into the runner CWD**; `cli_try_parse_rejects_no_subcommand` (`:80`); `cli_try_parse_accepts_init` (`:87`); `cli_command_debug_assert` (`:94`, uses `clap::CommandFactory::command().debug_assert()`) |
| `src/config.rs:21` | `default_config_serializes_to_expected_toml` (`:26`); `config_round_trip_serialize_deserialize` (`:34`) |
| `src/errors.rs:57` | `display_includes_source_and_message` (`:62`); `from_io_error_tags_io` (`:71`); `from_toml_ser_error_tags_toml_ser` (`:79`, builds a failing `serde::Serialize` impl to force `toml::ser::Error`); `serialize_round_trip` (`:93`); `partial_eq` (`:101`); `error_trait_impl` (`:110`) |
| `src/format.rs:26` | `green_string_is_plain` (`:31`); `red_string_is_plain` (`:36`); `yellow_string_is_plain` (`:41`); `all_helpers_strip_ansi_under_test` (`:46`) |

## Fixture patterns (must be preserved or consciously changed)

- Temp dir: `tempfile::tempdir().unwrap()` (regular dep, `Cargo.toml:15`), used at `tests/init_creates_file.rs:12,30,42,60`.
- Redirect child CWD: `Command::cargo_bin("orksorksorks").unwrap()` + `cmd.current_dir(temp.path())` (`init_creates_file.rs:13-14,31-32,47-48,65-66`). `json_output.rs` sets **no** `current_dir` — runs in repo root.
- Readonly setup/restore (both readonly tests): `fs::metadata(path).permissions()` → `perms.set_readonly(true)` → `fs::set_permissions(...)` (`init_creates_file.rs:43-45,61-63`), then set_readonly(false) restore (`:53-55,75-77`) so `tempfile` can clean up. File-level `#![allow(clippy::permissions_set_readonly_false)]` at `:1-4`. `std::fs` here is the prelude std, and the crate's own `std::fs` import in `init_creates_file.rs:8` is that same module.
- No env vars are ever set on spawned children (no `.env(...)` anywhere).

## Dependencies (`Cargo.toml:10-23`)

Regular: clap 4.6.6 (`features=["derive"]`, `:11`), colored 3.1.1 (`:12`), serde 1.0.229 (`features=["derive"]`, `:13`), serde_json 1.0.151 (`:14`), tempfile 3.27.0 (`:15`), tokio 1.53.1 (`features=["full"]`, `:16`), toml 1.1.5 (`:17`). Dev: assert_cmd 2.2.2 (`:19`), predicates 3.1.4 (`:20`), pretty_assertions 1.4.1 (`:21`). Build: chrono 0.4.45 (`:23`). **No dirs/fs/path/home crate; no `env` reads in src.**

## Build/verify gotchas surfaced by research

- **Binary-only + in-module split**: integration tests must hardcode expected strings (e.g. exact toml serialization `version = "0.1.0"\n` at `init_creates_file.rs:24`); anything needing `Config` internals must be a unit test.
- **`select_command_routes_init` writes into the runner CWD** (`mod.rs:57`, `:70-78`) — a side effect that pollutes the test directory; any change to the write location changes this test's behavior.
- **Color stripping depends on `cfg!(test)`**: `apply_color` returns input unchanged under test (`src/format.rs:7-9`), which is why `json_output.rs:33` can assert no ANSI and `mod.rs:77` can assert the exact success string. Building a test binary outside the test cfg would break these.
- **`cargo_bin` requires the binary to exist** — assert_cmd locates the compiled artifact; vs `cargo nextest` profile building it (`_reusable-test.yml:23`).
- **Tests run under nextest, not cargo test** — `.config/nextest.toml`, `scripts/test.sh:15`, CI `_reusable-test.yml:23`. Nextest honors `#[test]`/`#[cfg(test)]` the same, but the runner/profile differs (retries=2, junit path).
- **Committed `orksorksorks.toml` at repo root is byte-identical to init's output** (`orksorksorks.toml:1` = `version = "0.1.0"`); running init truncates/rewrites it — tests in the repo root (`json_output.rs`) therefore exercise the overwrite path against tracked content.
- **Environment is compile-time only**: `env!` (`mod.rs:5-16`) and `build.rs:6,10,11` — there is no runtime env plumbing to reuse; any test-time env override would be new infrastructure.
- **Exact line anchors that keep shifting**: `init_command` is `src/commands/mod.rs:52-62` (task framing said 48-61); `select_command` is `mod.rs:45-49`; `main` entry is `src/main.rs:91-95`.