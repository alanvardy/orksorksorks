# Research Questions

## Context

`orksorksorks` is a small Rust CLI crate (single binary, ~250 LOC) with a clap-derive command surface, an ANSI output layer, a JSON output mode, and a committed test/CI gate. Explore how commands are structured, how the process environment and filesystem are handled, how the git branch identity appears across the repo, and what dependencies and test infrastructure are available.

## Questions

1. Trace the full command flow: how does clap parsing connect to the `Commands` enum, the command handler functions, and the `CommandResult` output envelope in `src/main.rs`? What exact contract must one command handler satisfy — return type, error propagation via `src/errors.rs`, JSON vs text output modes, bells, and exit codes?

2. How does the codebase currently handle the process working directory, environment variables, and filesystem operations? Where are relative paths used relative to CWD, how is the CWD-relative behavior tested (e.g. `assert_cmd` + `tempfile::tempdir` + `current_dir`), and what happens when the target path is not writable?

3. How is the current git branch represented and referenced throughout the repo — in CI workflows, scripts, QRSPI documentation under `.pi/qrspi/<branch>/`, and the worktree setup? What naming/keying convention ties a branch name to directories or artifact paths, and how would a string of the branch name be sourced (git command output, env vars, `.git` files)?

4. What dependencies are available in `Cargo.toml` (regular, dev, and build dependencies), what does `build.rs` inject at build time, and what do the test gates require — the `scripts/test.sh` order, the nextest config, the CI reusable workflows, and the integration-test conventions in `tests/`?

5. How are strings reported by commands formatted and surfaced to the user today? Which color helpers exist in `src/format.rs`, how do tests assert absence of ANSI codes, and how does the error type render its message and source — in both text and `json` modes?