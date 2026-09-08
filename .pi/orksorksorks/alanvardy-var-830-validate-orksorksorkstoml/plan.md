# Implementation Plan

## Overview

Add post-parse validation to `read_config` (`src/config.rs`) so every command (`step`/`model`/`thinking`/`prompt`) fails fast on an invalid `orksorksorks.toml` before any git/artifact work. Validation is a pure `Config::validate()` decision function emitting distinct `config:*` error tags in fixed order; serde-level `deny_unknown_fields` hardens the parse; the error contract (tag, exit 1, JSON envelope, no ANSI) is pinned by integration tests.

---

## Phase 1: Schema hardening (serde parse layer)

Add `deny_unknown_fields` to all four structs. No signature change; parse-time rejection flows through the existing `From<toml::de::Error>` (`src/errors.rs:57-64`) and emits the existing `"toml::de"` tag.

### Changes

#### 1. `Step`, `Model`, `Prompt`, `Config` — add `deny_unknown_fields`
**File**: `src/config.rs`
**Action**: modify

Add a struct-level serde attribute to each of the four derives (they currently have none; `Config` only has field-level `#[serde(default, ...)]` attributes):

```rust
/// A single named step, gated on the presence of a trigger artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Step { /* ... unchanged fields ... */ }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Model { /* ... */ }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Prompt { /* ... */ }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config { /* ... unchanged fields ... */ }
```

#### 2. Unit test — unknown key → `"toml::de"`
**File**: `src/config.rs` (append to `#[cfg(test)] mod tests`)
**Action**: add

```rust
#[test]
fn unknown_step_key_is_rejected_as_toml_de() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("orksorksorks.toml");
    std::fs::write(
        &path,
        "version = \"0.1.0\"\n[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"small\"\nbogus = \"x\"\n",
    )
    .unwrap();
    let err = read_config(&path, ConfigPathSource::ExplicitFlag).unwrap_err();
    assert_eq!(err.source, "toml::de");
}
```

### Verification
#### Automated
- [x] `cargo nextest run --no-tests pass -E 'test(config)'` — new unknown-key test green; existing round-trips (`steps_round_trip`, `models_round_trip`, `prompts_round_trip`, `config_round_trip_serialize_deserialize`) still green (no fixture carries unknown keys)
- [x] `scripts/test.sh` green

#### Manual
- [ ] `cargo run -- step --config <tmp.toml>` with an extra key in `[[steps]]` → exit 1, stderr contains the `toml::de` phrasing, no ANSI

---

## Phase 2: Validation logic (`Config::validate()`)

The pure decision function — the heart of the work. Returns `Ok(())` or the first `Error::new("<tag>", &format!(...))`. Fixed check order (first-error-only):

| # | Tag | Rule |
|---|---|---|
| 1 | `config:version` | `version != "0.1.0"` |
| 2 | `config:duplicate-name` | same `name` twice within any of `steps`/`prompts`/`models` |
| 3 | `config:empty-name` | any `name` empty in any list |
| 4 | `config:empty-model` | any `step.model` empty |
| 5 | `config:duplicate-trigger` | two steps share a non-empty `trigger_artifact` |
| 6 | `config:multiple-default` | more than one empty `trigger_artifact` |
| 7 | `config:missing-prompt` | a `step.name` has no matching `prompts[].name` |
| 8 | `config:missing-model` | a `step.model` has no matching `models[].name` |

### Changes

#### 1. Add `use std::collections::HashSet;`
**File**: `src/config.rs`
**Action**: modify (import block, after `use std::...` — currently only `crate::*` + `serde` imports exist):

```rust
use std::collections::HashSet;
```

#### 2. Add `Config::validate()` + two private helpers
**File**: `src/config.rs`
**Action**: add (place `impl Config` block and helpers after `read_config`)

```rust
/// The config format version this build reads and writes.
pub const CONFIG_VERSION: &str = "0.1.0";

impl Config {
    /// Validate this config, failing fast with a `config:*` tag on the first
    /// violation in a fixed deterministic order.
    ///
    /// Returns `Ok(())` when the config is consistent, otherwise the first
    /// error: `config:version`, `config:duplicate-name`, `config:empty-name`,
    /// `config:empty-model`, `config:duplicate-trigger`,
    /// `config:multiple-default`, `config:missing-prompt`, or
    /// `config:missing-model`.
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.version != CONFIG_VERSION {
            return Err(Error::new(
                "config:version",
                &format!(
                    "unsupported config version {:?} (expected {:?})",
                    self.version, CONFIG_VERSION
                ),
            ));
        }

        let step_names: Vec<&str> = self.steps.iter().map(|s| s.name.as_str()).collect();
        let model_names: Vec<&str> = self.models.iter().map(|m| m.name.as_str()).collect();
        let prompt_names: Vec<&str> = self.prompts.iter().map(|p| p.name.as_str()).collect();

        if let Some(msg) = duplicated_name(&step_names, "steps")
            .or_else(|| duplicated_name(&prompt_names, "prompts"))
            .or_else(|| duplicated_name(&model_names, "models"))
        {
            return Err(Error::new("config:duplicate-name", &msg));
        }

        if let Some(msg) = empty_name(&step_names, "steps")
            .or_else(|| empty_name(&prompt_names, "prompts"))
            .or_else(|| empty_name(&model_names, "models"))
        {
            return Err(Error::new("config:empty-name", &msg));
        }

        for step in &self.steps {
            if step.model.is_empty() {
                return Err(Error::new(
                    "config:empty-model",
                    &format!("step {:?} has an empty model reference", step.name),
                ));
            }
        }

        let mut triggers: HashSet<&str> = HashSet::new();
        for step in &self.steps {
            if !step.trigger_artifact.is_empty() && !triggers.insert(&step.trigger_artifact) {
                return Err(Error::new(
                    "config:duplicate-trigger",
                    &format!(
                        "trigger artifact {:?} is used by more than one step",
                        step.trigger_artifact
                    ),
                ));
            }
        }

        let default_count = self
            .steps
            .iter()
            .filter(|s| s.trigger_artifact.is_empty())
            .count();
        if default_count > 1 {
            return Err(Error::new(
                "config:multiple-default",
                &format!(
                    "{default_count} steps have an empty trigger artifact (at most one default step is allowed)"
                ),
            ));
        }

        let prompt_set: HashSet<&str> = prompt_names.iter().copied().collect();
        for step in &self.steps {
            if !prompt_set.contains(step.name.as_str()) {
                return Err(Error::new(
                    "config:missing-prompt",
                    &format!("step {:?} has no matching prompt", step.name),
                ));
            }
        }

        let model_set: HashSet<&str> = model_names.iter().copied().collect();
        for step in &self.steps {
            if !model_set.contains(step.model.as_str()) {
                return Err(Error::new(
                    "config:missing-model",
                    &format!(
                        "step {:?} references unknown model {:?}",
                        step.name, step.model
                    ),
                ));
            }
        }

        Ok(())
    }
}

/// Return a message naming the duplicated value, if `names` has a duplicate.
fn duplicated_name(names: &[&str], section: &str) -> Option<String> {
    let mut seen = HashSet::new();
    for &name in names {
        if !seen.insert(name) {
            return Some(format!("duplicate name {name:?} in [{section}]"));
        }
    }
    None
}

/// Return a message identifying `section` if any listed name is empty.
fn empty_name(names: &[&str], section: &str) -> Option<String> {
    if names.contains(&"") {
        return Some(format!("[{section}] contains an entry with an empty name"));
    }
    None
}
```

Note: `duplicated_name`/`empty_name` are private (no `missing_docs` obligation) but get doc comments per convention. `CONFIG_VERSION` is a new `pub const` — replace the inline `"0.1.0"` in `Default for Config` and in this module's tests with `CONFIG_VERSION` (or leave the literals; the const is used by `validate` and is the single source for the string).

#### 3. Unit tests for every rule + ordering
**File**: `src/config.rs` (append to `#[cfg(test)] mod tests`)
**Action**: add

Add a helper that parses a TOML string and runs `validate` (no fixture files):

```rust
fn validate_toml(s: &str) -> Result<(), Error> {
    toml::from_str::<Config>(s).unwrap().validate()
}

/// A minimal consistent config used as the happy-path baseline.
const VALID_TOML: &str = concat!(
    "version = \"0.1.0\"\n",
    "[[steps]]\n",
    "name = \"one\"\n",
    "trigger_artifact = \"a.txt\"\n",
    "model = \"small\"\n",
    "[[models]]\n",
    "name = \"small\"\n",
    "model = \"openrouter/deepseek/flash\"\n",
    "thinking = \"high\"\n",
    "[[prompts]]\n",
    "name = \"one\"\n",
    "content = \"one\"\n",
);
```

Then one test per rule + a valid case + an ordering case:

```rust
#[test]
fn validate_accepts_consistent_config() {
    assert!(validate_toml(VALID_TOML).is_ok());
}

#[test]
fn validate_rejects_wrong_version() {
    let err = validate_toml("version = \"9.9.9\"\n").unwrap_err();
    assert_eq!(err.source, "config:version");
}

#[test]
fn validate_rejects_duplicate_step_name() {
    let err = validate_toml(concat!(
        "version = \"0.1.0\"\n",
        "[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"small\"\n",
        "[[steps]]\nname = \"one\"\ntrigger_artifact = \"b.txt\"\nmodel = \"small\"\n",
        "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
        "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
    ))
    .unwrap_err();
    assert_eq!(err.source, "config:duplicate-name");
}
```

Add the remaining seven sad-path tests following the same shape (parse a TOML string via `validate_toml`, `unwrap_err`, `assert_eq!(err.source, "<tag>")`):

- `validate_rejects_duplicate_prompt_name` → `"config:duplicate-name"`
- `validate_rejects_duplicate_model_name` → `"config:duplicate-name"`
- `validate_rejects_empty_step_name` (a `[[steps]]` with `name = ""`) → `"config:empty-name"`
- `validate_rejects_empty_model_reference` (a valid step whose `model = ""`) → `"config:empty-model"`
- `validate_rejects_duplicate_trigger` (two steps sharing `trigger_artifact = "a.txt"`, distinct names/prompts/models) → `"config:duplicate-trigger"`
- `validate_rejects_multiple_defaults` (two steps with `trigger_artifact = ""`) → `"config:multiple-default"`
- `validate_rejects_missing_prompt` (step `one` + model `small`, no `[[prompts]]`) → `"config:missing-prompt"`
- `validate_rejects_missing_model` (step `one` model `nope` + prompt `one`, no `[[models]]`) → `"config:missing-model"`
- `validate_errors_are_deterministic_first_error_wins` (config violating BOTH duplicate-name and missing-prompt → assert `"config:duplicate-name"`)

For ambitious cases (empty-model, missing-prompt, missing-model), the fixture must be otherwise consistent for every EARLIER rule so the target rule is reached — e.g. `missing-model` needs a matching prompt, `duplicate-trigger` needs distinct non-empty triggers + a default-free set + full model/prompt coverage.

### Verification
#### Automated
- [x] `cargo nextest run --no-tests pass -E 'test(config)'` — 1 valid + 8 sad + 1 ordering `validate` tests green; CLI suite untouched (validation not yet wired)
- [x] `scripts/test.sh` green

#### Manual
- [x] `cargo clippy --tests -- -D warnings` clean (doc comments on `CONFIG_VERSION`/`validate`)


---

## Phase 3: Chokepoint wiring + fixture reconciliation

Wire `validate()` into `read_config`, then make every partial test fixture fully consistent so the existing suite stays green under the new coverage checks.

### Changes

#### 1. `read_config` — validate before returning
**File**: `src/config.rs`
**Action**: modify

```rust
pub fn read_config(path: &std::path::Path, source: ConfigPathSource) -> Result<Config, Error> {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => { /* unchanged "io" arm */ }
    };
    let config = toml::from_str(&contents)?;
    config.validate()?;
    Ok(config)
}
```

#### 2. Reconcile the unit fixture in `read_config_loads_steps_from_disk`
**File**: `src/config.rs`
**Action**: modify — this unit test writes a steps-only config (no models/prompts) that would now fail `validate` with `config:missing-prompt`. Add `[[models]]` (small) + `[[prompts]]` (one):

```rust
std::fs::write(
    &path,
    concat!(
        "version = \"0.1.0\"\n",
        "[[steps]]\n",
        "name = \"one\"\n",
        "trigger_artifact = \"a.txt\"\n",
        "model = \"small\"\n",
        "[[models]]\n",
        "name = \"small\"\n",
        "model = \"openrouter/deepseek/flash\"\n",
        "thinking = \"high\"\n",
        "[[prompts]]\n",
        "name = \"one\"\n",
        "content = \"one\"\n",
    ),
)
.unwrap();
```

(`read_config_loads_prompts_from_disk` has zero steps → still valid, no change.)

#### 3. Reconcile `tests/step.rs`
**File**: `tests/step.rs`
**Action**: modify — four inline writers gain models/prompts so every `step.model` resolves and every `step.name` has a prompt:

- `write_config`: add `[[models]]` `small`→`openrouter/deepseek/flash` + `high`→`openrouter/deepseek/pro` (both `thinking = "high"`), and `[[prompts]]` `one`/`two` (`content = "one"`/`"two"`).
- `step_with_default_returns_default_when_no_artifacts`: add `[[models]]` `small` + `[[prompts]]` `one`/`default`.
- `step_real_artifact_beats_default_step`: add `[[models]]` `small`/`high` + `[[prompts]]` `default`/`two`.
- `step_without_flag_reads_config_dir`: add `[[models]]` `small` + `[[prompts]]` `one`.

Each edit is appending `concat!` string literals directly after the last `[[steps]]` entry in that writer, e.g. for the single-step writers:

```rust
"[[models]]\n",
"name = \"small\"\n",
"model = \"openrouter/deepseek/flash\"\n",
"thinking = \"high\"\n",
"[[prompts]]\n",
"name = \"one\"\n",
"content = \"one\"\n",
```

#### 4. Reconcile `tests/model.rs`
**File**: `tests/model.rs`
**Action**: modify + delete

- `write_config`: add `[[prompts]]` `one`/`two` (content not asserted by any model/thinking test — use `content = "one"`/`"two"`).
- `model_without_flag_reads_config_dir` and `thinking_without_flag_reads_config_dir`: add `[[prompts]]` `one`.
- **Delete** `model_unknown_model_reference_fails` and `thinking_unknown_model_reference_fails`. Rationale (deviation, see bottom): `resolve_model`'s runtime `"model"` tag becomes CLI-unreachable once validation guarantees every `step.model` resolves — there is no longer a runtime path for these tests to exercise, `resolve_model`/unit tests remain intact, and dangling-model coverage moves to `config:missing-model` in Phase 4.

#### 5. Reconcile `tests/prompt.rs`
**File**: `tests/prompt.rs`
**Action**: modify

- `write_config`: add `[[models]]` `small`→`openrouter/deepseek/flash` + `high`→`openrouter/deepseek/pro` (`thinking = "high"`) to cover the `model = "small"`/`"high"` refs.
- `prompt_without_flag_reads_config_dir`: add `[[models]]` `small`.
- `prompt_unknown_step_name_fails`: **no change** — it already writes a zero-step config (only `[[prompts]]` `research`), which passes `validate`, and then fails at runtime via the `prompt nope` override → `"prompt"` tag.

#### 6. `read_config_*` unit tests proving validation is wired
**File**: `src/config.rs` (append to tests module)
**Action**: add

```rust
#[test]
fn read_config_rejects_missing_prompt_with_config_tag() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("orksorksorks.toml");
    std::fs::write(
        &path,
        concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"small\"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
        ),
    )
    .unwrap();
    let err = read_config(&path, ConfigPathSource::ExplicitFlag).unwrap_err();
    assert_eq!(err.source, "config:missing-prompt");
}

#[test]
fn read_config_rejects_missing_model_with_config_tag() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("orksorksorks.toml");
    std::fs::write(
        &path,
        concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"nope\"\n",
            "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
        ),
    )
    .unwrap();
    let err = read_config(&path, ConfigPathSource::ExplicitFlag).unwrap_err();
    assert_eq!(err.source, "config:missing-model");
}
```

### Verification
#### Automated
- [x] `scripts/test.sh` green — reconciled fixtures + new wiring coexist; existing `read_config_loads_steps_from_disk` / `_prompts_from_disk` / `_malformed_toml_tags_toml_de` / `_missing_file_tags_io` stay green

#### Manual
- [ ] `cargo run -- step --config <bad.toml>` (step with no prompt) exits 1 with `config:missing-prompt` on stderr, no ANSI
- [ ] `cargo run -- step --config <good.toml>` still prints the winning step name

---

## Phase 4: CLI integration pinning (`config:*` surface)

New integration test file pinning the real binary's error contract for each rule family: exit 1, no ANSI, text-stderr phrase, and JSON `error.source == "config:<tag>"`.

### Changes

#### 1. New `tests/config_validation.rs`
**File**: `tests/config_validation.rs` (new)
**Action**: create — follows the `assert_cmd::Command::cargo_bin("orksorksorks")` + `tempfile::tempdir()` + inline `std::fs::write` pattern; each test initializes a git repo and writes an inline config.

Shared helpers (copy of the standard `init_git_repo` used across `tests/`):

```rust
use assert_cmd::Command;

const BRANCH: &str = "main";

fn git_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let status = std::process::Command::new("git")
        .args(["init", "-b", BRANCH])
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success(), "git init failed");
    dir
}

/// Run `step --config <file>` in text and JSON modes against an invalid
/// config, pinning exit 1, a text phrase, no ANSI, and the JSON source tag.
fn assert_invalid_config(config: &str, text_phrase: &str, json_source: &str) {
    let dir = git_repo();
    std::fs::write(dir.path().join("orksorksorks.toml"), config).unwrap();

    // Text mode: exit 1, stderr phrase, no ANSI.
    let output = Command::cargo_bin("orksorksorks")
        .unwrap()
        .args(["step", "--config", "orksorksorks.toml"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success(), "expected exit 1 for {json_source}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(text_phrase), "stderr: {stderr}");
    assert!(!stderr.contains('\x1b'), "stderr had ANSI: {stderr}");

    // JSON mode: source tag pin on stdout.
    let output = Command::cargo_bin("orksorksorks")
        .unwrap()
        .args(["step", "--config", "orksorksorks.toml", "-j"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["error"]["source"].as_str(), Some(json_source));
    assert!(!stdout.contains('\x1b'), "stdout had ANSI: {stdout}");
}
```

Per-tag fixtures (each a `#[test]` calling `assert_invalid_config`):

- `config_bad_version_fails` → config `"version = \"9.9.9\"\n"`, phrase `"unsupported config version"`, tag `"config:version"`
- `config_duplicate_step_name_fails` → two steps both `name = "one"`, phrase `"duplicate name"`, tag `"config:duplicate-name"`
- `config_empty_name_fails` → step `name = ""`, phrase `"empty name"`, tag `"config:empty-name"`
- `config_empty_model_fails` → step with `model = ""`, phrase `"empty model"`, tag `"config:empty-model"`
- `config_duplicate_trigger_fails` → two steps sharing `trigger_artifact = "a.txt"`, phrase `"used by more than one step"`, tag `"config:duplicate-trigger"`
- `config_multiple_defaults_fail` → two steps `trigger_artifact = ""`, phrase `"empty trigger artifact"`, tag `"config:multiple-default"`
- `config_missing_prompt_fails` → step + model but no prompt, phrase `"no matching prompt"`, tag `"config:missing-prompt"`
- `config_missing_model_fails` → step + prompt but model `nope` missing, phrase `"references unknown model"`, tag `"config:missing-model"`

Each fixture must be otherwise consistent for every earlier rule so the target tag fires (e.g. the empty-name fixture needs model/prompt coverage; the missing-model fixture needs a matching prompt).

Plus a fully-valid success test:

```rust
#[test]
fn valid_config_succeeds() {
    let dir = git_repo();
    std::fs::write(
        dir.path().join("orksorksorks.toml"),
        concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"first.txt\"\nmodel = \"small\"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
            "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
        ),
    )
    .unwrap();
    let artifact_dir = dir.path().join(".pi").join("orksorksorks").join(BRANCH);
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("first.txt"), "").unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd
        .args(["step", "--config", "orksorksorks.toml"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim_end(), "one");
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}
```

### Verification
#### Automated
- [x] `scripts/test.sh` green — new `tests/config_validation.rs` pins the `config:*` contract end-to-end

#### Manual
- [ ] For two tags, `cargo run -- step --config bad.toml` → exit 1 + tag on stderr; `cargo run -- step --config bad.toml -j` → `{"error":{"message":…,"source":"config:<tag>"}}` on stdout

---

## Testing Checkpoints (run in order; halt on any red)

- [x] After Phase 1: `cargo nextest run --no-tests pass -E 'test(config)'`
- [x] After Phase 2: `cargo nextest run --no-tests pass -E 'test(config)'`
- [x] After Phase 3: full `scripts/test.sh`
- [x] After Phase 4: full `scripts/test.sh`

## Deviations from `structure.md` (and why)

- **Deleted `model_unknown_model_reference_fails` / `thinking_unknown_model_reference_fails`** instead of keeping them "valid config + bad runtime lookup". `structure.md` Stage 3 lists them alongside `prompt_unknown_step_name_fails`, but that only holds for the prompt case: `resolve_model` has no runtime override, so once validation guarantees every `step.model` resolves, the `"model"` runtime tag is CLI-unreachable (confirmed by `design.md` Open Risks). Keeping them would leave tests that silently changed what they assert. `config:missing-model` CLI coverage is restored in Phase 4.
- **Added a `config:empty-model` CLI test** (8 failure tests, not the 7 enumerated in Stage 4). Stage 4 says "one integration test per rule family"; `design.md`'s tag table has 8 families and empty-model was omitted from the enumerations but is a real rule, so it is pinned here too.
- **Reconciled `read_config_loads_steps_from_disk`** (a `src/config.rs` unit fixture) in Phase 3. `structure.md` lists it under "existing tests stay green" but not in the explicit fixture-reconciliation file list; without adding models/prompts to its inline TOML, the Stage 3 wiring would break it.