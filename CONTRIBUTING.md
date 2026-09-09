# Contributing to orksorksorks

Thanks for contributing! This file is the contract for opening a PR that gets
merged quickly.

## Pre-PR checklist

- [ ] `scripts/test.sh` passes locally — it runs `cargo fmt --all`,
      `cargo check`, `cargo clippy --tests -- -D warnings`,
      `cargo nextest run --no-tests pass`, and a scan that fails on
      `TODO:`/`FIXME`/`dbg!`/`DEBUG:`/`FIXTURE:` markers in `*.rs` sources.
- [ ] Tests cover the happy path **and** the sad path for the change.
- [ ] `CHANGELOG.md` is updated under `## [Unreleased]`.
- [ ] Notable design decisions are recorded in `docs/adr/` (see
      [docs/adr/README.md](docs/adr/README.md)).
- [ ] The PR targets `main`.
- [ ] The required `CI Gate` check is green on the PR.
- [ ] The PR has one approving review.

## Branch protection

`main` is protected: the `CI Gate` check is required, pull requests need one
approving review, and merging is rebase-only (the branch is rebased, then
deleted). Keep PRs focused and rebase-friendly.

## Coding conventions

### Error handling

All failures flow through the central `Error` type in `src/errors.rs`:
a lowercase `source` tag (for example `io`, `config-exists`, `step`, `git`)
plus a human-readable `message`. Never emit bare status codes or build ad-hoc
error plumbing: construct `Error::new(source, message)` (or use the existing
`From<...>` impls for wrapped I/O and TOML errors) and return it through
`Result`. Display is owned by `Error`; callers must not pre-apply ANSI.

### Output and color

All ANSI coloring goes through the chokepoint in `src/format.rs`
(`green_string`, `red_string`, `yellow_string`). Text output goes to stdout on
success and stderr on error; `-j/--json` emits `{"data": ...}` on success and
`{"error": {"message", "source"}}` on failure, and errors exit with status 1.

### Configuration

TOML config is validated against the schema in `src/config.rs`
(`deny_unknown_fields`, a `version` check, and cross-reference validation).
Parse-level failures carry `io` / `toml::de` tags; semantic violations carry
`config:*` tags.

### CLI

Subcommands are declared with clap derive in `src/commands/mod.rs`, routed
through `select_command`, and dispatched to per-command handlers. Match the
existing flag surface (`--config`, `--step`, the positional `STEP_NAME` on
`script`) and the global `-j/--json`.

### Tests

The crate is binary-only, so automated coverage lives in `tests/` as
integration tests that execute the built binary via `cargo nextest`; unit
tests that need isolation live alongside the code in `#[cfg(test)]`
modules. Cover the failure path too — every new command or flag should have a
matching error test.

### Forbidden markers

The `scripts/test.sh` gate and CI both fail on `TODO:`, `FIXME`, `dbg!`,
`DEBUG:`, and `FIXTURE:` markers in `*.rs` files. Do not merge code that
contains them.

### Commit messages

Short, capitalized, imperative subjects matching repo style ("Add script",
"Validate orksorksorks.toml", "Phase N: <description>"), with `ci:`/`test:`/
`chore:` prefixes for infra-only changes.

## Reporting security issues

Do **not** open a public issue for vulnerabilities. Report privately via the
GitHub Security Advisories flow — see [SECURITY.md](SECURITY.md).