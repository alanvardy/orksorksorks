# Research Questions

## Context

This is a freshly scaffolded Rust binary crate (`orksorksorks`) with a hello-world `main.rs`, an empty `[dependencies]` table, and no tests, CI, or config files. The author's established Rust conventions live in sibling projects under `~/dev/` — most relevantly the `tod` clap-based CLI. Research should map those conventions (structure, testing, error handling, file I/O, toolchain) since this project has no local precedent. There is no `toml`-rs usage anywhere in the author's Rust codebases yet, and no `clap` dependency here.

## Questions

1. How does the `tod` project structure its clap-based CLI entrypoint and subcommands? Trace the full shape: the `Cli` struct and `Commands` enum (derive features, global flags, aliases), how arg structs are organized across modules, and how dispatch from parsed args to per-command logic is wired.

2. What testing conventions do the author's Rust CLIs follow? Survey how `tod` (and any sibling project) writes tests: integration tests vs inline `#[cfg(test)]` modules, which crates are used (`assert_cmd`, `predicates`, `tempfile`, `pretty_assertions`), the `Cli::command().debug_assert()` smoke-test idiom, and any `nextest` config or CI lint/test gates.

3. What error-handling and terminal-output patterns do the author's Rust projects use? Describe the `tod` error type (shape, `source`/`message` fields, `From` impls, colored `Display`), how it differs from `thiserror`/`anyhow`, and the success-output convention (e.g. the `✓` symbol, colored strings).

4. How do the author's Rust projects create and write files? Trace the file-writing helpers (e.g. `touch_file`, `create_dir_all` parent creation, `flush`/`sync_all`), how I/O errors are propagated, and whether any project in the author's codebases uses the `toml` crate — if so, how is it configured and used?

5. What build, dependency, and toolchain conventions do the author's Rust projects follow? Survey `Cargo.toml` structure across projects (edition, dependency version-pinning style, `[dev-dependencies]`, `[[bin]]`/workspace layout), `rust-toolchain` pinning (e.g. the `1.97.1` channel), `rustfmt`/`clippy` config, and CI workflows (lint gates like `cargo fmt --check` and `clippy -D warnings`).
