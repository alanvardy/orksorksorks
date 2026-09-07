# Research Questions

## Context

`orksorksorks` is a small Rust CLI binary crate (single package, no lib/workspace) with a clap-derive command surface, a TOML config struct, an ANSI/JSON output layer, a hand-rolled error type, and a committed test/CI gate. Explore how the command surface, config parsing, filesystem/path handling, and test tooling actually work today.

## Questions

1. Trace the config story end-to-end: what does the `Config` struct in `src/config.rs` define and derive, how is it defaulted and (de)serialized via `toml`, and is there any production code path that reads a TOML file from disk versus only serializing to a string? Are there any `Vec` / array-of-tables / `Option` fields anywhere in the config or structs today, and if so what serde shape do they use?

2. How is the CLI arg surface declared and dispatched? Walk from `Cli`/`Commands` in `src/commands/mod.rs` through `select_command` and `run_command` in `src/main.rs` to the output layer: what argument types exist (flags vs positional args), how is a command handler wired in, and what return/`CommandResult` contract must each handler satisfy (error propagation, JSON vs text output, bells, exit codes)? Is there any existing example of a positional path/config argument, and what does clap 4.6.6 offer for typed `Path` args?

3. How is the artifact directory path derived and used? Trace `artifact_dir_path` and the `artifact_directory` command in `src/commands/mod.rs`, and how the same path format is mirrored in `tests/artifact_directory.rs`. What exactly is the path contract (cwd, `.pi/orksorksorks`, branch normalization, trailing slash), and how is the CWD and current branch sourced?

4. What filesystem operations exist in the codebase today? Where is the process working directory obtained, are there any production file-existence checks (e.g. `path::exists` on `std::path::Path`, or anything `try_exists`-style), and how are relative paths joined and tested (the `tempfile::tempdir` + `assert_cmd` patterns)? What async/filesystem reads does the `tokio` dependency actually enable?

5. How are errors modeled and propagated? Describe `src/errors.rs`: the `Error` struct fields, its constructors, the `From` conversions that exist (which sources map to which `source` tags), the `std::error::Error` and `fmt::Display` impls, and how errors surface through `main.rs` in both text and JSON modes. Which `From` conversions exist and which are missing (e.g. a TOML deserialization error path)?

6. What does the test gate require? Summarize the two test tiers (inline `#[cfg(test)]` units in `src/` vs integration tests in `tests/` spawning the real binary), how tests are run (`scripts/test.sh`, nextest config, CI reusable workflows), and the conventions for asserting stdout/JSON/bell/error output. Any existing fixtures, snapshot or golden-file conventions?