# Implementation Plan

## Overview

Replace process-env mutation in unit tests with explicit `ConfigEnv` injection, so every `unsafe { set_var/remove_var }` disappears from the suite, `APPDATA` gets coverage, and the unchanged gate (`./scripts/test.sh`) stays green with the same tests (rewritten, not removed).

---

## Phase 1: `ConfigEnv` + pure resolution core

### Changes

#### 1. `src/config_dir.rs` — imports
**File**: `src/config_dir.rs`
**Action**: modify

Add `OsString` to the std imports (needed by the new struct's field types):

```rust
use std::ffi::OsString;
use std::path::{Path, PathBuf};
```

#### 2. `src/config_dir.rs` — `ConfigEnv` type + `from_env`
**File**: `src/config_dir.rs`
**Action**: create (insert after `const FILE_NAME`, before the `config_file_path` doc comment)

```rust
/// Environment variables consulted by config-directory resolution.
///
/// All fields are `None` by default so tests get a clean slate. Production
/// code populates this via [`ConfigEnv::from_env`]; tests construct it
/// directly with the values they want to inject.
pub(crate) struct ConfigEnv {
    pub xdg_config_home: Option<OsString>,
    pub home: Option<OsString>,
    pub appdata: Option<OsString>,
}

impl Default for ConfigEnv {
    fn default() -> Self {
        Self {
            xdg_config_home: None,
            home: None,
            appdata: None,
        }
    }
}

impl ConfigEnv {
    /// Read the config-directory env vars from the process environment,
    /// populating only the fields the current platform consults.
    pub(crate) fn from_env() -> Self {
        Self {
            xdg_config_home: std::env::var_os("XDG_CONFIG_HOME"),
            home: if cfg!(windows) {
                None
            } else {
                std::env::var_os("HOME")
            },
            appdata: if cfg!(windows) {
                std::env::var_os("APPDATA")
            } else {
                None
            },
        }
    }
}
```

> **Platform note**: the `cfg!(windows)` switch lives in `from_env`, not the
> resolution core. On Unix, `appdata` is always `None` in production; on
> Windows, `home` is always `None`. This keeps production behavior identical
> while making the core (below) testable on any host.

#### 3. `src/config_dir.rs` — `resolve_config_dir_with_env` (pure core)
**File**: `src/config_dir.rs`
**Action**: create (insert directly before `resolve_config_dir`)

Move the body of today's `resolve_config_dir()` here, converting each
`std::env::var_os(...)` read into a field read on `env`. Same `"config-dir"`
error tag and message:

```rust
/// Pure resolution core — every env read is a field access on `env`.
///
/// Platform-agnostic: `ConfigEnv::from_env` populates only the fields the
/// current platform consults, so production behavior is unchanged while the
/// logic is testable on any host.
pub(crate) fn resolve_config_dir_with_env(env: &ConfigEnv) -> Result<PathBuf, Error> {
    // XDG wins on every platform, but only when set to an absolute path.
    // Unset, empty, and relative values all mean "fall back" (XDG spec).
    if let Some(xdg) = &env.xdg_config_home {
        let dir = PathBuf::from(xdg);
        if dir.is_absolute() {
            return Ok(dir);
        }
    }

    if let Some(appdata) = &env.appdata {
        return Ok(PathBuf::from(appdata));
    }

    if let Some(home) = &env.home {
        return Ok(PathBuf::from(home).join(".config"));
    }

    Err(Error::new(
        "config-dir",
        "could not determine a config directory: set XDG_CONFIG_HOME or HOME",
    ))
}
```

#### 4. `src/config_dir.rs` — `resolve_config_dir` becomes a thin wrapper
**File**: `src/config_dir.rs`
**Action**: modify (replace the old `resolve_config_dir` body)

The old env-reading body (lines 24-46) is replaced by the wrapper:

```rust
/// Read ambient env and resolve the config directory (thin wrapper).
fn resolve_config_dir() -> Result<PathBuf, Error> {
    resolve_config_dir_with_env(&ConfigEnv::from_env())
}
```

`config_file_path` is **unchanged** in this phase — it still calls
`resolve_config_dir()` for its `None` arm, so the wrapper is not dead code yet.

#### 5. `src/config_dir.rs` — rewrite 5 tests + add 1 (`appdata_used_on_windows`)
**File**: `src/config_dir.rs` (`#[cfg(test)] mod tests`, lines 48-114)
**Action**: modify

Replace the five env-mutating tests with `resolve_config_dir_with_env(&ConfigEnv { ... })`
calls. Assertions now check the resolved **directory** (no `FILE_NAME` join,
since these tests target the resolution core directly). `explicit_path_passthrough`
is unchanged.

```rust
#[test]
fn absolute_xdg_is_used() {
    let path = resolve_config_dir_with_env(&ConfigEnv {
        xdg_config_home: Some("/tmp/ork-cfg".into()),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(path, std::path::PathBuf::from("/tmp/ork-cfg"));
}

#[test]
fn unset_xdg_falls_back_to_home_dot_config() {
    let path = resolve_config_dir_with_env(&ConfigEnv {
        home: Some("/home/ork".into()),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(path, std::path::PathBuf::from("/home/ork/.config"));
}

#[test]
fn empty_xdg_falls_back_to_home() {
    let path = resolve_config_dir_with_env(&ConfigEnv {
        xdg_config_home: Some("".into()),
        home: Some("/home/ork".into()),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(path, std::path::PathBuf::from("/home/ork/.config"));
}

#[test]
fn relative_xdg_falls_back_to_home() {
    let path = resolve_config_dir_with_env(&ConfigEnv {
        xdg_config_home: Some("relative/dir".into()),
        home: Some("/home/ork".into()),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(path, std::path::PathBuf::from("/home/ork/.config"));
}

#[test]
fn explicit_path_passthrough() {
    let p = std::path::Path::new("/custom/override.toml");
    assert_eq!(config_file_path(Some(p)).unwrap(), p.to_path_buf());
}

#[test]
fn appdata_used_on_windows() {
    let path = resolve_config_dir_with_env(&ConfigEnv {
        appdata: Some("C:\\Users\\ork\\AppData\\Roaming".into()),
        ..Default::default()
    })
    .unwrap();
    // APPDATA returns raw: no absolute check, no .join (current behavior).
    assert_eq!(
        path,
        std::path::PathBuf::from("C:\\Users\\ork\\AppData\\Roaming")
    );
}

#[test]
fn unresolved_home_yields_config_dir_error() {
    let err = resolve_config_dir_with_env(&ConfigEnv::default()).unwrap_err();
    assert_eq!(err.source, "config-dir");
}
```

### Verification
#### Automated
- [x] `cargo nextest run --no-tests pass config_dir` — 7 green
- [x] `rg -n 'set_var|remove_var' src/config_dir.rs` — no matches

#### Manual
- [ ] `cargo run -- init` with `XDG_CONFIG_HOME` set to an absolute dir writes `$XDG_CONFIG_HOME/orksorksorks.toml` (unchanged behavior)

---

## Phase 2: `config_file_path_with_env` — filename composition

### Changes

#### 1. `src/config_dir.rs` — add `config_file_path_with_env`
**File**: `src/config_dir.rs`
**Action**: create (insert directly after `resolve_config_dir_with_env`)

```rust
/// Explicit-env variant of [`config_file_path`].
///
/// `Some(path)` passes through unchanged; `None` resolves the config dir from
/// the injected `env` and joins [`FILE_NAME`].
pub(crate) fn config_file_path_with_env(
    explicit: Option<&Path>,
    env: &ConfigEnv,
) -> Result<PathBuf, Error> {
    match explicit {
        Some(path) => Ok(path.to_path_buf()),
        None => resolve_config_dir_with_env(env).map(|dir| dir.join(FILE_NAME)),
    }
}
```

#### 2. `src/config_dir.rs` — `config_file_path` becomes a thin wrapper
**File**: `src/config_dir.rs`
**Action**: modify (replace the existing `config_file_path` body)

Public signature unchanged:

```rust
pub fn config_file_path(explicit: Option<&Path>) -> Result<PathBuf, Error> {
    config_file_path_with_env(explicit, &ConfigEnv::from_env())
}
```

#### 3. `src/config_dir.rs` — delete `resolve_config_dir`
**File**: `src/config_dir.rs`
**Action**: delete (the wrapper added in Phase 1, step 4)

`resolve_config_dir()` is now unreferenced — `config_file_path` routes through
`config_file_path_with_env`, which calls `resolve_config_dir_with_env` directly.
Leaving it would trip `dead_code` under `cargo clippy -- -D warnings`.

```rust
fn resolve_config_dir() -> Result<PathBuf, Error> {
    resolve_config_dir_with_env(&ConfigEnv::from_env())
}
```
↑ remove this function entirely.

### Verification
#### Automated
- [x] `cargo nextest run --no-tests pass config_dir` — 7 green
- [x] `cargo clippy --tests -- -D warnings` — clean (proves no dead `resolve_config_dir`)

#### Manual
- [ ] `cargo run -- init` with no `XDG_CONFIG_HOME` falls back to `$HOME/.config/orksorksorks.toml` (unchanged)

---

## Phase 3: thread env through `select_command`

### Changes

#### 1. `src/commands/mod.rs` — add `select_command_with_env`
**File**: `src/commands/mod.rs`
**Action**: create (insert directly before the existing `select_command`)

Same match arms as today; only the `Init` and `Step` arms change to use
`config_file_path_with_env`. `Branch` / `ArtifactDirectory` arms unchanged:

```rust
/// Route a parsed CLI to its handler, injecting `env` for config-dir resolution.
fn select_command_with_env(
    cli: &Cli,
    env: &crate::config_dir::ConfigEnv,
) -> Result<String, Error> {
    match &cli.command {
        Commands::Init { config } => {
            let path = crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            init_command(&path)
        }
        Commands::Branch => branch_command(),
        Commands::ArtifactDirectory => artifact_directory_command(),
        Commands::Step { config } => {
            let path = crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            step_command(&path)
        }
    }
}
```

#### 2. `src/commands/mod.rs` — `select_command` becomes a thin wrapper
**File**: `src/commands/mod.rs`
**Action**: modify (replace the existing `select_command` body)

Public signature unchanged; `src/main.rs` needs no change:

```rust
pub fn select_command(cli: &Cli) -> Result<String, Error> {
    select_command_with_env(cli, &crate::config_dir::ConfigEnv::from_env())
}
```

#### 3. `src/commands/mod.rs` — rewrite `select_command_routes_init` (lines 177-190)
**File**: `src/commands/mod.rs` (`#[cfg(test)] mod tests`)
**Action**: modify

Delete the `unsafe { std::env::set_var("XDG_CONFIG_HOME", temp.path()) }` block
and inject the tempdir via `ConfigEnv` instead. Assertions and tempdir lifetime
unchanged (tempdir still auto-cleans on drop):

```rust
#[test]
fn select_command_routes_init() {
    let temp = tempfile::tempdir().unwrap();
    let env = crate::config_dir::ConfigEnv {
        xdg_config_home: Some(temp.path().into()),
        ..Default::default()
    };
    let cli = Cli {
        json: false,
        command: Commands::Init { config: None },
    };
    let result = select_command_with_env(&cli, &env).unwrap();
    let expected_path = temp.path().join("orksorksorks.toml");
    assert!(expected_path.exists(), "config file not created");
    // Under cfg!(test) color is stripped
    assert_eq!(result, format!("✓ Created {}", expected_path.display()));
}
```

All other ~29 tests in `src/commands/mod.rs` are unchanged — they pass explicit
`--config` paths (bypassing resolution) or exercise arms that ignore env.

### Verification
#### Automated
- [x] `cargo nextest run --no-tests pass commands` — all command tests green
- [x] `rg -n 'set_var|remove_var' src/` — no matches anywhere in `src/`

#### Manual
- [ ] `cargo run -- init` (no `--config`) still writes to the config dir and prints `✓ Created …` (unchanged)

---

## Phase 4: full gate

### Changes
None (verification only).

### Verification
#### Automated
- [x] `./scripts/test.sh` passes (fmt → check → clippy `-D warnings` → nextest → forbidden-strings grep)
- [x] `cargo clippy --all-targets --all-features --locked -- -D warnings` passes (CI mirror)

#### Manual
- [ ] `cargo run -- init` and `cargo run -- step` behave identically to before on this host (spot-check with and without `XDG_CONFIG_HOME`/`HOME` set)

---

## Testing Checkpoints

| After Stage | Must be green before advancing |
|---|---|
| 1 | `cargo nextest run --no-tests pass config_dir` — 7 tests; no `set_var`/`remove_var` in `config_dir.rs` |
| 2 | `cargo nextest run --no-tests pass config_dir` — 7 tests; `cargo clippy --tests -- -D warnings` clean |
| 3 | `cargo nextest run --no-tests pass commands` — all command tests; no `set_var`/`remove_var` in `src/` |
| 4 | `./scripts/test.sh` — full local gate |

Resume rule: if context resets mid-implementation, re-run the checkpoint for the last completed stage and only continue once it is green.

---

## Deviations from `structure.md` (for the implementer)

1. **`resolve_config_dir_with_env` is platform-agnostic, not "body moved verbatim".**
   The structure says "body moved verbatim", which would keep `cfg!(windows)` inside
   the core and make the new `appdata_used_on_windows` test *fail on non-Windows
   hosts* (the APPDATA branch is compiled out). Instead, the `cfg!(windows)` switch
   moves into `ConfigEnv::from_env` (populating only the platform-consulted fields),
   per `design.md` decision 3 ("the `cfg!(windows)` branch … is a compile-time switch
   in the wrapper"). Production behavior is byte-for-byte identical.

2. **`resolve_config_dir()` is deleted in Phase 2, not kept module-private.**
   `design.md` decision 6 says the private wrapper "stays module-private", but after
   `config_file_path` becomes a thin wrapper over `config_file_path_with_env`
   (structure Stage 2), `resolve_config_dir()` has no callers and would trip
   `dead_code` under `cargo clippy -- -D warnings`. It is removed in Phase 2.
