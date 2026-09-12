# Structure Outline

## Approach

Behavior-preserving, mechanical file split of `src/commands/mod.rs` (~1,460 lines) into one
file per concern under `src/commands/`. `Cli`, `Commands`, `select_command_with_env`,
`select_command` and the parse/routing tests stay in `mod.rs`; the shared resolution helpers move
to `resolve.rs`; each handler family moves to its own file. Only change to logic: promoted
visibility (`pub(crate)`) on moved items and updated `use`/import hygiene. Every stage ends with the
same green gate, because the existing 10 integration files + `tests/architecture.rs` are the
verification harness and must pass **unedited** at every stage. Because this is a *move*, the "tests
that accompany the code" are the existing unit tests travelling with their subject plus the
integration suite; each stage keeps them green (moved unit tests re-point `use super::*;` at
`crate::commands::{Cli, Commands}` where needed).

**Cross-cutting note (horizontal-build exception):** the router in `mod.rs` cannot be stubbed out —
it must be re-pointed at the moved path (`init::init_command`) in the *same* stage the callee moves,
or the tree stops compiling. So the router's *arm spellings* change incrementally (one line per
stage), while the router's *body* (match arms, `config_file_path_with_env` resolution) is only
finalized as the last handler leaves. Each stage re-verifies the routing tests, so a typo is caught
immediately rather than at the end.

---

## Stage 1: Baseline freeze (no code)

Record the pre-move reference state so any un-pinned behavior drift is detectable. The design flags
"no golden files" as the top verification risk; this stage buys the cheap safety net.

**Files**: none modified. Optional captures under `.pi/orksorksorks/<branch>/` (help text, a few
JSON envelopes) are artifacts, not sources.
**Key changes**: none.

**Tests**: none new. `scripts/test.sh` on the pristine tree.
**Verify**: `scripts/test.sh` green. Capture `cargo run -- --help`, `cargo run -- -j step`
(valid + invalid config) stdout bytes for later diffing. Also record
`wc -l src/commands/mod.rs` and the test counts (nextest summary) as the line-count/coverage
baseline.

---

## Stage 2: Shared resolution layer — `commands/resolve.rs` (bottom-most)

Extracts the mutually-recursive helper tier every config-driven handler calls. Its green unit tests
prove artifact-path composition, default/priority step derivation, and all six `resolve_*`
hit/miss/error-tag contracts — the foundation Stages 4–5 consume.

**Files**: new `src/commands/resolve.rs`; modified `src/commands/mod.rs`.
**Key changes**:
- `pub(crate) fn artifact_dir_path(cwd: &std::path::Path, branch: &str) -> String` — moved, now `pub(crate)`
- `pub(crate) fn determine_step(config: &Config, artifact_dir: &str) -> Result<Step, Error>` — moved
- `pub(crate) fn resolve_model(config: &Config, name: &str) -> Result<crate::config::Model, Error>` — moved
- `pub(crate) fn resolve_step(config: &Config, name: &str) -> Result<Step, Error>` — moved
- `pub(crate) fn resolve_prompt(config: &Config, name: &str) -> Result<String, Error>` — moved
- `pub(crate) fn resolve_script(config: &Config, name: &str) -> Result<String, Error>` — moved
- `mod.rs`: add private `mod resolve;`, call sites become `resolve::artifact_dir_path(...)` etc.;
  drop helpers from the root; prune now-unused root imports (`Config`, `Step` stay only if the
  router still needs them — it does not); each moved item keeps its `///` doc verbatim
  (`missing_docs` + `clippy -D warnings`).
- `resolve.rs` imports: `crate::config::{Config, Step}`, `crate::errors::Error`, `crate::git`;
  trailing `#[cfg(test)] mod tests` with `use super::*;`, `tempfile`, `pretty_assertions`.

**Tests**: relocated `artifact_dir_path_*`, `determine_step_*` (default fallback, reverse priority,
no-match `"step"` error), and `resolve_model_*`/`resolve_step_*`/`resolve_prompt_*`/`resolve_script_*`
(hit, miss, exact `Error { source, message }` pair) — happy + sad paths both already present.
**Verify**: `cargo check` after the move, then `scripts/test.sh` green; fast loop
`cargo nextest run resolve::`. Router tests (`select_command_routes_*`) must still pass — the arms
now path through `resolve::` for step derivation.

---

## Stage 3: Leaf handler families — `init.rs`, `branch.rs`, `artifact_directory.rs`

The three handlers with the least coupling: `init` is standalone (template write + `config-exists`
guard), `branch` is a one-line git delegate, `artifact_directory` consumes `resolve::artifact_dir_path`.
Green tests prove the file-creation/no-overwrite contract and the composed path shape before any
config-driven handler moves.

**Files**: new `src/commands/init.rs`, `src/commands/branch.rs`,
`src/commands/artifact_directory.rs`; modified `src/commands/mod.rs`.
**Key changes**:
- `pub(crate) fn init_command(path: &std::path::Path) -> Result<String, Error>` — moved; keeps
  `include_str!("../../templates/default.toml")` byte-identical (same directory depth as today —
  verified in design decision 6) and `crate::format::green_string`.
- `pub(crate) fn branch_command() -> Result<String, Error>` — moved; `git::current_branch()`.
- `pub(crate) fn artifact_directory_command() -> Result<String, Error>` — moved; calls
  `resolve::artifact_dir_path`.
- `mod.rs`: add `mod init; mod branch; mod artifact_directory;`; arms become
  `init::init_command(&path)`, `branch::branch_command()`,
  `artifact_directory::artifact_directory_command()`.

**Tests**: handler-direct unit tests move with each file (init success message, `config-exists`
tag, `artifact_dir_path` composition); integration `tests/init_creates_file.rs` (8 tests: template
byte-equality, readonly failure, `io`/`config-exists`/`config-dir` JSON sources, `--config`
override, no-overwrite) and `tests/branch.rs`, `tests/artifact_directory.rs` (path shape, macOS
canonicalization, no ANSI, `-j`) — **unedited**.
**Verify**: `scripts/test.sh` green; `cargo nextest run init_ branch_ artifact_`.

---

## Stage 4: Config-driven step/model/thinking — `step.rs`, `model.rs`, `thinking.rs`

The three handlers that share the identical "read config → explicit `--step` or derive from
cwd+git+artifacts → resolve" shape. Green tests prove `--step` override works git-less/detached,
missing-config fails with `"io"`, and unknown names surface the right tags.

**Files**: new `src/commands/step.rs`, `src/commands/model.rs`, `src/commands/thinking.rs`;
modified `src/commands/mod.rs`.
**Key changes**:
- `pub(crate) fn step_command(path: &std::path::Path, source: crate::config_dir::ConfigPathSource, step: Option<String>) -> Result<String, Error>` — moved
- `pub(crate) fn model_command(...) -> Result<String, Error>` — moved (same 3 params)
- `pub(crate) fn thinking_command(...) -> Result<String, Error>` — moved (same 3 params)
- Body helpers now path-qualified: `crate::config::read_config`, `resolve::resolve_step`,
  `resolve::artifact_dir_path`, `resolve::determine_step`, `resolve::resolve_model`,
  `git::current_branch`.
- `mod.rs`: add three `mod` decls; arms become `step::step_command(&path, source, step.clone())`
  etc.; prune root imports that are now handler-only.

**Tests**: relocated handler-direct tests for step/model/thinking (exact model strings, tag/message
pairs); integration `tests/step.rs` (artifact-derived step, default fallback, XDG, cwd-config
guard, missing-path stderr spelling, `--step` git-less/detached, unknown step text-vs-JSON) and
`tests/model.rs` (exact `openrouter/...` strings, thinking values, `-j`, `--step` override) — **unedited**.
**Verify**: `scripts/test.sh` green; `cargo nextest run step:: model:: thinking::`.

---

## Stage 5: Frontmatter/script handlers — `prompt.rs`, `script.rs`

The two remaining handler families — the ones with extra contract surface (frontmatter emission,
positional `script <STEP_NAME>`, `show_frontmatter = false`). Moving them last means the largest
behavioral pins are exercised against an otherwise-finished split.

**Files**: new `src/commands/prompt.rs`, `src/commands/script.rs`; modified
`src/commands/mod.rs`.
**Key changes**:
- `pub(crate) fn prompt_command(path, source, step: Option<String>) -> Result<String, Error>` — moved;
  frontmatter `format!` string preserved byte-for-byte.
- `pub(crate) fn script_command(path, source, step_name: Option<String>) -> Result<String, Error>` —
  moved; `"script"` error for no-script/unknown-name preserved.
- `mod.rs`: add two `mod` decls; final arm updates; root now holds only `Cli`, `Commands`,
  `select_command_with_env`, `select_command`, `mod` declarations, and the parse/routing tests.

**Tests**: relocated `prompt_command`/`script_command` unit tests; integration `tests/prompt.rs`
(frontmatter exact text, `step =`/`branch =`/`artifact_directory =` lines, `show_frontmatter=false`,
positional rejected exit 2) and `tests/script.rs` (never emits frontmatter, positional override,
`"script"` tags, XDG) — **unedited**.
**Verify**: `scripts/test.sh` green; `cargo nextest run prompt_ script_ json_output`.

---

## Stage 6: Final verification sweep (no new behavior)

Proves the end state matches the design: root file small and readable, nothing outside
`src/commands/` touched, architecture graph unchanged.

**Files**: `src/commands/mod.rs` (only if a leftover unused import/dead item remains).
**Key changes**: none functional. `mod.rs` must be ≤ ~300 lines and hold only parser types, router,
and parse/routing tests.

**Tests**: none new.
**Verify**: `scripts/test.sh` green (fmt, check, clippy `-D warnings`, nextest, forbidden strings);
`git status --short` shows changes only under `src/commands/`; `tests/` and `src/main.rs` untouched
(`git diff --stat`); `cargo nextest run architecture` passes; diff Stage-1 captures against fresh
`--help`/`-j` output; `wc -l src/commands/*.rs` all under ~300; per-file diff review against the
original `mod.rs` regions (the design's "mechanical move" mitigation).

---

## Testing Checkpoints

| Stage | Must be green before advancing |
|---|---|
| 1 Baseline | Full `scripts/test.sh` on pristine tree + captures recorded |
| 2 `resolve.rs` | `scripts/test.sh`; helper unit tests relocated intact; router tests pass |
| 3 Leaf handlers | `scripts/test.sh`; `tests/init_creates_file.rs`, `branch.rs`, `artifact_directory.rs` unedited and green |
| 4 step/model/thinking | `scripts/test.sh`; `tests/step.rs`, `tests/model.rs` green |
| 5 prompt/script | `scripts/test.sh`; `tests/prompt.rs`, `tests/script.rs`, `tests/json_output.rs` green |
| 6 Sweep | `scripts/test.sh` + `architecture` green; only `src/commands/**` changed; all files <300 lines |

Resume rule: if any stage's gate fails, fix within that stage — the stages below it are already
independently valuable and can land on their own (each is a parent-consistent, compiling tree).

Next: run `!1` to plan