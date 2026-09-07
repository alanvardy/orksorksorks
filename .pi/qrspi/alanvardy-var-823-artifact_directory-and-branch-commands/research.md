# Research Findings

Repo: `orksorksorks` — small Rust CLI crate, single binary, edition 2024, v0.1.0
(`Cargo.toml:1-4`). Binary-only: no `[lib]` target, so integration tests cannot
import crate types (`tests/init_creates_file.rs:19-20`). One subcommand exists
today (`init`); `src/main.rs` is the single binary entry.

## Q1: Command flow — clap parsing → Commands enum → handlers → CommandResult

### Findings
- **Entry**: `#[tokio::main] async fn main()` at `src/main.rs:91-94`. It calls
  `commands::Cli::parse()` (`src/main.rs:93`) then `run_command(cli)`
  (`src/main.rs:94`). Module wiring: `mod commands; mod config; mod errors;
  mod format;` (`src/main.rs:6-9`).
- **Cli struct**: `src/commands/mod.rs:19-34`. `#[derive(Parser, Clone)]`
  (clap 4.6.6 with `derive` feature, `Cargo.toml:11`). Command metadata comes
  from build-time env consts: `NAME`/`AUTHOR`/`ABOUT`/`LONG_VERSION`
  (`src/commands/mod.rs:5-14`); `arg_required_else_help = true`
  (`src/commands/mod.rs:27`, clap `Validator` renders help to **stderr**, exit 2).
- **Two Cli fields**: `json: bool` with `#[arg(short = 'j', long, global =
  true, default_value_t = false)]` (`src/commands/mod.rs:29-31`); and
  `command: Commands` with `#[command(subcommand)]` (`src/commands/mod.rs:32-34`).
- **Commands enum**: single-variant `pub enum Commands { Init }`
  (`src/commands/mod.rs:39-42`), `#[derive(Subcommand, Debug, Clone)]`.
  clap registration: subcommand name is the lowercased variant name (`init`).
- **Dispatch**: `pub fn select_command(cli: &Cli) -> Result<String, Error>`
  (`src/commands/mod.rs:45-50`) — a `match &cli.command` over the enum variants.
- **Handler contract** (derived from `select_command` + `run_command`):
  - Zero-argument `fn xxx_command() -> Result<String, Error>` (the `&Cli` routing
    is done by the caller). Example: `init_command` at `src/commands/mod.rs:52-62`.
  - Success = `Ok(String)` — that string is the entire text payload
    (`src/commands/mod.rs:61`); it is printed verbatim by the output layer.
  - Failure = `Err(Error)` — built via `Error::new(source, message)`
    (`src/errors.rs:18-23`), or via `From<std::io::Error>` (`src/errors.rs:39-46`)
    and `From<toml::ser::Error>` (`src/errors.rs:48-55`), which tag
    `source` as `"io"` / `"toml::ser"`.
  - Handlers never print and never exit themselves — the output layer owns both.
- **CommandResult envelope**: `src/main.rs:17-26` — `result: Result<String,
  Error>`, `bell_success: bool`, `bell_failure: bool`, `json: bool`.
- **Assembly**: `run_command` (`src/main.rs:68-89`): reads `cli.json`
  (`src/main.rs:69`), calls `select_command` (`src/main.rs:70`), wraps in
  `CommandResult` (`src/main.rs:71-83`), calls `output_result(&cr)`
  (`src/main.rs:85`), then `std::process::exit(1)` iff error (`src/main.rs:86-88`).
- **Output dispatch**: `output_result` (`src/main.rs:59-66`) routes on
  `cr.json` to either `output_text` or `output_json`.
- **Text mode**: `output_text` (`src/main.rs:29-42`) — `Ok` → `println!("{data}")`
  to stdout + bell `print!("\x07")` (`src/main.rs:31-34`); `Err` →
  `eprintln!("\n\n{e}")` to **stderr** + bell (`src/main.rs:36-39`).
- **JSON mode**: `output_json` (`src/main.rs:45-56`) — `Ok` →
  `serde_json::json!({"data": data})` to stdout (`src/main.rs:47-49`); `Err` →
  `{"error": {"message": e.message, "source": e.source}}` to **stdout**
  (`src/main.rs:51-53`). Note: JSON errors print to stdout, text errors to stderr.
- **Exit codes**: handler `Ok` → 0 (falls off end of `run_command`); handler
  `Err` → 1 (`src/main.rs:86-88`); clap usage errors (`arg_required_else_help`,
  unknown subcommand) → 2 from clap's `Error::exit()`. `--help`/`--version`
  handled by clap before dispatch, exit 0.
- **Bells**: `bell_success`/`bell_failure` fields are set (`src/main.rs:72-83`)
  but never read anywhere — actual bell emission is hardcoded inside
  `output_text` only (`src/main.rs:34,38-39`). JSON path has no bell.
- **Handler-command wiring is manual**: adding a command means a new enum
  variant (`src/commands/mod.rs:39-42`), a match arm (`src/commands/mod.rs:45-50`),
  and a handler fn of the signature above.
- **Unit-test seam**: handlers are tested via the same `&Cli` construction —
  `select_command_routes_init` (`src/commands/mod.rs:70-78`) and clap parse
  tests (`src/commands/mod.rs:80-99`).

## Q2: Working directory, environment variables, filesystem operations

### Findings
- **The only production filesystem write** is `init_command` at
  `src/commands/mod.rs:52-62`: `std::fs::File::create("orksorksorks.toml")`
  (`src/commands/mod.rs:57`) — the **only path literal in `src/`**, a bare
  relative path resolved against the process CWD. Write pipeline:
  `write_all` → `flush` → `sync_all` (`src/commands/mod.rs:58-60`).
- Runtime code never reads CWD, never chdirs, never composes absolute paths,
  has no home-dir lookup, no directory enumeration, no config-file *read*.
- **Environment** is build-time-only: `build.rs:1-16` emits
  `cargo:rustc-env=BUILD_TARGET` (from `env::var("TARGET")`, `build.rs:6`),
  `BUILD_PROFILE` (from `env::var("PROFILE")`, `build.rs:10`), and
  `BUILD_TIMESTAMP` (synthesized RFC3339, `build.rs:12-14`). Consumed as
  compile-time `env!("BUILD_*")` consts in `src/commands/mod.rs:5-14`.
  **No runtime env reads exist** (`std::env` never used in `src/`; no
  `HOME`, `GITHUB_*`, or `getenv`-style access).
- **CWD-relative testing convention** — `tests/init_creates_file.rs`, all four
  tests: `tempfile::tempdir()` + `Command::cargo_bin("orksorksorks")` +
  `cmd.current_dir(temp.path())`:
  - `init_creates_orksorksorks_toml_with_default_content`
    (`tests/init_creates_file.rs:11-26`): asserts the file appears at
    `temp.path().join("orksorksorks.toml")` (the child's CWD) with exact bytes
    `version = "0.1.0"\n` (`tests/init_creates_file.rs:24`).
  - `init_returns_success_message` (`tests/init_creates_file.rs:29-38`): same
    shape, asserts stdout contains `"✓ Created orksorksorks.toml"`.
  - `current_dir` call sites: `tests/init_creates_file.rs:14,32,48,66`.
  - `tests/json_output.rs` tests call neither `current_dir` nor `tempdir`; the
    child inherits the runner CWD and only asserts stdout/JSON shape.
- **Unwritable target**: tested via read-only tempdir — flip permissions with
  `fs::metadata(...).permissions().set_readonly(true)` + `fs::set_permissions`
  (`tests/init_creates_file.rs:41-56`), then restore afterward so `tempfile`
  can clean up (`tests/init_creates_file.rs:54-56`, `:77-84`; file-level lint
  allow at `tests/init_creates_file.rs:4`).
- **Failure path in production**: `File::create` fails →
  `From<std::io::Error>` tags `source = "io"` (`src/errors.rs:39-46`) → text:
  error to stderr + bell, exit 1 (`src/main.rs:36-39,86-88`); JSON mode: error
  envelope to stdout (`src/main.rs:51-53`). Asserted by
  `init_json_error_in_readonly_dir_shows_source_io` (`tests/init_creates_file.rs:59-84`).
  No retry, no fallback location, no pre-write writability probe.
- **fs:: usage sweep**: production `src/commands/mod.rs:57`; tests
  `src/../tests/init_creates_file.rs:8` (import), `:21` (read), `:43,53,61,75`
  (metadata), `:45,55,63,77` (set_permissions). Nothing in `src/config.rs`,
  `src/errors.rs`, `src/format.rs`, `src/main.rs`.

## Q3: Git branch representation and referencing across the repo

### Findings
- **QRSPI tree is keyed one-directory-per-branch**: `.pi/qrspi/` contains
  exactly `alanvardy-var-816-init/`, `alanvardy-var-817-copy-over-scripts-and-test-infrastructure/`,
  and the current `alanvardy-var-823-artifact_directory-and-branch-commands/`
  (which currently holds only `questions.md` + `task.md`).
- **Branch naming pattern**: `alanvardy-var-<ticket>-<slug>`, e.g.
  `alanvardy-var-823-artifact_directory-and-branch-commands` (slug mixes `_`
  and `-`). The same string keys: the git ref, the worktree gitdir name, the
  worktree root dir, and the `.pi/qrspi/<branch>/` docs dir.
- **Git worktree setup**: repo-root `.git` is a gitfile —
  `gitdir: /Users/vardy/dev/orksorksorks/.git/worktrees/alanvardy-var-823-artifact_directory-and-branch-commands`
  (`.git:1`). The worktree gitdir's `HEAD` = `ref: refs/heads/<branch>`;
  `commondir: ../..` back-references `/Users/vardy/dev/orksorksorks/.git`.
  Main repo `.git/config` has `[branch "<name>"]` sections with
  `remote = origin` / `merge = refs/heads/<branch>`.
- **Branch references in CI**: `.github/workflows/ci-pr.yml:5` — the **only**
  branch reference in all of CI (`branches: [main]` on the pull_request
  trigger). Reusable workflows `_reusable-lint.yml` and `_reusable-test.yml`
  contain no ref/env-var logic. **Negative finding**: zero matches for
  `GITHUB_REF|GITHUB_HEAD_REF|GITHUB_BASE_REF|GITHUB_REF_NAME|GITHUB_SHA`
  anywhere — CI consumes no branch-name string.
- **Branch references in scripts**: none. `scripts/test.sh:1-21` has no `git`,
  branch, or worktree logic.
- **Branch-in-path convention is documented**: `questions.md:13` states the
  `.pi/qrspi/<branch>/` convention explicitly; sibling artifacts reference it
  (`.pi/qrspi/alanvardy-var-817-.../research.md:75-77`); `.pi/qrspi/alanvardy-var-816-init/plan.md:420-423`
  (planned but not shipped `cyan_string`).
- **How a branch-name string could be sourced today** (verified working):
  - `.git` files: gitfile `gitdir:` final path component (`.git:1`); the
    worktree gitdir `HEAD` (`ref: refs/heads/<branch>`); `[branch "<name>"]`
    sections in the main `.git/config`.
  - Git command output: `git branch --show-current`, `git rev-parse
    --abbrev-ref HEAD`, `git symbolic-ref --short HEAD`.
  - Env vars: **no convention exists** — no CI/script reads a branch env var.
- **src/ and tests/ never mention git/branch**: zero matches for `git`,
  `branch`, `.pi`, `HEAD`, `refs`, `GITHUB` in `src/` or `tests/`.
- **Task-scoped artifact-dir convention** (from `task.md:5-6`, the only place
  it exists today): the planned `artifact_directory` output has the form
  `$PWD/.pi/orksorksorks/<branch>/` — the branch name is the final path
  segment, mirroring `.pi/qrspi/<branch>/`.
- **Tracking**: `.gitignore:1` contains only `/target`; all 17 QRSPI files are
  git-tracked (`git ls-files .pi`).

## Q4: Dependencies, build injection, and test gates

### Findings
- **`[dependencies]`** (`Cargo.toml:10-17`): `clap 4.6.6` (features: derive),
  `colored 3.1.1`, `serde` 1.0.229 (derive), `serde_json 1.0.151`,
  `tempfile 3.27.0` (a *regular* dependency, used by tests), `tokio 1.53.1`
  (features: full), `toml 1.1.5`.
- **`[dev-dependencies]`** (`Cargo.toml:19-22`): `assert_cmd 2.2.2`,
  `predicates 3.1.4`, `pretty_assertions 1.4.1` (used only in unit tests,
  e.g. `src/config.rs:1-47`, `src/errors.rs:53-101`).
- **`[build-dependencies]`** (`Cargo.toml:24-25`): `chrono 0.4.45` (used by
  `build.rs:12-14`).
- **`build.rs`** (`build.rs:1-16`): the only build script. Emits three
  `cargo:rustc-env` vars — `BUILD_TARGET`, `BUILD_PROFILE`,
  `BUILD_TIMESTAMP` — consumed as `env!` consts in
  `src/commands/mod.rs:5-16` for the clap long-version string
  (`src/commands/mod.rs:13-16`). Nothing else is injected.
- **Toolchain pin**: `rust-toolchain.toml:1` — `channel = "1.98.1"`,
  `components = ["clippy", "rustfmt"]`.
- **`scripts/test.sh` order** (`scripts/test.sh:1-21`, `set -euo pipefail`
  at `:2`): 1) `cargo fmt --all` (`:5`); 2) `cargo check` (`:8`);
  3) `cargo clippy --tests -- -D warnings` (`:11`); 4)
  `cargo nextest run --no-tests pass` (`:14`); 5) forbidden-strings gate
  `rg -i -g '*.rs' 'TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:' .` → exit 1
  (`:17-19`); 6) `echo "=== SUCCESS ==="` (`:21`).
- **nextest config** (`.config/nextest.toml`): `[profile.ci]` —
  `retries = 2`, `fail-fast = false`, `slow-timeout = { period = "60s" }`
  (`:1-4`); `[profile.ci.junit]` — `path = "target/nextest/ci/junit.xml"`,
  `store-success-output = false`, `store-failure-output = true` (`:6-9`).
  **No `serial` markers, no OS gating, no `filter`/`platform`/`slow`
  settings anywhere.** `test.sh` runs the default profile; only CI passes
  `--profile ci`.
- **CI reusable workflows**: `ci-pr.yml:8-11` calls `_reusable-lint.yml` and
  `_reusable-test.yml` with no `with:`/`secrets:`. `_reusable-lint.yml`
  (single `ubuntu-latest` job): `cargo check --locked --all-features`;
  `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features
  --locked -- -D warnings`; the same forbidden-strings rg gate. `_reusable-test.yml`
  (input `upload-coverage`, default false): `cargo nextest run --profile ci
  --all-features --no-tests pass`; coverage steps gated on
  `inputs.upload-coverage` (never true via `ci-pr.yml` — coverage/JUnit
  uploads only fire when a caller sets the input).
- **Integration-test conventions** (`tests/`): two files, binary-invocation
  style via `assert_cmd::Command::cargo_bin("orksorksorks")`; crate is
  binary-only so no cross-import. `tests/init_creates_file.rs` (4 tests:
  content, message, readonly exit, readonly JSON source `"io"`),
  `tests/json_output.rs` (2 tests: JSON data field; plain text with no ANSI).
  Unit tests live inline: `src/config.rs:21-41`, `src/commands/mod.rs:64-99`,
  `src/errors.rs:53-101`, `src/format.rs:26-55`.

## Q5: String formatting and surfacing

### Findings
- **Color chokepoint**: `apply_color` at `src/format.rs:6-12` — the single
  ANSI control point. `if cfg!(test) { s.to_string() } else {
  s.color(color).to_string() }` (`src/format.rs:7-11`): under the compile-time
  test flag the string is returned unchanged; otherwise `colored`'s
  `Colorize::color()` applies ANSI.
- **Three public helpers** — all thin wrappers: `green_string`
  (`src/format.rs:14-16`), `red_string` (`src/format.rs:18-20`),
  `yellow_string` (`src/format.rs:22-24`). Only Green/Red/Yellow are used in
  `src/`. (A `cyan_string` was planned at `.pi/qrspi/alanvardy-var-816-init/plan.md:420-423`
  but never shipped.)
- **`cfg!(test)` is the testability seam for strings**: handlers may apply
  color directly to their return value (e.g. `init_command` returns
  `Ok(crate::format::green_string("✓ Created orksorksorks.toml"))`,
  `src/commands/mod.rs:61`), and tests see plain text because `cfg!(test)`
  strips ANSI.
- **Absence-of-ANSI assertions**:
  - Unit: `src/format.rs:31-54` — `green_string_is_plain`,
    `red_string_is_plain`, `yellow_string_is_plain`, and
    `all_helpers_strip_ansi_under_test` (asserts no `\x1b` byte for
    `["hello", "✓", "error text"]`).
  - Integration: `init_no_json_prints_plain_text` asserts
    `!stdout.contains('\x1b')` (`tests/json_output.rs:35-41`, text mode).
  - Related: `select_command_routes_init` (`src/commands/mod.rs:74-78`)
    asserts the exact plain string; `display_includes_source_and_message`
    (`src/errors.rs:57-63`) asserts plain substrings.
- **Error struct**: `src/errors.rs:9-13` — `message: String`, `source:
  String` (lowercase tag, `src/errors.rs:6-7`); `Serialize` derive
  (`src/errors.rs:9`) enables the JSON envelope. `Display` owns all coloring —
  callers must not pre-apply ANSI (`src/errors.rs:7-8`).
- **Text rendering**: `impl fmt::Display for Error`
  (`src/errors.rs:26-33`): `write!(f, "Error from {}:\n{}",
  yellow_string(&self.source), red_string(&self.message))` — so text form is
  `Error from <yellow source>:\n<red message>`.
- **JSON rendering bypasses Display**: `output_json` serializes the raw
  fields (`src/main.rs:51-53`), producing `{"error":{"message":…,"source":…}}`
  on stdout with no ANSI and no `Error from` prefix.
- **End-to-end trace** (`init`): handler returns
  `Ok(green_string("✓ Created orksorksorks.toml"))` (`src/commands/mod.rs:61`)
  → `select_command` (`src/commands/mod.rs:45-50`) → `CommandResult`
  (`src/main.rs:71-83`) → `output_result` (`src/main.rs:59-66`) → text:
  `println!("{data}")` + bell to stdout (`src/main.rs:31-34`); JSON:
  `{"data": ...}` to stdout (`src/main.rs:47-49`). Error text goes to stderr
  via Display, JSON error to stdout, both exit 1.

## Cross-Cutting Observations
- **One binary, one handler pattern**: every command is a zero-arg
  `Result<String, Error>`-returning fn wired through the `Commands` enum match;
  the output layer (`output_text`/`output_json`/`output_result`) is fully
  generic over that contract.
- **`cfg!(test)` is the single testability seam for both color and (for the
  unit tests) anything else that needs to appear plain.** The integration
  tests spawn the real binary and rely on the same flag via the build profile.
- **JSON mode changes stream semantics**: success data and error envelopes
  both go to stdout in JSON mode; only text mode uses stderr for errors.
- **Bell flags are dead weight**: `bell_success`/`bell_failure` are set but
  never read; the bell lives hardcoded in `output_text`.
- **CWD is a runtime black box**: the binary resolves its only path literal
  against CWD without ever reading CWD; tests pin the behavior with
  `assert_cmd current_dir` + `tempfile::tempdir`.
- **Branch identity is a repo-operational concern, not a runtime one**: the
  branch name lives in git metadata/worktree paths/docs dirs; the binary has
  zero branch awareness today.
- **The gate is small but strict**: fmt → check → clippy → nextest (default
  profile, `--no-tests pass`) → forbidden-strings rg. CI inlines the same
  steps with `--locked --all-features` and the `ci` profile.

## Open Areas
- **No web/network behavior** was found; nothing in `src/` touches the
  network, so any branch/artifact source would be entirely new surface.
- **No env-var convention for branch sourcing exists** — the only working
  sources today are git commands and `.git` files (verified by the Q3 agent).
- **`pretty_assertions` is the sole consumer of a non-assert_cmd dev-dep** —
  `src/config.rs:4` and `src/errors.rs`, `src/format.rs` unit tests use it;
  the integration tests use std assertions and `predicates`.
- **Coverage/JUnit upload paths exist in `_reusable-test.yml` but are never
  triggered** by `ci-pr.yml` (the `upload-coverage` input stays default false).
- **Question 2's "not writable" case is IO-error-only** (read-only dir); no
  code or test covers full-disk, permission-denied at a deeper level, or an
  existing-unwritable-file collision — the behavior would still funnel through
  the `From<std::io::Error>` → `"io"` path.