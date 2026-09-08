# Implementation Plan

## Overview

Add a per-subcommand `--step <NAME>` override to `step`/`model`/`thinking`/`prompt` that replaces artifact-based step derivation with a direct `config.steps` name lookup via a new `resolve_step` helper, and remove `prompt`'s positional `STEP_NAME`. No schema changes, no global flag, no changes to `init`/`branch`/`artifact_directory`.

All editing is in one production file, `src/commands/mod.rs`, plus integration tests in `tests/{step,model,prompt}.rs`. Layers are built bottom-up; each stage leaves `scripts/test.sh` green before the next starts.

> **Deviation from `structure.md` (see note at end):** the `prompt` positional `STEP_NAME` is removed and its two integration tests rewritten in **Stage 3** (the behavior-wiring stage), not Stage 2. This keeps Stage 2 genuinely behavior-neutral — removing the positional in Stage 2 would break `tests/prompt.rs` (`prompt one`, `prompt nope`) before the override is wired, so the "existing tests pass unchanged" Stage-2 gate could not hold.

---

## Stage 1: Domain layer — `resolve_step` lookup

### Changes

#### 1. New resolver helper
**File**: `src/commands/mod.rs`
**Action**: modify (add `resolve_step` after `resolve_model`, which ends right before `step_command`)

`Step` is already imported at the top (`use crate::config::{Config, Step};`), so the return type needs no new import.

```rust
/// Look up the named step in `config.steps` and return the matching entry.
/// First match wins; an unknown name errors with the `"step"` tag (the same
/// tag `determine_step` uses, so callers discriminate by message).
fn resolve_step(config: &Config, name: &str) -> Result<Step, Error> {
    for s in config.steps.iter() {
        if s.name == name {
            return Ok(s.clone());
        }
    }
    Err(Error::new("step", &format!("no step named {name:?}")))
}
```

#### 2. Unit tests
**File**: `src/commands/mod.rs` (inside the existing `#[cfg(test)] mod tests`)
**Action**: modify (add three tests mirroring the `resolve_model_*` tests)

```rust
#[test]
fn resolve_step_returns_matching_step() {
    let config = Config {
        version: "0.1.0".to_string(),
        steps: vec![Step {
            name: "one".to_string(),
            trigger_artifact: "first.txt".to_string(),
            model: "small".to_string(),
        }],
        models: vec![],
        prompts: vec![],
    };
    let step = resolve_step(&config, "one").unwrap();
    assert_eq!(step.name, "one");
    assert_eq!(step.trigger_artifact, "first.txt");
    assert_eq!(step.model, "small");
}

#[test]
fn resolve_step_unknown_name_tags_step_error() {
    let config = Config {
        version: "0.1.0".to_string(),
        steps: vec![Step {
            name: "one".to_string(),
            trigger_artifact: String::new(),
            model: "small".to_string(),
        }],
        models: vec![],
        prompts: vec![],
    };
    let err = resolve_step(&config, "nope").unwrap_err();
    assert_eq!(err.source, "step");
    assert!(
        err.message.contains("no step named \"nope\""),
        "{}",
        err.message
    );
}

#[test]
fn resolve_step_first_match_wins() {
    let config = Config {
        version: "0.1.0".to_string(),
        steps: vec![
            Step { name: "dup".to_string(), trigger_artifact: String::new(), model: "small".to_string() },
            Step { name: "dup".to_string(), trigger_artifact: String::new(), model: "high".to_string() },
        ],
        models: vec![],
        prompts: vec![],
    };
    let step = resolve_step(&config, "dup").unwrap();
    assert_eq!(step.model, "small");
}
```

### Verification

#### Automated
- [x] `scripts/test.sh` passes (fmt, check, clippy `-D warnings`, nextest, TODO/dbg gate)
- [x] `cargo nextest run resolve_step` passes

#### Manual
- [ ] None — the helper is not yet reachable from the CLI.

---

## Stage 2: Parameter surface — clap `--step` flag + dispatch threading (behavior-neutral)

Adds `--step` to all four variants and threads it through dispatch as an ignored `_step` binding. `prompt`'s positional `step_name` is **kept** here (removed in Stage 3). No handler behavior changes.

### Changes

#### 1. `Commands` enum — add `step` field to four variants
**File**: `src/commands/mod.rs`
**Action**: modify

`Step`/`Model`/`Thinking` each gain a second field (same shape as `--config`):

```rust
    Step {
        /// Path to the TOML config file; defaults to the config directory
        /// (the same resolution as `init`)
        #[arg(long, value_name = "CONFIG")]
        config: Option<PathBuf>,

        /// Override the current step (from `config.steps`) by name
        #[arg(long, value_name = "STEP")]
        step: Option<String>,
    },
```

Apply the identical `step` field to `Model` and `Thinking`.

`Prompt` keeps its positional for now and appends the flag:

```rust
    Prompt {
        /// Step name override; defaults to the current step derived from
        /// present trigger artifacts
        #[arg(value_name = "STEP_NAME")]
        step_name: Option<String>,

        /// Path to the TOML config file; defaults to the config directory
        /// (the same resolution as `init`)
        #[arg(long, value_name = "CONFIG")]
        config: Option<PathBuf>,

        #[arg(long, value_name = "STEP")]
        step: Option<String>,
    },
```

> The positional + flag coexist only for this stage (never shipped). `step_name` and `step` are independent clap fields, so nothing conflicts.

#### 2. Dispatch `match` — destructure and pass `step` through
**File**: `src/commands/mod.rs` (`select_command_with_env`)
**Action**: modify

```rust
        Commands::Step { config, step } => {
            let (path, source) =
                crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            step_command(&path, source, step.clone())
        }
        Commands::Model { config, step } => {
            let (path, source) =
                crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            model_command(&path, source, step.clone())
        }
        Commands::Thinking { config, step } => {
            let (path, source) =
                crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            thinking_command(&path, source, step.clone())
        }
        Commands::Prompt { step_name, config, step } => {
            let (path, source) =
                crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            prompt_command(&path, source, step_name.clone(), step.clone())
        }
```

#### 3. Handler signatures — trailing ignored `_step`
**File**: `src/commands/mod.rs`
**Action**: modify (signatures only; bodies unchanged)

```rust
fn step_command(
    path: &std::path::Path,
    source: crate::config_dir::ConfigPathSource,
    _step: Option<String>,
) -> Result<String, Error> {
    // ... body unchanged ...
}
```

Same trailing `_step: Option<String>` for `model_command` and `thinking_command`. `prompt_command` gains the flag as a 4th arg while keeping `step_name`:

```rust
fn prompt_command(
    path: &std::path::Path,
    source: crate::config_dir::ConfigPathSource,
    step_name: Option<String>,
    _step: Option<String>,
) -> Result<String, Error> {
    // ... body unchanged (still branches on step_name positional) ...
}
```

The `_` prefix keeps clippy's `-D warnings` green while the field is ignored.

#### 4. Update existing unit tests (compile-unit churn)
**File**: `src/commands/mod.rs` (`#[cfg(test)] mod tests`)
**Action**: modify — every `match` destructure / struct construction of the four variants must mention the new `step` field.

Add `step` to the destructures/constructions in:
- `cli_try_parse_accepts_step_with_custom_config`, `cli_try_parse_step_without_config_is_none`, `select_command_routes_step`
- `cli_try_parse_model_without_config_is_none`, `cli_try_parse_model_with_custom_config`, `select_command_routes_model`
- `cli_try_parse_thinking_without_config_is_none`, `cli_try_parse_thinking_with_custom_config`, `select_command_routes_thinking`
- `cli_try_parse_prompt_without_step_name_derives_automatically`, `cli_try_parse_prompt_reads_step_name`, `cli_try_parse_prompt_with_custom_config`, `select_command_routes_prompt`

`match` arms can use `..` to ignore `config`/`step_name` where only one field is asserted, e.g. `Commands::Step { step, .. }`. Constructions (`select_command_routes_*`, which build `Cli { command: Commands::Step { config: Some(...) } }`) add `step: None` explicitly.

#### 5. New parse tests (`--step` binds / absent → None)
**File**: `src/commands/mod.rs` (`#[cfg(test)] mod tests`)
**Action**: modify (add; mirror for all four subcommands)

```rust
#[test]
fn cli_try_parse_step_flag_binds_value() {
    use clap::Parser;
    let cli = Cli::try_parse_from(["orksorksorks", "step", "--step", "one"]).unwrap();
    match cli.command {
        Commands::Step { step, .. } => assert_eq!(step, Some("one".to_string())),
        _ => panic!("expected Commands::Step"),
    }
}

#[test]
fn cli_try_parse_step_flag_absent_is_none() {
    use clap::Parser;
    let cli = Cli::try_parse_from(["orksorksorks", "step"]).unwrap();
    match cli.command {
        Commands::Step { step, .. } => assert_eq!(step, None),
        _ => panic!("expected Commands::Step"),
    }
}
```

Add the same pair for `model`, `thinking`, and `prompt` (match `Commands::Prompt { step, .. }`). The "prompt positional now rejected" test is deferred to Stage 3 (see deviation note).

### Verification

#### Automated
- [x] `scripts/test.sh` passes — the full existing `tests/{step,model,prompt}.rs` suite is untouched and green
- [x] `cargo nextest run cli_try_parse` passes

#### Manual
- [ ] `cargo run -- step --help`, `cargo run -- prompt --help` show the `--step <STEP>` option; `prompt --help` still shows the `[STEP_NAME]` positional this stage.

---

## Stage 3: Application layer — handler override branching

Wires the flag into all four handlers and removes `prompt`'s positional (the confirmation of the Stage-2 deviation: flag-decl and dispatch are re-touched here together).

### Changes

#### 1. `Commands::Prompt` — drop positional, keep flag
**File**: `src/commands/mod.rs`
**Action**: modify

```rust
    Prompt {
        /// Step name override; defaults to the current step derived from
        /// present trigger artifacts
        #[arg(long, value_name = "STEP")]
        step: Option<String>,

        /// Path to the TOML config file; defaults to the config directory
        /// (the same resolution as `init`)
        #[arg(long, value_name = "CONFIG")]
        config: Option<PathBuf>,
    },
```

#### 2. Dispatch arm — `Prompt` loses `step_name`
**File**: `src/commands/mod.rs` (`select_command_with_env`)
**Action**: modify

```rust
        Commands::Prompt { step, config } => {
            let (path, source) =
                crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            prompt_command(&path, source, step.clone())
        }
```

#### 3. `step_command` — branch on `step`
**File**: `src/commands/mod.rs`
**Action**: modify

```rust
fn step_command(
    path: &std::path::Path,
    source: crate::config_dir::ConfigPathSource,
    step: Option<String>,
) -> Result<String, Error> {
    let cfg = crate::config::read_config(path, source)?;
    if let Some(name) = step {
        return Ok(resolve_step(&cfg, &name)?.name);
    }
    let cwd = std::env::current_dir()?;
    let artifact_dir = artifact_dir_path(&cwd, &git::current_branch()?);
    let step = determine_step(&cfg, &artifact_dir)?;
    Ok(step.name)
}
```

#### 4. `model_command` / `thinking_command` — same branch
**File**: `src/commands/mod.rs`
**Action**: modify

```rust
fn model_command(
    path: &std::path::Path,
    source: crate::config_dir::ConfigPathSource,
    step: Option<String>,
) -> Result<String, Error> {
    let cfg = crate::config::read_config(path, source)?;
    let step = if let Some(name) = step {
        resolve_step(&cfg, &name)?
    } else {
        let cwd = std::env::current_dir()?;
        let artifact_dir = artifact_dir_path(&cwd, &git::current_branch()?);
        determine_step(&cfg, &artifact_dir)?
    };
    let model = resolve_model(&cfg, &step.model)?;
    Ok(model.model)
}
```

`thinking_command` is identical except the tail is `Ok(model.thinking)`.

#### 5. `prompt_command` — branch + field rename
**File**: `src/commands/mod.rs`
**Action**: modify

```rust
fn prompt_command(
    path: &std::path::Path,
    source: crate::config_dir::ConfigPathSource,
    step: Option<String>,
) -> Result<String, Error> {
    let cfg = crate::config::read_config(path, source)?;
    let name = if let Some(name) = step {
        resolve_step(&cfg, &name)?.name
    } else {
        // No explicit step: derive the current step from trigger artifacts,
        // exactly like `step`/`model`/`thinking`.
        let cwd = std::env::current_dir()?;
        let artifact_dir = artifact_dir_path(&cwd, &git::current_branch()?);
        determine_step(&cfg, &artifact_dir)?.name
    };
    resolve_prompt(&cfg, &name)
}
```

#### 6. Update/rewrite prompt unit tests
**File**: `src/commands/mod.rs` (`#[cfg(test)] mod tests`)
**Action**: modify

- `cli_try_parse_accepts_prompt` (asserts `prompt questions` parses) → flip to `cli_try_parse_rejects_prompt_positional`: `Cli::try_parse_from(["orksorksorks", "prompt", "questions"]).is_err()`.
- `cli_try_parse_prompt_reads_step_name` (positional `prompt design`) → replace with the `--step` "binds value" test already added in Stage 2; delete the positional variant.
- `cli_try_parse_prompt_with_custom_config` → change args to `["orksorksorks", "prompt", "--step", "questions", "--config", "custom.toml"]` and match `Commands::Prompt { step, config }`.
- `cli_try_parse_prompt_without_step_name_derives_automatically` → match `Commands::Prompt { step, config }` (both `None`).
- `select_command_routes_prompt` → construct `Commands::Prompt { step: None, config: Some(...) }` (drop `step_name`).

#### 7. New handler-level tests (override path, no git)
**File**: `src/commands/mod.rs` (`#[cfg(test)] mod tests`)
**Action**: modify (add; mirror for `model`/`thinking`/`prompt` where useful)

```rust
#[test]
fn step_command_flag_succeeds_in_non_git_dir() {
    // A tempdir with NO git repo: bare step would fail in git::current_branch,
    // so success proves `--step` never shells out to git or reads cwd.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("orksorksorks.toml"),
        "version = \"0.1.0\"\n[[steps]]\nname = \"one\"\ntrigger_artifact = \"first.txt\"\nmodel = \"small\"\n",
    )
    .unwrap();
    let out = step_command(
        &dir.path().join("orksorksorks.toml"),
        crate::config_dir::ConfigPathSource::ExplicitFlag,
        Some("one".to_string()),
    )
    .unwrap();
    assert_eq!(out, "one");
}

#[test]
fn step_command_flag_unknown_name_tags_step() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("orksorksorks.toml"),
        "version = \"0.1.0\"\n",
    )
    .unwrap();
    let err = step_command(
        &dir.path().join("orksorksorks.toml"),
        crate::config_dir::ConfigPathSource::ExplicitFlag,
        Some("nope".to_string()),
    )
    .unwrap_err();
    assert_eq!(err.source, "step");
    assert!(err.message.contains("no step named \"nope\""), "{}", err.message);
}
```

Add `prompt_command` variants proving the two `"step"`-error paths differ by message:
- `prompt --step nope` (step absent) → `source == "step"`, message contains `no step named`.
- `prompt --step one` where step `one` exists but has no `[[prompts]]` entry → `source == "prompt"`, message contains `no prompt named`.

#### 8. Rewrite prompt integration tests
**File**: `tests/prompt.rs`
**Action**: modify

- `prompt_with_explicit_step_name_overrides_step` → change args from `["prompt", "one", "--config", "orksorksorks.toml"]` to `["prompt", "--step", "one", "--config", "orksorksorks.toml"]`; assertions unchanged. (Rename the test to `..._step_flag_...` for clarity.)
- `prompt_unknown_step_name_fails` (the `prompt nope` test) → rewrite to assert the **new** contract:

```rust
#[test]
fn prompt_step_flag_unknown_name_fails_with_step_source() {
    let dir = tempfile::tempdir().unwrap();
    // No [[steps]]: `--step nope` fails in resolve_step, not resolve_prompt.
    std::fs::write(
        dir.path().join("orksorksorks.toml"),
        concat!(
            "version = \"0.1.0\"\n",
            "[[prompts]]\n",
            "name = \"research\"\n",
            "content = \"\"\"\n",
            "# Research — Answer the Questions\n",
            "\"\"\"\n",
        ),
    )
    .unwrap();

    let output = Command::cargo_bin("orksorksorks")
        .unwrap()
        .args(["prompt", "--step", "nope", "--config", "orksorksorks.toml"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no step named \"nope\""), "stderr: {stderr}");
}
```

### Verification

#### Automated
- [ ] `scripts/test.sh` passes (includes the rewritten `tests/prompt.rs`)
- [ ] `cargo nextest run step_command_flag` and `cargo nextest run prompt` pass

#### Manual
- [ ] `cargo run -- step --config <fixture> --step one` prints `one` from a repo with no matching artifacts

---

## Stage 4: Transport/E2E layer — integration coverage + live checks

Test-only. No production code changes.

### Changes

#### 1. `tests/step.rs` — flag success (git-free) and error routing
**File**: `tests/step.rs`
**Action**: modify (add tests; existing `write_config`/`artifact_dir` helpers reused)

```rust
#[test]
fn step_flag_works_in_non_git_dir() {
    // No `.git`: bare `step` would fail in git resolution, so success here
    // proves `--step` skips current_dir/git/artifact_dir entirely.
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path());

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd
        .args(["step", "--step", "one", "--config", "orksorksorks.toml", "-j"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["data"].as_str(), Some("one"));
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}
```

Add a `step` error-routing pair (text → stderr, JSON → stdout) for `--step nope`:

```rust
#[test]
fn step_flag_unknown_text_error_goes_to_stderr() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path());

    let output = Command::cargo_bin("orksorksorks")
        .unwrap()
        .args(["step", "--step", "nope", "--config", "orksorksorks.toml"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.is_empty(), "text error must not go to stdout: {stdout}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no step named \"nope\""), "stderr: {stderr}");
}

#[test]
fn step_flag_unknown_json_error_goes_to_stdout() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path());

    let output = Command::cargo_bin("orksorksorks")
        .unwrap()
        .args(["step", "--step", "nope", "--config", "orksorksorks.toml", "-j"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["error"]["source"].as_str(), Some("step"));
    assert!(json["error"]["message"].as_str().unwrap().contains("no step named"), "{json}");
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}
```

#### 2. `tests/model.rs` — flag returns model / thinking strings
**File**: `tests/model.rs`
**Action**: modify (add tests; reuse `write_config` which has `small`/`high` models)

- `model_flag_returns_model_string`: `["model", "--step", "one", "--config", "orksorksorks.toml", "-j"]` → `v["data"] == Some("openrouter/deepseek/flash")`; keep the `\x07`/`\x1b` guards (bell strip via `.trim_end_matches('\x07').trim_end()` for the text variant, or assert JSON directly as above).
- `thinking_flag_returns_thinking_budget`: `["thinking", "--step", "one", "--config", "orksorksorks.toml", "-j"]` → `v["data"] == Some("high")`.

(The shared `resolve_step` unknown-name error path is already covered by the `step` tests; no duplicate unknown test needed here.)

#### 3. `tests/prompt.rs` — flag returns content + positional rejected
**File**: `tests/prompt.rs`
**Action**: modify (add; two directional tests plus the Stage-3 rewrite)

- `prompt_flag_returns_content`: `["prompt", "--step", "one", "--config", "orksorksorks.toml", "-j"]` → `v["data"]` contains `"# Prompt for step one"`.
- `prompt_rejects_positional_step_name`: the unmerged positional must now be a parse error:

```rust
#[test]
fn prompt_rejects_positional_step_name() {
    let dir = init_git_repo();
    write_config(dir.path());

    Command::cargo_bin("orksorksorks")
        .unwrap()
        .args(["prompt", "one", "--config", "orksorksorks.toml"])
        .current_dir(dir.path())
        .assert()
        .code(2); // clap usage-error exit code
}
```

- Regression guard: `prompt_without_step_name_infers_current_step` and `prompt_without_step_name_no_artifacts_fails` already exercise no-`--step` derivation and stay untouched (proving byte-for-byte unchanged behavior without the flag).

#### 4. `tests/json_output.rs`
**File**: `tests/json_output.rs`
**Action**: no change — the `{data}/{error}` envelope is already exercised via the new `-j` tests above and the existing suite; nothing struct-specific was added.

### Verification

#### Automated
- [ ] `scripts/test.sh` passes (full gate: fmt, check, clippy `-D warnings`, nextest)
- [ ] `cargo nextest run -- --test step --test model --test prompt` passes

#### Manual
- [ ] Build a fixture `orksorksorks.toml` with `one`/`two` steps, `[[models]]`, and a multiline `[[prompts]]`:

```toml
version = "0.1.0"

[[steps]]
name = "one"
trigger_artifact = "first.txt"
model = "small"

[[steps]]
name = "two"
trigger_artifact = "second.txt"
model = "high"

[[models]]
name = "small"
model = "openrouter/deepseek/flash"
thinking = "high"

[[prompts]]
name = "one"
content = """
# Prompt for step one
"""
```

- [ ] From a repo with no matching trigger artifacts, run each and check the output:
  - [ ] `cargo run -- step --config ./orksorksorks.toml --step one` → `one`
  - [ ] `cargo run -- model --config ./orksorksorks.toml --step one` → `openrouter/deepseek/flash`
  - [ ] `cargo run -- thinking --config ./orksorksorks.toml --step one` → `high`
  - [ ] `cargo run -- prompt --config ./orksorksorks.toml --step one` → `# Prompt for step one`
- [ ] `cargo run -- step --config ./orksorksorks.toml --step nope` exits non-zero with `no step named "nope"` on stderr
- [ ] From a directory with no `.git` (e.g. `cd /tmp`), `cargo run --manifest-path <repo>/Cargo.toml -- step --config <abs-path>/orksorksorks.toml --step one` succeeds — the decisive git-skip proof
- [ ] `cargo run -- prompt one` fails with a clap usage error (positional removed)

---

## Sequencing rules

- Each stage's `scripts/test.sh` gate must be green before starting the next; a red gate blocks all higher stages.
- Stage 2's clap-field + dispatch `match` are one compile unit: adding `step` to a variant forces its dispatch arm and every `Commands::*` test destructure/construction to be updated in the same edit, or the crate stops compiling.
- `resolve_step` (`"step"` tag) collides with `determine_step`'s no-match `"step"` tag; every test asserting `source == "step"` must also assert the message (`no step named` vs `no trigger artifact matched`).
- No schema migration, no codegen — not applicable.

## Deviation from `structure.md`

1. **`prompt` positional removal moved Stage 2 → Stage 3.** `structure.md` Stage 2 says "`Prompt` loses its `step_name` positional" and verifies "existing `tests/{step,model,prompt}.rs` all pass unchanged", but removing the positional before the override is wired breaks `tests/prompt.rs` (`prompt one` in `prompt_with_explicit_step_name_overrides_step`, `prompt nope` in `prompt_unknown_step_name_fails`) and can't be made green while the flag is still ignored. The plan therefore keeps the positional through Stage 2 (truly behavior-neutral) and removes it + rewrites those two tests in Stage 3, where the override is wired. Everything else — stage order, file set, final behavior — is unchanged from the outline.