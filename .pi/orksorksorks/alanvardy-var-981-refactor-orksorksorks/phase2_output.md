# Phase 2 output — Shared resolution layer (`commands/resolve.rs`)

## Commit
- SHA: `d9f5fbb` — `Phase 2: shared resolution layer — commands/resolve.rs`
- Pushed to `origin/alanvardy-var-981-refactor-orksorksorks` (fast-forward `265a2bb..d9f5fbb`; no force needed).

## Changes
- **Created `src/commands/resolve.rs`** (542 lines): imports `use crate::config::{Config, Step};` + `use crate::errors::Error;`; the six helpers moved **verbatim** from `mod.rs` in original source order (`artifact_dir_path`, `determine_step`, `resolve_model`, `resolve_step`, `resolve_prompt`, `resolve_script`), each with its `///` doc, visibility promoted to `pub(crate)`; `#[cfg(test)] mod tests` with `use super::*;` and the **22 relocated tests** (5 path-composition, 7 step-derivation, 10 resolve hit/miss) — no `use crate::git;` (none of the helpers calls git).
- **Modified `src/commands/mod.rs`** (1459 → 924 lines): added `mod resolve;` after the import block; every helper call-site in the still-resident handlers prefixed with `resolve::` (7× `artifact_dir_path`, 5× `determine_step`, 5× `resolve_step`, 2× `resolve_model`, 1× `resolve_prompt`, 1× `resolve_script`); deleted the six helper definitions; removed `use crate::config::{Config, Step};` from the module top and from the test module. Top imports now exactly: `use crate::errors::Error; use crate::git; use clap::{Parser, Subcommand}; use std::path::PathBuf;`.
- No other files touched (`git diff --stat`: only the two files above).

## Verification
| Command | Result |
|---|---|
| `cargo check` | passed |
| `scripts/test.sh` (fmt → check → clippy -D warnings → nextest → forbidden strings) | passed — **197 tests / 197 passed** (matches baseline) |
| `cargo nextest run resolve::` | passed — **22/22** relocated tests |
| `cargo nextest run select_command_routes` | passed — **6/6** router tests |
| `cargo clippy --tests -- -D warnings` | clean (no unused `Config`/`Step` import) |
| `./target/debug/orksorksorks --help` vs `baseline/help.txt` | identical (`diff` clean) |
| `./target/debug/orksorksorks -j branch` | `{"data":"alanvardy-var-981-refactor-orksorksorks"}` (manual item, confirmed) |

## Byte-identity check
Scripted comparison of every moved item against `git show HEAD:src/commands/mod.rs`:
- All 22 test bodies byte-identical after the planned 4-space re-indent.
- All 6 helper bodies byte-identical after the planned `pub(crate) ` promotion on the signature line.
- Assertions also confirmed no moved fn/test still exists in `mod.rs` and exactly the expected prefix counts landed.

## plan.md
Phase 2 automated checkboxes (5) marked `[x]`; the 2 manual items left unchecked.

## Observations / notes
1. The plan's Phase 2 automated list actually contains **5** checkboxes (`cargo check`, `test.sh`, `resolve::`, `select_command_routes`, `clippy`), not 4 — all 5 checked.
2. An initial transform attempt corrupted the `use clap::CommandFactory;` indent (substring-replace matched both import sites); it was caught by diff review and fixed with line-oriented removal. Re-verified; final `scripts/test.sh` gate (includes `cargo fmt --all`) confirms formatting is canonical.
3. `mod.rs` tests remaining in place rely on `step_command`/`prompt_command`/`script_command` (still resident) — none of the remaining tests reference the moved helpers or `Config`/`Step` types, so removing both imports was safe (clippy confirms).
4. Total crate line delta: 1466 vs 1459 original (+7: `mod resolve;`, its blank line, and resolve.rs separator lines) — within the plan's "± a few" allowance.

## Risks for later phases
- None blocking. Later phases must remember `resolve::` is now the only way to reach these helpers, and `mod.rs` no longer imports `Config`/`Step` (Phase 4 handlers must add their own imports or use `crate::config::...` fully qualified as the plan specifies).