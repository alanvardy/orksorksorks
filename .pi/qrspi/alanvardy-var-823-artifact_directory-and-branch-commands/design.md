# Design Discussion

## Current State

`orksorksorks` is a binary-only Rust CLI crate (`Cargo.toml:1-4`; no `[lib]`,
so integration tests spawn the real binary rather than importing types —
`tests/init_creates_file.rs:19-20`). It has exactly one subcommand today, `init`.

The command surface works like this:

- clap-derive `Cli { json: bool, command: Commands }` (`src/commands/mod.rs:19-34`)
  parses globally (`json` short `-j`, `default_value_t = false`) and dispatches
  on a `#[derive(Subcommand)] enum Commands { Init }`
  (`src/commands/mod.rs:39-42`).
- `select_command(&Cli) -> Result<String, Error>` is a `match` over enum
  variants (`src/commands/mod.rs:45-50`); each arm calls a zero-argument handler.
- Handler contract: `fn xxx_command() -> Result<String, Error>`. Success returns
  the *entire* text payload (`init_command` returns
  `Ok(green_string("✓ Created orksorksorks.toml"))`, `src/commands/mod.rs:61`);
  failure returns `Error`, built via `Error::new(source, message)`
  (`src/errors.rs:18-23`) or `From<std::io::Error>` → `"io"`
  (`src/errors.rs:39-46`). Handlers never print or exit — the output layer does.
- `run_command` (`src/main.rs:68-89`) wraps the handler result in `CommandResult`
  (`src/main.rs:17-26`) and routes through `output_result` (`src/main.rs:59-66`):
  text mode prints `Ok` payload to stdout + bell, `Err` to stderr via `Display`
  (`src/main.rs:29-42`); JSON mode prints `{"data": data}` or
  `{"error": {"message", "source"}}` — both to **stdout** (`src/main.rs:45-56`).
- Exit codes: `Ok` → 0; handler `Err` → 1 (`src/main.rs:86-88`); clap usage
  errors → 2.

Relevant gaps the research established:

- The binary currently reads **no** runtime environment and spawns **no**
  subprocess; `src/`'s only path literal is a bare relative
  `"orksorksorks.toml"` resolved against CWD (`src/commands/mod.rs:57`).
- `std::env` is never used in `src/`; branch identity today lives only in git
  metadata / worktree paths / the `.pi/qrspi/<branch>/` docs dir, not in the
  binary (research Q3).
- `Error.source` is a lowercase tag: `"io"`, `"toml::ser"`
  (`src/errors.rs:39-55`). `Display` owns all coloring
  (`src/errors.rs:26-33`); a handler's success `String` is embedded verbatim
  into the JSON `data` field (`src/main.rs:47-49`), so ANSI in a returned
  string leaks escapes into JSON.

## Desired End State

Two new subcommands, both following the existing handler contract exactly:

- `branch` → prints the current git branch name as a plain string.
- `artifact_directory` → prints `<cwd>/.pi/orksorksorks/<branch>/` as a plain
  string (trailing slash included, per `task.md:5`).

Verification:

- Unit: `select_command` routes both variants; clap parses both subcommand
  tokens and rejects the no-subcommand case (extend the existing
  `src/commands/mod.rs:64-99` style tests).
- Integration (spawn the binary via `assert_cmd::Command::cargo_bin`, matching
  `tests/init_creates_file.rs` / `tests/json_output.rs`):
  - `branch` → stdout equals `git branch --show-current` in the same dir.
  - `branch -j` → valid JSON, `{"data": "<branch>"}`, no ANSI in `data`.
  - `artifact_directory` → stdout equals `<cwd>/.pi/orksorksorks/<branch>/`.
  - `artifact_directory -j` → valid JSON, no ANSI in `data`.
- `./scripts/test.sh` passes (fmt → check → clippy → nextest → forbidden-strings;
  `scripts/test.sh:1-21`).

## Patterns to Follow

- **One handler, one match arm per command.** Extend `Commands` with two
  variants, add two `select_command` arms, and two zero-arg
  `fn -> Result<String, Error>` handlers — the exact manual wiring already
  documented at `src/commands/mod.rs:39-62`.
- **Errors flow through the central `Error` type.** Use `Error::new(..)` for
  non-io failures (`src/errors.rs:18-23`) and let failed subprocess spawns ride
  the `From<std::io::Error>` → `"io"` path (`src/errors.rs:39-46`). Never
  return bare statuses or print/exit from handlers (matches the contract at
  `src/commands/mod.rs:52-62` and `src/main.rs:29-56`).
- **No ANSI in payloads** that become JSON `data`. These commands' return value
  *is* the machine-consumable payload, so return plain strings (contrast with
  `init_command`'s colored message, `src/commands/mod.rs:61`).
- **CWD-relative testing** via `tempfile::tempdir()` + `cmd.current_dir(...)`
  (`tests/init_creates_file.rs:14,32`) — but see Open Risks: `artifact_directory`
  still needs `git` to resolve, so its CWD must stay inside the repo.
- **`cfg!(test)` seam untouched.** We do not depend on it here (plain strings),
  but must not break the existing assertions that do
  (`src/format.rs:7-11`, `tests/json_output.rs:35-41`).

Patterns to **NOT** follow:

- Do **not** color these two return values — the ANSI-in-JSON leak the
  research flagged (conventions.md) would become a real bug for machine
  consumption.
- Do **not** read `PWD` or any other env var — the binary has zero runtime env
  reads today and adding one invents a new convention (research Q2/Q3).
- Do **not** add a `git2`/`gix` dependency, and do not parse `.git` files —
  both couples to costly/private surfaces when a subprocess is idiomatic here.

## Design Decisions

1. **Branch source — `git` subprocess**: `std::process::Command::new("git")
   .arg("branch").arg("--show-current").output()`. Self-contained, correct in
   this repo's worktree layout, no new dependency. Blocking `std` (not tokio)
   since handlers are synchronous like `init_command`.
2. **Detached HEAD → error**: `--show-current` prints empty when not on a
   branch; treat empty output as `Error::new("git", "not on a branch (detached
   HEAD)")`. Failing loudly keeps `artifact_directory` from producing a broken
   path segment.
3. **Shared helper `current_branch() -> Result<String, Error>`**: both handlers
   call it; it owns spawn, stderr capture into the message, trimming, and the
   empty-output check. (Also a small pure helper `artifact_dir_path(cwd, branch)`
   so path composition is unit-testable without git.)
4. **`artifact_directory` is pure (no mkdir)**: returns the path string only.
   Downstream phases own directory creation; this matches `task.md:5`
   ("returns a string").
5. **`$PWD` means `std::env::current_dir()`**: the actual CWD, not the `PWD`
   env var (which can be stale or unset). Its `io::Error` flows through the
   existing `"io"` tag.
6. **New `"git"` error tag**: a distinct lowercase source for git-specific
   failures (non-zero exit, detached HEAD) via `Error::new("git", ..)`, so JSON
   consumers can distinguish "git is broken" from generic I/O. Spawn failure
   (git not on PATH) remains `"io"` via the existing `From`.
7. **Subcommand naming**: `branch` is natural. For `artifact_directory`, pin the
   literal snake-case token with `#[command(name = "artifact_directory")]` rather
   than clap's default kebab-case (`artifact-directory`), to match `task.md:3`
   verbatim.

## What We're NOT Doing

- **NOT** creating `.pi/orksorksorks/<branch>/` — the command only reports the
  path; it has no filesystem side effects.
- **NOT** reading env vars, `.git` files, or the QRSPI docs tree to find the
  branch. Only the `git` subprocess.
- **NOT** adding build-time injection, new dependencies, or runtime network
  behavior.
- **NOT** touching the output layer (`main.rs`), the `Error`/`Display` contract,
  the bell flags, or `init`'s behavior — except to extend the `Commands` enum
  and `select_command`.
- **NOT** handling full-disk / permission-denied / collision cases beyond what
  `std::env::current_dir()` and the git subprocess already surface through the
  existing `"io"` / `"git"` paths.

## Open Risks

- **Detached-HEAD error wording** is a user-facing judgment call made here
  without a separate confirmation; the exit-1 behavior is the design, but the
  message text is cheap to change at implementation.
- **Integration-testing `artifact_directory` CWD**: it needs a CWD inside the
  repo so `git` resolves, while also asserting the absolute path. A fully
  isolated `tempdir` CWD would make `git branch --show-current` fail. Plan:
  point `current_dir` at the repo root (or a repo subdir), and derive the
  expected string from `std::env::current_dir()` in the test. Confirm this is
  stable under nextest's working-directory handling.
- **Subcommand spelling** (`artifact_directory` vs `artifact-directory`) is
  pinned to snake_case per the task, but if CLI ergonomics later prefer
  kebab-case, it's a one-line `#[command(...)]` change with test fallout.
- **First runtime subprocess in the binary**: no async context (handlers are
  sync), so `Command::output()` blocks; acceptable for this CLI's scope, but a
  future async refactor would need to revisit.
- **Windows not in scope** (CI is `ubuntu-latest`; local dev is macOS) — path
  separators and `--show-current` availability are assumed Unix-like.

Next: run `/4_structure`.