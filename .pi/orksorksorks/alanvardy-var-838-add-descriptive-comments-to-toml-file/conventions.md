# Conventions

## Canonical commands

Gate (local): `./scripts/test.sh` — runs, in order (scripts/test.sh):
1. `cargo fmt --all`
2. `cargo check`
3. `cargo clippy --tests -- -D warnings`
4. `cargo nextest run --no-tests pass`
5. Forbidden-strings gate: `rg -i -g '*.rs' 'TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:' .` must find nothing, else exit 1

CI lint (`.github/workflows/_reusable-lint.yml`): `cargo check --locked --all-features` → `cargo fmt --all -- --check` → `cargo clippy --all-targets --all-features --locked -- -D warnings` → same forbidden-strings rg (as a `! rg ...` negation).

CI test (`.github/workflows/_reusable-test.yml`): `cargo nextest run --profile ci --all-features --no-tests pass`; coverage on main only: `cargo llvm-cov nextest --profile ci --all-features --lcov --output-path lcov.info` → codecov upload. Tools installed via `taiki-e/install-action`: `cargo-nextest`, `cargo-llvm-cov`.

Install: `./scripts/install.sh` → `cargo install --path . --locked`.

Toolchain: `rust-toolchain.toml` — channel `1.98.1`, components `clippy`, `rustfmt`. Edition 2024 (`Cargo.toml`). Binary-only crate (`crate = "orksorksorks"`, **no lib target** — integration tests cannot import `Config`/private items).

## Test-suite inventory

| Path | Covers | Notes |
|---|---|---|
| `tests/init_creates_file.rs` | `init` end-to-end: default content, success message, readonly errors, `--config` flag, HOME-missing tag | Only file with byte-exact content assertions — exactly two: `"version = \"0.1.0\"\nshow_frontmatter = true\n"` at lines 22-26 and 104-108. 6 tests, all inline, `#![allow(clippy::permissions_set_readonly_false)]` (line 4) |
| `tests/model.rs`, `step.rs`, `prompt.rs`, `script.rs` | `model`/`step`/`prompt`/`script` subcommands against git-repo fixtures | Per-file `init_git_repo()` (lines 12-22) + `write_config(dir)` (prompt.rs:35-80, script.rs:34-98) building TOML via `concat!` literals incl. `"""` multi-line content; `artifact_dir(dir)` → `dir/.pi/orksorksorks/main` |
| `tests/artifact_directory.rs`, `branch.rs` | artifact-dir/branch derivation | `git_repo_on_branch(branch)` (17-41) commits with `-c user.name/email`; macOS `/var`→`/private/var` canonicalized (artifact_directory.rs:39-43) |
| `tests/config_validation.rs` | error tags/exit codes for invalid configs | Single `assert_invalid_config` helper (8-77); feeds hand-written configs, asserts `v["error"]["source"]` tags (`config:*` etc.) |
| `tests/json_output.rs`, `format.rs` unit tests | JSON output mode, ANSI handling | Every test asserts no `\x1b` in stdout; `cfg!(test)` color strip in src/format.rs:5-17; unit tests use `pretty_assertions::assert_eq` in `#[cfg(test)]` modules (src/format.rs:18-32) |
| `src/config.rs` unit tests (~247-769) | serde/round-trip/validate behavior incl. `default_config_serializes_to_expected_toml` (260-266), `unknown_step_key_is_rejected_as_toml_de` (527-537), `validate_errors_are_deterministic_first_error_wins` (727-738), `validate_accepts_unreferenced_prompts_and_models` (543-558) | Config substring-pins `"0.1.0"` and `show_frontmatter = true` |

Platform gating: no `cfg!(windows)` inside tests; Windows surface is source-side only (`ConfigEnv::from_env` APPDATA branch, src/config_dir.rs:29-34). IO/format error tags pinned by tests: `io`, `config-dir`, `toml::ser`, `toml::de`, `config:version|duplicate-name|empty-name|empty-model|duplicate-trigger|multiple-default|missing-prompt|missing-model` (docs contract src/config.rs:135-138; note `config:duplicate-trigger` and `config:multiple-default` are not in the docs list's suffix naming but are live tags).

## Build/verify gotchas
- `cargo clippy` is the toolchain-wrapped form (not plain `cargo clippy` upstream) — `D warnings` requirement; use `--all-targets --all-features --locked` to match CI.
- Tests run via `cargo nextest` (nextest harness), not `cargo test`; `--profile ci` sets `retries = 2`, `fail-fast = false`, `slow-timeout` 60s, JUnit at `target/nextest/ci/junit.xml` (`.config/nextest.toml`).
- `codecov.yml`: project coverage `auto`/`threshold 10%`; patch target `50%`; `src/main.rs` ignored.
- Forbidden-string gate rejects `TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:` in any `.rs` — comments containing these are illegal (relevant when writing descriptive comments in source).
- Integration tests invoke the built binary via `Command::cargo_bin("orksorksorks")` and pass config explicitly as `--config orksorksorks.toml` from the repo root; without the flag there is **no** fallback to a CWD config (step.rs:292-301); clap usage errors pinned as exit code 2 (prompt.rs:362).
- Dev-deps available: `assert_cmd 2.2.2`, `predicates 3.1.4`, `pretty_assertions 1.4.1`; main deps: `clap 4.6.6 (derive)`, `colored 3.1.1`, `serde 1.0.229 (derive)`, `serde_json 1.0.151`, `tempfile 3.27.0`, `tokio 1.53.1 (full)`, `toml 1.1.5`; build-dep `chrono 0.4.45` (`Cargo.toml`).
- The `toml` crate serializer cannot emit comments (`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/toml-1.1.5+spec-1.1.0/`, zero comment support in `src/ser/`); `toml_edit` is not a dependency (absent from Cargo.lock) but IS present in the local registry cache (0.25.13, 0.22.27) if a dependency change were ever considered.
- Working tree hygiene: `git status` shows a pre-existing tracked file `DELETEME` deleted (uncommitted) and this artifact dir untracked — do not confuse with task-side changes.