# Design Discussion

## Current State

`orksorksorks` is a binary-only Rust CLI (`edition 2024`) whose `src/main.rs:8-13` declares six
modules: `commands` (the only directory module), `config`, `config_dir`, `errors`, `format`, `git`
(all flat single files). `src/commands/mod.rs` is 1,459 lines — half the crate's 2,884 — and mixes
four concerns with clean internal seams (research Q1):

- **Parser** (`:1-117`): `const NAME/AUTHOR/ABOUT/LONG_VERSION` (`:7-18`); `pub struct Cli`
  (`:21-38`, `#[derive(Parser, Clone)]`, `--json` global flag, `#[command(subcommand)]`);
  `pub enum Commands` (`:40-117`) with eight variants (`Init`, `Branch`, `ArtifactDirectory`,
  `Step`, `Model`, `Thinking`, `Prompt`, `Script`), registered purely through clap-derive
  attributes — no manual registry.
- **Router** (`:119-157`): private `select_command_with_env(cli, &ConfigEnv)` resolves the config
  path per variant via `config_dir::config_file_path_with_env` and `?`-propagates, then dispatches
  to each handler; public `select_command(&Cli)` delegates with `ConfigEnv::from_env()`.
- **Handlers** (`:161-434`): eight private `fn -> Result<String, Error>` — `init_command`,
  `branch_command`, `artifact_directory_command`, `step_command`, `model_command`,
  `thinking_command`, `prompt_command`, `script_command`. Handlers never print; they return data.
- **Helpers** (`:195-359`): private `artifact_dir_path`, `determine_step`, `resolve_model`,
  `resolve_step`, `resolve_prompt`, `resolve_script` — a mutually-recursive resolution layer sharing
  the `Error::new(tag, message)` idiom.
- **Tests** (`:437-1460`, ~1,000 lines): a single flat `#[cfg(test)] mod tests` with `use super::*`,
  covering parse routes, routing arms, handler strings, helper results, and exact
  `Error` tag/message pairs.

Only three symbols leave `commands`: `Cli` (+ its `pub json`/`command` fields), `Commands`, and
`select_command` (research Q2). `src/main.rs:62-64,83` uses exactly those. The crate is heavily
pinned: 10 integration test files spawn the real binary and assert exit codes, exact stdout/stderr
bytes, the `{"data"}`/`{"error":{"message","source"}}` JSON envelope, absence of `\x1b`, template
byte-equality, and clap parse rejection (research Q4). `tests/architecture.rs` uses rust_arkitect:
a `commands` import allowlist in both `crate::` and `orksorksorks::` spellings (`:56-75`) plus a
total module-order cycle ban `main < commands < config < config_dir < git < errors < format`
(`:86-140`). rust_arkitect attributes every file under `src/commands/` to the logical module
`orksorksorks::commands` (`:21-25`), so moving code *within* `commands` produces no external edge
changes and needs no new architecture rules.

A prior attempt (draft PR #29) was never merged and contains no usable scheme — only a `DELETEME`
file.

## Desired End State

`src/commands/mod.rs` becomes a small, readable root file holding the public surface (`Cli`,
`Commands`, `select_command`), the router, and the parser tests. Each handler family lives in its
own submodule file under `src/commands/`, and the shared resolution helpers live in
`commands/resolve.rs`. Every new file follows the sibling-module shape exactly: imports → items →
helpers → trailing `#[cfg(test)] mod tests`.

Concretely, `src/commands/` becomes:

```
mod.rs                  Cli, Commands, select_command_with_env, select_command, parse tests
init.rs                 init_command + tests
branch.rs               branch_command + tests
artifact_directory.rs   artifact_directory_command + tests
step.rs                 step_command + tests
model.rs                model_command + tests
thinking.rs             thinking_command + tests
prompt.rs               prompt_command + tests
script.rs               script_command + tests
resolve.rs              artifact_dir_path, determine_step, resolve_model/step/prompt/script + tests
```

Verification is behavioral, not structural: the full gate (`scripts/test.sh`) passes unchanged, and
specifically:

- All 10 integration test files pass with **no edits** — same CLI surface, same text output, same
  JSON envelope, same exit codes, same error tags, no ANSI in captures.
- `tests/architecture.rs` passes untouched — the `commands` logical module's dependency set is
  identical, and no submodule file introduces a `use` outside the existing allowlist.
- Unit tests move with their subject and continue to pin the same parse routes, dispatch arms,
  handler strings, helper results, and `Error` tag/message pairs.
- `src/main.rs` is unmodified.
- No file in `src/commands/` exceeds ~300 lines (largest is `mod.rs` with `Cli`+`Commands`+router+
  parse tests; handlers are 20–60 lines each).

## Patterns to Follow

- **Trailing test module per file** — `mod tests` at the *end* of every module, `use super::*;`,
  behavior-sentence names, `tempfile::tempdir()`, `pretty_assertions` (src/config.rs:254,
  src/config_dir.rs:116, src/git.rs:39). Each new submodule gets its own; parse tests stay with
  `Cli` in `mod.rs`; `resolve_*` tests move to `resolve.rs`; handler tests move with their handler.
- **Three-tier visibility** — `pub` for the crate surface (`Cli`, `Commands`, `select_command`),
  `pub(crate)` for cross-module internals, private for file-local helpers (src/config.rs:131,
  src/config_dir.rs:72,106; src/git.rs:28). Moved handlers/helpers become `pub(crate)` so
  `mod.rs`'s router can call them.
- **`///` docs on every `pub` and `pub(crate)` item** — `#![warn(missing_docs)]` (src/main.rs:6)
  plus clippy `-D warnings` makes this a hard gate; struct fields each get a doc line
  (src/config_dir.rs:11-17, src/config.rs:10-19). This is the single most likely way to break the
  build during the move.
- **`Error::new(tag, message)` with lowercase, namespaced tags** — `"config-exists"`, `"step"`,
  `"model"`, `"prompt"`, `"script"`, `"config:*"` (src/commands/mod.rs:171-434, src/config.rs:134-231).
  Do not invent new tags; preserve every existing tag verbatim.
- **`_in`-suffixed injectable-dir twins for testability** — src/git.rs:8,17. Only relevant if a
  helper gains a dir parameter; do not add one gratuitously (behavior-preserving move).
- **Free snake_case imperative functions, no artificial parent type** — the crate uses free `fn`s;
  do not introduce a `Commands` impl or context struct just to avoid passing arguments.
- **rust_arkitect dual-spelling allowlist** — any `use crate::…`/`use orksorksorks::…` added by the
  move must already be one of the 10 allowlisted sibling spellings (tests/architecture.rs:56-75).
  Sibling five + `clap`/`std`/test deps cover everything `commands` imports today.
- **Patterns NOT to follow**: do **not** add a second nesting level (`commands/handlers/*`),
  `pub mod` declarations, or a central `tests.rs` — the repo has never used them (research Q3) and
  the convention is per-module trailing tests.

## Design Decisions

1. **Domain-per-command files**: one submodule per handler family plus `resolve.rs` — mirrors
   crate naming (`commands::step` ↔ `config::Step`) and the 1:1 `tests/*.rs` files; keeps each file
   20–60 lines.
2. **Helpers co-located in `commands/resolve.rs`**: the six resolution helpers stay together — they
   are a mutually-recursive layer with one shared idiom and one cohesive test group.
3. **Tests co-located per submodule**: each new file carries its own trailing `mod tests` — the only
   option consistent with the crate-wide convention; avoids recreating a 1,000-line file.
4. **`pub(crate)` handlers in private submodules**: `mod.rs` declares `mod init;` etc. (private),
   handlers/helpers are `pub(crate)`, and `select_command_with_env` `use`s them. Root keeps only the
   three `pub` items, so `src/main.rs` is untouched.
5. **`Cli`/`Commands` stay in `commands/mod.rs`**: smallest diff, no re-export indirection,
   `main.rs` contract literally unchanged.
6. **`include_str!` stays as-is**: `../../templates/default.toml` (src/commands/mod.rs:181) resolves
   relative to the source file, and `commands/init.rs` is the same directory depth as
   `commands/mod.rs` — verified that repo-root `templates/default.toml` is the target, so the
   literal survives the move with no edit.
7. **Behavior-preserving, mechanical move**: no logic edits, no signature changes beyond
   `pub(crate)` visibility, no tag/message changes. Any behavioral difference is a bug.
8. **No child tickets**: all work lands on the main ticket.

## What We're NOT Doing

- **No logic refactors** — no deduplication of the `resolve_*` family, no extraction of shared
  error-construction helpers, no restructuring of `select_command_with_env`.
- **No change to `Cli`/`Commands`/clap attributes** — variant names, flag names, `long_about=None`,
  `arg_required_else_help`, and kebab naming all stay byte-identical.
- **No changes to `src/main.rs`, `src/config.rs`, `src/config_dir.rs`, `src/errors.rs`,
  `src/format.rs`, `src/git.rs`, `build.rs`, or `scripts/`.
- **No edits to any file in `tests/`** — they are the verification harness; if one needs editing, the
  move changed behavior.
- **No new architecture rules** and no changes to `tests/architecture.rs` — submodule files inherit
  the `commands` identity.
- **No second nesting level, no `pub mod`, no re-export shims, no central test file.**
- **No new dependencies, no Cargo.toml changes.**
- **No documentation-site or README work.**

## Open Risks

- **`missing_docs` / clippy `-D warnings`**: newly `pub(crate)` handlers and helpers need `///`
  docs. Every moved item must carry its doc across; the gate will catch omissions but they cost a
  round-trip.
- **Test-module `use` hygiene**: inline tests use `use super::*;` plus `crate::config` and
  `CommandFactory` (src/commands/mod.rs:439-441). After the split, `super::*` no longer reaches
  `Cli`/`Commands` from, e.g., `step.rs`'s tests; those tests will need explicit
  `use crate::commands::{Cli, Commands};` or equivalent. This is the most likely mechanical
  breakage and is behavior-neutral.
- **Router arm edits**: each arm changes from a bare call to a path (`init::init_command(&path)`).
  A typo here changes routing; integration tests (`tests/step.rs`, `tests/model.rs`, etc.) must
  catch it — they dispatch each variant and assert output.
- **`determine_step` / `artifact_dir_path` cross-references**: helpers in `resolve.rs` call
  `crate::git::current_branch` and take `&Step`; their move is straightforward but the helper test
  group (~570-833) is large and must be relocated without alteration.
- **Line-count assumption**: research's 487-vs-809 discrepancy on `src/config.rs` was flagged
  immaterial; no design decision here depends on it, and `src/config.rs` is out of scope.
- **No golden files**: verification relies entirely on the existing test suite; if a behavior is
  *not* pinned by a test, the move could silently change it. Mitigation: keep the move mechanical
  and diff-review each new file against its original region.
