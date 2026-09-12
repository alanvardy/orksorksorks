# Phase 4 Output — Config-driven handlers: `step.rs`, `model.rs`, `thinking.rs`

## Result: DONE — committed `4362481`, pushed to origin

## Commit

```
4362481 Phase 4: config-driven handlers — step, model, thinking
4 files changed, 133 insertions(+), 116 deletions(-)
create mode 100644 src/commands/model.rs
create mode 100644 src/commands/step.rs
create mode 100644 src/commands/thinking.rs
```

Pushed: `519076f..4362481 alanvardy-var-981-refactor-orksorksorks -> alanvardy-var-981-refactor-orksorksorks`

## Changes made

1. **`src/commands/step.rs` (new, 79 lines)** — `pub(crate) fn step_command` moved verbatim from
   `mod.rs` (doc + body), helper calls already carrying the `resolve::` prefix from Phase 2
   (`resolve::resolve_step`, `resolve::artifact_dir_path`, `resolve::determine_step`),
   `git::current_branch` unchanged. Imports: `use super::resolve; use crate::errors::Error;
   use crate::git;`. Trailing `#[cfg(test)] mod tests { use super::*; }` with the two relocated
   tests `step_command_flag_succeeds_in_non_git_dir` and
   `step_command_flag_unknown_name_tags_step` (bodies verbatim; `ConfigPathSource::ExplicitFlag`
   fully qualified, no extra imports).
2. **`src/commands/model.rs` (new, 24 lines)** — `pub(crate) fn model_command` moved verbatim
   (returns `model.model`). Same three imports. No test module (per plan deviation 3/5).
3. **`src/commands/thinking.rs` (new, 24 lines)** — `pub(crate) fn thinking_command` moved
   verbatim (returns `model.thinking`). Same three imports. No test module.
4. **`src/commands/mod.rs`** — added `mod model; mod step; mod thinking;` declarations;
   re-pointed the three router arms to `step::step_command(...)`, `model::model_command(...)`,
   `thinking::thinking_command(...)`; deleted the three handler definitions and the two
   `step_command_*` tests. Imports unchanged (`crate::git` and `Error` still used by
   prompt/script handlers and the router). Diff: +6/−116 lines net.

## Verification (all passed)

| Check | Result |
|---|---|
| `cargo check` | PASS |
| `scripts/test.sh` (fmt → check → clippy `--tests -- -D warnings` → nextest → forbidden strings) | PASS — 197/197, `=== SUCCESS ===` |
| `cargo nextest run step_command` (2 relocated unit tests) | PASS (2/2, `commands::step::tests::*`) |
| `cargo nextest run step` (integration `tests/step.rs`) | PASS (67/67 matched) |
| `cargo nextest run model` (integration `tests/model.rs`) | PASS (23/23 matched) |
| `cargo nextest run thinking` (thinking block in `tests/model.rs`) | PASS (12/12 matched) |
| `cargo nextest run select_command_routes` (router arms after re-pointing) | PASS (6/6) |
| `step --help` vs `baseline/step-help.txt` | byte-identical diff |
| `-j step --config /nonexistent/orks.toml` vs `baseline/step-missing.json` | byte-identical diff |
| text `step --config /nonexistent/orks.toml` vs `baseline/step-missing.txt` | byte-identical diff |

Line counts vs plan criteria: `step.rs` 79 (~85 expected), `model.rs`/`thinking.rs` 24 each
(~40 expected, no test module), `mod.rs` 775 (continues to shrink in Phase 5).

## plan.md checkboxes

All 5 Phase 4 AUTOMATED items checked (`- [x]`); both Manual items left unchecked.
(Note: the task text said "3 AUTOMATED items" but the plan lists 5 — all 5 automated items
were run and checked. The `.pi/` dir is gitignored, so this edit is not part of the commit.)

## Working tree state

Only `?? .pi/orksorksorks/alanvardy-var-981-refactor-orksorksorks/` untracked (gitignored
artifact dir). No source changes outside `src/commands/`. `git status --short` clean otherwise.

## Observations / risks

- None blocking. Body bytes were copied character-identical (see the mod.rs diff — removed
  regions and new files match); only additions were `pub(crate)`, module declarations, and the
  namespaced router arms.
- `cargo nextest run step`/`model`/`thinking` are substring filters that also match
  `config::`/`config_dir::` tests; the plan's intent (integration + relocated unit suites) is
  satisfied — verified explicitly with the `step_command` filter and `select_command_routes`.
- Next phase (5) will move `prompt.rs`/`script.rs` and drop the last `crate::git` import from
  `mod.rs`; imports are still consistent in the current tree.