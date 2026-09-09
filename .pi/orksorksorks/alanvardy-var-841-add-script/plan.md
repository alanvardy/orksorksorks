# Implementation Plan

## Overview

Add a named-text `[[scripts]]` collection and a nullable `step.script` reference to the config, surfaced by a new `script` subcommand that prints the raw script content (no frontmatter) for the current or explicitly named step — a near-verbatim twin of `prompt`. No version bump, no migration: serde defaults keep old configs parsing identically.

Phase order and names follow `structure.md` exactly. All work is on the main ticket (no child tickets).

---

## Phase 1 (Stage 1): Config schema — `Script`, `Config.scripts`, `Step.script`

### Changes

#### 1. Add the `Script` struct
**File**: `src/config.rs`
**Action**: modify — insert after the `Prompt` struct (config.rs:29-34), before the `Config` struct. Identical derives to `Prompt`.

```rust
/// A single named script, referenced by `Step::script` and returned by the
/// `script` subcommand.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Script {
    /// Lookup key referenced by `Step::script`.
    pub name: String,
    /// Raw script text (a TOML multi-line string).
    pub content: String,
}
```

#### 2. Extend `Config`
**File**: `src/config.rs`
**Action**: modify — add a `scripts` field after `prompts` (config.rs:54-55). Mirrors the `prompts` attribute exactly.

```rust
    /// Named scripts referenced by `Step::script`, returned by the `script`
    /// subcommand.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scripts: Vec<Script>,
```

#### 3. Extend `Step`
**File**: `src/config.rs`
**Action**: modify — add an optional `script` field after `model` (config.rs:7-14). **Must use both** `default` **and** `skip_serializing_if = "Option::is_none"` — `toml` 1.1.5's serializer errors on `serialize_none`, so a bare `None` would break `steps_round_trip`.

```rust
    /// Optional name of a `[[scripts]]` entry this step references.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
```

#### 4. Extend `Config::default()`
**File**: `src/config.rs`
**Action**: modify — add `scripts: Vec::new(),` to the returned `Self` (config.rs:64-73).

```rust
            prompts: Vec::new(),
            scripts: Vec::new(),
```

#### 5. Fix existing struct literals (mechanical fallout)
**File**: `src/config.rs` (4 `Config` literals + 2 `Step` literals) and `src/commands/mod.rs` (11 `Config` literals + 13 `Step` literals)
**Action**: modify — every `Config { … }` literal now needs `scripts: vec![],`; every `Step { … }` literal now needs `script: None,`.

- `src/config.rs`: `steps_round_trip` (add `scripts: vec![]` to the Config; `script: None` to its 2 Steps), `prompts_round_trip`, `models_round_trip`, `show_frontmatter_false_round_trips` (each add `scripts: vec![]`). Do **not** touch `read_config_*` / `missing_*` tests (they deserialize strings, not literals).
- `src/commands/mod.rs`: all 7 `determine_step_*` tests (6 with 2 Steps + 1 with 1 Step → 13 `Step` literals), plus `resolve_model_*` (2 Configs) and `resolve_prompt_*` (2 Configs). Add `scripts: vec![]` to all 11 Config literals and `script: None` to all 13 Step literals.

Use a sed-style placement: put `scripts: vec![],` adjacent to `prompts: vec![],` in each Config literal, and `script: None,` after each `model: …` line.

#### 6. New tests
**File**: `src/config.rs` (module `#[cfg(test)]`, after `prompts_round_trip`)
**Action**: modify — add four tests.

- `scripts_round_trip` — mirrors `prompts_round_trip`; content contains a trailing newline to exercise TOML multi-line serialization:

```rust
    #[test]
    fn scripts_round_trip() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![],
            models: vec![],
            prompts: vec![],
            scripts: vec![Script {
                name: "run-one".to_string(),
                content: "#!/usr/bin/env bash\n\necho hello\n".to_string(),
            }],
        };
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }
```

- `missing_scripts_deserializes_to_empty_vec` — extend the existing `missing_steps_deserializes_to_empty_vec` (config.rs:148-153): its input `version = "0.1.0"` also lacks `scripts`; add `assert!(config.scripts.is_empty());` alongside the existing three emptiness asserts.
- `step_script_deserializes_to_none` — `toml::from_str` a `[[steps]]` block with no `script` key, assert `config.steps[0].script == None`.
- `step_script_deserializes_to_some` — same block plus `script = "run-one"`, assert `config.steps[0].script.as_deref() == Some("run-one")`.
- `step_script_empty_string_deserializes_to_some_empty` — `script = ""`, assert `config.steps[0].script == Some(String::new())` (documents the deliberate `""` ⇒ `Some("")` semantics).

### Verification
#### Automated
- [x] `cargo check` passes
- [x] `cargo nextest run config::tests` passes — includes the 4 new tests plus **unchanged** `steps_round_trip` (proves the `is_none` skip) and `default_config_serializes_to_expected_toml` (proves `init` output is unchanged)

#### Manual
- [ ] `cargo fmt --all` ; confirm no stray diffs in existing test literals beyond the mechanical field additions

---

## Phase 2 (Stage 2): Resolver layer — `resolve_script`, `resolve_step`

### Changes

#### 1. Add `resolve_script`
**File**: `src/commands/mod.rs`
**Action**: modify — add after `resolve_prompt` (commands/mod.rs:269-276). Note: private `fn`, matching `resolve_prompt`/`resolve_model` (tests are in-module, no `pub` needed — structure.md's `pub` is overridden for codebase consistency).

```rust
/// Look up the named script (a `config.scripts` key referenced by a step's
/// `script` field) and return its raw content.
fn resolve_script(config: &Config, name: &str) -> Result<String, Error> {
    for s in config.scripts.iter() {
        if s.name == name {
            return Ok(s.content.clone());
        }
    }
    Err(Error::new("script", &format!("no script named {name:?}")))
}
```

#### 2. Add `resolve_step`
**File**: `src/commands/mod.rs`
**Action**: modify — add after `resolve_model` (commands/mod.rs:210-217). New helper: `determine_step` resolves by artifact, not name; the explicit-`STEP_NAME` path in Phase 3 needs name lookup. `Step` is already imported at the top of the file.

```rust
/// Look up a step by name (used by the explicit `STEP_NAME` path of the
/// `script` subcommand, which resolves by name — unlike `determine_step`,
/// which resolves by trigger artifact).
fn resolve_step(config: &Config, name: &str) -> Result<Step, Error> {
    for s in config.steps.iter() {
        if s.name == name {
            return Ok(s.clone());
        }
    }
    Err(Error::new("step", &format!("no step named {name:?}")))
}
```

#### 3. New tests
**File**: `src/commands/mod.rs` (`#[cfg(test)]`)
**Action**: modify — add five tests (all build `Config` literals, so include the `scripts:`/`script:` field fixes from Phase 1):

- `resolve_script_hit_returns_content` — one `Script { name: "run-one", content: "#!/bin/bash\n" }`, assert `resolve_script(&config, "run-one").unwrap() == "#!/bin/bash\n"`.
- `resolve_script_miss_returns_script_tag` — `unwrap_err()`, assert `err.source == "script"` and `err.message.contains("no script named")`.
- `resolve_script_first_match_wins` — two `Script`s, first `name == "dup"`, second `name == "dup"` with different content; assert the first's content wins.
- `resolve_step_hit` — one `Step { name: "one", … , script: Some("run-one") }`; assert `resolve_step(&config, "one").unwrap().name == "one"`.
- `resolve_step_miss_returns_step_tag` — `unwrap_err()`, assert `err.source == "step"` and `err.message.contains("no step named")`.

### Verification
#### Automated
- [x] `cargo check` passes
- [x] `cargo nextest run commands::tests::resolve_` passes (matches `resolve_script_*` + `resolve_step_*` + the pre-existing `resolve_model_*` / `resolve_prompt_*`)

#### Manual
- [ ] Confirm no `resolve_*` function exists that mutates `Config` — all return owned clones/content and take `&Config`

---

## Phase 3 (Stage 3): Command + CLI wiring — `script_command`, `Script` variant, dispatch

### Changes

#### 1. Add the `Script` command variant
**File**: `src/commands/mod.rs`
**Action**: modify — add after the `Prompt` variant (commands/mod.rs:81-91). Positional `STEP_NAME` + `--config`, mirror of `Prompt`.

```rust
    /// Print the raw script for the current step (from `config.scripts`)
    Script {
        /// Step name override; defaults to the current step derived from
        /// present trigger artifacts
        #[arg(value_name = "STEP_NAME")]
        step_name: Option<String>,

        /// Path to the TOML config file; defaults to the config directory
        /// (the same resolution as `init`)
        #[arg(long, value_name = "CONFIG")]
        config: Option<PathBuf>,
    },
```

#### 2. Add the dispatch arm
**File**: `src/commands/mod.rs`
**Action**: modify — add after the `Prompt` arm in `select_command_with_env` (commands/mod.rs:118-122).

```rust
        Commands::Script { step_name, config } => {
            let (path, source) =
                crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            script_command(&path, source, step_name.clone())
        }
```

#### 3. Add `script_command`
**File**: `src/commands/mod.rs`
**Action**: modify — add after `prompt_command` (commands/mod.rs:284-325). Ordering mirrors `prompt_command` (`read_config` first; step resolution; script-name check; content resolve); no frontmatter, so no post-resolve git/branch work.

```rust
/// Handle the `script` subcommand: read the config and return the raw
/// `[[scripts]]` content referenced by the current step's `script` field,
/// or by the explicitly named step when `step_name` overrides detection.
/// No frontmatter is emitted (the content is meant to be run/piped).
fn script_command(
    path: &std::path::Path,
    source: crate::config_dir::ConfigPathSource,
    step_name: Option<String>,
) -> Result<String, Error> {
    let cfg = crate::config::read_config(path, source)?;
    let step = if let Some(name) = step_name {
        // Explicit name: resolve by name (no git/artifact work), so unknown
        // script-name / no-script errors surface as `"script"`, never `"git"`.
        resolve_step(&cfg, &name)?
    } else {
        // No explicit step: derive the current step from trigger artifacts,
        // exactly like `step`/`model`/`thinking`/`prompt`.
        let cwd = std::env::current_dir()?;
        let artifact_dir = artifact_dir_path(&cwd, &git::current_branch()?);
        determine_step(&cfg, &artifact_dir)?
    };
    let script_name = step.script.ok_or_else(|| {
        Error::new(
            "script",
            &format!("step {:?} has no script configured", step.name),
        )
    })?;
    resolve_script(&cfg, &script_name)
}
```

#### 4. New tests
**File**: `src/commands/mod.rs` (`#[cfg(test)]`)
**Action**: modify — add routing + dispatch + command tests, mirroring the `Prompt` counterparts:

- `cli_try_parse_accepts_script` — `["orksorksorks", "script", "run-one"]` is `Ok`.
- `cli_try_parse_script_without_step_name_is_none` — `script` parses; `step_name == None`, `config == None`.
- `cli_try_parse_script_reads_step_name` — `script run-one` ⇒ `step_name == Some("run-one")`.
- `cli_try_parse_script_with_custom_config` — `script run-one --config custom.toml` ⇒ both fields populated.
- `select_command_routes_script` — `Commands::Script { step_name: None, config: Some(PathBuf::from("definitely-missing-config-file.toml")) }`; `select_command(&cli).unwrap_err().source == "io"` (proves dispatch reached `script_command`, whose `read_config` fails first).

- `script_command_step_without_script_errors` — write a temp config with `version` + one `[[steps]]` entry named `one` (no `script` key) and no `[[scripts]]`; call `script_command(&path, ConfigPathSource::ExplicitFlag, Some("one".to_string()))`. Assert `err.source == "script"` and `err.message.contains("has no script configured")`. (Explicit-name path avoids git, so the test needs no repo.)

  `script_command` is private; the test may call it directly (same module).

### Verification
#### Automated
- [x] `cargo check` passes
- [x] `cargo nextest run commands::tests::` passes — includes the new routing/dispatch tests plus the 43 pre-existing ones

#### Manual
- [ ] `cargo run -- script --help` shows the `STEP_NAME` positional + `--config`, distinct from `prompt --help`

---

## Phase 4 (Stage 4): Black-box integration — `tests/script.rs` + full gate

### Changes

#### 1. New integration test file
**File**: `tests/script.rs`
**Action**: create — copy the harness from `tests/prompt.rs` (binary-only crate, so harness is duplicated, not imported): `init_git_repo()` (`git init -b main` unborn-branch), `artifact_dir()` = `dir/.pi/orksorksorks/main`, `assert_cmd::Command::cargo_bin("orksorksorks")`, plus `serde_json` envelope decoding. No `\x1b` in any asserted output.

Key harness snippets (adapted from `prompt.rs`):

```rust
const BRANCH: &str = "main";

fn init_git_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let status = std::process::Command::new("git")
        .args(["init", "-b", BRANCH])
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success(), "git init failed");
    dir
}

fn artifact_dir(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join(".pi").join("orksorksorks").join(BRANCH)
}
```

`write_config(dir)` — two steps, both with `script` fields, two `[[scripts]]` entries:

```rust
fn write_config(dir: &std::path::Path) {
    std::fs::write(
        dir.join("orksorksorks.toml"),
        concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\n", "name = \"one\"\n", "trigger_artifact = \"first.txt\"\n",
            "model = \"small\"\n", "script = \"run-one\"\n",
            "[[steps]]\n", "name = \"two\"\n", "trigger_artifact = \"second.txt\"\n",
            "model = \"high\"\n", "script = \"run-two\"\n",
            "[[scripts]]\n", "name = \"run-one\"\n", "content = \"\"\"\n",
            "# Script for step one\n", "\"\"\"\n",
            "[[scripts]]\n", "name = \"run-two\"\n", "content = \"\"\"\n",
            "# Script for step two\n", "\"\"\"\n",
        ),
    )
    .unwrap();
}
```

`write_config_step_without_script(dir)` — same, but step `two` has **no** `script` key (and only a `run-one` `[[scripts]]` entry) for the no-script sad path.

#### 2. Tests (7 assertions across 6 tests)
- `script_without_step_name_infers_current_step` — `write_config`, write `second.txt` artifact → current step `two` → `run-two`; stdout contains `# Script for step two`; stdout does **not** contain `## Important variables`; no `\x1b`.
- `script_with_explicit_step_name_overrides_step` — `write_config`, write `second.txt` (derived step would be `two`), run `script one --config …`; stdout contains `# Script for step one`; no `\x1b`.
- `script_json_returns_valid_json_with_data_field` — `write_config`, write `first.txt`, run `-j`; decode `serde_json::Value`; `v["data"].as_str()` contains `# Script for step one` and does **not** start with `## Important variables`; no `\x1b`.
- `script_unknown_script_name_fails` — temp config: one `[[steps]]` `one` with `script = "nope"`, one `[[scripts]]` `run-other`; run `script one --config … -j`; assert failure + JSON `error.source == "script"` + `error.message` contains `no script named`.
- `script_step_without_script_field_fails` — `write_config_step_without_script`, run `script two --config … -j` (or write `second.txt` and omit the positional); assert failure + `error.source == "script"` + `error.message` contains `has no script configured`.
- `script_without_flag_reads_config_dir` — mirror `prompt_without_flag_reads_config_dir`: config in `$XDG_CONFIG_HOME/orksorksorks.toml` (one step + one matching script), write `first.txt`, run bare `script` with `XDG_CONFIG_HOME` set; stdout contains the content; no `\x1b`.

### Verification
#### Automated
- [ ] `bash scripts/test.sh` — the only merge gate (see conventions): `cargo fmt --all` clean, `cargo check`, `cargo clippy --tests -- -D warnings`, `cargo nextest run --no-tests pass`, forbidden-string `rg` (no `TODO:`/`dbg!`/`FIXTURE:` etc. in `tests/script.rs`)

#### Manual
- [ ] `cargo build && ./target/debug/orksorksorks script --help` renders correctly
- [ ] Run `./target/debug/orksorksorks script` in a real repo with a script-configured step and confirm raw content on stdout (no ANSI, no frontmatter); `echo $?` is `0`
- [ ] `./target/debug/orksorksorks script nonexistent-step -j` exits `1` with `{"error":{"message":…,"source":"step"}}`

---

## Testing Checkpoints (resume points)

- **After Phase 1**: `cargo nextest run config::tests` green; `steps_round_trip` / `default_config_serializes_to_expected_toml` unchanged.
- **After Phase 2**: `cargo nextest run commands::tests::resolve_` green.
- **After Phase 3**: `cargo nextest run commands::tests::` green.
- **After Phase 4**: `bash scripts/test.sh` green — merge gate.

## Notes / deviations from `structure.md`

1. `resolve_script`, `resolve_step`, and `script_command` are declared private `fn` (not `pub`), matching the codebase convention for `resolve_prompt`/`resolve_model`/`prompt_command` (all in-module-tested).
2. Error wording pinned for the plan: unknown script ⇒ `"no script named {name:?}"`; no-script step ⇒ `"step {name:?} has no script configured"` (both `source == "script"`, exit 1).
3. `missing_scripts_deserializes_to_empty_vec` is implemented as an added assertion in the existing `missing_steps_deserializes_to_empty_vec` rather than a standalone test, since the same minimal input exercises both.
4. No schema/version changes and no migrations — the plan intentionally adds no `version` bump and no codegen steps (there is none in this crate).