# Research Findings

## Q1: How is the `orksorksorks.toml` config modeled in `src/config.rs`?

### Findings
- Struct hierarchy (all `derive(Debug, Clone, Serialize, Deserialize, PartialEq)`):
  - `Step { name: String, trigger_artifact: String, model: String }` — config.rs:7-14. `name` required (no default), config.rs:9; `trigger_artifact` = filename whose presence in the artifact dir marks the step current, config.rs:11; `model` = "a key into Config::models", config.rs:12-13. **No serde attributes on Step and no optional fields today.**
  - `Model { name, model, thinking }` — config.rs:18-25; `name` is the lookup key, config.rs:20; `model` is the concrete provider id (e.g. `openrouter/...`), config.rs:22; `thinking` is a "reasoning-budget hint", config.rs:24.
  - `Prompt { name, content }` — config.rs:29-34; `name` is "step name this prompt belongs to; also the lookup key", config.rs:31; `content` is "full prompt text (a TOML multi-line string)", config.rs:33. **This is the `name`+blob-text-collection precedent.**
  - `Config { version: String, show_frontmatter: bool, steps: Vec<Step>, models: Vec<Model>, prompts: Vec<Prompt> }` — config.rs:41-56.
- Serde attributes — all on `Config`, none on leaf structs (config.rs:45-55):
  - `show_frontmatter: #[serde(default = "default_true")]` (config.rs:45); `default_true() -> bool { true }` at config.rs:60-62; key always emitted on serialize (no `skip_serializing_if`).
  - `steps` / `models` / `prompts`: each `#[serde(default, skip_serializing_if = "Vec::is_empty")]` (config.rs:48-49, 51-52, 54-55) — missing key ⇒ `Vec::new()`, empty vec omitted from output.
  - `version`: no serde attributes — required on deserialize, always serialized (config.rs:42-43).
- `impl Default for Config` (config.rs:64-73): `version: "0.1.0"`, `show_frontmatter: true`, three empty vecs.
- Format `version` field: doc says "tracks the config format version so future migrations can detect and upgrade older files" (config.rs:38-39). Today it is only written and round-tripped: `Config::default()` sets `"0.1.0"` (config.rs:67); `init_command` writes `toml::to_string(&Config::default())` (commands/mod.rs:138-139). **No code reads `cfg.version`; no migrations/validation exist.**
- `read_config(path, source) -> Result<Config, Error>` (config.rs:81-98) — exactly two error paths:
  1. `std::fs::read_to_string` failure ⇒ `Error::new("io", "could not read config file at {path} ({source}): {e}")` (config.rs:85-91); `source` = `ConfigPathSource` Display phrase ("specified via --config" / "resolved from XDG_CONFIG_HOME" / "resolved from HOME/.config" / "resolved from %APPDATA%", config_dir.rs:59-62).
  2. `toml::from_str(&contents)?` ⇒ `"toml::de"` via `From<toml::de::Error>` (config.rs:96, errors.rs:57-62). Covers malformed TOML and missing required fields (e.g. `Step` without `name`).
  - No version check, no semantic validation on success.
- Callers (all via `?`): `step_command` (commands/mod.rs:230), `model_command` (:244), `thinking_command` (:259), `prompt_command` (:289).
- Unit tests (config.rs:100-291, 12 tests): default serialization contains `version`/`0.1.0`/`show_frontmatter = true` (:106-112); Config/steps/prompts/models round-trips (:115-120, :123-146, :173-187, :216-230); `missing_steps_deserializes_to_empty_vec` (:148-153); `read_config_loads_steps/prompts_from_disk` (:156-169, :190-212); `show_frontmatter_false_round_trips` (:234-247); missing file ⇒ `"io"` with resolution phrase (:252-265, :284-290); malformed ⇒ `"toml::de"` (:270-281).

## Q2: How does the `prompt` subcommand work end to end?

### Findings
- CLI wiring: `Cli::parse()` (main.rs:59) → `run_command` (main.rs:40-51). `Cli` (commands/mod.rs:30-37) derives `Parser, Clone`, `arg_required_else_help = true` (:29), global `-j/--json` bool (:32-33), `pub command: Commands` (:35-36).
- `Commands` enum (commands/mod.rs:40-92): `Prompt { step_name: Option<String> /* positional STEP_NAME, :84-85 */, config: Option<PathBuf> /* --config, :89-90 */ }` (:81-91). `Step`/`Model`/`Thinking` variants carry only `config` (:57-62, :65-70, :73-78).
- Dispatch: `select_command` (:127-129) → `select_command_with_env` (:95-124); Prompt arm resolves `config_file_path_with_env(...)` then `prompt_command(&path, source, step_name.clone())` (:118-122).
- Config-path resolution (config_dir.rs): `ConfigEnv` reads XDG_CONFIG_HOME always, HOME non-Windows, APPDATA on Windows (:26-40); `config_file_path_with_env` = explicit path passthrough, else `resolve_config_dir_with_env(env)` + `FILE_NAME = "orksorksorks.toml"` (:106-113); resolution order XDG → APPDATA → HOME/.config → `Error::new("config-dir", ...)` (:72-99).
- `determine_step(config, artifact_dir) -> Result<Step, Error>` (commands/mod.rs:183-205) — the current-step contract:
  - Returns the whole `Step` object "so callers read different fields" (:175-182); `artifact_dir` must end in a trailing slash, same convention as `artifact_dir_path` (:150-154).
  - Reverse iteration; first step with a present `trigger_artifact` (probed via `Path::try_exists()` at `{artifact_dir}{filename}`, :193-194) wins (:185-195).
  - Empty `trigger_artifact` ⇒ default-step fallback: `default.get_or_insert_with(|| step.clone())`, last-in-list wins (:186-192), returned only when nothing matches (:198-200).
  - Neither ⇒ `Error::new("step", "{artifact_dir}: no trigger artifact matched")` (:201-204).
- `artifact_dir_path(cwd, branch)` (commands/mod.rs:155-161): `format!("{}/.pi/orksorksorks/{}/", cwd, branch.replace('/', "-"))` — branch slashes normalized to hyphens, trailing slash is the documented contract; pure function (no git/I/O).
- `resolve_prompt(config, name) -> Result<String, Error>` (commands/mod.rs:269-276): linear scan of `config.prompts`, first `p.name == name` ⇒ `Ok(p.content.clone())`; none ⇒ `Error::new("prompt", "no prompt named {name:?}")` (debug repr via `{name:?}`).
- `prompt_command(path, source, step_name)` (commands/mod.rs:284-325):
  1. `read_config(path, source)?` **before** any git/artifact logic (:289) — missing/unreadable config deterministically fails `"io"` (doc :222-225).
  2. Name selection (:290-298): explicit `step_name` passes through; else derive `current_dir` + `git::current_branch()` + `artifact_dir_path` + `determine_step(...)?.name`.
  3. `let content = resolve_prompt(&cfg, &name)?;` (:299).
  4. Branch/artifact resolution **deferred until after the prompt resolves** (:301-306) — an unknown prompt keeps its `"prompt"` error instead of a surface a git error.
  5. Frontmatter (:307-314): if `cfg.show_frontmatter`, return `format!("## Important variables\nUse these everywhere you see $<variable>\nstep = {}\nbranch = {}\nartifact_directory = {}\n\n{}", name, branch, artifact_dir, content)`; else `Ok(content)`.
- Printing: `run_command` → `output_result` — text Ok `println!` stdout, Err colored `eprintln!` stderr (main.rs:27-36); JSON `{"data": ...}` / `{"error": {"message", "source"}}` both to stdout (main.rs:39-50); exit 1 on error (main.rs:76-77).
- Same-pattern siblings: `step_command` (:226-235) prints `step.name`; `model_command` (:240-250) adds `resolve_model(&cfg, &step.model)?` → `Ok(model.model)`; `thinking_command` (:255-265) → `Ok(model.thinking)`. `resolve_model` (:210-217) is the `resolve_*` twin returning the whole `Model`.
- git: `current_branch` runs `git branch --show-current` (git.rs:8-27); nonzero exit ⇒ `"git"` tag w/ trimmed stderr (:30-32); empty output ⇒ `"git"` "not on a branch (detached HEAD)" (:33-35).

## Q3: Test-tooling inventory

### Findings
- Runner: nextest. Local: `cargo nextest run --no-tests pass` (scripts/test.sh:14); CI: `cargo nextest run --profile ci --all-features --no-tests pass` (_reusable-test.yml:22-23) with retries=2, fail-fast=false (nextest.toml:2-4).
- Integration tests are black-box: crate is binary-only (no lib target), so tests/ cannot import `Config` (init_creates_file.rs:9-11).
- `tests/` files (all spawn via `assert_cmd::Command::cargo_bin("orksorksorks")`):
  - `artifact_directory.rs` (3 tests): trailing-slash path output; JSON `data` canonicalized; no ANSI; outside-repo failure.
  - `branch.rs` (3): branch text / JSON `data` / no ANSI / outside-repo failure.
  - `init_creates_file.rs` (6): exact default serialization `version = "0.1.0"\nshow_frontmatter = true`; success message; readonly-dir failure; `--config` beats hostile XDG; no env ⇒ `"config-dir"`.
  - `json_output.rs` (2): `init -j` `"data"` envelope; text plain.
  - `model.rs` (12): model/thinking current-step values, first-matching-artifact (manifest trigger), JSON, no-artifact failure, unknown-model failure, config-dir resolution; BELL `\x07` trimming (model.rs:51-58, 204-216).
  - `prompt.rs` (8): inferred step, no-artifact failure, explicit step-name wins, JSON data includes frontmatter, unknown-prompt failure, config-dir resolution, frontmatter default-on, off via `show_frontmatter=false`.
  - `step.rs` (10): current-step, JSON, no-artifact, empty-trigger default, config-dir read, CWD-config-ignored regression, missing-config stderr phrases, JSON error envelope.
- `src/` unit modules: config.rs:101 (12), config_dir.rs:117 (8), commands/mod.rs:320 (43 — clap try_parse routing for every subcommand, dispatch-arms proven by `"io"` on missing config, artifact_dir_path trailing slash, determine_step last-match/default/error, resolve_model `"model"`, resolve_prompt `"prompt"`), errors.rs:67 (7), format.rs:27 (4), git.rs:40 (5).
- Recurring harness (duplicated per file): `git init -b main` temp repos (unborn branch gives `branch --show-current` = main) or `git init` + commit variants; hand-built `write_config` TOML via `concat!`; `artifact_dir(dir)` helper = `dir/.pi/orksorksorks/<BRANCH>`; `XDG_CONFIG_HOME` env injection + `env_remove("HOME")`; JSON assertions decode stdout into `serde_json::Value` then read `data` / `error.message` / `error.source`; every output test asserts no `\x1b` ANSI.
- Platform gating: **none** — no `#if`/whole-file gating on tests; `cfg!(windows)` only in production config_dir.rs:29,34; `cfg!(test)` compile-time disables ANSI in format.rs:7. macOS `/var → /private/var` canonicalization documented in tests (prompt.rs:279-285).

## Q4: Error type, source tags, output rendering

### Findings
- `Error { message: String, source: String }` (errors.rs:10-12), derives Debug, Clone, PartialEq, Eq, **Serialize** (errors.rs:9). Doc contract (errors.rs:4-8): source = lowercase tag; message = human-readable; **Display owns all coloring** — callers never pre-apply ANSI.
- `Display` = `"Error from {yellow source}:\n{red message}"` (errors.rs:26-34); `std::error::Error` impl (:37).
- `From` conversions (implicit `?`): `io` (:39-46), `toml::ser` (:48-54), `toml::de` (:56-62).
- Exhaustive production source tags:
  1. `"io"` — From (errors.rs:42) on `?` std::io::Error at git.rs:9,21 and commands/mod.rs:171,231,245,260,295,308; explicit wrap in `read_config` (config.rs:85-93).
  2. `"toml::ser"` (errors.rs:51); `"toml::de"` (errors.rs:60) via config.rs:96.
  3. `"config-dir"` — config_dir.rs:95-98.
  4. `"git"` — git.rs:30 (non-zero exit, stderr trimmed) and git.rs:34 (detached HEAD).
  5. `"step"` — commands/mod.rs:201-204 (no trigger artifact, no default).
  6. `"model"` — commands/mod.rs:216 (`"no model named {name:?}"`).
  7. `"prompt"` — commands/mod.rs:275 (`"no prompt named {name:?}"`).
- Output rendering (main.rs): `CommandResult { result, json }` (:19-24); `output_text` — Ok `println!` stdout, Err `eprintln!("\n\n{e}")` stderr with two leading blank lines (:27-36); `output_json` — both envelopes to **stdout**, Ok `{"data": ...}` (:42), Err `{"error": {"message", "source"}}` (:46), no ANSI (:39-50); `output_result` selects by `cr.json` (:53-59); `run_command` exits 1 on error after printing (:62-79).

## Q5: Build/lint/test/verify commands and gates

### Findings
- Local gate `bash scripts/test.sh` (set -euo pipefail): `cargo fmt --all` (in place) → `cargo check` → `cargo clippy --tests -- -D warnings` → `cargo nextest run --no-tests pass` → forbidden-string rg gate `rg -i -g "*.rs" "TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:" .` (test.sh:2-21).
- CI (ci-pr.yml, PR → main): lint job = check `--locked --all-features`, `fmt --all -- --check`, `clippy --all-targets --all-features --locked -- -D warnings`, forbidden-string rg (_reusable-lint.yml:15-22); test job = `cargo nextest run --profile ci --all-features --no-tests pass` (_reusable-test.yml:22-23); `upload-coverage` is **never set** by ci-pr.yml so llvm-cov/codecov steps and codecov thresholds are not enforced on PRs (ci-pr.yml:5-11, _reusable-test.yml:24-36).
- `--locked` everywhere in CI (lint :15,19; test :23; install.sh:5); test.sh unlocked. Cargo.lock committed.
- Gotchas: forbidden-string gate is case-insensitive and would flag any `TODO:`/`dbg!`/`FIXTURE:` in a new test; CI checkout is detached HEAD so `git branch --show-current` is empty ⇒ `"git"` detached error (any new test touching artifact-dir/step resolution needs its own temp repo); no `cargo build`/`cargo test` job exists — compile gate is `cargo check`; `build.rs` emits BUILD_TARGET/PROFILE/TIMESTAMP into `LONG_VERSION` (build.rs:3-13, commands/mod.rs:11-17); toolchain pinned 1.98.1 + clippy/rustfmt (rust-toolchain.toml:1-3).

## Cross-Cutting Observations
- The `Prompt { name, content }` struct + `[[prompts]]` collection + `resolve_prompt` + `prompt_command` is the complete end-to-end template for a named-text collection surfaced by a subcommand: schema struct, `Config` Vec field with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`, linear name lookup with `"<name>"` error tag, and a subcommand whose resolve-then-resolve-step ordering keeps unknown-name errors ahead of git errors.
- Every `resolve_*` (`resolve_model` commands/mod.rs:210-217, `resolve_prompt` :269-276) returns the whole object or content rather than mutating config; error messages use `{name:?}` debug repr.
- `determine_step` returns the `Step` object so new step-fields (like a script name) are surfacable without changing step resolution.
- Step struct has no optional fields today — a nullable field would be the first `Option` on `Step`.
- New error tags follow the lowercase convention: `"script"` would mirror `"prompt"`/`"model"`.
- All three text-blob commands (`step`/`model`/`thinking`) + `prompt` share the spine: `read_config` → `current_dir`+`git::current_branch()` → `artifact_dir_path` → `determine_step` → resolve → print content via `output_result`.
- Serde defaults are the backward-compat mechanism: missing collections ⇒ empty vecs; a new optional field/collection is invisible to old config files (config.rs:45-55, :148-153).
- Config collection order is preserved and round-tripped (`Vec` not `HashMap`), so "first match wins" in resolve is deterministic (config.rs:123-146).

## Open Areas
- The `script` field's exact semantics (what a step naming an unknown script should error with; whether the `scripts` collection allows a step referencing multiple scripts) are not answerable from the codebase — no `script` concept exists anywhere (verified `rg '\bscript\b'` = zero matches; locator + git history confirm).
- How `scripts` content should be rendered (frontmatter like `prompt`, or raw like `step`? trailing whitespace policies, multi-line TOML `"""` handling as in prompts_round_trip config.rs:173-187) is a design decision, not a fact about the current code.
- Whether the new `script` subcommand takes an optional `STEP_NAME` positional (mirroring `Prompt`) or not is undetermined by the codebase.