# Implementation Plan

## Overview

`init` writes a hand-authored commented template (`templates/default.toml`, embedded via `include_str!`) instead of serializer output, refuses to overwrite an existing file (new `config-exists` tag), and the user's live dotfiles config receives the same in-place descriptive comments as a manual one-off edit.

---

## Phase (Stage) 1: Template asset — single source of truth (data layer)

Delivers the commented template file and a unit test proving it is valid TOML that still equals `Config::default()`.

### Changes

#### 1. Create the commented template
**File**: `templates/default.toml` (new directory `templates/`)
**Action**: create

Full content (ends with a single trailing newline):

```toml
# Default configuration for orksorksorks.
#
# The two lines below are the only live settings; everything else documents
# the available sections with commented examples. The authoritative schema
# lives in src/config.rs — see that file for field-level details and
# validation rules.
#
# Sections are documented as they appear in a typical config:
# steps → models → scripts → prompts.

version = "0.1.0"
show_frontmatter = true

# ------------------------------------------------------------
# Steps
#
# Checked in order: the current step is the last whose trigger_artifact
# file exists in the artifact directory. `name` must match a [[prompts]]
# entry and `model` must match a [[models]] entry; `script` is optional.
# ------------------------------------------------------------
# [[steps]]
# name = "example"
# trigger_artifact = "example.md"
# model = "small"
# script = "my_script"

# ------------------------------------------------------------
# Models
#
# Named model definitions referenced by a step's `model` key.
# ------------------------------------------------------------
# [[models]]
# name = "small"
# model = "openrouter/example/model"
# thinking = "high"

# ------------------------------------------------------------
# Scripts
#
# Named scripts referenced by a step's `script` key and returned by the
# `script` subcommand.
# ------------------------------------------------------------
# [[scripts]]
# name = "my_script"
# content = """
# echo "hello"
# """

# ------------------------------------------------------------
# Prompts
#
# Named prompt text keyed by step name and returned by the `prompt`
# subcommand.
# ------------------------------------------------------------
# [[prompts]]
# name = "example"
# content = """
# Your prompt body goes here.
# """
```

Rules to hold:
- Only `version` and `show_frontmatter` are live keys; every example block is fully `#`-commented so the file deserializes to `Config::default()`.
- Full-line `#` comments only — no `key = value # note` trailing comments.
- Section order is steps → models → scripts → prompts (matches the live file), not serializer field order.
- Blank-line separation between blocks; `#`-dash banners per section.

#### 2. Add the guard unit test
**File**: `src/config.rs`
**Action**: modify — add one test inside the existing `#[cfg(test)] mod tests` (which already has `use super::*;` and `use pretty_assertions::assert_eq;`). Place it near `default_config_serializes_to_expected_toml` (~line 262).

```rust
#[test]
fn template_deserializes_to_default() {
    let template = include_str!("../templates/default.toml");
    let parsed: Config = toml::from_str(template).unwrap();
    assert_eq!(parsed, Config::default());
}
```

`Config` already derives `PartialEq` + `Debug` (config.rs:58). The `include_str!` path resolves relative to `src/config.rs`, i.e. `templates/default.toml`.

### Verification

#### Automated
- [x] `cargo nextest run config` passes (runs `config::tests::template_deserializes_to_default` plus existing config tests)
- [x] `./scripts/test.sh` passes (fmt / check / clippy `-D warnings` / nextest / forbidden-strings — the template is `.toml`, so the `rg -g '*.rs'` gate is untouched)

#### Manual
- [ ] Visually confirm `templates/default.toml` renders a header, two live keys, and four commented example sections with dash banners in steps → models → scripts → prompts order

---

## Phase (Stage) 2: Write-safety guard — `init` becomes non-clobbering

Makes `init` refuse to truncate an existing file, surfacing the `config-exists` tag with exit 1.

### Changes

#### 1. Register the new tag
**File**: `src/errors.rs`
**Action**: modify — add `"config-exists"` to the tag-registry doc comment (the paragraph at errors.rs:5-16 listing per-command lookups).

Change:
```rust
/// (`"config-dir"`, `"step"`, `"model"`, `"prompt"`, `"git"`).
```
to:
```rust
/// (`"config-dir"`, `"config-exists"`, `"step"`, `"model"`, `"prompt"`, `"git"`).
```

No new type — reuse `Error::new(source, message)` (errors.rs:19).

#### 2. Replace `File::create` with non-clobbering `OpenOptions`
**File**: `src/commands/mod.rs` (`init_command`, ~line 161)
**Action**: modify — replace the `File::create(path)` line with an `OpenOptions` write that maps `AlreadyExists` to `Error::new("config-exists", …)` and everything else through the existing `From<io::Error>`.

New `init_command` body for this stage (content still the serialized default; the template swap happens in Stage 3):

```rust
fn init_command(path: &std::path::Path) -> Result<String, Error> {
    use std::io::Write;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let config = crate::config::Config::default();
    let toml_str = toml::to_string(&config)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                Error::new(
                    "config-exists",
                    &format!("{} already exists; not overwriting", path.display()),
                )
            } else {
                Error::from(e)
            }
        })?;
    file.write_all(toml_str.as_bytes())?;
    file.flush()?;
    file.sync_all()?;
    Ok(crate::format::green_string(&format!(
        "✓ Created {}",
        path.display()
    )))
}
```

`create_new(true)` is atomic (no check-then-create race). Non-`AlreadyExists` errors (`NotFound`, `PermissionDenied`, …) fall through `Error::from(e)` → `"io"`, preserving the existing readonly-dir behavior.

#### 3. Add the clobber integration test
**File**: `tests/init_creates_file.rs`
**Action**: modify — add one test after `init_with_config_flag_writes_to_given_path`.

```rust
#[test]
fn init_refuses_to_overwrite_existing_file() {
    let temp = tempfile::tempdir().unwrap();
    let config_dir = temp.path().join("config");
    fs::create_dir(&config_dir).unwrap();
    let config_path = config_dir.join("orksorksorks.toml");
    let existing = "keep these bytes\n";
    fs::write(&config_path, existing).unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.env("XDG_CONFIG_HOME", &config_dir);

    let output = cmd.args(["init", "-j"]).output().unwrap();
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(r#""source":"config-exists""#),
        "stdout: {stdout}"
    );

    // The pre-existing content is byte-unchanged.
    assert_eq!(fs::read_to_string(&config_path).unwrap(), existing);

    // Symlink note: `create_new` follows the link, so `init` against
    // ~/.config/orksorksorks.toml (→ dotfiles) also refuses — desired.
}
```

### Verification

#### Automated
- [x] `cargo nextest run init_creates_file` passes (new clobber test exits 1 with `config-exists`; all 6 existing tests still green — they all target fresh destinations)
- [x] `./scripts/test.sh` passes

#### Manual
- [ ] `cargo run -- init` against a path that already exists prints `Error from config-exists:` with an "already exists; not overwriting" message and exits nonzero; the file is untouched

---

## Phase (Stage) 3: Template emission — `init` writes the commented template

Switches the emitted bytes from the serializer to the embedded template, on top of the now-safe write path.

### Changes

#### 1. Emit the template bytes
**File**: `src/commands/mod.rs` (`init_command`)
**Action**: modify — drop the two serializer lines and write the template literal.

Replace in `init_command`:
```rust
    let config = crate::config::Config::default();
    let toml_str = toml::to_string(&config)?;
```
with nothing, and change:
```rust
    file.write_all(toml_str.as_bytes())?;
```
to:
```rust
    file.write_all(include_str!("../../templates/default.toml").as_bytes())?;
```

Final `init_command`:

```rust
fn init_command(path: &std::path::Path) -> Result<String, Error> {
    use std::io::Write;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                Error::new(
                    "config-exists",
                    &format!("{} already exists; not overwriting", path.display()),
                )
            } else {
                Error::from(e)
            }
        })?;
    file.write_all(include_str!("../../templates/default.toml").as_bytes())?;
    file.flush()?;
    file.sync_all()?;
    Ok(crate::format::green_string(&format!(
        "✓ Created {}",
        path.display()
    )))
}
```

The `include_str!` path is relative to `src/commands/mod.rs`, so the correct depth is `../../templates/default.toml` (not `../templates/`).

#### 2. Point both byte-exact assertions at the template
**File**: `tests/init_creates_file.rs`
**Action**: modify — replace the two hardcoded 42-byte literals with `include_str!`.

In `init_creates_orksorksorks_toml_with_default_content` (lines ~22-27), replace:
```rust
    // The crate is binary-only (no lib target), so integration tests can't
    // import `Config`; assert the exact serialization of the default value.
    let expected = "version = \"0.1.0\"\nshow_frontmatter = true\n";
    assert_eq!(content, expected);
```
with:
```rust
    // The crate is binary-only (no lib target), so integration tests can't
    // import `Config`; assert the exact bytes of the shared template.
    assert_eq!(content, include_str!("../templates/default.toml"));
```

In `init_with_config_flag_writes_to_given_path` (line ~104), replace:
```rust
    let content = fs::read_to_string(&target).unwrap();
    assert_eq!(content, "version = \"0.1.0\"\nshow_frontmatter = true\n");
```
with:
```rust
    let content = fs::read_to_string(&target).unwrap();
    assert_eq!(content, include_str!("../templates/default.toml"));
```

The test path is `tests/init_creates_file.rs`, so `../templates/default.toml` resolves to the same file — source/test drift becomes impossible.

### Verification

#### Automated
- [ ] `cargo nextest run init_creates_file` passes (both byte assertions now compare against `include_str!`)
- [ ] `./scripts/test.sh` passes

#### Manual
- [ ] `cargo run -- init` with a fresh `XDG_CONFIG_HOME`, then `cat` the emitted file: it is the full commented template, not the 42-byte default

---

## Phase (Stage) 4: Live config — one-off in-place comment edit (manual, out-of-repo)

Applies the same full-line `#` comments to the user's dotfiles file, preserving all 1269 lines of custom content.

**File**: `/Users/vardy/dev/dotfiles/orksorksorks/orksorksorks.toml` (git-managed in the dotfiles repo — **not** this checkout)
**Action**: manual edit (user-driven; no repo code or tests involved)

### Changes

1. Insert a header comment block at the top (above `version = "0.1.0"`, line 1) describing the file and pointing at the schema, matching Stage 1's header.
2. Insert per-section `#`-dash banners before each section in order steps → models → scripts → prompts:
   - before the first `[[steps]]` (line 4)
   - before the first `[[models]]` (line 53)
   - before the first `[[scripts]]` (line 65)
   - before the first `[[prompts]]` (line 99)
3. Normalize the stray leading space on the two existing dividers: ` # ---` → `# ---` at lines 63 and 97.
4. Hard constraints: no line removed, no trailing `key = value # note` comments, no existing step/model/script/prompt content modified. (The single-space-only blank lines at 64/89 are left as-is — they are harmless blanks.)

### Verification

#### Automated
- [ ] `python3 -c "import tomllib,sys; tomllib.load(open(sys.argv[1],'rb'))" /Users/vardy/dev/dotfiles/orksorksorks/orksorksorks.toml` parses without error

#### Manual
- [ ] `git diff` in the dotfiles repo (`/Users/vardy/dev/dotfiles`) shows only added comment lines plus the two normalized dividers; all existing steps/models/scripts/prompts byte-identical except the divider whitespace
- [ ] Commit is made in the dotfiles repo (user drives this — separate from this checkout's PR)

---

## Out of scope (explicitly NOT done)

- No change to default data (`version` + `show_frontmatter` remain the only live keys).
- No new dependency (`toml_edit` / comment crate).
- No `--force` flag or idempotent `init`; `init` errors on an existing file.
- No migration/rewrite code to annotate existing files programmatically.
- `toml::to_string` left untouched at config.rs:262,271,299 (value-equality round-trips remain correct).