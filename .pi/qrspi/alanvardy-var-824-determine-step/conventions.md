# Conventions — shared factual appendix

`orksorksorks` — binary-only Rust CLI crate, edition 2024 (`Cargo.toml:4`), toolchain pinned in `rust-toolchain.toml` (channel `1.98.1`). No lib target (noted `tests/init_creates_file.rs:20-21`). Modules: `main`, `commands`, `config`, `errors`, `format`.

## Canonical commands

### Local gate — `scripts/test.sh` (bash, `set -euo pipefail`)
1. `cargo fmt --all` — `scripts/test.sh:5`
2. `cargo check` — `scripts/test.sh:8`
3. `cargo clippy --tests -- -D warnings` — `scripts/test.sh:11`
4. `cargo nextest run --no-tests pass` — `scripts/test.sh:14` (runs both tiers; `--no-tests pass` means no failure if a profile has zero tests)
5. Forbidden-string gate — `rg -i -g '*.rs' 'TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:'` exits 1 on any hit — `scripts/test.sh:17-19`
6. Prints `=== SUCCESS ===` (`scripts/test.sh:21`); `set -euo pipefail` means any earlier failure aborts.

### Nextest config — `.config/nextest.toml`
- `profile.ci`: `retries = 2`, `fail-fast = false`, `slow-timeout = { period = "60s" }` (lines 1-4)
- `profile.ci.junit`: `path = "target/nextest/ci/junit.xml"`, `store-success-output = false`, `store-failure-output = true` (lines 6-9)

### CI — `.github/workflows/`
- `ci-pr.yml` — entry point; triggers on `pull_request` to `main` (lines 4-5); jobs `lint` → `_reusable-lint.yml`, `test` → `_reusable-test.yml` (lines 8-11).
- `_reusable-lint.yml` — `actions/checkout@v4` (L10), `dtolnay/rust-toolchain@stable` with `clippy, rustfmt` (L11-13), `cargo check --locked --all-features` (L15), `cargo fmt --all -- --check` (L17), `cargo clippy --all-targets --all-features --locked -- -D warnings` (L19), same forbidden-strings gate (L21-22).
- `_reusable-test.yml` — `workflow_call` input `upload-coverage: bool = false` (L3-9); `taiki-e/install-action@v2` for `cargo-nextest,cargo-llvm-cov` (L19-21); `cargo nextest run --profile ci --all-features --no-tests pass` (L23); coverage step gated on `upload-coverage`: `cargo llvm-cov nextest --profile ci --all-features --lcov --output-path lcov.info` (L26) → `codecov/codecov-action@v5` (L29-31) → `codecov/test-results-action@v1` with `target/nextest/ci/junit.xml` (L34-36).
- Coverage policy — `codecov.yml`: project target auto/10% threshold, patch target 50%; `src/main.rs` ignored.

### Key differences local vs CI
- CI adds `--locked` and `--all-features`/`--all-targets` (CI above); local gate does not.
- local `cargo clippy --tests`; CI `cargo clippy --all-targets --all-features --locked`.
- local uses default nextest profile; CI uses `--profile ci` (retries 2, junit output).

## Test-suite inventory

### Tier 1 — inline units in `src/` (33 tests, run in-process; color stripped under `cfg!(test)`)

| File | Tests | What they cover |
|---|---|---|
| `src/commands/mod.rs:124-280` | 21 | routing (`select_command` matches), clap parsing via `Cli::try_parse_from` (rejects no-subcommand, kebab-case `artifact-directory`), `artifact_dir_path` pure-function contract (trailing slash, no-ANSI, `feature/x`→`feature-x`, `a/b/c`, plain), `current_branch`, `parse_branch_output` (trim, detached-HEAD `source:"git"`, non-zero-exit stderr), handler plainness |
| `src/config.rs:22-41` | 2 | default serializes to exact TOML string; `toml::from_str` round-trip (the only deserialize in the crate) |
| `src/errors.rs:57-116` | 6 | `Display` format, `From<io::Error>`→`"io"`, `From<toml::ser::Error>`→`"toml::ser"` (via forced-fail serializer), serde JSON round-trip, PartialEq, dyn `std::error::Error` |
| `src/format.rs:27-52` | 4 | `green/red/yellow_string` return plain strings under test |

### Tier 2 — integration in `tests/` (12 tests, spawn the real binary via `assert_cmd::Command::cargo_bin("orksorksorks")`; dev-deps `Cargo.toml:20-22`)

| File | Tests | What they cover |
|---|---|---|
| `tests/init_creates_file.rs` (78 lines) | 4 (L11, 29, 41, 59) | happy file creation + exact bytes `"version = \"0.1.0\"\n"` (L22-25); success message substring (L34-36); read-only-dir failure + restore-with-cleanup pattern (L42-55); read-only JSON error with `"source":"io"` (L59-77). File-level `#![allow(clippy::permissions_set_readonly_false)]` (L4) |
| `tests/json_output.rs` (36 lines) | 2 (L4, 21) | `init -j` valid JSON with `"data"` field; plain-text mode has no ANSI |
| `tests/branch.rs` (44 lines) | 3 (L13, 25, 39) | plain branch output vs runtime-derived `git branch --show-current` (helper L3-10), `-j` JSON, outside-repo failure (L40-43) |
| `tests/artifact_directory.rs` (49 lines) | 3 (L18, 30, 44) | path output vs independent `expected_artifact_directory()` re-implementation (L3-14), `-j` JSON, outside-repo failure (L45-48) |

### Assertion conventions
- **Stdout text**: `.assert().success()` + either substring (`predicates::str::contains`, `tests/init_creates_file.rs:34-36`) or exact equality after **trimming exactly one trailing bell then whitespace**: `stdout.trim_end_matches('\x07').trim_end()` (`tests/branch.rs:20`, `tests/artifact_directory.rs:25`).
- **No-ANSI is asserted everywhere**: `assert!(!stdout.contains('\x1b'), ...)` (`tests/branch.rs:21`, `tests/artifact_directory.rs:26`, `tests/json_output.rs:33`).
- **JSON**: parse with `serde_json::from_str(&stdout).unwrap()` and inspect `v["data"]` (`tests/branch.rs:32-33`, `tests/artifact_directory.rs:37-38`) or raw-substring + parse-validity (`tests/json_output.rs:17-18`). Envelope shapes defined in `src/main.rs:48-53`.
- **Errors**: `.assert().failure()` for sad paths; exit code 1 (`src/main.rs:86-87`); text errors are `Error from <source>:\n<message>` on stderr (`src/main.rs:37-40`); JSON errors are stdout `{"error":{...}}` and asserted by substring (`tests/init_creates_file.rs:76`).
- **Unit-test color**: `apply_color` strips ANSI when `cfg!(test)` is true (`src/format.rs:6-12`), so units assert plain strings (`src/commands/mod.rs:134-136`).
- **Expected values** are hardcoded inline (exact TOML bytes) or derived at runtime (git branch, cwd composition) — **no fixtures, no snapshot/golden files anywhere** (rg confirms; only hits are the forbidden-string regexes `scripts/test.sh:17`, `_reusable-lint.yml:22`).

## Build / verify gotchas

- **`cfg!(test)` color stripping** (`src/format.rs:6-12`) — production uses ANSI; every unit test asserting output text relies on this, and every integration no-ANSI assertion relies on production emitting color that tests reject.
- **Bell is part of text-mode stdout** (`\x07` on success stdout, `src/main.rs:34`; and on error stderr, L39) — integration comparisons must strip it first (`trim_end_matches('\x07')`).
- **`set_readonly` tests must restore permissions** (`set_readonly(false)`) before the tempdir is dropped or `tempfile` cleanup fails (`tests/init_creates_file.rs:54, 76`); the file-level clippy allow exists for this (`tests/init_creates_file.rs:4`).
- **`tempfile::tempdir()` is a runtime dependency** (`Cargo.toml:15`) used only by tests — do not read its absence from `[dev-dependencies]` as a signal.
- **One production `current_dir()`/one `git` spawn pattern**: tests that re-derive expectations must run in the crate root (a git worktree) — the outside-repo tests deliberately use fresh tempdirs.
- **`--locked` only in CI**: local `test.sh` runs unlocked; a lockfile change can pass locally yet fail CI lint (`_reusable-lint.yml:15,19`).
- **CI installs nextest/llvm-cov via `install-action`** — local machines may lack `cargo nextest`/`cargo llvm-cov`; `test.sh` assumes nextest is already installed.
- **Forbidden-strings gate runs twice** (local `scripts/test.sh:17`, CI `_reusable-lint.yml:21-22`) — `FIXTURE:` is in the deny list, so even a comment about fixtures trips it.
- **`rust-toolchain.toml` pins 1.98.1 with clippy+rustfmt**; edition 2024 std `Result` (`Ok`/`Err`) + `From` conversions are the idiomatic error path — `?` requires a `From` impl (only `io`, `toml::ser` exist).