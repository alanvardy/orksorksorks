# Conventions

Shared factual appendix — canonical commands, test-suite inventory, and build
gotchas. Dense references; design/plan phases rely on this instead of
re-opening the tree.

## Canonical commands

| Purpose | Command | Where |
|---|---|---|
| Local gate (must pass before merge) | `bash scripts/test.sh` | scripts/test.sh:1-21 |
| Format (in place) | `cargo fmt --all` | scripts/test.sh:5 |
| Compile check | `cargo check` | scripts/test.sh:8 |
| Lint (warnings as errors) | `cargo clippy --tests -- -D warnings` | scripts/test.sh:11 |
| Test (local) | `cargo nextest run --no-tests pass` | scripts/test.sh:14 |
| Test (CI) | `cargo nextest run --profile ci --all-features --no-tests pass` | .github/workflows/_reusable-test.yml:22-23 |
| Install | `cargo install --path . --locked` | scripts/install.sh:5 |
| CI lint job | `cargo check --locked --all-features` + `cargo fmt --all -- --check` + `cargo clippy --all-targets --all-features --locked -- -D warnings` | .github/workflows/_reusable-lint.yml:15-19 |
| Forbidden strings (both gates, case-insensitive) | `rg -i -g "*.rs" "TODO:\|todo:\|FIXME\|fixme\|dbg!\|DEBUG:\|FIXTURE:" .` | scripts/test.sh:17-19; _reusable-lint.yml:20-22 |
| Coverage (not enforced on PRs) | `cargo llvm-cov nextest --profile ci --all-features --lcov --output-path lcov.info` | _reusable-test.yml:24-26; codecov.yml:8-12 |

Toolchain: channel `1.98.1`, components clippy+rustfmt (rust-toolchain.toml:1-3).
`--locked` in all CI + install.sh (Cargo.lock committed); `scripts/test.sh` runs
unlocked. `build.rs` emits BUILD_TARGET/PROFILE/TIMESTAMP consumed by
`LONG_VERSION` (build.rs:3-13, src/commands/mod.rs:11-17). nextest `ci` profile:
retries=2, fail-fast=false, junit at target/nextest/ci/junit.xml
(.config/nextest.toml:2-9). codecov: ignore src/main.rs, project target auto
threshold 10%, patch target 50% — only if upload-coverage is set (ci-pr.yml
never sets it).

## Test-suite inventory

All integration tests spawn the binary via
`assert_cmd::Command::cargo_bin("orksorksorks")` — crate is binary-only, tests/
cannot import `Config` (tests/init_creates_file.rs:9-11).

### Integration (tests/)
| File | # | Covers |
|---|---|---|
| artifact_directory.rs | 3 | `.pi/orksorksorks/<branch-dashed>/` trailing-slash output; JSON `data` canonicalized; no ANSI; outside-repo `"git"` failure |
| branch.rs | 3 | branch text; JSON `data`; no ANSI; outside-repo failure |
| init_creates_file.rs | 6 | exact default serialization (`version = "0.1.0"\nshow_frontmatter = true`); success msg; readonly-dir `"io"`; `--config` beats hostile XDG; no env ⇒ `"config-dir"` |
| json_output.rs | 2 | `init -j` `"data"` envelope; text mode plain |
| model.rs | 12 | model/thinking resolution (incl. manifest-trigger step), JSON envelopes, no-artifact failure, unknown model ⇒ `"model"`, config-dir read; trims BELL `\x07` |
| prompt.rs | 8 | inferred vs explicit step; no-artifact failure; JSON data includes frontmatter; unknown prompt ⇒ `"prompt"`; config-dir read; frontmatter default-on and off via `show_frontmatter=false` |
| step.rs | 10 | current step; JSON; no-artifact failure; empty-trigger default step; config-dir read; CWD-config-ignored regression; missing-config stderr phrases (text + JSON error envelope) |

### Unit (`#[cfg(test)]` in src/)
| File:line | # | Covers |
|---|---|---|
| src/config.rs:101 | 12 | default serialization; Config/steps/prompts/models round-trips; missing collections ⇒ empty Vec; read_config from disk; `"io"` + resolution phrase; `"toml::de"` |
| src/config_dir.rs:117 | 8 | env resolution matrix (XDG/APPDATA/HOME); explicit passthrough; `"config-dir"`; Display phrases pinned |
| src/commands/mod.rs:320 | 43 | clap try_parse for every subcommand; dispatch arms via `"io"` on missing config; artifact_dir_path; determine_step (last-match, default, `"step"`); resolve_model `"model"`; resolve_prompt `"prompt"` |
| src/errors.rs:67 | 7 | Display format; From io/toml::ser/toml::de tags; JSON round-trip; PartialEq |
| src/format.rs:27 | 4 | color helpers plain under cfg!(test) |
| src/git.rs:40 | 5 | current_branch_in on temp repo; detached-HEAD; parse_branch_output edge cases |

### Harness conventions (duplicated per file — copy, don't import)
- Temp git repo, two variants: `git init -b main` unborn-branch (model.rs:7-15, step.rs:7-14, prompt.rs:7-15) or `git init` + `checkout -b` + commit with `-c user.name=t -c user.email=t@t` (artifact_directory.rs:8-33, branch.rs:8-33).
- `write_config(dir)`: hand-built TOML via `concat!` with `version = "0.1.0"` + `[[steps]]` (+ `[[prompts]]`/`[[models]]`); variants for hidden frontmatter (prompt.rs:27-85).
- `artifact_dir(dir) = dir/.pi/orksorksorks/<BRANCH>` (model.rs:46, step.rs:36, prompt.rs:18); expected paths canonicalized via `std::fs::canonicalize` on macOS (`/var` → `/private/var`, prompt.rs:279-285).
- Invocation: `.current_dir(...).args(...).env("XDG_CONFIG_HOME", ...).env_remove("HOME").output()` then `.assert().success()/.failure()` (e.g. init_creates_file.rs:112-124).
- JSON assertions: `serde_json::from_str(&stdout)` → `Value`, read `data` / `error.message` / `error.source` (step.rs:289-316, prompt.rs:155-178).
- Every output test asserts no `\x1b` ANSI; unit tests use `pretty_assertions::assert_eq`.

## Gotchas
- **Forbidden-string gate is case-insensitive and covers all `*.rs`** — any `TODO:`, `dbg!`, `FIXTURE:` string in new code/tests fails both test.sh:17-19 and _reusable-lint.yml:20-22.
- **CI checkout is detached HEAD** — `git branch --show-current` prints empty ⇒ `"git"` error "not on a branch (detached HEAD)" (git.rs:33-35). Any integration test requiring branch/step resolution must create its own temp repo (no reliance on checkout state).
- CI does not run `cargo build` or `cargo test` — compile gate is `cargo check` (lock + all-features); nextest drives tests.
- `cargo fmt --all` rewrites in place locally; CI demands clean diff (`--check`).
- Coverage thresholds only bite when `upload-coverage` is passed; ci-pr.yml never sets it, so they are not PR gates today (codecov.yml:2-12).
- JSON-mode errors go to **stdout** (envelope `{"error": …}`); text-mode errors go to **stderr** with two leading blank lines; exit code 1 either way (main.rs:27-50, :76-77).
- Error `Display` owns all ANSI coloring (`"Error from {yellow source}:\n{red message}"`, errors.rs:26-34); callers never color themselves — `cfg!(test)` strips color at the format.rs:6-11 chokepoint.
- macOS path canonicalization `/var` → `/private/var` affects expected-path assertions in tests (prompt.rs:279-285, artifact_directory.rs:38-44).
- Config path resolution order: explicit `--config` → XDG_CONFIG_HOME → %APPDATA% (Windows) → $HOME/.config → `"config-dir"` error (config_dir.rs:72-99).