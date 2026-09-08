# Research Questions

## Context

Focus on the configuration layer of this tool: how `orksorksorks.toml` is
defined, parsed, loaded, and used (notably the `steps` section and any
prompt-related section), and on the conventions that govern error reporting
and testing around it. The config is a binary-only crate, so behavior is
observed through unit tests and CLI integration tests.

## Questions

1. **Config schema**: What sections and keys does `orksorksorks.toml`
   currently support (e.g. `version`, `steps`, and any `prompt`/`prompts`
   section)? What fields does each entry have, which are required vs
   optional, and are entries referenced or cross-referenced by name
   anywhere? Cite the serde types in `src/config.rs`.

2. **Validation surface**: Beyond what serde enforces at parse time, what
   validation exists for parsed config — custom `Deserialize` impls,
   `validate()` methods, or checks for duplicate names, duplicate
   artifacts, ordering, or cross-section references? Where would semantic
   (post-parse) validation be applied, if anywhere?

3. **Config load + use flow**: Trace how the config is loaded and consumed
   from CLI entry (`--config` / env path resolution) through to runtime
   step selection. Where in the load order does the config get read, and
   what happens to parsed values afterward (e.g. `version`, `steps`,
   trigger artifacts)? Anchor in `src/commands/mod.rs`, `src/config.rs`,
   `src/config_dir.rs`.

4. **Error handling conventions**: How are config errors represented and
   surfaced to the user — the custom error type, its `source` tags, and how
   path/resolution context is embedded in messages? What would a new
   validation-failure error need to conform to (message shape, JSON
   envelope, exit code)? Anchor in `src/errors.rs`, `src/main.rs`.

5. **Testing conventions**: How are config parse/validation behaviors
   tested — unit tests in-module vs CLI integration tests in `tests/`? How
   are TOML samples provided (inline `std::fs::write` vs fixtures), and
   what runner/CI constraints apply (nextest profile, coverage)? What
   regression patterns exist for pinning exact error messages?

6. **Schema references outside the crate**: Are there any docs, example
   TOML files, or references in the broader tooling that describe the
   intended config schema — in particular the relationship between a
   "step" and a "prompt" (name-based association or otherwise)? If none
   exist, state that explicitly.
