# Phase 3 Report — Leaf handlers: `init.rs`, `branch.rs`, `artifact_directory.rs`

## Commit
- `519076f` "Phase 3: leaf handlers — init, branch, artifact_directory" — pushed to origin (fast-forward, no force needed).

## Changed files
- `src/commands/init.rs` (new, 31 lines) — `pub(crate) fn init_command`, body moved verbatim from `mod.rs` (incl. inner `use std::io::Write;`, `include_str!("../../templates/default.toml")`, `crate::format::green_string`). Import: `crate::errors::Error`. No test module (per plan).
- `src/commands/branch.rs` (new, 7 lines) — `pub(crate) fn branch_command` delegating to `git::current_branch()`. Imports: `crate::errors::Error`, `crate::git`. No test module.
- `src/commands/artifact_directory.rs` (new, 10 lines) — `pub(crate) fn artifact_directory_command` using `std::env::current_dir()?` + `resolve::artifact_dir_path(&cwd, &git::current_branch()?)`. Imports: `super::resolve`, `crate::errors::Error`, `crate::git`. No test module.
- `src/commands/mod.rs` (modified) — added `mod artifact_directory; mod branch; mod init;`; re-pointed the three router arms to `init::init_command(&path)`, `branch::branch_command()`, `artifact_directory::artifact_directory_command()`; deleted the three handler definitions (45 lines removed). Imports unchanged this phase (`Error`, `git` still used by remaining handlers).

## Verification results
| Check | Result |
|---|---|
| `cargo check` | passed |
| `scripts/test.sh` (fmt, check, clippy `--tests -- -D warnings`, nextest 197/197, forbidden strings) | passed |
| `cargo nextest run branch` | 11 passed, 186 skipped |
| `cargo nextest run artifact_directory` | 5 passed, 192 skipped |
| `cargo nextest run init` | 13 passed, 184 skipped (includes the 8 `tests/init_creates_file.rs` integration tests + init routing/parse unit tests) |
| `./target/debug/orksorksorks -j step --config /nonexistent/orks.toml` vs `baseline/step-missing.json` | byte-identical |
| `./target/debug/orksorksorks init --help` | unchanged clap output |
| `./target/debug/orksorksorks -j branch` / `-j artifact_directory` | correct data in real checkout |

## plan.md
All 5 Phase 3 automated items checked `[x]`; Manual items left unchecked.

## Observation (filter naming, not a code mismatch)
The plan's `cargo nextest run init_` and `cargo nextest run artifact_` filters match **zero** tests under the installed nextest (the suites are keyed by bin name `init_creates_file` / `artifact_directory`, and a trailing `_` prefix pattern doesn't bind). The intended suites were run by their real names and all pass; the full gate (197 tests) also passes. No plan deviation needed.