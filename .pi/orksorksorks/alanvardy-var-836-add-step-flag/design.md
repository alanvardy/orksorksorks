# Design Discussion

## Current State

Post-rebase, this branch carries `main` (tip `2d661d1`) plus the three
`add-model-command` commits (`58c4913` model, `333ab82` thinking,
`33a1b60` config-dir wiring fix); `cargo check` is green.

The CLI has five subcommands (`src/commands/mod.rs:37-95`):

- `init`, `branch`, `artifact_directory` — never derive a step.
- `step`, `model`, `thinking`, `prompt` — all derive the "current step"
  from trigger artifacts via `determine_step` (`src/commands/mod.rs:183`),
  which reverse-scans `config.steps`, stats each `trigger_artifact` with
  `try_exists`, falls back to the last empty-trigger default, and errors
  `"step" ... no trigger artifact matched` when nothing matches.

Derivation is assembled identically in each handler as cwd → branch →
`artifact_dir_path` → `determine_step`:

- `step_command`   `src/commands/mod.rs:226-234` → returns `step.name`
- `model_command`  `src/commands/mod.rs:240-253` → `resolve_model(&cfg, &step.model)?.model`
- `thinking_command` `src/commands/mod.rs:255-268` → `resolve_model(...)?.thinking`
- `prompt_command` `src/commands/mod.rs:281-292` → `resolve_prompt(&cfg, &name)`

`determine_step` returns the whole `Step` (`src/commands/mod.rs:183`), so
each caller reads a different field. Two exact-match name resolvers already
exist as precedent: `resolve_model` (`:210`, miss → `Error::new("model",
"no model named {name:?}")`) and `resolve_prompt` (`:269`, miss →
`Error::new("prompt", "no prompt named {name:?}")`).

`prompt` alone also takes a positional `STEP_NAME` (`src/commands/mod.rs:84-85`)
that short-circuits derivation entirely — `prompt_command` skips cwd/git when
present (`src/commands/mod.rs:284-292`) — and feeds the name straight into
`resolve_prompt` **without** validating it against `config.steps`.

The config schema (`src/config.rs`) is `Step { name, trigger_artifact,
model }` (`:7-14`), `Model { name, model, thinking }` (`:18-25`),
`Prompt { name, content }` (`:29-34`), `Config { version, steps, models,
prompts }` (`:41-53`). There is no name→`Step` lookup anywhere: `determine_step`
is the only path from "what exists on disk" to a `Step`.

## Desired End State

A `--step <NAME>` override on `step`, `model`, `thinking`, and `prompt` that,
when present, replaces artifact-based derivation with a direct name lookup
against `config.steps`:

- `orksorksorks step --step one`     → prints `one`
- `orksorksorks model --step one`    → prints the model string for step `one`
- `orksorksorks thinking --step one` → prints the thinking budget for step `one`
- `orksorksorks prompt --step one`   → prints the prompt content for step `one`

When `--step` is given, `step`/`model`/`thinking` do **not** call
`current_dir()`, `git::current_branch()`, or `artifact_dir_path` — they work
on a detached HEAD and need only the config. `prompt --step` skips step
*derivation* the same way, but its frontmatter block still resolves the git
branch (unless `show_frontmatter = false`), so `prompt --step <NAME>` still
needs a git checkout. When `--step` is absent, behavior is exactly as today.

Unknown names fail consistently: a `resolve_step` helper mirrors
`resolve_model`/`resolve_prompt`, erroring with source `"step"` and message
`no step named {name:?}`.

`prompt`'s positional `STEP_NAME` is removed; the flag is the single override
surface for all four commands.

Verification: `step|model|thinking|prompt --step <known>` return the right
field; `--step <unknown>` exits 1 with `source == "step"` (text to stderr,
JSON error to stdout, per `src/main.rs:33,47`); `step|model|thinking --step`
on a detached-HEAD, unborn, or empty repo succeeds where derivation would
need git, and `prompt --step` does the same only when `show_frontmatter =
false` (the default frontmatter still needs a git branch); without `--step`
all current behavior is unchanged.

## Patterns to Follow

- **Name resolvers as the validation precedent.** `resolve_model`
  (`src/commands/mod.rs:210-217`) and `resolve_prompt` (`:269-276`) are the
  template for `resolve_step`: linear scan, exact `==` on `name`, first match
  wins, miss → `Error::new("<tag>", "no <kind> named {name:?}")`. New helper
  uses tag `"step"` to match `determine_step`'s existing no-match tag
  (`:198-201`).
- **Per-subcommand `--config` option shape.** `#[arg(long, value_name =
  "CONFIG")]` on `Option<PathBuf>` (`src/commands/mod.rs:60,68,76,89`). The
  new `--step` arg should follow this shape: `#[arg(long, value_name = "STEP")]
  step: Option<String>`, declared per-subcommand (Q1=B).
- **Config read stays first.** `read_config(path, source)?` runs before any
  step resolution in every handler today (`:227`, `:242`, `:257`, `:283`) so a
  missing/malformed config deterministically fails with `"io"`/`"toml::de"`
  before git or artifact logic. Preserve this ordering — the override still
  needs `config.steps` to validate.
- **Dispatch chokepoint.** `select_command_with_env` (`src/commands/mod.rs:93-125`)
  destructures each `Commands::*` variant, resolves `(path, source)` via
  `config_file_path_with_env`, and threads them into the handler. The new
  `step` field rides the same destructure-and-pass flow; `env` never reaches
  handlers.
- **Handler shape.** `(path, source)` pair plus command-specific args
  (`step_command`/`model_command`/`thinking_command` are 2-arg; `prompt_command`
  is 3-arg with `step_name`). Add the `Option<String>` override as a trailing
  arg (or fold into the existing `step_name` arg for `prompt`).
- **The `(path, source)` round-trip contract.** `ConfigPathSource::Display`
  phrases (`config_dir.rs:54-64`) are pinned by unit and integration tests —
  do not change them.
- **Integration-test conventions.** Duplicated per-file helpers (`init_git_repo`
  at `tests/step.rs:7`, `tests/model.rs:7`, `tests/prompt.rs:7`; per-file
  `write_config`/`artifact_dir`); `Command::cargo_bin` + `.args([...])` +
  `.current_dir`; JSON asserted via `serde_json::Value["data"]`; errors via
  `source`/`message` fields; no-ANSI guard `assert!(!out.contains('\x1b'))`.

### Do NOT follow

- **`prompt`'s positional override.** `prompt <STEP_NAME>` skips `config.steps`
  validation (`src/commands/mod.rs:284-292`), so it can resolve names that are
  not steps. This is the inconsistency the flag is meant to replace — remove it
  (Q2=B), don't replicate its validation-skipping behavior.
- **Global `--step`.** `-j/--json` (`:32-33`) is global, but `--step` is only
  meaningful to the deriving family; a global flag would leak onto
  `init`/`branch`/`artifact_directory`.
- **`amc:` branch is not a separate reference anymore.** With the rebase, the
  "unmerged add-model-command" framing in `research.md`/`conventions.md` is
  stale; the model/thinking/prompt code is now on *this* branch and compiles.

## Design Decisions

1. **Flag placement — per-subcommand `--step` (Q1=B).** Each of
   `step`/`model`/`thinking`/`prompt` gains `#[arg(long, value_name = "STEP")]
   step: Option<String>`, matching the `--config` shape. No global flag; the
   step-deriving family is the only consumer and a global flag would appear on
   `init`/`branch`/`artifact_directory` with no meaning.
2. **Unify `prompt` onto the flag, remove positional (Q2=B).** The positional
   `STEP_NAME` (`src/commands/mod.rs:84-85`) is the one override that bypasses
   `config.steps` validation. Replace it with `--step` so all four commands
   share one surface and one validation path. This removes the only
   in-repo short-circuit precedent, so Q4 below defines the new short-circuit
   behavior uniformly.
3. **Add `resolve_step(config, name) -> Result<Step, Error>` (Q3=A).**
   Linear scan of `config.steps` by exact `name`, miss → `Error::new("step",
   "no step named {name:?}")`, mirroring `resolve_model`/`resolve_prompt`
   (`src/commands/mod.rs:210`, `:269`). Validation happens against
   `config.steps`, satisfying "unknown step names handled consistently with
   existing error paths."
4. **`--step` skips step-derivation cwd/git/artifact-dir (Q4=A).** Each
   handler reads config, then branches: `Some(name)` → `resolve_step(&cfg,
   &name)?`; `None` → existing derive path (`current_dir` +
   `git::current_branch` + `artifact_dir_path` + `determine_step`). For
   `step`/`model`/`thinking` this makes the flag work on detached HEAD /
   unborn repos. `prompt_command` still resolves the git branch in its
   frontmatter block (to render `branch`/`artifact_directory` accurately), so
   `prompt --step` needs git unless `show_frontmatter = false` — a scoped
   caveat to the detached-HEAD claim, not a derivation leak.
5. **Field naming — `step`, not `step_name`.** The new arg is `step` (named
   after the `--step` flag), replacing `prompt`'s `step_name` field. The
   `prompt_command` third param becomes `step: Option<String>` and the handler
   uses `resolve_step(...)?.name` when set. Invocation `step --step foo` is
   awkward-but-consistent; the name is task-mandated.

## What We're NOT Doing

- No changes to `init`, `branch`, or `artifact_directory` — they derive no step.
- No global `--step` flag (leaks onto non-deriving commands).
- No config-schema changes; `Step`/`Model`/`Prompt`/`Config` stay as-is
  (`src/config.rs:7-53`).
- No alias/back-compat shim for `prompt`'s removed positional `STEP_NAME` — the
  surface is unmerged, so we cut it clean rather than carry two override paths.
- No load-time cross-validation of `Step::model` against `Config::models` (still
  deferred; unknown model names keep surfacing only when `model`/`thinking` run,
  via `resolve_model` `src/commands/mod.rs:210`).
- No change to the `-j/--json` envelope, error routing (`src/main.rs:33,47`), or
  the `(path, source)` config contract.
- No touching the trailing-bell `\x07` strip in `tests/model.rs` assertions —
  unrelated to this change.

## Open Risks

- **Removing `prompt`'s positional is an API break** on a surface that has
  already shipped to `add-model-command` (and now this branch) but not `main`.
  It's the right call for consistency, but any external UX already relying on
  `prompt <name>` will change. If that matters, revisit Q2.
- **`step --step foo` reads awkwardly** (flag name == subcommand name). Task
  mandates `--step`; worth a follow-up if UX feedback objects.
- **`prompt_unknown_step_name_fails` semantics shift.** Today
  `prompt nope` fails via `resolve_prompt` (`"no prompt named"`); post-change
  `prompt --step nope` fails earlier via `resolve_step` (`"no step named"`), and
  only if the step exists but lacks a prompt does `"no prompt named"` surface.
  The error contract changes — tests at `tests/prompt.rs:116-135` must be
  rewritten to assert the new `"step"` source path.
- **Detached-HEAD success is a behavior expansion** (Q4=A): `step --step foo`
  now works where bare `step` fails with `"not on a branch (detached HEAD)"`
  (`src/git.rs`). This is intended, but integration tests must prove the flag
  genuinely skips git — not just that it happens to succeed with a branch
  set. The guarantee is scoped: `prompt --step` still resolves the branch for
  frontmatter, so it is git-free only when `show_frontmatter = false` (pinned
  by the `prompt` integration tests).
- **Error-source collisions.** `resolve_step`'s new `"step"` error shares its
  tag with `determine_step`'s no-match error. Both are correct, but tests
  asserting `source == "step"` must distinguish them by message.