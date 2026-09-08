# Implementation Summary

Branch: `alanvardy-var-836-add-step-flag` — PR #12 "Add step flag"

Add a per-subcommand `--step <NAME>` override to `step`/`model`/`thinking`/`prompt` that replaces artifact-based step derivation with a direct `config.steps` name lookup via the new `resolve_step` helper, and remove `prompt`'s positional `STEP_NAME`. No schema changes, no global flag, no changes to `init`/`branch`/`artifact_directory`.

## Commits

| Phase | Commit | Description |
|-------|--------|-------------|
| 1     | d196ea2 | Domain layer — `resolve_step` lookup |
| 2     | 4d068ea | Parameter surface — clap `--step` flag + dispatch threading (behavior-neutral) |
| 3     | 9ca7a76 | Application layer — handler override branching (prompt positional removed) |
| 4     | 8980e45 | Transport/E2E — integration coverage + live checks (test-only) |

All pushed to `origin/alanvardy-var-836-add-step-flag`; each phase's commit landed with a green `scripts/test.sh` gate before the next started.

## Automated Checks

- [x] Full gate `scripts/test.sh` (fmt --all, check, clippy `-D warnings`, nextest, TODO/dbg gate) — **144/144 tests pass** (parent re-run after Phase 4)
- [x] `cargo nextest run resolve_step` — 3/3 (Stage 1)
- [x] `cargo nextest run cli_try_parse` — full parse suite incl. 8 new `--step` bind/absent tests (Stage 2)
- [x] Full existing `tests/{step,model,prompt}.rs` suite passes **unchanged** at the Stage-2 gate (behavior-neutral promise)
- [x] `cargo nextest run step_command_flag` — 2/2; `cargo nextest run prompt` — 22/22 (Stage 3)
- [x] New Stage-4 integration tests all pass: `step_flag_works_in_non_git_dir`, `step_flag_unknown_text_error_goes_to_stderr`, `step_flag_unknown_json_error_goes_to_stdout`, `model_flag_returns_model_string`, `thinking_flag_returns_thinking_budget`, `prompt_flag_returns_content`, `prompt_rejects_positional_step_name` (exit code 2)
- [x] `tests/json_output.rs` untouched (no envelope shape change) — plan intent honored

## Manual Verification Items (from the plan)

- [ ] **Stage 1** — none (helper not yet reachable from the CLI)
- [ ] **Stage 2** — `cargo run -- step --help` and `cargo run -- prompt --help` show the `--step <STEP>` option. ⚠️ The plan's Stage-2 wording also expected `prompt --help` to still show `[STEP_NAME]`; the positional was removed in Stage 3 (never shipped), so current help shows only `--step`/`--config`.
- [ ] **Stage 3** — `cargo run -- step --config <fixture> --step one` prints `one` from a repo with no matching artifacts
- [ ] **Stage 4** — build fixture `orksorksorks.toml` with `one`/`two` steps, `[[models]]`, and a multiline `[[prompts]]` (snippet in plan.md §Stage 4 Manual)
- [ ] From a repo with no matching trigger artifacts, run each and check the output:
  - [ ] `cargo run -- step --config ./orksorksorks.toml --step one` → `one`
  - [ ] `cargo run -- model --config ./orksorksorks.toml --step one` → `openrouter/deepseek/flash`
  - [ ] `cargo run -- thinking --config ./orksorksorks.toml --step one` → `high`
  - [ ] `cargo run -- prompt --config ./orksorksorks.toml --step one` → `# Prompt for step one` ⚠️ output is now default-prefixed with the frontmatter block from the merged var-837 feature (`show_frontmatter` defaults true); the prompt content appears below it.
- [ ] `cargo run -- step --config ./orksorksorks.toml --step nope` exits non-zero with `no step named "nope"` on stderr
- [ ] From a directory with no `.git` (e.g. `cd /tmp`), `cargo run --manifest-path <repo>/Cargo.toml -- step --config <abs-path>/orksorksorks.toml --step one` succeeds — the decisive git-skip proof
- [ ] `cargo run -- prompt one` fails with a clap usage error (positional removed)

## Adaptations (codebase drift since the plan was written)

`origin/main` advanced after `/5_plan`: the var-837 frontmatter feature landed (commits `36652d2`/`719e803`/`cb15ca2`), touching the same files. Adaptations:

1. **`Config.show_frontmatter: bool` field** (serde-default `true`) — every new `Config { ... }` literal in unit tests got `show_frontmatter: true` (plan snippets predate the field).
2. **`prompt_command` frontmatter tail preserved** — the plan's Stage-3 `prompt_command` replacement snippet predates frontmatter and would have deleted it; the implemented version re-uses the exact plan's `step`-flag `name` computation and keeps the existing frontmatter rendering verbatim (keyed off the effective prompt `name`).
3. **`prompt_flag_returns_content`** runs in a `git init`ed tempdir — default `show_frontmatter` calls `git::current_branch()`, which fails in a git-less dir; assertion is containment of `# Prompt for step one` (JSON `data` holds frontmatter + content).
4. **Plan's literal nextest command** `cargo nextest run -- --test step --test model --test prompt` is unsupported by this nextest version (`--` argument parsing); ran per-binary filters instead — every test in `tests/{step,model,prompt}.rs` passes.

## Notes / residual

- `prompt --step one` from a **git-less** directory still fails at the frontmatter block (`git::current_branch()`) when `show_frontmatter` is unset (defaults true) — inherited var-837 behavior, identical for the no-flag path. `step`/`model`/`thinking` fully skip git with `--step` (decisive, covered by `step_flag_works_in_non_git_dir`). Use `show_frontmatter = false` for git-less `prompt` runs.
- No schema migration, no codegen — not applicable. No child tickets created; all work on PR #12.