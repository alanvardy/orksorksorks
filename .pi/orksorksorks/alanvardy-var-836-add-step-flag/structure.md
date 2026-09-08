# Structure Outline

## Approach

Add a per-subcommand `--step <NAME>` override to `step`/`model`/`thinking`/`prompt` that replaces artifact-based step derivation with a direct `config.steps` name lookup via a new `resolve_step` helper (mirroring `resolve_model`/`resolve_prompt`); remove `prompt`'s positional `STEP_NAME`. No schema changes, no global flag, no changes to `init`/`branch`/`artifact_directory`.

Layers are built bottom-up, each fully tested (green `scripts/test.sh`) before the next is started. The pure lookup lands first; the CLI surface second (behavior-neutral); handler wiring third; end-to-end verification last. The only cross-cutting seam — clap field decls and the dispatch `match` destructure are one compile unit — is stubbed with ignored bindings in Stage 2 so each stage stays green.

---

## Stage 1: Domain layer — `resolve_step` lookup

Delivers the single new validation primitive every handler consumes: a pure name→`Step` lookup against `config.steps` with the `"step"` error tag. Green here proves the override's validation rule in isolation, independent of CLI plumbing.

**Files**: `src/commands/mod.rs` (next to `resolve_model` / `resolve_prompt`)

**Key changes**:
- `fn resolve_step(config: &Config, name: &str) -> Result<Step, Error>` — new; linear scan, exact `==` on `name`, first match wins, returns `step.clone()` (`Step: Clone` already derived, `src/config.rs:8`), miss → `Error::new("step", &format!("no step named {name:?}"))`

**Tests** (unit, `#[cfg(test)] mod tests` in `src/commands/mod.rs`, mirroring `resolve_model`'s precedent):
- `resolve_step_returns_matching_step` — known name → the matching `Step` (happy)
- `resolve_step_unknown_name_tags_step_error` — `source == "step"`, message `no step named "nope"` (sad)
- `resolve_step_first_match_wins` — duplicate names resolve to the first (priority convention)

**Verify**: `scripts/test.sh` green; targeted `cargo nextest run resolve_step`.

---

## Stage 2: Parameter surface — clap `--step` flag + dispatch threading (behavior-neutral)

Delivers the CLI contract: `--step <NAME>` parses on all four subcommands, `prompt`'s positional is gone. Handlers gain the field but ignore it, so **no behavior changes** and the existing suite must stay fully green — this stage proves the parse surface without coupling to the override logic.

**Files**: `src/commands/mod.rs` (the `Commands` enum variants, `select_command_with_env` destructure arms, four handler signatures)

**Key changes**:
- `Commands::{Step, Model, Thinking, Prompt}` each gain `#[arg(long, value_name = "STEP")] step: Option<String>` (same shape as `#[arg(long, value_name = "CONFIG")]`)
- `Prompt` loses its `step_name: Option<String>` positional (`step_name` field removed, replaced by `step`)
- Dispatch arms destructure and pass `step` through; handlers accept a trailing `step: Option<String>` bound as `_step` for now (keeps clippy `-D warnings` green and behavior identical)
  - `step_command(path, source, _step: Option<String>)` (+ same for `model_command`, `thinking_command`; `prompt_command`'s `step_name` param becomes `_step`)

**Tests** (unit, clap parse via `Cli::try_parse_from`):
- `--step one` binds `Some("one")` on each of the four subcommands (happy)
- absent flag → `None` on each (happy)
- `prompt one` positional now rejected with a parse error (sad — proves positional removal)

**Verify**: `scripts/test.sh` green — existing `tests/{step,model,prompt}.rs` all pass unchanged (behavior is untouched).

---

## Stage 3: Application layer — handler override branching

Delivers the actual behavior: each handler reads config first, then branches `Some(name)` → `resolve_step` (no cwd/git/artifact work) vs `None` → existing derive path. Consumes Stage 1's lookup and Stage 2's surface; green here proves the override is correct at the handler level, including that it never touches git.

**Files**: `src/commands/mod.rs` (four handler bodies)

**Key changes**:
- `step_command(path, source, step: Option<String>) -> Result<String, Error>` — `Some(name)` → `let step = resolve_step(&cfg, &name)?; Ok(step.name)`; `None` → existing `current_dir()` + `git::current_branch()` + `artifact_dir_path` + `determine_step`
- `model_command` / `thinking_command`: same branch, then `resolve_model(&cfg, &step.model)?` → `.model` / `.thinking`
- `prompt_command(path, source, step: Option<String>)`: `Some` → `resolve_step(&cfg, &name)?.name`; `None` → derive; then `resolve_prompt(&cfg, &name)` (note: field is renamed `step`, per design decision 5)

**Tests** (unit + updated existing):
- Handler called with `Some("one")` in a tempdir that is **not** a git repo succeeds — proves no `git::current_branch()`/`current_dir()` on the override path (happy)
- `Some("unknown")` → error `source == "step"` (sad)
- Rewrite `tests/prompt.rs:116-135`: `prompt nope` (now a parse error) becomes `prompt --step nope`, asserting the **new** `"step"` source; `"no prompt named"` only when the step exists but has no prompt
- Distinguish the two `"step"`-tagged errors (this helper vs `determine_step`'s "no trigger artifact matched") by message

**Verify**: `scripts/test.sh` green (including the rewritten `prompt` assertions).

---

## Stage 4: Transport/E2E layer — integration coverage + live checks

Delivers end-to-end proof of the whole contract from the command line: the flag works where bare derivation cannot (detached HEAD / unborn / empty repo), unknown names fail with the right source/envelope, and no-`--step` behavior is byte-for-byte unchanged. This layer only runs against the three already-green stages.

**Files**: `tests/step.rs`, `tests/model.rs`, `tests/prompt.rs` (plus `tests/json_output.rs` for envelope shape if needed)

**Key changes** (no production code — test-only):
- Per-command `--step one` returns the correct field (name / model string / thinking budget / prompt content); text and `-j` JSON (`v["data"]`)
- `--step nope` exits 1: text error → **stderr**, JSON error → **stdout** with `["error"]["source"] == "step"`
- `--step one` succeeds in a detached-HEAD / unborn repo (`git init -b main`, no commit) and an empty non-repo dir — the key proof Q4=A skips git
- Prompt positional gone: `prompt one` fails; `prompt --step one` succeeds
- Regression: without `--step`, all existing tests pass unchanged; keep the no-ANSI `!contains('\x1b')` guard; leave the `\x07` bell strip in `tests/model.rs` untouched

**Verify**: full `scripts/test.sh`; live manual check — build a fixture config (`steps` `one`/`two`, `models`, `prompts` with TOML multiline `content`) and run `cargo run -- step --step one` / `model --step one` / `thinking --step one` / `prompt --step one`, plus `cargo run -- step --step nope` in a repo without `content`-matching artifacts (no `curl`/JWT — this is a CLI, not an HTTP service).

---

## Testing Checkpoints

Resume only when each gate is green; a failed stage blocks all higher stages:
1. **Stage 1 green** → `scripts/test.sh` + `cargo nextest run resolve_step` (lookup correct, `"step"` tag + message).
2. **Stage 2 green** → `scripts/test.sh` with the full existing suite (parse binds, positional rejected, behavior unchanged).
3. **Stage 3 green** → `scripts/test.sh` + rewritten `tests/prompt.rs` (handlers branch correctly, override path never calls git).
4. **Stage 4 green** → full `scripts/test.sh` + live `cargo run` checks on detached HEAD (end-to-end contract).

## Cross-cutting notes

- **Flag-decl + dispatch is one compile unit** (clap fields drive the `match` destructure). Staged via the ignored-`_step` binding in Stage 2; wired in Stage 3 — do not add the field without updating every arm, else the crate stops compiling.
- **Binary-only crate, no lib target** — `resolve_step` and its unit tests live in `src/commands/mod.rs` `#[cfg(test)] mod tests`; integration tests assert output strings, not crate types.
- **`"step"` error-source collision** (resolve_step vs `determine_step`) — both are correct; tests must discriminate by message, never by tag alone.
- **`prompt`'s removed positional is the only behavior break** — it's an unmerged surface, cut clean with no alias (design "What We're NOT Doing").

Next: run `!1` to plan