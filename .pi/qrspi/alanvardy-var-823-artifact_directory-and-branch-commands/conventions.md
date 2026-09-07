# Conventions — shared factual appendix

Source-of-truth files: `Cargo.toml`, `build.rs`, `scripts/test.sh`, `.config/nextest.toml`,
`rust-toolchain.toml`, `.github/workflows/{ci-pr,_reusable-lint,_reusable-test}.yml`,
`src/{main,commands/mod,errors,format,config}.rs`, `tests/{init_creates_file,json_output}.rs`,
`.gitignore`.

## Canonical commands

**Local developer gate — the only project gate; no Makefile exists:**
```
./scripts/test.sh
```
Step order (`scripts/test.sh:1-21`, `set -euo pipefail` at `:2`):
| Step | Command | Ref |
|---|---|---|
| Format | `cargo fmt --all` | scripts/test.sh:5 |
| Check | `cargo check` | scripts/test.sh:8 |
| Clippy | `cargo clippy --tests -- -D warnings` | scripts/test.sh:11 |
| Tests | `cargo nextest run --no-tests pass` (default profile; `--no-tests` so empty selections never fail) | scripts/test.sh:14 |
| Forbidden strings | `rg -i -g '*.rs' 'TODO:\|todo:\|FIXME\|fixme\|dbg!\|DEBUG:\|FIXTURE:' .` → exit 1 on match | scripts/test.sh:17-19 |
| Success | `echo "=== SUCCESS ==="` | scripts/test.sh:21 |

**CI equivalents** (`.github/workflows/` — CI does **not** run test.sh; steps are inlined):
- `cargo nextest run --profile ci --all-features --no-tests pass` — `_reusable-test.yml:20` (the only `--profile ci` invocation)
- `cargo llvm-cov nextest --profile ci --all-features --lcov --output-path lcov.info` — `_reusable-test.yml:23` (coverage run)
- `cargo check --locked --all-features` — `_reusable-lint.yml:15`
- `cargo fmt --all -- --check` — `_reusable-lint.yml:17`
- `cargo clippy --all-targets --all-features --locked -- -D warnings` — `_reusable-lint.yml:19`
- Toolchain: `dtolnay/rust-toolchain@stable` with `components: clippy, rustfmt` — `_reusable-lint.yml:11-13`, `_reusable-test.yml:11-13`; nextest/coverage tools via `taiki-e/install-action@v2` (`tool: cargo-nextest,cargo-llvm-cov`) — `_reusable-test.yml:14-18`
- PR CI (`ci-pr.yml:8-11`) calls both reusables with **no `with:`** → `upload-coverage` stays default `false` (`_reusable-test.yml:6-9`); coverage + JUnit uploads (`_reusable-test.yml:22-31`) never fire on PR CI.

**Toolchain pin**: `rust-toolchain.toml:1` — `channel = "1.98.1"`,
`components = ["clippy", "rustfmt"]`.

## Test-suite inventory

Platform gating: **none**. No `serial` markers, no `#[cfg(os...)]`, no
`filter`/`platform`/`slow` in `.config/nextest.toml` or `tests/`. CI runs
`ubuntu-latest`; local runs on the dev machine (macOS).

| File | Tests | Coverage | Ref |
|---|---|---|---|
| `src/config.rs` | 2 unit: default TOML serialization; round-trip serialize→deserialize | `Config { version: String }`, `Default` → `"0.1.0"` | src/config.rs:21-41 |
| `src/commands/mod.rs` | 4 unit: select_command routes init; try_parse rejects no-subcommand; accepts init; `Cli::command().debug_assert()` | clap wiring + handler routing | src/commands/mod.rs:64-99 |
| `src/errors.rs` | 5 unit: Display text; From<io::Error> tag; From<toml::ser::Error> tag; serialize round-trip; PartialEq; Error-as-std::error | error contract | src/errors.rs:53-101 |
| `src/format.rs` | 4 unit: green/red/yellow plain under cfg!(test); loop asserting no `\x1b` | `apply_color` chokepoint | src/format.rs:26-55 |
| `tests/init_creates_file.rs` | 4 integration (spawns binary): creates file w/ exact bytes `version = "0.1.0"\n`; success message; readonly-dir → failure; readonly-dir + `-j` → JSON `"source":"io"` | CWD-relative write + IO error path | tests/init_creates_file.rs:7-84 |
| `tests/json_output.rs` | 2 integration: `init -j` → valid JSON with `"data"`; `init` → plain stdout with no `\x1b` | output envelopes | tests/json_output.rs:4-42 |

Integration harness convention (all 6): `assert_cmd::Command::cargo_bin("orksorksorks")`.
CWD isolation (init_creates_file only): `tempfile::tempdir()` + `cmd.current_dir(temp.path())`;
read-only variants restore permissions afterward so tempdir cleanup works
(`tests/init_creates_file.rs:54-56,77-84`; lint allow at `:4`). The binary-only
crate (no `[lib]`) forces byte-level string assertions instead of type imports
(`tests/init_creates_file.rs:19-20`).

`pretty_assertions::assert_eq` is the assertion style in unit tests
(`src/config.rs:4`, `src/errors.rs:53`, `src/format.rs:27`); integration tests
use `predicates::str::contains` (`tests/init_creates_file.rs:37`) and std asserts.
`tempfile` is a **regular** dependency (`Cargo.toml:15`), not dev.

## Build / verify gotchas (verified)

- **`cfg!(test)` is the ANSI off-switch**: `apply_color` returns the input
  unchanged when `cfg!(test)` is set (`src/format.rs:7-11`). Unit tests that
  compare formatted strings (`src/commands/mod.rs:74-78`,
  `src/errors.rs:57-63`, `src/format.rs:31-54`) and integration tests that
  assert no `\x1b` (`tests/json_output.rs:35-41`) all depend on it. Removing
  or renaming `cfg!(test)` breaks the plain-text assertions.
- **JSON error output goes to stdout, text error output to stderr**: `output_json`
  prints both envelopes via `println!` (`src/main.rs:47-53`); `output_text`
  prints errors via `eprintln!` (`src/main.rs:36-39`). Exit code 1 is set
  centrally in `run_command` (`src/main.rs:86-88`) for both modes.
- **Bell flags are inert**: `bell_success`/`bell_failure` are populated but never
  read; the bell is hardcoded in `output_text` only (`src/main.rs:34,38-39`).
- **`Error.source` tags are lowercase tokens** (`"io"`, `"toml::ser"`,
  `src/errors.rs:39-55`) and are asserted verbatim in JSON (`tests/init_creates_file.rs:72`).
  `Display` must own all coloring — callers must not pre-apply ANSI to
  message/source (`src/errors.rs:7-8`).
- **A handler's success `String` is printed verbatim** — pre-colored strings are
  the norm (`src/commands/mod.rs:61`); JSON mode embeds whatever the handler
  returned into `{"data": ...}` (`src/main.rs:47-49`) — an ANSI-wrapped string
  would leak escapes into JSON.
- **Build-time envs are required**: `build.rs` calls `env::var("TARGET").unwrap()`
  and `env::var("PROFILE").unwrap()` (`build.rs:6,10`) — cargo provides both.
  `BUILD_TARGET`/`BUILD_PROFILE`/`BUILD_TIMESTAMP` are compile-time consts only
  (`src/commands/mod.rs:5-16`); the binary reads **no** runtime env vars.
- **The rg gate pattern is duplicated**: identical regex in `scripts/test.sh:17-19`
  and `_reusable-lint.yml:21-22`; both exit/fail non-zero on any match.
- **`--no-tests pass`** is required on every nextest invocation (`scripts/test.sh:14`,
  `_reusable-test.yml:20`) so an empty test selection cannot fail the gate.
  The `ci` profile (`retries = 2`, `fail-fast = false`, `slow-timeout 60s`)
  exists at `.config/nextest.toml:1-4`; JUnit XML at
  `target/nextest/ci/junit.xml` (`.config/nextest.toml:7`) is consumed only by
  the (never-triggered) `codecov/test-results-action` (`_reusable-test.yml:29-31`).
- **`.gitignore` is just `/target`** (`.gitignore:1`): build artifacts like
  `lcov.info` / JUnit XML are not ignored.
- **Toolchain**: rustup resolves `rust-toolchain.toml` from the repo root; the
  pinned 1.98.1 channel must be installed for local `cargo` invocations.
- **QRSPI docs are committed** (`git ls-files .pi` → all branch dirs tracked);
  the `.pi/qrspi/<branch>/` directory name equals the git branch name, which is
  also the worktree gitdir name (`.git:1` gitfile) and the `HEAD` ref target.