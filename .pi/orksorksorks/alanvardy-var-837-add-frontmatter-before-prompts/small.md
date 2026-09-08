# Task

When running `orksorksorks prompt`, prepend the following frontmatter block above the prompt output:

```
step = $step
branch = $branch
artifact_directory = $artifact_directory
```

(`step`, `branch`, `artifact_directory` are the current values, not literal `$vars`.)

Add a `show_frontmatter` boolean key to the `orksorksorks.toml` config (default `true`). When set to `false`, the frontmatter must not be shown. The config key is `show_frontmatter = true` in the toml.

## Why SMALL

Single module (one Rust crate) touched across 3 files (`src/config.rs`, `src/commands/mod.rs`, `tests/prompt.rs`); follows the existing pattern of config field + command behavior + prompt tests. No schema/migration, no new subsystem, no design decisions (format and flag name are specified), 0 unknowns — the three frontmatter values are already computed inside `prompt_command` via `git::current_branch()`, `determine_step`, and `artifact_dir_path`, so only prepend logic + a serde-defaulted config field are needed.

## Key files (recon found)

- `src/config.rs` — `Config` struct (line ~41): add `show_frontmatter: bool` with default `true`, and to the TOML serialization.
- `src/commands/mod.rs` — `prompt_command` (line ~281): after resolving the prompt, prepend the frontmatter block when `cfg.show_frontmatter` is true, using the already-in-scope step name, `git::current_branch()`, and artifact dir.
- `tests/prompt.rs` (198 lines) — add local tests for: frontmatter shown by default, hidden when `show_frontmatter = false`. Beware `toml::de`/serde handling for a new field on existing configs (default value needed so old configs without the key still parse).