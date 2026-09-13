# AGENTS.md

Instructions for agents and contributors working in this repository.

`orksorksorks` — a Rust CLI (`Cargo.toml`, `src/`, `tests/`); the
`orksworksorks` typo mis-creates `.pi/orksworksorks` parents. Shell/commit/
editing rules: `~/.pi/agent/AGENTS.md`; below are only repo-specific rules.

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
- `templates/` — `default.toml` (commented config template written by `init`)
- `.github/workflows/` — CI: lint, test, and the required `CI Gate`
- `docs/adr/` — architecture decision records

## Commands

### Local gate

```bash
scripts/test.sh
```

`cargo fmt --all` → `cargo check` → `cargo clippy --tests -- -D warnings`
→ `cargo nextest run --no-tests pass`, then a gate failing on
`TODO:`/`FIXME`/`dbg!`/`DEBUG:`/`FIXTURE:` markers in `*.rs`.

### Install

```bash
scripts/install.sh   # cargo install --path . --locked → ~/.cargo/bin
```

### CI equivalents

CI runs the same locked/all-features checks behind the required `CI Gate`
— see `CONTRIBUTING.md`.

## Conventions

### Errors

All failures use the `Error` type from `src/errors.rs` — a lowercase `source`
tag plus a `message`; `Display` is owned by `Error` and colored exclusively
through `src/format.rs`. JSON mode (`-j/--json`) emits `{"data": ...}` on
success and `{"error": {"message", "source"}}` — always status 1.

### Config

`orksorksorks.toml` is validated against `src/config.rs` with
`deny_unknown_fields`: unknown keys fail parse (`toml::de`), and semantic
violations fail with `config:*` tags (`config:version`,
`config:duplicate-name`, `config:empty-name`, `config:empty-model`,
`config:duplicate-trigger`, `config:multiple-default`,
`config:missing-prompt`, `config:missing-model`); `version` must equal
`CONFIG_VERSION` (`"0.1.0"`).

### CLI

Subcommands live in `src/commands/mod.rs` with clap derive: `init`
(`-c/--config PATH`), `branch`, `artifact_directory`, `step`, `model`,
`thinking`, `prompt` (each `--config PATH --step NAME`), and `script`
(positional `STEP_NAME`, `--config PATH`). `-j/--json` is a global flag.

### Tests

Integration tests in `tests/` target the built binary (`cargo nextest run
--no-tests pass`; CI adds `--profile ci --all-features`); unit tests live
in `#[cfg(test)]` modules next to the code. Cover happy and sad paths,
isolating process state via `tempfile`/`assert_cmd`, not ambient state.

### Environment variables

Env surface (`XDG_CONFIG_HOME`, `HOME`, `APPDATA`, `BUILD_*` build-time
vars): see `README.md`; the config filename is always `orksorksorks.toml`.

### Artifact directory

Commands operate on `$PWD/.pi/orksorksorks/<branch>/` (`/` → `-`; see the
`artifact_directory` subcommand). Each phase commits its own artifact — never a
bulk sweep; `.ignore` hides `.pi/orksorksorks` from ripgrep only.

## Ticket workflow

Ticket branches are worktrees `~/dev/alanvardy-var-<n>-<slug>`, each
started with a tracked `DELETEME` bootstrap marker — `git rm` before
merging; it must never reach `main` (docs merge `bc53ab8e` leaked it
once). Artifacts: `.pi/orksorksorks/<branch>/`.

### Closing a ticket

`gh pr ready` (check `isDraft` before merging) → `gh pr merge --rebase
--delete-branch` → verify `mergedAt` → `git -C <root> worktree remove` →
`branch -D` → archive the Linear ticket.
Circuit breaker: never `git worktree remove` the directory you are standing
in — `Working directory does not exist` is fatal; bash validates the
session cwd before spawning, and `cd <root> &&` cannot recover it.

## Commit and PR conventions

- `main` is protected: `CI Gate` required, one approving review, rebase-only
  merge with branch deletion.
- Commit subjects are short and imperative, with `ci:`/`test:`/`chore:`
  prefixes for infra-only changes.
- Before committing: run `scripts/test.sh`; add happy/sad-path tests; update
  `CHANGELOG.md` under `[Unreleased]`; record decisions in `docs/adr/`.

## Docs policy

User-visible changes update the README/CHANGELOG and — when a design
decision is involved — an ADR in `docs/adr/`; see `docs/adr/README.md`,
`CONTRIBUTING.md`, `SECURITY.md`.