# Structure Outline

## Approach
Add two subcommands (`branch`, `artifact_directory`) to the existing clap-derive
command surface, each a zero-arg `fn -> Result<String, Error>` handler matching
the `init_command` contract exactly. Both commands resolve the branch via a
blocking `git branch --show-current` subprocess and return plain (uncolored)
strings; `artifact_directory` composes `<cwd>/.pi/orksorksorks/<branch>/` from
`std::env::current_dir()` + the branch, with a distinct `"git"` error tag for
git-specific failures. Built as five horizontal layers, bottom-up, each green
before the next.

---

## Stage 1: Pure path composition

The bottom-most unit: a pure, side-effect-free function that turns a cwd and a
branch into the artifact-directory string. No git, no I/O, no `Error` — the
only thing `artifact_directory_command` cannot unit-test without a subprocess
is fenced off here.

**Files**: `src/commands/mod.rs`

**Key changes**:
- `fn artifact_dir_path(cwd: &std::path::Path, branch: &str) -> String` — new.
  Returns `format!("{}/.pi/orksorksorks/{}/", cwd.display(), branch)`
  (trailing slash included). No error path.

**Tests** (inline `#[cfg(test)]`, existing `select_command_routes_init` style):
- `artifact_dir_path_appends_trailing_slash` — happy path: `Path::new("/repo")`
  + `"main"` → `/repo/.pi/orksorksorks/main/`.
- `artifact_dir_path_is_plain` — no `\x1b` byte in the result (guards the
  ANSI-in-JSON leak from ever entering the payload).
- `artifact_dir_path_handles_branch_with_slashes` — sad-path-ish: branch
  `"feature/x"` composes without panic.

**Verify**: `./scripts/test.sh` (fast loop: `cargo nextest run --bin orksorksorks`).

---

## Stage 2: Git branch resolution

The data-access layer: owns the subprocess, error mapping, and the
detached-HEAD decision. The *decision* is split into a pure
string-in/string-out seam so the sad paths are unit-testable without ever
detaching the repo's HEAD — the binary-level detached-HEAD case is deliberately
not exercised at the integration layer.

**Files**: `src/commands/mod.rs`

**Key changes**:
- `fn current_branch() -> Result<String, Error>` — new. Spawns
  `std::process::Command::new("git").arg("branch").arg("--show-current").output()`,
  then delegates to the seam below. Spawn failure rides the existing
  `From<std::io::Error>` → `"io"` path.
- `fn parse_branch_output(exit_ok: bool, stdout: &str, stderr: &str) -> Result<String, Error>` —
  new, pure. `!exit_ok` → `Error::new("git", stderr.trim())`; `exit_ok` but
  `stdout.trim().is_empty()` → `Error::new("git", "not on a branch (detached HEAD)")`;
  else `Ok(stdout.trim().to_string())`.

**Tests** (inline): `current_branch` happy path returns the *actual* branch
(tests run inside the repo), asserted as a non-empty string; `parse_branch_output`
happy (trims trailing newline), empty-stdout → detached-HEAD error with
`source == "git"`, non-zero-exit + stderr → git error, `from_io`-style spawn
failure tag stays `"io"`.

**Verify**: `./scripts/test.sh` (fast: `cargo nextest run --bin orksorksorks`).

---

## Stage 3: Command handlers

The service layer: two zero-arg handlers that consume Stages 1–2 and produce the
final `Result<String, Error>` payload. Both return plain strings — no
`green_string` — because their value *is* the JSON `data` field.

**Files**: `src/commands/mod.rs`

**Key changes**:
- `fn branch_command() -> Result<String, Error>` — new. Returns
  `current_branch()` directly.
- `fn artifact_directory_command() -> Result<String, Error>` — new.
  `let cwd = std::env::current_dir()?;` (io error → `"io"`) then
  `Ok(artifact_dir_path(&cwd, &current_branch()?))`.

**Tests** (inline, call the private fns via `super::*`): each handler returns a
non-empty plain string with no `\x1b`; `artifact_directory_command` output
starts with the process cwd and ends with the branch + `/`.

**Verify**: `./scripts/test.sh` (fast: `cargo nextest run --bin orksorksorks`).

---

## Stage 4: Command surface / dispatch

The transport layer: register both subcommands with clap and route them in
`select_command`. Enables the binary to reach the proven handlers; no handler
logic changes here.

**Files**: `src/commands/mod.rs`

**Key changes**:
- `enum Commands` — add `Branch` and `#[command(name = "artifact_directory")] ArtifactDirectory`
  variants (pin snake_case per `task.md`; `Branch` needs no rename — lowercased
  variant already yields `branch`, per `src/commands/mod.rs:39-42`).
- `fn select_command(cli: &Cli) -> Result<String, Error>` — add two match arms:
  `Commands::Branch => branch_command()`,
  `Commands::ArtifactDirectory => artifact_directory_command()`.

**Tests** (inline, extend `src/commands/mod.rs:64-99`): `select_command` routes
each variant; clap `try_parse_from` accepts `branch` and `artifact_directory`;
rejects `artifact-directory` (proves the name pin); no-subcommand still
rejected; `Cli::command().debug_assert()` still holds.

**Verify**: `./scripts/test.sh` (fast: `cargo nextest run --bin orksorksorks`).

---

## Stage 5: Integration tests (binary spawn)

The presentation layer: prove the end-to-end behavior through the real binary,
matching the `assert_cmd::Command::cargo_bin` convention. `artifact_directory`
CWD must stay in-repo (open risk: git must resolve), so point `current_dir` at
the repo root / a repo subdir and derive the expected string from
`std::env::current_dir()` in the test rather than a tempdir.

**Files**: `tests/branch.rs` (new), `tests/artifact_directory.rs` (new)

**Key changes** (no production code — tests only):
- `branch` → stdout equals `git branch --show-current` result in the same dir.
- `branch -j` → valid JSON `{"data": "<branch>"}`, no ANSI in `data`.
- `artifact_directory` → stdout equals the composed absolute path with trailing
  slash.
- `artifact_directory -j` → valid JSON, no ANSI in `data`.
- (Optional sad path) `branch`/`artifact_directory` under a cwd outside any
  repo → non-zero exit (the "not a git repository" `"git"` error).

**Verify**: `./scripts/test.sh` (full gate: fmt → check → clippy → nextest →
forbidden-strings).

---

## Testing Checkpoints
Resume-safe green gates; do not advance past a red one:
1. **Stage 1** — `artifact_dir_path` unit tests green.
2. **Stage 2** — `current_branch` + `parse_branch_output` unit tests green.
3. **Stage 3** — `branch_command` / `artifact_directory_command` unit tests green.
4. **Stage 4** — clap parse + `select_command` routing unit tests green.
5. **Stage 5** — `cargo nextest run` (all tests) + full `./scripts/test.sh` green.

## Cross-cutting note
The detached-HEAD sadness is not reachable through the spawned binary (you can't
detach HEAD inside an integration test). It is stubbed deliberately at
**Stage 2** via the pure `parse_branch_output` seam, so the branch is covered
there and the integration layer only asserts happy paths + JSON shape. The
exact detached-HEAD message wording remains an implementation-time string
(open risk, cheap to change).