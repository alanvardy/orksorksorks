# Implementation Plan

## Overview

Split `src/commands/mod.rs` (1,459 lines) into a small root file plus one submodule per
handler family and a shared `resolve.rs`, by mechanically moving existing items —
**no logic changes** except visibility promotions (`pub(crate)`) and import hygiene.
Every stage ends with the unchanged `scripts/test.sh` gate green and the 10 integration
test files plus `tests/architecture.rs` untouched.

This plan implements `structure.md` stages 1–6 in order. It depends on `design.md`,
`research.md`, and `conventions.md`; line numbers below are orientation only — every item is
named so it is unambiguous even after earlier stages shift line numbers.

### Ground rules (apply to every stage)

- **Move verbatim.** Cut each named item (function + its `///` docs + its tests) from
  `mod.rs` and paste it unchanged, except: add `pub(crate)` to moved fns, re-point moved
  tests, and re-point call sites with a `resolve::` prefix.
- **Never edit anything outside `src/commands/`** — `src/main.rs`, `src/config*.rs`,
  `src/errors.rs`, `src/format.rs`, `src/git.rs`, `templates/`, `tests/`, `scripts/`,
  `Cargo.toml`, `build.rs` are all frozen.
- **Gate:** run `scripts/test.sh` at the end of every stage (fmt → check → clippy
  `--tests -- -D warnings` → nextest → forbidden-strings). Never declare a stage done while
  it fails.
- **`#![warn(missing_docs)]` + clippy `-D warnings`:** every moved `pub(crate)` item keeps its
  `///` doc. This is the most likely self-inflicted gate failure.
- **Intra-`commands` paths use `super::resolve`**, never `crate::commands::resolve` — keeps
  rust_arkitect free of a `commands → commands` edge. Sibling paths (`crate::config`,
  `crate::config_dir`, `crate::errors`, `crate::format`, `crate::git`) are already allowlisted
  in both spellings (`tests/architecture.rs:56-75`).
- **No child tickets** — all work lands on this ticket, one branch.
- **Deviation from `structure.md` Stage 6 / design "≤ ~300 lines"** (read this): with
  `Cli`+`Commands`+router+parse/routing tests kept in `mod.rs` (design decision 5) and all
  `resolve_*` tests co-located in `resolve.rs` (design decisions 2–3), the two files land at
  **~600 and ~560 lines** respectively — the arithmetic is in "Corrected acceptance criteria"
  below. The 300-line figure is not attainable without moving parse tests out of `mod.rs`,
  which design explicitly forbids. The Stage 6 check below uses the corrected criterion. No
  other structural change is needed.

### Corrected acceptance criteria

| File | Expected after split |
|---|---|
| `mod.rs` | ~600 lines: imports, `NAME`/`AUTHOR`/`ABOUT`/`LONG_VERSION`, `Cli`, `Commands`, router, parse/routing tests only |
| `resolve.rs` | ~560 lines: 6 helpers + 22 helper tests |
| `init.rs` / `branch.rs` / `artifact_directory.rs` | < 40 lines each, no test module |
| `step.rs` | ~85 lines (handler + 2 tests) |
| `model.rs` / `thinking.rs` | ~40 lines each, no test module |
| `prompt.rs` | ~115 lines (handler + 2 tests) |
| `script.rs` | ~80 lines (handler + 1 test) |

Every file is < 300 lines **except** `mod.rs` (parser + parser tests) and `resolve.rs`
(shared layer + its test group). Total crate lines unchanged (± a few for module declarations
and import lines). If the owner wants those two under 300, stop and re-run the `structure`
step with a different grouping — do **not** silently move tests out.

---

## Phase 1: Baseline freeze (no code)

Record the pre-move reference state so any un-pinned behavior drift is detectable. The design
flags "no golden files" as the top verification risk; this buys the cheap safety net.

### Changes

None. Captures are artifacts under the ticket directory, not sources.

### Verification

#### Automated
- [x] `scripts/test.sh` passes on the pristine tree
- [x] Record `wc -l src/commands/mod.rs` → expect `1459`
- [x] Record the nextest summary counts (e.g. `cargo nextest run --no-tests pass 2>&1 | tail -5`)
      into `baseline/nextest.txt`
- [x] Capture reference output bytes (all deterministic, no tempdirs needed):
  ```bash
  mkdir -p .pi/orksorksorks/alanvardy-var-981-refactor-orksorksorks/baseline
  cargo build --quiet
  ./target/debug/orksorksorks --help                > .pi/orksorksorks/alanvardy-var-981-refactor-orksorksorks/baseline/help.txt 2>&1
  ./target/debug/orksorksorks step --help           > .pi/orksorksorks/alanvardy-var-981-refactor-orksorksorks/baseline/step-help.txt 2>&1
  ./target/debug/orksorksorks script --help         > .pi/orksorksorks/alanvardy-var-981-refactor-orksorksorks/baseline/script-help.txt 2>&1
  ./target/debug/orksorksorks -j branch             > .pi/orksorksorks/alanvardy-var-981-refactor-orksorksorks/baseline/branch.json 2>&1
  ./target/debug/orksorksorks -j step --config /nonexistent/orks.toml > .pi/orksorksorks/alanvardy-var-981-refactor-orksorksorks/baseline/step-missing.json 2>&1
  ./target/debug/orksorksorks step --config /nonexistent/orks.toml    > .pi/orksorksorks/alanvardy-var-981-refactor-orksorksorks/baseline/step-missing.txt 2>&1
  ```
- [x] `git status --short` shows only the new untracked `baseline/` directory (no source edits)

#### Manual
- [ ] Eye-check `baseline/help.txt` contains all eight subcommand names and the `LONG_VERSION`
      block (build target/profile/timestamp lines)

---

## Phase 2: Shared resolution layer — `commands/resolve.rs`

Extracts the mutually-recursive helper tier every config-driven handler calls. This is the
bottom-most stage: every later handler file consumes `resolve::`.

### Changes

#### 1. `src/commands/resolve.rs` — **create**

Module header + the six helpers, moved verbatim (docs included), visibility promoted to
`pub(crate)`, in original source order:

```rust
use crate::config::{Config, Step};
use crate::errors::Error;

/// <artifact_dir_path doc — moved verbatim>
pub(crate) fn artifact_dir_path(cwd: &std::path::Path, branch: &str) -> String { /* moved */ }

/// <determine_step doc — moved verbatim>
pub(crate) fn determine_step(config: &Config, artifact_dir: &str) -> Result<Step, Error> { /* moved */ }

/// <resolve_model doc — moved verbatim>
pub(crate) fn resolve_model(config: &Config, name: &str) -> Result<crate::config::Model, Error> { /* moved */ }

/// <resolve_step doc — moved verbatim>
pub(crate) fn resolve_step(config: &Config, name: &str) -> Result<Step, Error> { /* moved */ }

/// <resolve_prompt doc — moved verbatim>
pub(crate) fn resolve_prompt(config: &Config, name: &str) -> Result<String, Error> { /* moved */ }

/// <resolve_script doc — moved verbatim>
pub(crate) fn resolve_script(config: &Config, name: &str) -> Result<String, Error> { /* moved */ }

#[cfg(test)]
mod tests {
    use super::*;
    // + the 22 relocated tests listed below (each already uses `tempfile::tempdir()`,
    //   `pretty_assertions::assert_eq` inline; `Config`/`Step` come in via `super::*`)
}
```

**Note (deviation from `structure.md`):** `resolve.rs` needs **no `use crate::git;`** — none
of the six helpers calls git. `artifact_directory_command`/`step_command`/etc. (the git
callers) are the ones that import it.

Relocated tests (cut from `mod.rs`, keep bodies byte-identical):

| Function (current line) | Group |
|---|---|
| `artifact_dir_path_appends_trailing_slash` (536) | path composition |
| `artifact_dir_path_is_plain` (543) | path composition |
| `artifact_dir_path_handles_branch_with_slashes` (549) | path composition |
| `artifact_dir_path_replaces_all_slashes` (556) | path composition |
| `artifact_dir_path_leaves_slashless_branch_unchanged` (563) | path composition |
| `determine_step_returns_step_with_present_artifact` (570) | step derivation |
| `determine_step_prefers_last_step_in_reverse` (599) | step derivation |
| `determine_step_empty_trigger_is_default_fallback` (629) | step derivation |
| `determine_step_empty_trigger_does_not_shadow_real_match` (660) | step derivation |
| `determine_step_empty_trigger_matches_before_later_artifact` (689) | step derivation |
| `determine_step_latest_empty_trigger_wins` (720) | step derivation |
| `determine_step_no_match_errors_with_step_tag` (814) | step derivation |
| `resolve_model_returns_model_for_matching_name` (840) | resolve hit/miss |
| `resolve_model_missing_name_errors_with_model_tag` (861) | resolve hit/miss |
| `resolve_step_returns_matching_step` (880) | resolve hit/miss |
| `resolve_step_unknown_name_tags_step_error` (901) | resolve hit/miss |
| `resolve_step_first_match_wins` (925) | resolve hit/miss |
| `resolve_prompt_returns_content_for_matching_name` (1368) | resolve hit/miss |
| `resolve_prompt_missing_name_errors_with_prompt_tag` (1387) | resolve hit/miss |
| `resolve_script_hit_returns_content` (1405) | resolve hit/miss |
| `resolve_script_miss_returns_script_tag` (1421) | resolve hit/miss |
| `resolve_script_first_match_wins` (1439) | resolve hit/miss |

These tests use `crate::config::Model`/`Prompt`/`Script` fully qualified and reach
`Config`/`Step` through `use super::*;` — no extra imports.

#### 2. `src/commands/mod.rs` — **modify**

- Add module declaration near the top: `mod resolve;`
- Re-point every helper call site (in the still-resident handlers) with a `resolve::` prefix:
  - `artifact_directory_command`: `artifact_dir_path(` → `resolve::artifact_dir_path(`
  - `step_command`, `model_command`, `thinking_command`, `prompt_command`, `script_command`:
    prefix `artifact_dir_path`, `determine_step`, `resolve_step`, `resolve_model`,
    `resolve_prompt`, `resolve_script` calls with `resolve::`
- Delete the six helper definitions.
- Remove the now-unused module-level import `use crate::config::{Config, Step};` (handlers use
  only `crate::config::read_config` fully qualified and the returned values).
- In the test module, delete `use crate::config::{Config, Step};` (no remaining `mod.rs` test
  uses them); keep `use super::*;` and `use clap::CommandFactory;`.

Import ledger — `mod.rs` top after this phase:

```rust
use crate::errors::Error;
use crate::git;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
```

### Verification

#### Automated
- [x] `cargo check` passes (fast sanity before the full gate)
- [x] `scripts/test.sh` passes
- [x] `cargo nextest run resolve::` passes — all 22 relocated tests
- [x] `cargo nextest run select_command_routes` passes — router arms unchanged
- [x] `cargo clippy --tests -- -D warnings` clean (no unused `Config`/`Step` import left behind)

#### Manual
- [ ] `cargo run --quiet -- -j branch` output still parses as `{"data":"<branch>"}`
- [ ] `cargo run --quiet -- --help` matches `baseline/help.txt` (`diff`)

---

## Phase 3: Leaf handlers — `init.rs`, `branch.rs`, `artifact_directory.rs`

The three least-coupled handlers. `init` is standalone (template write + guard), `branch` is a
one-line git delegate, `artifact_directory` consumes `resolve::artifact_dir_path`. Green tests
prove file-creation/no-overwrite and the composed path shape before any config-driven handler
moves.

### Changes

#### 1. `src/commands/init.rs` — **create**

```rust
use crate::errors::Error;

/// Create a new `orksorksorks.toml` file with default configuration.
pub(crate) fn init_command(path: &std::path::Path) -> Result<String, Error> {
    // body moved verbatim (mod.rs:161-193)
}
```

Body keeps `use std::io::Write;` inside the fn, `include_str!("../../templates/default.toml")`
(depth is identical to `mod.rs` — verified in design decision 6), and
`crate::format::green_string(...)`. No test module (no handler-direct init test exists; init is
covered by `select_command_routes_init`, which stays in `mod.rs`, and by
`tests/init_creates_file.rs`).

#### 2. `src/commands/branch.rs` — **create**

```rust
use crate::errors::Error;
use crate::git;

/// Handle the `branch` subcommand: return the current git branch, plain.
pub(crate) fn branch_command() -> Result<String, Error> {
    git::current_branch()
}
```

No test module.

#### 3. `src/commands/artifact_directory.rs` — **create**

```rust
use super::resolve;
use crate::errors::Error;
use crate::git;

/// Handle the `artifact_directory` subcommand: return
/// `$PWD/.pi/orksorksorks/<branch>/`, plain (no directory creation).
pub(crate) fn artifact_directory_command() -> Result<String, Error> {
    let cwd = std::env::current_dir()?;
    Ok(resolve::artifact_dir_path(&cwd, &git::current_branch()?))
}
```

No test module (path-composition unit tests already live in `resolve.rs`).

#### 4. `src/commands/mod.rs` — **modify**

- Add `mod init; mod branch; mod artifact_directory;` (with `mod resolve;`).
- Re-point the three router arms in `select_command_with_env`:
  - `init_command(&path)` → `init::init_command(&path)`
  - `branch_command()` → `branch::branch_command()`
  - `artifact_directory_command()` → `artifact_directory::artifact_directory_command()`
- Delete the three handler definitions.
- Imports unchanged this phase (`Error` and `git` are still used by the remaining handlers).

### Verification

#### Automated
- [x] `cargo check` passes
- [x] `scripts/test.sh` passes
- [x] `cargo nextest run init_` passes (integration `tests/init_creates_file.rs`, 8 tests)
- [x] `cargo nextest run branch` passes (integration `tests/branch.rs` + routing test)
- [x] `cargo nextest run artifact_` passes (integration `tests/artifact_directory.rs`)

#### Manual
- [ ] `./target/debug/orksorksorks init --help` matches `baseline/help.txt`-style expectations
      (unchanged clap output)
- [ ] `./target/debug/orksorksorks -j step --config /nonexistent/orks.toml` still emits the same
      error envelope as `baseline/step-missing.json`

---

## Phase 4: Config-driven handlers — `step.rs`, `model.rs`, `thinking.rs`

The three handlers sharing the identical "read config → explicit `--step` or derive from
cwd+git+artifacts → resolve" shape. `resolve.rs` must already exist (Phase 2).

### Changes

#### 1. `src/commands/step.rs` — **create**

```rust
use super::resolve;
use crate::errors::Error;
use crate::git;

/// Handle the `step` subcommand: ... (doc moved verbatim)
pub(crate) fn step_command(
    path: &std::path::Path,
    source: crate::config_dir::ConfigPathSource,
    step: Option<String>,
) -> Result<String, Error> {
    let cfg = crate::config::read_config(path, source)?;
    let step = if let Some(name) = step {
        resolve::resolve_step(&cfg, &name)?
    } else {
        let cwd = std::env::current_dir()?;
        let artifact_dir = resolve::artifact_dir_path(&cwd, &git::current_branch()?);
        resolve::determine_step(&cfg, &artifact_dir)?
    };
    Ok(step.name)
}

#[cfg(test)]
mod tests {
    use super::*;
    // 2 relocated tests (bodies verbatim)
}
```

Relocated tests (cut from `mod.rs`):

| Function (current line) | Asserts |
|---|---|
| `step_command_flag_succeeds_in_non_git_dir` (1239) | `--step` override returns `"one"` with no git repo |
| `step_command_flag_unknown_name_tags_step` (1263) | error `source == "step"`, message contains `no step named "nope"` |

Both call `crate::config_dir::ConfigPathSource::ExplicitFlag` fully qualified and need no extra
imports beyond `super::*`.

#### 2. `src/commands/model.rs` — **create**

Same shape, `pub(crate) fn model_command(path, source, step)`, body moved verbatim from
`mod.rs:301-320`, with `resolve::resolve_step`, `resolve::artifact_dir_path`,
`resolve::determine_step`, `resolve::resolve_model`, `git::current_branch`. Imports:

```rust
use super::resolve;
use crate::errors::Error;
use crate::git;
```

No test module (model is covered by `resolve_model_*` unit tests in `resolve.rs` +
`tests/model.rs` integration; no direct `model_command` unit test exists today).

#### 3. `src/commands/thinking.rs` — **create**

Identical shape to `model.rs` (`thinking_command`, moved verbatim from `mod.rs:322-339`;
returns `model.thinking`). Same three imports. No test module.

#### 4. `src/commands/mod.rs` — **modify**

- Add `mod step; mod model; mod thinking;`.
- Re-point the three arms: `step::step_command(&path, source, step.clone())`,
  `model::model_command(&path, source, step.clone())`,
  `thinking::thinking_command(&path, source, step.clone())`.
- Delete `step_command`, `model_command`, `thinking_command`.
- Delete the two `step_command_*` tests from the test module.
- Imports unchanged (`git` still used by prompt/script handlers; `Error` by the router).

### Verification

#### Automated
- [x] `cargo check` passes
- [x] `scripts/test.sh` passes
- [x] `cargo nextest run step` passes (integration `tests/step.rs` + 2 relocated unit tests)
- [x] `cargo nextest run model` passes (integration `tests/model.rs`)
- [x] `cargo nextest run thinking` passes (integration `tests/model.rs` thinking block)

#### Manual
- [ ] `./target/debug/orksorksorks step --help` matches `baseline/step-help.txt`
- [ ] `./target/debug/orksorksorks -j step --config /nonexistent/orks.toml` matches
      `baseline/step-missing.json`; text mode matches `baseline/step-missing.txt`

---

## Phase 5: Frontmatter/script handlers — `prompt.rs`, `script.rs`

The two remaining handler families, with the largest behavioral pins (frontmatter emission,
positional `script <STEP_NAME>`, `show_frontmatter = false`). Moving them last exercises those
pins against an otherwise-finished split.

### Changes

#### 1. `src/commands/prompt.rs` — **create**

```rust
use super::resolve;
use crate::errors::Error;
use crate::git;

/// Handle the `prompt` subcommand: ... (doc moved verbatim)
pub(crate) fn prompt_command(
    path: &std::path::Path,
    source: crate::config_dir::ConfigPathSource,
    step: Option<String>,
) -> Result<String, Error> {
    // body moved verbatim from mod.rs:372-409
}
```

The `format!("## Important variables\n...")` frontmatter string must be preserved
byte-for-byte; only the helper calls gain `resolve::`.

Relocated tests (cut from `mod.rs`):

| Function (current line) | Asserts |
|---|---|
| `prompt_command_step_flag_unknown_name_tags_step` (1285) | `--step nope` → `source == "step"` |
| `prompt_command_step_flag_known_step_missing_prompt_tags_validation` (1309) | `config:missing-prompt` |

#### 2. `src/commands/script.rs` — **create**

```rust
use super::resolve;
use crate::errors::Error;
use crate::git;

/// Handle the `script` subcommand: ... (doc moved verbatim)
pub(crate) fn script_command(
    path: &std::path::Path,
    source: crate::config_dir::ConfigPathSource,
    step_name: Option<String>,
) -> Result<String, Error> {
    // body moved verbatim from mod.rs:411-434
}
```

Preserve the `"script"` tag for no-script/unknown-name and the no-frontmatter contract.

Relocated test (cut from `mod.rs`):

| Function (current line) | Asserts |
|---|---|
| `script_command_step_without_script_errors` (1336) | `source == "script"`, message contains `has no script configured` |

#### 3. `src/commands/mod.rs` — **modify**

- Add `mod prompt; mod script;`.
- Re-point the final two arms: `prompt::prompt_command(&path, source, step.clone())`,
  `script::script_command(&path, source, step_name.clone())`.
- Delete `prompt_command`, `script_command` and the three relocated tests.
- Remove the now-unused module-level `use crate::git;`.

`mod.rs` top after this phase:

```rust
use crate::errors::Error;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
```

The root now holds only `NAME`/`AUTHOR`/`ABOUT`/`LONG_VERSION`, `Cli`, `Commands`,
`select_command_with_env`, `select_command`, the nine `mod` declarations, and the parse/routing
tests.

### Verification

#### Automated
- [x] `cargo check` passes
- [x] `scripts/test.sh` passes
- [x] `cargo nextest run prompt` passes (integration `tests/prompt.rs` + 2 relocated unit tests)
- [x] `cargo nextest run script` passes (integration `tests/script.rs` + 1 relocated unit test)
- [x] `cargo nextest run json_output` passes (envelope + no-ANSI)
- [x] `cargo clippy --tests -- -D warnings` clean (no unused `crate::git` import)

#### Manual
- [ ] `./target/debug/orksorksorks script --help` matches `baseline/script-help.txt`
- [ ] `./target/debug/orksorksorks -j branch` matches `baseline/branch.json`

---

## Phase 6: Final verification sweep (no new behavior)

Proves the end state matches the design: root file small and readable, nothing outside
`src/commands/` touched, architecture graph unchanged.

### Changes

None functional. Touch `mod.rs` only if a leftover unused import or dead item makes the gate
fail (e.g. removing a stray `use`).

### Verification

#### Automated
- [x] `scripts/test.sh` passes (fmt, check, clippy `-D warnings`, nextest, forbidden strings)
- [x] `cargo nextest run architecture` passes — `tests/architecture.rs` untouched
- [x] `git status --short` shows changes only under `src/commands/` (plus the untracked
      `baseline/` artifact dir)
- [x] `git diff --stat` confirms `tests/` and `src/main.rs` untouched
- [x] `wc -l src/commands/*.rs` — every file under 300 lines **except** `mod.rs` (~600) and
      `resolve.rs` (~560); see "Corrected acceptance criteria"
- [x] Baseline diffs are empty:
  ```bash
  ./target/debug/orksorksorks --help > /tmp/ork-help.txt 2>&1
  diff .pi/orksorksorks/alanvardy-var-981-refactor-orksorksorks/baseline/help.txt /tmp/ork-help.txt
  ./target/debug/orksorksorks -j branch > /tmp/ork-branch.txt 2>&1
  diff .pi/orksorksorks/alanvardy-var-981-refactor-orksorksorks/baseline/branch.json /tmp/ork-branch.txt
  ./target/debug/orksorksorks -j step --config /nonexistent/orks.toml > /tmp/ork-step.json 2>&1
  diff .pi/orksorksorks/alanvardy-var-981-refactor-orksorksorks/baseline/step-missing.json /tmp/ork-step.json
  ```
- [x] `rg -n 'TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:' src/commands/` matches nothing
      (also enforced by `scripts/test.sh`)

#### Manual
- [ ] Per-file diff review: each new file's body is character-identical to its original
      `mod.rs` region apart from `pub(crate)`, the `resolve::` prefixes, and the trailing test
      module relocation. Use `git diff` and the baseline captures to confirm no silent change.
- [ ] Confirm the routing arms read cleanly in `mod.rs` and each namespaced call matches its
      file (e.g. `prompt::prompt_command`), i.e. no typo slipped past the integration tests.

---

## Import ledger (per stage, `src/commands/mod.rs`)

| Phase | `crate::config::{Config, Step}` | `crate::errors::Error` | `crate::git` | `clap` | `std::path::PathBuf` |
|---|---|---|---|---|---|
| Start | yes | yes | yes | yes | yes |
| 2 | **remove** | yes | yes | yes | yes |
| 3 | – | yes | yes | yes | yes |
| 4 | – | yes | yes | yes | yes |
| 5 | – | yes | **remove** | yes | yes |

`mod resolve;` is added in Phase 2 and stays forever (siblings reach it via `super::resolve`);
it is a declaration, not an import, so it never warns.

## New-file import summary

| File | Non-test imports |
|---|---|
| `resolve.rs` | `crate::config::{Config, Step}`, `crate::errors::Error` |
| `init.rs` | `crate::errors::Error` |
| `branch.rs` | `crate::errors::Error`, `crate::git` |
| `artifact_directory.rs` | `super::resolve`, `crate::errors::Error`, `crate::git` |
| `step.rs` | `super::resolve`, `crate::errors::Error`, `crate::git` |
| `model.rs` | `super::resolve`, `crate::errors::Error`, `crate::git` |
| `thinking.rs` | `super::resolve`, `crate::errors::Error`, `crate::git` |
| `prompt.rs` | `super::resolve`, `crate::errors::Error`, `crate::git` |
| `script.rs` | `super::resolve`, `crate::errors::Error`, `crate::git` |

All `crate::` spellings are already in the rust_arkitect allowlist (`crate::config`,
`crate::errors`, `crate::git`); `super::resolve` is an intra-`commands` path and records no
external edge. No `tests/architecture.rs` change is required.

## Deviations from `structure.md` (recorded)

1. **`resolve.rs` does not import `crate::git`** (structure Stage 2 lists it). None of the six
   helpers calls git — the git calls live in the handler files.
2. **`resolve.rs` does not touch `crate::format`.** Only `init_command` uses `green_string`.
3. **No empty `#[cfg(test)] mod tests`.** `init.rs`, `branch.rs`, `artifact_directory.rs`,
   `model.rs`, `thinking.rs` have no relocated handler-direct unit tests (none exist today —
   those behaviors are pinned by routing tests in `mod.rs` and by integration tests). Adding
   empty test modules would be dead code. `step.rs`, `prompt.rs`, `script.rs`, and `resolve.rs`
   carry real trailing test modules.
4. **Stage 6 line-count criterion corrected** from "all files < 300" to the table in
   "Corrected acceptance criteria": `mod.rs` and `resolve.rs` land at ~600/~560 under the
   design's mandated grouping. If strict < 300 is required, re-run `structure`/`design` with
   parse tests or the resolve test group split out — this plan does not do that.
5. **Phase 4 header** in structure says step/model/thinking all have relocated handler tests;
   only `step` does (`model`/`thinking` coverage is `resolve_*` unit tests + `tests/model.rs`).

## Resume rule

Every phase produces a parent-consistent, compiling tree. If a stage's gate fails, fix within
that stage; the phases above it are already independently valuable and can land on their own.