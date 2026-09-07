# Implementation Plan

## Overview

Add two subcommands — `branch` (prints the current git branch) and
`artifact_directory` (prints `<cwd>/.pi/orksorksorks/<branch>/`) — each a
zero-arg `fn -> Result<String, Error>` handler following the `init` contract,
wired through clap and covered by unit + spawn-the-binary integration tests.

All stages touch `src/commands/mod.rs` except Stage 5 (two new `tests/` files).
No dependencies are added; no schema migration; no codegen. `git` is reached via
`std::process::Command` and errors flow through the existing `Error` type.

---

## Phase 1: Pure path composition

### Changes

#### 1. Add `artifact_dir_path` helper
**File**: `src/commands/mod.rs`
**Action**: modify

Place just below the `init_command` fn (after `init_command`'s closing brace,
before `#[cfg(test)] mod tests`).

```rust
/// Compose the artifact-directory path from a cwd and a branch name.
///
/// Pure — no git, no I/O, no error path. The trailing slash is part of the
/// contract (see `task.md`).
fn artifact_dir_path(cwd: &std::path::Path, branch: &str) -> String {
    format!("{}/.pi/orksorksorks/{}/", cwd.display(), branch)
}
```

#### 2. Add unit tests
**File**: `src/commands/mod.rs` (inside existing `#[cfg(test)] mod tests`)
**Action**: modify

```rust
    #[test]
    fn artifact_dir_path_appends_trailing_slash() {
        use pretty_assertions::assert_eq;
        let path = artifact_dir_path(std::path::Path::new("/repo"), "main");
        assert_eq!(path, "/repo/.pi/orksorksorks/main/");
    }

    #[test]
    fn artifact_dir_path_is_plain() {
        let path = artifact_dir_path(std::path::Path::new("/repo"), "main");
        assert!(!path.contains('\x1b'), "{path}");
    }

    #[test]
    fn artifact_dir_path_handles_branch_with_slashes() {
        use pretty_assertions::assert_eq;
        let path = artifact_dir_path(std::path::Path::new("/repo"), "feature/x");
        assert_eq!(path, "/repo/.pi/orksorksorks/feature/x/");
    }
```

### Verification

#### Automated
- [x] `./scripts/test.sh`
- [x] fast loop: `cargo nextest run --bin orksorksorks` (3 new `artifact_dir_path` tests pass)

#### Manual
- [ ] n/a — pure function, no CLI surface yet.

---

## Phase 2: Git branch resolution

### Changes

#### 1. Add `current_branch` and the `parse_branch_output` seam
**File**: `src/commands/mod.rs`
**Action**: modify

Place after `artifact_dir_path` (from Phase 1). `current_branch` owns the
subprocess; `parse_branch_output` is the pure decision seam so detached-HEAD
and non-zero-exit paths are unit-testable without detaching the repo.

```rust
/// Resolve the current git branch by invoking `git branch --show-current`.
///
/// A failed spawn (git not on PATH) rides the existing `From<std::io::Error>`
/// → `"io"` tag; git-level failures use `Error::new("git", ...)`.
fn current_branch() -> Result<String, Error> {
    let output = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    parse_branch_output(output.status.success(), &stdout, &stderr)
}

/// Pure decision: map `git branch --show-current` output to a branch or error.
fn parse_branch_output(exit_ok: bool, stdout: &str, stderr: &str) -> Result<String, Error> {
    if !exit_ok {
        return Err(Error::new("git", stderr.trim()));
    }
    let stdout = stdout.trim();
    if stdout.is_empty() {
        return Err(Error::new("git", "not on a branch (detached HEAD)"));
    }
    Ok(stdout.to_string())
}
```

#### 2. Add unit tests
**File**: `src/commands/mod.rs` (inside `#[cfg(test)] mod tests`)
**Action**: modify

```rust
    #[test]
    fn current_branch_returns_actual_branch() {
        // Unit tests run with cwd = crate root (a git worktree), so git resolves.
        let branch = current_branch().unwrap();
        assert!(!branch.is_empty());
    }

    #[test]
    fn parse_branch_output_trims_trailing_newline() {
        use pretty_assertions::assert_eq;
        let branch = parse_branch_output(true, "main\n", "").unwrap();
        assert_eq!(branch, "main");
    }

    #[test]
    fn parse_branch_output_detached_head_errors() {
        let err = parse_branch_output(true, "", "").unwrap_err();
        assert_eq!(err.source, "git");
        assert!(err.message.contains("detached HEAD"), "{}", err.message);
    }

    #[test]
    fn parse_branch_output_nonzero_exit_uses_stderr() {
        use pretty_assertions::assert_eq;
        let err = parse_branch_output(false, "", "fatal: not a git repository\n").unwrap_err();
        assert_eq!(err.source, "git");
        assert_eq!(err.message, "fatal: not a git repository");
    }
```

Note: spawn-failure → `"io"` tag needs no new test — it is covered by the
existing `from_io_error_tags_io` in `src/errors.rs`, and `current_branch`'s
`.output()?` uses that same `From<std::io::Error>` impl.

### Verification

#### Automated
- [ ] `./scripts/test.sh`
- [ ] fast loop: `cargo nextest run --bin orksorksorks` (Phase 1 + Phase 2 tests pass)

#### Manual
- [ ] `git branch --show-current` returns the current branch in this worktree (sanity: matches `git branch --show-current` output shape, non-empty).
- [ ] Confirm the error string for the detached-HEAD case reads `not on a branch (detached HEAD)` (wording is final here; cheap to change later if desired).

---

## Phase 3: Command handlers

### Changes

#### 1. Add `branch_command` and `artifact_directory_command`
**File**: `src/commands/mod.rs`
**Action**: modify

Place after `parse_branch_output` (from Phase 2). Both return **plain**
strings — no `green_string`/`format.rs` import — because their value is the
machine-readable JSON `data` payload.

```rust
/// Handle the `branch` subcommand: return the current git branch, plain.
fn branch_command() -> Result<String, Error> {
    current_branch()
}

/// Handle the `artifact_directory` subcommand: return
/// `$PWD/.pi/orksorksorks/<branch>/`, plain (no directory creation).
fn artifact_directory_command() -> Result<String, Error> {
    let cwd = std::env::current_dir()?;
    Ok(artifact_dir_path(&cwd, &current_branch()?))
}
```

#### 2. Add unit tests
**File**: `src/commands/mod.rs` (inside `#[cfg(test)] mod tests`)
**Action**: modify

```rust
    #[test]
    fn branch_command_returns_plain_non_empty() {
        let branch = branch_command().unwrap();
        assert!(!branch.is_empty());
        assert!(!branch.contains('\x1b'), "{branch}");
    }

    #[test]
    fn artifact_directory_command_composes_cwd_and_branch() {
        use pretty_assertions::assert_eq;
        let result = artifact_directory_command().unwrap();
        let cwd = std::env::current_dir().unwrap();
        let branch = current_branch().unwrap();
        let expected = artifact_dir_path(&cwd, &branch);
        assert_eq!(result, expected);
        assert!(!result.is_empty());
        assert!(!result.contains('\x1b'), "{result}");
    }
```

### Verification

#### Automated
- [ ] `./scripts/test.sh`
- [ ] fast loop: `cargo nextest run --bin orksorksorks` (Phases 1–3 tests pass)

#### Manual
- [ ] n/a — handlers are exercised through `select_command`, live in Phase 5.

---

## Phase 4: Command surface / dispatch

### Changes

#### 1. Extend the `Commands` enum
**File**: `src/commands/mod.rs`
**Action**: modify

```rust
/// Top-level subcommands.
#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// (i) Create a new orksorksorks.toml file
    Init,

    /// Print the current git branch
    Branch,

    /// Print the artifact directory path (cwd/.pi/orksorksorks/<branch>/)
    #[command(name = "artifact_directory")]
    ArtifactDirectory,
}
```

(`Branch` needs no rename — the lowercased variant already yields `branch`.
`ArtifactDirectory` is pinned to the literal snake_case token so clap does not
default to `artifact-directory`.)

#### 2. Extend `select_command`
**File**: `src/commands/mod.rs`
**Action**: modify

```rust
pub fn select_command(cli: &Cli) -> Result<String, Error> {
    match &cli.command {
        Commands::Init => init_command(),
        Commands::Branch => branch_command(),
        Commands::ArtifactDirectory => artifact_directory_command(),
    }
}
```

#### 3. Add/extend unit tests
**File**: `src/commands/mod.rs` (inside `#[cfg(test)] mod tests`)
**Action**: modify

```rust
    #[test]
    fn select_command_routes_branch() {
        let cli = Cli {
            json: false,
            command: Commands::Branch,
        };
        let result = select_command(&cli).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn select_command_routes_artifact_directory() {
        let cli = Cli {
            json: false,
            command: Commands::ArtifactDirectory,
        };
        let result = select_command(&cli).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn cli_try_parse_accepts_branch() {
        use clap::Parser;
        let result = Cli::try_parse_from(["orksorksorks", "branch"]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_try_parse_accepts_artifact_directory() {
        use clap::Parser;
        let result = Cli::try_parse_from(["orksorksorks", "artifact_directory"]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_try_parse_rejects_kebab_case_artifact_directory() {
        use clap::Parser;
        let result = Cli::try_parse_from(["orksorksorks", "artifact-directory"]);
        assert!(result.is_err());
    }
```

Existing `cli_try_parse_rejects_no_subcommand` and `cli_command_debug_assert`
remain unchanged and still pass (the debug_assert covers the new variants).

### Verification

#### Automated
- [ ] `./scripts/test.sh`
- [ ] fast loop: `cargo nextest run --bin orksorksorks` (all four phases' unit tests pass)

#### Manual
- [ ] `cargo run -- --help` lists `init`, `branch`, `artifact_directory` as subcommands.
- [ ] `cargo run -- artifact_directory` prints the path (proves the snake_case pin accepted).

---

## Phase 5: Integration tests (binary spawn)

### Changes

#### 1. `tests/branch.rs` (new)
**File**: `tests/branch.rs`
**Action**: create

Text-mode success prints the payload then a hardcoded bell to stdout
(`println!("{data}")` then `print!("\x07")`, `src/main.rs:31-34`), so stdout is
`{branch}\n\x07`. Assert the exact value in JSON mode (no bell on the JSON
path) and assert content in text mode, stripping the bell.

```rust
use assert_cmd::Command;
use predicates::prelude::*;

fn git_branch() -> String {
    let output = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

#[test]
fn branch_prints_current_branch() {
    let expected = git_branch();
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd.arg("branch").output().unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim_end_matches('\x07').trim_end(), expected);
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn branch_json_returns_valid_json_with_no_ansi() {
    let expected = git_branch();
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd.args(["branch", "-j"]).output().unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["data"].as_str(), Some(expected.as_str()));
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

// Optional sad path: outside any repo, git fails → non-zero exit, "git" source.
#[test]
fn branch_outside_repo_fails() {
    let temp = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.current_dir(temp.path());
    cmd.arg("branch").assert().failure();
}
```

(Both tests run inside the repo: nextest sets each test's cwd to the crate
root — a git worktree — and the spawned binary inherits it, so `git branch
--show-current` resolves identically in both processes.)

#### 2. `tests/artifact_directory.rs` (new)
**File**: `tests/artifact_directory.rs`
**Action**: create

CWD must stay in-repo so `git` resolves; derive the expected string from
`std::env::current_dir()` in the test rather than a `tempfile::tempdir()`.

```rust
use assert_cmd::Command;
use predicates::prelude::*;

fn expected_artifact_directory() -> String {
    let cwd = std::env::current_dir().unwrap();
    let branch = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap();
    format!("{}/.pi/orksorksorks/{}/", cwd.display(), branch)
}

#[test]
fn artifact_directory_prints_composed_path() {
    let expected = expected_artifact_directory();
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd.arg("artifact_directory").output().unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim_end_matches('\x07').trim_end(), expected);
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn artifact_directory_json_returns_valid_json_with_no_ansi() {
    let expected = expected_artifact_directory();
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd.args(["artifact_directory", "-j"]).output().unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["data"].as_str(), Some(expected.as_str()));
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

// Optional sad path: outside any repo, git fails → non-zero exit.
#[test]
fn artifact_directory_outside_repo_fails() {
    let temp = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.current_dir(temp.path());
    cmd.arg("artifact_directory").assert().failure();
}
```

### Verification

#### Automated
- [ ] `cargo nextest run` (full suite: existing 21 tests + ~10 new unit + 4 new integration)
- [ ] `./scripts/test.sh` (full gate: fmt → check → clippy → nextest → forbidden-strings)

#### Manual
- [ ] `cargo run -- branch` → prints the current branch, no ANSI.
- [ ] `cargo run -- branch -j` → `{"data":"<branch>"}`.
- [ ] `cargo run -- artifact_directory` → `<cwd>/.pi/orksorksorks/<branch>/` with trailing slash.
- [ ] `cargo run -- artifact_directory -j` → JSON `data` holds the same path, no ANSI.
- [ ] From outside a git repo (`cd /tmp && cargo run --manifest-path ... branch`) → non-zero exit, `Error from git:` message.

---

## Testing Checkpoints (resume-safe)

1. **Phase 1** — `artifact_dir_path` unit tests green.
2. **Phase 2** — `current_branch` + `parse_branch_output` unit tests green.
3. **Phase 3** — `branch_command` / `artifact_directory_command` unit tests green.
4. **Phase 4** — clap parse + `select_command` routing unit tests green.
5. **Phase 5** — `cargo nextest run` (all tests) + full `./scripts/test.sh` green.

Do not advance past a red checkpoint.

## Implementation notes

- **No new dependencies.** `std::process::Command`, `std::env::current_dir`,
  and the existing `Error` type cover everything. `tempfile`, `serde_json`,
  `predicates`, `assert_cmd` are all already available to integration tests.
- **No ANSI in the two payloads.** These handlers return plain strings (the
  JSON `data` field); do not import `green_string` here.
- **Bell handling in text-mode integration asserts**: success stdout is
  `"{data}\n\x07"`; strip the trailing `\x07` before exact comparison, and
  assert exact values via the JSON `data` field (bell-free path).
- **`Error.source` tags** asserted: git failures → `"git"`; spawn failure of the
  git subprocess → `"io"` (existing `From`). No change to `src/errors.rs`.
- **No change to `src/main.rs`, `src/errors.rs`, `src/format.rs`, `src/config.rs`.**

## Files touched (summary)

| File | Action |
|---|---|
| `src/commands/mod.rs` | modify — 4 helpers, 2 handlers, 2 enum variants, 2 match arms, ~10 unit tests |
| `tests/branch.rs` | create — 2–3 integration tests |
| `tests/artifact_directory.rs` | create — 2–3 integration tests |