# AGENTS.md

Instructions for agents and contributors working in this repository.

## Layout

- `src/` — crate source (binary-only; no lib target)
  - `src/main.rs` — entrypoint, text/JSON output dispatch, exit codes
  - `src/commands/mod.rs` — clap CLI definitions and command handlers
  - `src/config.rs` — TOML config schema and validation
  - `src/config_dir.rs` — config-directory resolution
  - `src/errors.rs` — the central `Error` type
  - `src/format.rs` — the single color/ANSI chokepoint
  - `src/git.rs` — git-branch resolution
- `tests/` — integration tests that run the built binary (`cargo nextest`)
- `scripts/` — `install.sh` (install) and `test.sh` (local gate)
- `templates/` — `default.toml` (the commented config template written by
  `init`)
- `.github/workflows/` — CI: lint, test, and the required `CI Gate`
- `docs/adr/` — architecture decision records

## Commands

### Local gate

```bash
scripts/test.sh
```

runs, in order: `cargo fmt --all`, `cargo check`,
`cargo clippy --tests -- -D warnings`, `cargo nextest run --no-tests pass`,
then a gate that fails on `TODO:`/`FIXME`/`dbg!`/`DEBUG:`/`FIXTURE:` markers
in `*.rs` files.

### Install

```bash
scripts/install.sh   # cargo install --path . --locked → ~/.cargo/bin
```

### CI equivalents

The lint job runs `cargo check --locked --all-features`,
`cargo fmt --all -- --check`, and
`cargo clippy --all-targets --all-features --locked -- -D warnings`, plus the
same forbidden-string gate. The test job runs
`cargo nextest run --profile ci --all-features --no-tests pass`. The reusable
test workflow also installs `cargo-llvm-cov` and carries a coverage step, but
that step is gated behind an `upload-coverage` input that no current caller
passes (`ci-pr.yml` triggers on pull requests only and leaves it at its
default of `false`), so coverage is not uploaded in CI today. The required
status check is `CI Gate`, wired as a `ci-gate` job that depends on lint and
test.

## Conventions

### Errors

All failures use the `Error` type from `src/errors.rs` — a lowercase `source`
tag plus a `message`. `Display` is owned by `Error` and colored exclusively
through `src/format.rs`; callers never emit bare status codes or ANSI
directly. JSON mode (`-j/--json`) emits `{"data": ...}` on success and
`{"error": {"message", "source"}}` on failure; errors exit with status 1.

### Config

`orksorksorks.toml` is validated against `src/config.rs` with
`deny_unknown_fields`: unknown keys fail parse (`toml::de`), and semantic
violations fail with `config:*` tags (`config:version`,
`config:duplicate-name`, `config:empty-name`, `config:empty-model`,
`config:duplicate-trigger`, `config:multiple-default`,
`config:missing-prompt`, `config:missing-model`). `version` must equal
`CONFIG_VERSION` (`"0.1.0"`).

### CLI

Subcommands live in `src/commands/mod.rs` with clap derive: `init`
(`-c/--config PATH`), `branch`, `artifact_directory`, `step`, `model`,
`thinking`, `prompt` (each `--config PATH --step NAME`), and `script`
(positional `STEP_NAME`, `--config PATH`). `-j/--json` is a global flag.

### Tests

Integration tests in `tests/` target the built binary through `cargo nextest`
(`cargo nextest run --no-tests pass`; CI adds `--profile ci --all-features`).
Unit tests live in `#[cfg(test)]` modules next to the code. Test both happy
and sad paths, and isolate process state (env, tempdirs) with the
`tempfile`/`assert_cmd` helpers already in use rather than ambient state.

### Environment variables

- `XDG_CONFIG_HOME` — config directory; honored on any platform only when set
  to an **absolute** path; unset, empty, and relative values fall back.
- `HOME` — used for the `$HOME/.config` fallback (non-Windows).
- `APPDATA` — used for the `%APPDATA%` fallback (Windows).
- The config filename is always `orksorksorks.toml`.
- `BUILD_TARGET`, `BUILD_PROFILE`, `BUILD_TIMESTAMP` — embedded at build time
  by `build.rs` and surfaced in the version string.

### Artifact directory

Commands operate on `$PWD/.pi/orksorksorks/<branch>/`, with `/` in branch
names normalized to `-` (see the `artifact_directory` subcommand). The
repo-local `.ignore` file hides `.pi/orksorksorks` — never commit it.

## Commit and PR conventions

- `main` is protected: `CI Gate` required, one approving review, rebase-only
  merge with branch deletion.
- Commit subjects are short, capitalized, and imperative ("Add X",
  "Phase N: <description>"), with `ci:`/`test:`/`chore:` prefixes for
  infra-only changes.
- Before committing: run `scripts/test.sh`; add happy- and sad-path tests;
  update `CHANGELOG.md` under `[Unreleased]`; record notable decisions in
  `docs/adr/`.

## Docs policy

User-visible changes update the README and CHANGELOG, and — when a design
decision is involved — an ADR in `docs/adr/`. See `docs/adr/README.md`,
`CONTRIBUTING.md`, and `SECURITY.md` for the surrounding conventions.