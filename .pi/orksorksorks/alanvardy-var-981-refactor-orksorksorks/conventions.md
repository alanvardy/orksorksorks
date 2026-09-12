# Conventions — orksorksorks

Shared factual appendix: commands, test inventory, and gotchas. Research/design/structure/plan phases rely on this instead of re-reading the tree.

## Canonical commands

Local gate (authoritative, run before committing): `scripts/test.sh`
1. `cargo fmt --all` (scripts/test.sh:9)
2. `cargo check` (scripts/test.sh:12)
3. `cargo clippy --tests -- -D warnings` (scripts/test.sh:15)
4. `cargo nextest run --no-tests pass` (scripts/test.sh:18) — note **nextest**, not libtest
5. Forbidden-strings gate: `rg -i -g '*.rs' 'TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:'` must match nothing (scripts/test.sh:22-25)

CI (`.github/workflows/ci-pr.yml` → `_reusable-lint.yml` + `_reusable-test.yml`, ubuntu-latest, stable toolchain):
- lint: `cargo check --locked --all-features` (_reusable-lint.yml:17); `cargo fmt --all -- --check` (:19); `cargo clippy --all-targets --all-features --locked -- -D warnings` (:21); same forbidden-strings gate (:23-25)
- test: `cargo nextest run --profile ci --all-features --no-tests pass` (_reusable-test.yml:25); coverage `cargo llvm-cov nextest --profile ci --all-features --lcov` (:28) → codecov (:31); junit via test-results-action (:36-38)

Notes:
- Local clippy is `--tests`; CI is `--all-targets --all-features --locked`. `nextest` is required locally (not cargo test).
- `build.rs` emits `cargo:rustc-env` for BUILD_TARGET/PROFILE/TIMESTAMP (build.rs:5-11) using `chrono` (Cargo.toml:31); `#[tokio::main]` on main (src/main.rs:82).
- `#![warn(missing_docs)]` (src/main.rs:6) — clippy `-D warnings` makes missing `///` docs on `pub`/`pub(crate)` items a gate failure (convention: every pub item documented, see src/config_dir.rs:11-17, :24-26, :67-72).

## Test-suite inventory

Integration tests (10 files, `tests/`): every file spawns the compiled binary via `assert_cmd::Command::cargo_bin("orksorksorks")` (tests/init_creates_file.rs:17) with tempdirs + injected `XDG_CONFIG_HOME`; asserts exit status, raw stdout/stderr bytes, JSON envelopes. No `#[cfg(unix/windows)]` gating anywhere in tests/ (grep-verified). The crate is binary-only, so integration tests cannot import crate types (tests/init_creates_file.rs:22-24).

| File | Covers |
|---|---|
| `tests/architecture.rs` | rust_arkitect: commands import allowlist (:51-78) + module-order cycle ban (:89-140) |
| `tests/init_creates_file.rs` | init: template byte-equality, success message, readonly failure, `io`/`config-exists`/`config-dir` JSON sources, `--config` override, no-overwrite |
| `tests/json_output.rs` | JSON envelope `data` field, no-ANSI plain text |
| `tests/branch.rs` | branch name text + `-j`, outside-repo failure |
| `tests/artifact_directory.rs` | composed path w/ trailing slash + `/`→`-`, canonicalization, `-j`, outside-repo failure |
| `tests/config_validation.rs` | 8 exact (stderr phrase / `error.source`) pairs: config:version, config:duplicate-name, config:empty-name, config:empty-model, config:duplicate-trigger, config:multiple-default, config:missing-prompt, config:missing-model |
| `tests/step.rs` | artifact-derived step, default fallback, XDG, cwd-config-ignored guard, `--step` override (git-less/detached HEAD), missing-path stderr spelling, unknown-step text vs JSON |
| `tests/model.rs` (+thinking) | exact model strings, `-j` data, XDG, `--step` override |
| `tests/prompt.rs` | frontmatter exact text, `show_frontmatter=false`, positional rejected (exit 2) |
| `tests/script.rs` | never emits frontmatter, positional override, `script` error tags, XDG |

Unit tests: `#[cfg(test)] mod tests` at end of every module — src/commands/mod.rs:437 (≈1,000 lines: parse routes, routing, handler logic, helper resolution, error tag/message pairs), src/config.rs:254, src/config_dir.rs:116, src/errors.rs:73, src/format.rs:26, src/git.rs:39. Convention: `use super::*;`, pretty_assertions where useful, behavior-sentence test names.

## Gotchas

- **Relative-path include**: `include_str!("../../templates/default.toml")` (src/commands/mod.rs:181) and its integration-test twin `tests/init_creates_file.rs:25,107` resolve relative to the source file; path depth changes if the call moves into deeper submodule files.
- **ANSI colors**: `src/format.rs` `apply_color` strips under `cfg!(test)` (format.rs:6-12) — affects in-process unit tests only. For the real binary, `colored` 3.1.1 strips color on non-tty output (empirically verified: piped stdout has no `\x1b`; all "no ANSI" integration assertions rely on this). Color appears only on interactive terminals. `Error`'s `Display` owns all coloring; callers must not pre-apply ANSI (src/errors.rs:5-8).
- **rust_arkitect dual spellings**: dependencies are recorded "exactly as written" (tests/architecture.rs:21-25), so both `crate::x` and `orksorksorks::x` spellings are allowlisted (10 entries, :56-75) — any new `use crate::...` path in moved code must be a sibling-module spelling already covered. Submodule files under `src/commands/` keep the `orksorksorks::commands` identity — no new module-level rules needed.
- **stdout/stderr split**: text-mode errors → stderr with two leading blank lines (`\n\n{e}`, src/main.rs:32-33); JSON-mode errors → stdout as `{"error":{"message","source"}}` (src/main.rs:46-47); both exit 1 (src/main.rs:76-77). Handlers never print — they return `Result<String, Error>`.
- **Integration tests spawn real processes**: tempdirs are restored (readonly test explicitly `set_readonly(false)` for cleanup — tests/init_creates_file.rs:1-3); macOS tempdir canonicalization `/var`→`/private/var` needed (tests/artifact_directory.rs:40-42).
- **`--no-tests pass`**: nextest invocation tolerates empty test sets; `--profile ci` + `--all-features` only in CI.