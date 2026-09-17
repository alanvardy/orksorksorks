# Task

Write a `README.md` at the repo root for **orksorksorks** (a Rust CLI, binary-only crate; no README currently exists — a draft PR #34 already targets this same deliverable).

The README must cover, per the ticket:

1. **Purpose of the project** — what orksorksorks is and does. Ground it in the actual source: it is a Rust CLI (`src/main.rs`, `src/commands/mod.rs`) whose subcommands are `init`, `branch`, `artifact_directory`, `step`, `model`, `thinking`, `prompt`, and `script`; it reads a `workspaces.toml`-style config — actually `orksorksorks.toml` — that defines "workspaces" (named config blocks), drives step prompts/config for coding agents, and stores artifacts under `.pi/orksorksorks/<branch>/`.
2. **What it does not do** — be honest about non-goals (it is not a general-purpose agent, not a build tool, etc. — check the help text/CLI source for what is deliberately out of scope).
3. **Installation** — the repo ships `scripts/install.sh`, which does `cargo install --path . --locked` into `~/.cargo/bin`. Cover prerequisites (Rust toolchain; see `rust-toolchain.toml`) and the `init` subcommand which writes a commented `templates/default.toml` config.
4. **A basic `orksorksorks.toml`** — a worked minimal example derived from `templates/default.toml` and the `src/config.rs` schema: `version = "0.1.0"` (must equal `CONFIG_VERSION`), and one workspace block with `name`, `model`, prompt etc. as the schema and default template define; keep it consistent with what `init` actually generates.

Also follow repo conventions: user-visible changes go in `CHANGELOG.md` under `[Unreleased]` (only if the repo deems a README-add a changelog entry — check existing CHANGELOG policy; there may be no CHANGELOG file yet), and the local gate `scripts/test.sh` must pass (it checks CI-adjacent markers in `*.rs` only, so a pure docs change should not affect it). Do not run `cargo` unless needed to verify facts. A draft PR (#34) for this branch already exists — reconcile with its content rather than duplicating or clobbering it.

## Why SMALL

Single new file at repo root following the standard README pattern; no unknowns (ticket enumerates the exact sections), no schema/migration, no new subsystem or shared/convention code, no design decision, and no test surface — criteria A–F all hold. No LARGE or breadth triggers apply.

## Key files (if the recon found any)

- `README.md` — not yet present, to be created at repo root
- `scripts/install.sh` — the documented install path
- `templates/default.toml` — source for the basic config example
- `src/config.rs` + `src/commands/mod.rs` — ground the CLI surface and schema descriptions in what the code actually does
- Existing draft PR #34 (attachment on VAR-1037) — reconcile with in-flight content