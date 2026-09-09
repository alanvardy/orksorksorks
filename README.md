# orksorksorks

orksorksorks is a conventions CLI: it centralizes the contract for how a
repository is worked — step definitions, models, prompts, and scripts — in one
versioned TOML config, and ships that spine with typed errors, colored output,
and a JSON envelope for scripting.

Configuration lives in `orksorksorks.toml` (created and managed with
`orksorksorks init`) and drives a set of small plumbing commands:

| Command | Purpose |
|---|---|
| `init` | Write `orksorksorks.toml` (refuses to overwrite an existing file) |
| `branch` | Print the current git branch |
| `artifact_directory` | Print `$PWD/.pi/orksorksorks/<branch>/` |
| `step` | Print the current step (derived from trigger artifacts, or `--step`) |
| `model` | Print the model for the current step |
| `thinking` | Print the thinking budget for the current step |
| `prompt` | Print the prompt for the current step (with context frontmatter by default) |
| `script` | Print the raw script for the current step (or a named step) |

All commands accept a global `-j/--json` flag that emits a JSON envelope
(`{"data": ...}` on success, `{"error": {"message", "source"}}` on failure)
instead of text. Commands that read the config accept `--config PATH` to point
at a specific file, and `step`/`model`/`thinking`/`prompt` accept `--step NAME`
to override step derivation.

## Requirements

- Rust toolchain **1.98.1** — pinned in [`rust-toolchain.toml`](rust-toolchain.toml)
  (components: `clippy`, `rustfmt`).

## Install

Run the install script:

```bash
scripts/install.sh
```

which executes `cargo install --path . --locked` and installs the
`orksorksorks` binary to `~/.cargo/bin`.

## Local development

From a checkout of this repository:

```bash
scripts/test.sh
```

runs the full local gate: `cargo fmt --all`, `cargo check`,
`cargo clippy --tests -- -D warnings`, `cargo nextest run --no-tests pass`,
and a scan that fails on `TODO:`/`FIXME`/`dbg!`/`DEBUG:`/`FIXTURE:` markers
in `*.rs` sources.
See [CONTRIBUTING.md](CONTRIBUTING.md) for the pre-PR checklist.

## Configuration

`orksorksorks init` writes a commented `orksorksorks.toml` template. Without an
explicit `--config PATH`, the file is resolved to `orksorksorks.toml` under:

1. `$XDG_CONFIG_HOME` — honored on any platform, but only when set to an
   **absolute** path (unset, empty, or relative values fall back);
2. `%APPDATA%` — on Windows;
3. `$HOME/.config` — otherwise.

The config is strictly validated against the schema in `src/config.rs`:
unknown keys are rejected, `version` must match the supported format, and
cross-references must resolve — every `step` needs a matching `prompt` and
an existing `model`. `script` references are optional and resolve at run
time when the `script` command is used.
Supported sections are `version`, `show_frontmatter`, `[[steps]]`,
`[[models]]`, `[[scripts]]`, and `[[prompts]]` — see
[`templates/default.toml`](templates/default.toml) for the annotated template.

## Artifact directory

Step derivation operates on `$PWD/.pi/orksorksorks/<branch>/`, where `<branch>`
is the current git branch with `/` normalized to `-`. `worksorksorks
artifact_directory` prints the path; it does not create it.

## Documentation

- [CONTRIBUTING.md](CONTRIBUTING.md) — pre-PR checklist and coding conventions
- [SECURITY.md](SECURITY.md) — how to report vulnerabilities
- [CHANGELOG.md](CHANGELOG.md) — release notes
- [docs/adr/](docs/adr/README.md) — architecture decision records
- [AGENTS.md](AGENTS.md) — agent/contributor instructions

## License

MIT — see [LICENSE](LICENSE).