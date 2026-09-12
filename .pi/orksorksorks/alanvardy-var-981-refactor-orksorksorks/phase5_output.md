# Phase 5 Output — Frontmatter/script handlers (`prompt.rs`, `script.rs`)

## Commit
- **SHA:** `a20af38` — `Phase 5: frontmatter/script handlers — prompt, script`
- **Push:** fast-forward `4362481..a20af38` to `origin/alanvardy-var-981-refactor-orksorksorks` — succeeded, no force needed.
- Working tree post-commit: clean except the expected untracked/gitignored artifact dir `.pi/orksorksorks/`.

## Changes
- **Created `src/commands/prompt.rs`** (104 lines): `pub(crate) fn prompt_command` moved verbatim from mod.rs (docs + body, only `fn` → `pub(crate) fn`); imports `use super::resolve; use crate::errors::Error; use crate::git;`; trailing `#[cfg(test)] mod tests` with `use super::*;` containing the 2 relocated tests.
- **Created `src/commands/script.rs`** (69 lines): `pub(crate) fn script_command` moved verbatim; same import shape; trailing test module with the 1 relocated test.
- **Modified `src/commands/mod.rs`** (775 → 618 lines): removed `use crate::git;` (top imports are now exactly `use crate::errors::Error;`, `use clap::{Parser, Subcommand};`, `use std::path::PathBuf;`); added `mod prompt;` and `mod script;` (nine module declarations total); re-pointed the two router arms to `prompt::prompt_command` / `script::script_command`; deleted both handler definitions and the three handler-direct tests. Root now holds only imports, NAME/AUTHOR/ABOUT/LONG_VERSION, `Cli`, `Commands`, router, and parse/routing tests.

## Byte-identity confirmation
- Extracted the original handler blocks from `HEAD:src/commands/mod.rs` (before this phase) and the new files; diff empty after the single permitted `fn` → `pub(crate) fn` transformation on the signature line.
- **Frontmatter string** `"## Important variables\n..."` in `prompt.rs` is **byte-identical** to the original (regex-extracted and compared).
- **`"script"` error tag** `Error::new("script", ...)` preserved in `script.rs`; `script.rs` contains `## Important variables` nowhere (no-frontmatter contract intact).

## Verification results (all passed)
- `cargo check` — pass
- `scripts/test.sh` (fmt, check, clippy --tests -- -D warnings, nextest, forbidden-strings) — pass; **197/197 tests**
- `cargo nextest run prompt` — 29 tests passed (incl. integration `tests/prompt.rs` + relocated unit tests, which also ran as `cargo nextest run prompt_command_step` → 2/2 passed as `commands::prompt::tests::*`)
- `cargo nextest run script` — 19 tests passed (incl. `tests/script.rs`; relocated unit test confirmed via `cargo nextest run script_command_` → 1/1 passed as `commands::script::tests::script_command_step_without_script_errors`)
- json envelope/no-ANSI — the plan's `cargo nextest run json_output` filter matches 0 tests (test names are `init_json_returns_valid_json_with_data_field`, `init_no_json_prints_plain_text` in `tests/json_output.rs`); ran the actual names via `cargo nextest run init_json init_no_json` → 3 tests passed. Full gate's own nextest run already covered them.
- `cargo clippy --tests -- -D warnings` — clean (no leftover `crate::git` import warning)
- **Manual confirmations:** `script --help` vs `baseline/script-help.txt` — **identical**; `-j branch` vs `baseline/branch.json` — **identical**.

## plan.md
- Phase 5's 6 automated checkboxes checked; 2 manual items left unchecked.

## Observations
- The task prompt said "5 automated items"; plan.md Phase 5 lists 6 — all 6 were verified and checked (the count in the task was a miscount; the plan is authoritative).
- `cargo nextest run json_output` is a zero-match filter as written in plan.md — noted for Phase 6; actual json tests pass.
- `git show HEAD:src/commands/mod.rs` used for byte-identity reference (pre-phase state); nothing outside `src/commands/` touched; `tests/`, `src/main.rs`, etc. untouched.