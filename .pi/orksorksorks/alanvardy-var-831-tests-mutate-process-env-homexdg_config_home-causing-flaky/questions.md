# Research Questions

## Context

Focus on the `orksorksorks` Rust CLI (edition 2024, binary-only crate): config-directory
resolution and its unit tests (`src/config_dir.rs`), command routing (`src/commands/mod.rs`),
the central error type (`src/errors.rs`), the entry/output layer (`src/main.rs`), the test
tooling (`.config/nextest.toml`, `scripts/test.sh`, `.github/workflows/_reusable-test.yml`),
and the subprocess-spawning integration tests in `tests/`. The crate has no lib target; unit
tests live in-module and run in-process under nextest, while integration tests spawn the real
binary via `assert_cmd::cargo_bin`.

## Questions

1. How does config path resolution work end-to-end in `src/config_dir.rs`? Trace
   `config_file_path` and `resolve_config_dir`, naming every `std::env::var_os` read
   (`XDG_CONFIG_HOME`, `APPDATA`, `HOME`) and the exact conditions under which each variable
   is consulted or skipped (absolute-path check, empty/relative fall-through), plus how
   resolution failures are converted into the central `Error` type with the `"config-dir"`
   source tag.

2. How do the unit tests in `config_dir::tests` (`src/config_dir.rs:53-115`) and
   `select_command_routes_init` (`src/commands/mod.rs:177-190`) mutate the process
   environment? List each `std::env::set_var`/`remove_var` call, which variables it
   sets/unsets, the `tempfile::tempdir`/`current_dir` fixtures involved, and whether each
   mutation is restored after the test body — and what the resulting process-env state looks
   like mid-suite under nextest's in-process parallel execution.

3. How are tests executed in this repo? What exactly do `scripts/test.sh` and the CI workflow
   (`.github/workflows/_reusable-test.yml`) run, what does `.config/nextest.toml` configure
   (retries, timeouts, parallelism, output handling), and is there any per-test process
   isolation at the unit level? Where do tests already spawn real subprocesses (real `git` in
   `src/git.rs` unit tests, `assert_cmd::cargo_bin` in `tests/`), and what does that pattern
   look like?

4. What dependency-injection conventions already exist in the codebase? Trace functions that
   take explicit state instead of reading env/cwd (e.g. `git::current_branch_in(dir)`,
   `parse_branch_output`, `artifact_dir_path`, `determine_step`,
   `config_file_path(explicit)`), how their thin production wrappers read real env, and what
   the binary-only crate layout (no lib target) implies about what unit tests can import.

5. What facilities do the nextest test runner and the wider Rust ecosystem provide for
   isolating or sharing process-level state such as environment variables or the current
   directory — e.g. process-per-test options, env set/restore helpers, serialization, or
   fixture crates — and what are the documented semantics and limitations of each?

6. Where does the codebase read env or cwd beyond config resolution? List every `std::env::`
   call site in `src/` (`config_dir.rs`, `commands/mod.rs`, `git.rs`) and which tests
   exercise each; and describe the central `Error` type (`src/errors.rs`) — its fields, the
   `From` conversions (`"io"`, `"toml::ser"`, `"toml::de"`, `"config-dir"`), and how it is
   rendered in both text and JSON output modes (`src/main.rs`).