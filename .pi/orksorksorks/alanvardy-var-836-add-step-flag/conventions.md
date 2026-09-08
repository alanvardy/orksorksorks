# Conventions — factual appendix

Dense reference sheet for Design/Structure/Plan. Branch markers: bare = current branch (`alanvardy-var-836-add-step-flag`, source-identical to `main`); `amc:` = `add-model-command` (unmerged, tip `333ab82`).

## Canonical commands

### Local gate — `scripts/test.sh` (bash)
Pipefail, runs in order, first failure aborts:
1. `cargo fmt --all` (line 7)
2. `cargo check` (line 9)
3. `cargo clippy --tests -- -D warnings` (line 11) — note `cargo clippy`, a custom cargo subcommand, not `cargo clippy`/`clippy`
4. `cargo nextest run --no-tests pass` (line 13)
5. TODO/FIXME/dbg gate: `rg -i -g '*.rs' 'TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:' .` → exit 1 if any match (line 16)

### CI — `.github/workflows/`
- `_reusable-test.yml:8` single job on `ubuntu-latest`; `:22-23` runs `cargo nextest run --profile ci --all-features --no-tests pass`; `:26` `cargo llvm-cov nextest --profile ci --all-features --lcov --output-path lcov.info`; `:32-36` uploads `target/nextest/ci/junit.xml` via `codecov/test-results-action@v1`.
- `_reusable-lint.yml` and `ci-pr.yml` wrap the reusable jobs; CI is the only platform gate (no cfg(windows) gates anywhere in tests).
- `cargo llvm-cov` and `cargo clippy` are custom cargo subcommands provided by tooling (not in Cargo.toml).

### nextest profile — `.config/nextest.toml`
- `[profile.ci]` `retries = 2`, `fail-fast = false`, `slow-timeout = { period = "60s" }` (lines 1-4).
- `[profile.ci.junit]` path `target/nextest/ci/junit.xml`, `store-success-output = false`, `store-failure-output = true` (lines 6-10).

## Test-suite inventory

Unit tests live in `#[cfg(test)] mod tests` inside each `src/` file (e.g. `src/commands/mod.rs:180+`, `src/config_dir.rs`, `src/git.rs`, `src/config.rs`); integration tests live in `tests/`. No `tests/helpers.rs` — every integration file duplicates its own helpers (`init_git_repo` copy-pasted at `tests/step.rs:7`, `amc:tests/model.rs:7`, `amc:tests/prompt.rs:7`).

| Path | Covers | Platform gating |
|---|---|---|
| `tests/step.rs` (10 tests) | `step` command: derives step from triggers, default step, JSON `-j`, config-dir fallback, no-match error (source `"step"`), CWD-config-ignored regression, ANSI-free output | none (works in any git repo; uses unborn branch via `git init -b main`) |
| `tests/branch.rs` | `branch` command; committed temp repo via `git_repo_on_branch` (detached-HEAD-hermetic rationale at `:3-5`) | none |
| `tests/artifact_directory.rs` | `artifact_directory` output; macOS `/var`→`/private/var` canonicalization inside helper `expected_artifact_directory` (`:36-38`, `std::fs::canonicalize`) | macOS-only workaround lives in the test, not gated |
| `tests/init_creates_file.rs` | `init` creates config; `--config` vs hostile env; `HOME`+`XDG_CONFIG_HOME` removed → `"config-dir"` error (`:112-122`); read-only tempdir cleanup coupling (`:64-65`, `:90-91`) | none |
| `tests/json_output.rs` | `-j` JSON envelope shape across commands | none |
| `amc:tests/model.rs` (341 lines) | `model` + `thinking` commands; strips trailing bell `\x07` before compare (`:68` etc.) | none |
| `amc:tests/prompt.rs` (198 lines) | `prompt` command; explicit STEP_NAME positional override vs derivation; multiline content substring asserts | none |
| `src/*` unit tests | clap parse (`Cli::try_parse_from`), `select_command` routing, `determine_step` factories with real tempdirs, `parse_branch_output`, config round-trips | none |

Env-handling convention in tests: pass env per-invocation via `.env("XDG_CONFIG_HOME", &config_dir)` (`.env_remove` for hostile-default tests) — tests never rely on the ambient env; the binary's only env read is `ConfigEnv::from_env()` at `src/config_dir.rs:26-36`.

## Build/verify gotchas surfaced by research

- **The crate is binary-only** (no lib target): integration tests cannot `use` crate types; fixtures are asserted by string (`tests/init_creates_file.rs:20-21`).
- **add-model-command does not compile** (verified `cargo check` on `git archive` extraction): 3× E0425 `cannot find function config_file_path` at `amc:src/commands/mod.rs:109, 113, 117` + 3× E0061 `read_config` 1-arg calls at `amc:mod.rs:238, 250, 273` vs 2-arg def `amc:src/config.rs:71`. Any port of amc work must reconcile against `config_file_path_with_env` (`src/config_dir.rs:106-113`) and 2-arg `read_config` (`src/config.rs:41`).
- **Config-dir resolution precedence** (`src/config_dir.rs:72-92`): absolute `XDG_CONFIG_HOME` wins → `APPDATA` (Windows) → `HOME/.config` → else `Error::new("config-dir", "could not determine a config directory: set XDG_CONFIG_HOME or HOME")`.
- **`ConfigPathSource::Display` phrases are pinned in two layers** — unit tests `src/config_dir.rs:196-203` and integration tests `tests/step.rs:219-266` — treat them as API text.
- **Error output routing**: text-mode errors → stderr (`src/main.rs:33`), JSON-mode errors → stdout (`src/main.rs:47`), both `exit(1)` (`main.rs:76-78`). Envelope `{"error": {"message", "source"}}` (`main.rs:45-48`).
- **ANSI/color is a compile-time no-op under test** (`src/format.rs:7-12`), which is why tests assert `!stdout.contains('\x1b')` unconditionally.
- **`determine_step` string contract**: `artifact_dir` must end in `/` (`src/commands/mod.rs:134-136`); composed by `artifact_dir_path` (`mod.rs:111-117`) from cwd + branch (slashes → hyphens). `try_exists()?` (`mod.rs:149`) can surface io errors before fallback logic.
- git hermeticity rule: tests create disposable repos via `git init -b main` (unborn) or committed `git_repo_on_branch`; the ambient checkout is never used because CI leaves it detached (`tests/branch.rs:3-5`, duplicate rationale `src/git.rs:37-40`).