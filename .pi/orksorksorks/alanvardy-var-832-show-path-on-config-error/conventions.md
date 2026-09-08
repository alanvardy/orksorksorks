# Conventions — shared factual appendix for Design/Structure/Plan

## Commands (canonical gate: `scripts/test.sh`)

`scripts/test.sh` (order matters, all must pass to declare work done):
1. `cargo fmt --all` (formats in place — mutates source)
2. `cargo check`
3. `cargo clippy --tests -- -D warnings`
4. `cargo nextest run --no-tests pass`
5. TODO/FIXME/dbg gate: `rg -i -g '*.rs' 'TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:' .` must match nothing (exit 1 otherwise)

CI (`.github/workflows/ci-pr.yml` → `_reusable-lint.yml`, `_reusable-test.yml`, ubuntu-latest):
- lint: `cargo check --locked --all-features`; `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features --locked -- -D warnings`; same forbidden-strings rg.
- test: reusable workflow with `taiki-e/install-action` (nextest) and optional codecov upload (repo has `codecov.yml`).
- Toolchain pinned: `rust-toolchain.toml` → channel `1.98.1`, components `["clippy", "rustfmt"]`.
- Install path (not CI): `scripts/install.sh` → `cargo install --path . --locked`.

## Test suite inventory

### Unit tests (inline `#[cfg(test)] mod tests`)
| File | Tests | Covers |
|---|---|---|
| `src/errors.rs:66+` | 7 | Display format strings; `From` tags (`io`/`toml::ser`/`toml::de`); JSON field round-trip `errors.rs:119-123`; PartialEq; std::error trait |
| `src/config.rs:45+` | 7 | TOML ser/de round-trips; missing-file → `"io"` tag (`:111`); malformed TOML → `"toml::de"` tag (`:125`) |
| `src/config_dir.rs:48+` | 6 | XDG absolute-wins, Windows APPDATA, HOME/.config resolution (`:55-95`); explicit path passthrough (`:102`); unresolved-home → `"config-dir"` error (`:107-112`) |
| `src/commands/mod.rs:170+` | 26 | select_command routing incl. missing-config → `"io"` (`:457-471`); init success message (`:186-190`); artifact_dir_path composition (`:268-297`); determine_step no-match → `"step"` + message (`:482-488`) |
| `src/git.rs:39+` | 5 | branch parse/errors: `"git"` tag, detached HEAD message, verbatim stderr (`:110-115`) |
| `src/format.rs:26+` | 4 | `apply_color` strips ANSI under `cfg!(test)` (`:33-46`) |

### Integration tests (`tests/`, `assert_cmd::Command::cargo_bin("orksorksorks")`)
| File | Tests | Covers |
|---|---|---|
| `tests/init_creates_file.rs` | 6 | `init` writes default config + success message (`:44-48`); JSON error `"source":"io"` on read-only dir (`:81-84`); JSON `"source":"config-dir"` with env removed (`:118-122`); ANSI-free stdout |
| `tests/json_output.rs` | 2 | `init -j` yields `{"data": …}` valid JSON (`:14-19`); no ANSI in stdout (`:40-41`) |
| `tests/step.rs` | 7 | XDG config-dir resolution (`:160-194`); CWD config no longer honored → `.failure()` (`:198-213`); step success values; ANSI-free |
| `tests/branch.rs` | 3 | branch subcommand success JSON `data` value; git failure → `.failure()`; ANSI-free |
| `tests/artifact_directory.rs` | 3 | artifact-dir print; git failure → `.failure()`; ANSI-free |

### Platform / mode gating
- `#[cfg(test)]` mod-test gating only — **no** `#[cfg(unix)]`/`#[cfg(windows)]`/`#![cfg(...)]` on any test function.
- `if cfg!(windows)` runtime branch for `%APPDATA%` is the only platform conditional (`src/config_dir.rs:32-35`).
- `cfg!(test)` compile-mode check strips all ANSI in `format::apply_color` (`src/format.rs:7`) — this is what makes plain-substring assertions valid in both unit and integration runs.
- `tests/init_creates_file.rs:4` has `#![allow(clippy::permissions_set_readonly_false)]` (lint allow, not a gate).

## Build/verify gotchas surfaced by research
- **ANSI color is compile-time gated, not runtime.** Changing any message that flows through colors (`src/errors.rs:30-32` red/yellow; success green at `src/commands/mod.rs:94`) while asserting plain substrings is safe only because `cfg!(test)` strips escapes (`src/format.rs:7`). Note the contradictory comment at `tests/init_creates_file.rs:40` ("the child still applies color") — the assertions around it (`json_output.rs:41` etc.) require ANSI-free output, so the strip behavior is what's actually relied upon.
- **Nextest CI profile.** `.config/nextest.toml`: `[profile.ci]` retries=2, fail-fast=false, slow-timeout 60s, junit.xml written to `target/nextest/ci/junit.xml` (failure output stored, success output not). CI runs `cargo nextest run --no-tests pass` via the reusable workflow.
- **Missing-config message text is unconstrained.** `read_config_missing_file_tags_io` (`src/config.rs:107-112`) and `select_command_routes_step` (`src/commands/mod.rs:457-471`) assert only `err.source == "io"`. Any new message wording appended to that error will not break existing tests; the integration regression `tests/step.rs:198-213` asserts only non-zero exit.
- **The `io::Error` text does NOT contain the path** in this fork (no `with_path` in the stdlib), so a message that currently reads e.g. `No such file or directory (os error 2)` contains no filename — the resolved path exists only as a local variable at `src/config.rs:40`, `src/commands/mod.rs:75-76`, `:164`.
- **Message construction conventions to preserve** (documented at `src/errors.rs:5-7`, `:16-17`): lowercase initial, no trailing punctuation, dynamic values via `&format!`, values interpolated bare (no quotes), source tags lowercase (`io`/`step`/`git`), kebab (`config-dir`), or `::`-namespaced (`toml::ser`, `toml::de`). Paths stringify via `.display()`.
- **Stale-branch note for worktrees:** check the resolved XDG path contract in `tests/step.rs:160-161` comment before touching resolution behavior — the CWD-config fallback is intentionally gone.