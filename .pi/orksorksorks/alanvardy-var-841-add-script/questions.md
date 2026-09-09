# Research Questions

## Context

Focus areas: the `orksorksorks.toml` schema in `src/config.rs` (struct
hierarchy, serde default/skip patterns, the named-lookup collection pattern,
the format `version` field), the CLI subcommand dispatch and current-step
resolution in `src/commands/mod.rs` plus config-path resolution in
`src/config_dir.rs`, the central error type and text/JSON output envelope in
`src/errors.rs` and `src/main.rs`, and the test/build conventions in
`tests/`, `scripts/`, and `.github/workflows/`.

## Questions

1. How is the `orksorksorks.toml` config modeled in `src/config.rs` — the full
   struct hierarchy (`Step`, `Model`, `Prompt`, `Config`), every serde
   attribute in play (defaults, `skip_serializing_if`, etc.), what the format
   `version` field is for and how it is used today, and how `read_config`
   parses the file including every error path it can produce? Also describe
   the unit tests at config.rs and what they assert about serialization,
   round-trips, and missing collections.

2. How does the `prompt` subcommand work end to end in `src/commands/mod.rs`:
   the `Cli`/`Commands` enum wiring, `select_command_with_env` dispatch,
   config-path resolution via `config_dir.rs`, the `determine_step`
   current-step resolution contract (`artifact_dir_path` trailing-slash
   behavior, `trigger_artifact` presentness, the default-step fallback, error
   cases), `resolve_prompt` name lookup, frontmatter handling, and how the
   content is printed. Also note how `step`/`model`/`thinking` commands follow
   the same pattern.

3. What is the test-tooling inventory: for each file in `tests/` and each
   `#[cfg(test)]` module inside `src/`, what does it cover, and what harness
   helpers recur (git-repo setup, `write_config`, artifact-dir helpers,
   `assert_cmd` invocation, JSON assertions)? Note any platform gating or
   whole-file gating.

4. How is the central `Error` type in `src/errors.rs` structured, what error
   source tags exist across the codebase (enumerate every `Error::new` /
   tag usage with `file:line`), and how do `main.rs`'s `output_text`,
   `output_json`, and `output_result` render success and error output in text
   vs JSON mode?

5. What are the canonical build/lint/test/format/verify commands and gates —
   contents of `scripts/test.sh` and `scripts/install.sh`, the CI jobs in
   `.github/workflows/`, `.config/nextest.toml` profiles, `codecov.yml`, and
   `build.rs`/`rust-toolchain.toml`? Enumerate the exact commands that must
   pass before merge and any gotchas (forbidden-string gates, detached-HEAD
   behavior, coverage thresholds).