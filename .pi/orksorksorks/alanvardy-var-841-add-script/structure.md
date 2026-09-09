# Structure Outline

## Approach

Add a named-text `[[scripts]]` collection and a nullable `step.script` reference, surfaced by a new `script` subcommand that is a near-verbatim twin of `prompt` (`Prompt` → `Config.prompts` → `resolve_prompt` → `prompt_command`). Built bottom-up in horizontal layers, each fully tested before the next: config schema → pure resolvers → command/CLI wiring → black-box integration. No version bump, no migration (serde defaults keep old files parsing identically).

> **Two refinements to `design.md` found during structuring** (carried here, flagged to owner):
> 1. `Step.script` must be `#[serde(default, skip_serializing_if = "Option::is_none")]`, not `#[serde(default)]` alone. `toml` 1.1.5's serializer **errors on `serialize_none`** (verified in `toml-1.1.5+spec-1.1.0/src/ser/document/mod.rs:143-145`), so a bare `None` field would break the existing `steps_round_trip` (config.rs:122-146) on every serialized `Step`.
> 2. The explicit `STEP_NAME` path needs a name-based step lookup that does not exist today (`determine_step` resolves by artifact, not name; `prompt` sidesteps this because prompt names *are* step names). Add `resolve_step` (Stage 2).

---

## Stage 1: Config schema — `Script`, `Config.scripts`, `Step.script`

Delivers the data types the rest of the feature consumes. Green tests prove old configs still parse (`scripts` absent ⇒ empty vec, `Step` without `script` ⇒ `None`) and new configs round-trip (including multi-line `"""` content and the `None` field omitting itself on serialization).

**Files**: `src/config.rs`

**Key changes**:
- `pub struct Script { pub name: String, pub content: String }` — new, twin of `Prompt` (config.rs:29-34), same derives.
- `Config` gains `#[serde(default, skip_serializing_if = "Vec::is_empty")] pub scripts: Vec<Script>` (mirrors `prompts`, config.rs:54-55).
- `Step` gains `#[serde(default, skip_serializing_if = "Option::is_none")] pub script: Option<String>` (see refinement #1).
- `Config::default()` adds `scripts: Vec::new()`.

**Tests** (src/config.rs, extending the 12 existing): `scripts_round_trip` (mirrors `prompts_round_trip` :173-187, includes multi-line `"""` + trailing newline → happy); `missing_scripts_deserializes_to_empty_vec` (extend `missing_steps_…` :148-153); `step_script_deserializes_to_none` / `…_to_some` (both happy; `script = ""` ⇒ `Some("")` as a deliberate sad-path edge); **existing `steps_round_trip` and `default_config_serializes_to_expected_toml` must still pass** (proves the `is_none` skip and that `init` output is unchanged).

**Verify**: `cargo check` && `cargo nextest run config::tests`

---

## Stage 2: Resolver layer — `resolve_script`, `resolve_step`

Pure, side-effect-free name lookups over a ready `Config`. Green tests prove hit/miss/first-match semantics and the exact lowercase error tags the command will surface.

**Files**: `src/commands/mod.rs`

**Key changes**:
- `pub fn resolve_script(config: &Config, name: &str) -> Result<String, Error>` — linear scan of `config.scripts`, first `s.name == name` ⇒ `Ok(s.content.clone())`, else `Error::new("script", "no script named {name:?}")`. Twin of `resolve_prompt` (:269-276).
- `pub fn resolve_step(config: &Config, name: &str) -> Result<Step, Error>` — **new** (refinement #2): linear scan of `config.steps`, first `s.name == name` ⇒ `Ok(s.clone())`, else `Error::new("step", "no step named {name:?}")`. Consumed only by the explicit-`STEP_NAME` path in Stage 3.

**Tests** (src/commands/mod.rs `#[cfg(test)]`): `resolve_script_hit_returns_content` (happy); `resolve_script_miss_returns_script_tag` (sad — assert `source == "script"`); `resolve_script_first_match_wins` (happy/order); `resolve_step_hit` / `resolve_step_miss_returns_step_tag` (happy + sad).

**Verify**: `cargo check` && `cargo nextest run commands::tests::resolve_`

---

## Stage 3: Command + CLI wiring — `script_command`, `Script` variant, dispatch

Exposes the resolvers through the CLI with `prompt`'s exact ordering (config read first; step resolution; script-name (None) check; script-content resolve) so `"script"` errors precede `"git"` wherever possible, and the missing-config `"io"` dispatch proof holds.

**Files**: `src/commands/mod.rs`

**Key changes**:
- `Commands` gains `Script { step_name: Option<String>, config: Option<PathBuf> }` (mirrors `Prompt`, :81-91) — `clap` positional must not collide with `Prompt`'s `STEP_NAME`.
- Dispatch arm in `select_command_with_env`: resolve config path, then `script_command(&path, source, step_name.clone())` (mirrors :118-122).
- `pub fn script_command(path: &Path, source: ConfigPathSource, step_name: Option<String>) -> Result<String, Error>`:
  1. `let cfg = read_config(path, source)?;`
  2. `let step = match &step_name { Some(n) => resolve_step(&cfg, n)?, None => { … current_dir + git::current_branch + artifact_dir_path + determine_step(&cfg, &artifact_dir)? … } };`
  3. `let script_name = step.script.ok_or_else(|| Error::new("script", …naming the step…))?;`
  4. `resolve_script(&cfg, &script_name)` — returns raw content, **no frontmatter** (`show_frontmatter` ignored).
- Two `"script"` branches as designed: unknown referenced script name (via `resolve_script`), and step with `script: None`. Note: in the default path `determine_step` (git/artifact) necessarily runs before step 3 — consistent with `prompt_command`.

**Tests** (src/commands/mod.rs): clap `try_parse` routing for `script [STEP_NAME] [--config]` (mirrors the routing table at :320+); dispatch arm proven via `"io"` on a missing config (existing pattern); `script_command` unit test for the `None` step ⇒ `"script"` branch (sad).

**Verify**: `cargo check` && `cargo nextest run commands::tests::`

---

## Stage 4: Black-box integration — `tests/script.rs` + full gate

Proves the feature end-to-end through the compiled binary (no lib target — harness copied, not imported), closing the loop on the current-step and explicit-step paths and the JSON/text envelopes.

**Files**: `tests/script.rs` (new; copy the `git init -b main` + `write_config` + `artifact_dir` + `assert_cmd` + JSON-envelope helpers from `tests/prompt.rs`, per conventions).

**Tests**: current-step script text (happy); explicit `STEP_NAME` wins over artifact detection (happy); JSON `"data"` envelope (happy); unknown script name ⇒ `"script"` (sad); step with no `script` field ⇒ `"script"` (sad); `XDG_CONFIG_HOME` read (happy); no `\x1b` ANSI in every output (asserted in each test).

**Verify**: `bash scripts/test.sh` — full local gate (`cargo fmt --all` clean, `cargo check`, `cargo clippy --tests -- -D warnings`, `cargo nextest run --no-tests pass`, forbidden-string `rg`). No ANSI, no `TODO:`/`dbg!` strings.

---

## Testing Checkpoints

Resume points if context resets — each line is the state that must be green before advancing:

- **After Stage 1**: config schema + its unit tests green (`cargo nextest run config::tests`); `steps_round_trip`/`default_config_…` unchanged.
- **After Stage 2**: `resolve_script` + `resolve_step` unit tests green (`commands::tests::resolve_`).
- **After Stage 3**: `script_command` + clap routing + dispatch `"io"` proof green (`commands::tests::`).
- **After Stage 4**: full `bash scripts/test.sh` green — the only merge gate.