# Design Discussion

## Current State

`orksorksorks` is a binary-only Rust CLI (edition 2024, toolchain `1.98.1`;
`Cargo.toml:4`, no `[lib]`). Three subcommands exist — `init`, `branch`,
`artifact_directory` — all zero-payload clap variants (`src/commands/mod.rs:39-49`)
dispatched by an exhaustive `select_command` match calling zero-arg handlers
(`src/commands/mod.rs:52-58`). Every handler is `fn ...() -> Result<String, Error>`;
success is the full plain-text payload, failure is the typed `Error`
(`src/errors.rs:9-13`). The output layer (`src/main.rs:29-65`) owns text/JSON/
bell/exit-code policy.

- `Config` is a single field `version: String` with a manual `impl Default`
  (`src/config.rs:8-19`). Serialization via `toml::to_string`; the only
  deserialization in the crate is a unit-test round-trip (`src/config.rs:38`).
  **No production code reads a config from disk** — `init` is write-only
  (`src/commands/mod.rs:61-71`).
- `init` writes exactly `version = "0.1.0"\n`, pinned by an integration test
  (`tests/init_creates_file.rs:22-25`).
- The artifact directory is *derived*, not stored: pure
  `artifact_dir_path(cwd, branch)` → `<cwd>/.pi/orksorksorks/<branch>/` with a
  literal trailing slash (`src/commands/mod.rs:78-84`); branch comes from a
  `git branch --show-current` subprocess (`src/commands/mod.rs:91-109`).
- No `Vec`/`Option`/array-of-tables serde shape exists anywhere (research Q1).
  No CLI value/positional arg exists — the only flag is global `-j`
  (`src/commands/mod.rs:30-31`).
- No production file-existence check exists (`exists`/`try_exists`/`metadata`,
  research Q4). Path joining is string formatting, never `Path::join`.
- Errors: `Error { message, source }`, one constructor `Error::new`
  (`src/errors.rs:18-23`), two `From` impls — `"io"` (`src/errors.rs:39-46`) and
  `"toml::ser"` (`src/errors.rs:48-55`) — plus a direct `"git"` tag. **No
  `From<toml::de::Error>` exists** (research Q5).

## Desired End State

A new `step` subcommand:

```
orksorksorks step [--config <path>]
```

- Reads its config from `<path>` (optional flag, default `orksorksorks.toml`,
  resolved against CWD).
- Deserializes `Config`, whose `steps: Vec<Step>` holds
  `{ name, trigger_artifact }` (TOML `[[steps]]` array-of-tables).
- Computes the derived artifact directory via the existing `artifact_dir_path` +
  `current_branch()`.
- Iterates `steps` in reverse and returns (plain string, no ANSI) the `name` of
  the first step whose `trigger_artifact` exists at the artifact directory.
- If none match → `Err(Error::new("step", ...))` → exit 1 (Q1 = A).

Verification:

- **Unit** (tier 1, `src/`): default `Config` still serializes to
  `version = "0.1.0"\n`; round-trip of a `[[steps]]` config; `From<toml::de::Error>`
  → `"toml::de"`; `select_command` routes `step`; clap parses `step` with and
  without `--config`.
- **Integration** (tier 2, spawn the real binary in a git-repo tempdir): config
  with multiple steps, one artifact present → correct `name`; none present →
  `.failure()`; `-j` → `{"data": "<name>"}` with no ANSI.
- `./scripts/test.sh` green (fmt → check → clippy → nextest → forbidden-strings;
  `scripts/test.sh:1-21`).

## Patterns to Follow

- **One handler, one match arm per command.** Add a `Step { config: PathBuf }`
  variant, a `select_command` arm, and a `fn step_command(config)` handler — the
  wiring already documented at `src/commands/mod.rs:39-62`.
- **Errors flow through the central `Error` type.** New failures are either
  `From<std::io::Error>` → `"io"` (file read, `try_exists`) or a new
  `From<toml::de::Error>` → `"toml::de"`, plus direct `Error::new("step", ...)`
  for the no-match case. Never return bare statuses or print/exit from handlers
  (contract at `src/main.rs:29-56`).
- **Plain payload, no ANSI.** The returned `name` becomes machine-consumable JSON
  `data`; keep it uncolored (contrast `init_command`'s colored message,
  `src/commands/mod.rs:61`; matches `branch`/`artifact_directory`).
- **String-based path composition.** Join the `trigger_artifact` onto the
  trailing-slash dir with `format!("{}{}", dir, name)`, matching the convention
  at `src/commands/mod.rs:79-83` (`artifact_dir_path` already emits a trailing
  slash).
- **Reuse the existing pure/impure split.** `artifact_dir_path` (pure) and
  `current_branch`/`parse_branch_output` (impure, git) are separated exactly so
  consumers like `step` can compose them without re-deriving git logic
  (`src/commands/mod.rs:78-109`).
- **Test conventions.** `pretty_assertions::assert_eq!`/`assert_ne!`; no fixtures/
  snapshots (hardcoded or runtime-derived expectations);
  `trim_end_matches('\x07')` before comparing stdout; `.assert().success()`/
  `.failure()`; no-ANSI asserted (`tests/artifact_directory.rs:25-26`).

Patterns to **NOT** follow:

- Do **not** put an `artifact_directory` field in `Config` — a second prod source
  of truth that can disagree with the derived path (Q2 = A).
- Do **not** add a `git2`/`gix` dependency, read `.git` files, or add async fs —
  the binary is synchronous std except the `#[tokio::main]` shell (research Q4).
- Do **not** serialize an empty `steps` array into fresh configs — that would break
  the pinned `init` bytes (`tests/init_creates_file.rs:22-25`).

## Design Decisions

1. **Command shape — optional `--config` flag, default `orksorksorks.toml`**
   (Q4 = B). `Commands::Step { #[arg(long, value_name = "CONFIG",
   default_value = "orksorksorks.toml")] config: PathBuf }`. First value-carrying
   variant; clap 4.6.6 typed `PathBuf` (`ValueParser::path_buf()`, no existence
   check) is available and unused today (research Q2). Default parallels `init`'s
   implicit filename.

2. **Config shape — `steps: Vec<Step>` with serde default + skip-empty**
   (Q3 = B). `Step { name: String, trigger_artifact: String }` in `src/config.rs`;
   field declared `#[serde(default, skip_serializing_if = "Vec::is_empty")]`. A
   fresh/default config serializes byte-identically to today
   (`version = "0.1.0"\n`), leaving `tests/init_creates_file.rs:22-25` untouched;
   configs that omit `steps` deserialize to an empty vec. Manual `impl Default`
   gains `steps: Vec::new()`.

3. **Artifact directory — derived, via `current_branch()`** (Q2 = A). `step`
   recomputes `<cwd>/.pi/orksorksorks/<branch>/` using the existing
   `artifact_dir_path` + `current_branch()`; no config schema change beyond
   `steps`. This makes `step` the third command to spawn `git`, consistent with
   `branch`/`artifact_directory`.

4. **Determine step — reverse iteration with `Path::try_exists()`**
   (Q1 = A, Q5 = B). `for step in config.steps.iter().rev()`; the first
   `try_exists() == Ok(true)` wins (the last-forward, i.e. furthest-advanced,
   step whose artifact is present). `try_exists` distinguishes permission-denied
   (`Err`) from absent (`Ok(false)`); `Err` rides `From<std::io::Error>` → `"io"`.

5. **No-match → error** (Q1 = A). `Err(Error::new("step", "<dir>: no trigger
   artifact matched"))`, exit 1. Loud failure matches the detached-HEAD precedent
   (`src/commands/mod.rs:106-107`) rather than a silent empty string.

6. **New error tags — `"toml::de"` and `"step"`.** Add
   `From<toml::de::Error> for Error` alongside `src/errors.rs:48-55`, mapping to
   `"toml::de"`. This is the crate's first production TOML read (research Q5);
   the no-match state uses `Error::new("step", ...)`. Total tags become `"io"`,
   `"toml::ser"`, `"toml::de"`, `"git"`, `"step"`.

7. **Read path — `std::fs::read_to_string` + `toml::from_str`.** File-read errors
   ride `"io"`; parse errors ride the new `"toml::de"`. Resolves the default
   `orksorksorks.toml` against process CWD via `std::env::current_dir()`, the
   same single site pattern as `artifact_directory_command`
   (`src/commands/mod.rs:120`).

8. **Payload is the plain step `name`.** Returned uncolored so text mode prints
   just the name and JSON mode emits `{"data": "<name>"}` with no ANSI (consistent
   with `branch`/`artifact_directory`, `tests/artifact_directory.rs:37-38`).

## What We're NOT Doing

- **NOT** creating the artifact directory or any file — `step` is read-only,
  mirroring `artifact_directory`'s "plain (no directory creation)"
  (`src/commands/mod.rs:117-118`).
- **NOT** advancing/mutating step state; the task's "moves to the next step"
  clause is interpreted as out of scope for a determine-only command (Q1).
- **NOT** making the artifact directory configurable or adding an
  `artifact_directory` field to `Config` (Q2).
- **NOT** validating that `trigger_artifact` is a bare filename rather than a
  sub-path — the string-concat convention applies it verbatim.
- **NOT** touching the output layer (`src/main.rs`), the `Error`/`Display`
  contract, the bell flags, or `init`'s behavior — except adding the new
  `Commands` variant, `select_command` arm, `From<toml::de::Error>`, and config
  fields.
- **NOT** adding any dependency (git2/gix/async fs) or CI/workflow change beyond
  what the existing gate already covers.

## Open Risks

- **Integration-testing `step` needs a git-repo CWD** for branch resolution.
  Plan: `git init -b <name>` inside a tempdir, write `orksorksorks.toml` +
  `.pi/orksorksorks/<branch>/<artifact>` there, and run the binary with
  `current_dir(temp.path())`. Unborn-branch behavior of `git branch --show-current`
  in a freshly-initialized repo is the main unknown to pin down during `/5_plan`.
- **`[[steps]]` serde shape is untested territory** (no array-of-tables anywhere
  today). TOML requires uniform keys per table entry; a malformed config produces
  a `toml::de` error — intended and covered, but the exact message isn't pinned.
- **Reverse-iteration semantics assume ordered accumulation** — the "current"
  step is the last one whose artifact exists. If a project can have a later
  step's artifact without earlier ones, the interpretation silently shifts; we
  assume ordered progression per the task.
- **`try_exists` vs `exists` under nextest's working-directory handling** — the
  repo-root CWD note from the 823 design's Open Risks still applies; confirm the
  tempdir-git-repo tests are stable under nextest.
- **No integration test currently asserts the JSON error envelope or stderr
  text** (research Q6). The no-match test will exercise `.failure()`, but the
  `{"error":{...}}` shape itself remains covered only at the unit level
  (`src/errors.rs:91-98`) — worth a follow-up if machine consumers rely on it.

Next: run `/4_structure`.