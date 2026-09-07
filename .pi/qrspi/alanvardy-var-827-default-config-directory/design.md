# Design Discussion

## Current State

`orksorksorks` is a binary-only Rust CLI (edition 2024, no `[lib]`/`src/lib.rs`),
with a single subcommand `init` and one global flag `-j/--json`
(`src/commands/mod.rs:28-35`). Entry is `main` → `Cli::parse()` → `run_command` →
`select_command` (`src/main.rs:91-95`, `src/commands/mod.rs:45-49`).

- The only command handler, `init_command`, hardcodes the write target as the
  relative literal `"orksorksorks.toml"` resolved against the process CWD
  (`src/commands/mod.rs:52-62`, `File::create` at `:57`). It serializes
  `Config::default()` to TOML, truncate-writes, and returns the success string
  `✓ Created orksorksorks.toml` (`src/commands/mod.rs:55-61`) via `green_string`
  (`src/format.rs:14-16`).
- There is **no** config-dir / home / path resolution anywhere: no runtime
  `env` reads (only compile-time `env!`, `src/commands/mod.rs:5-16`), no
  `create_dir_all`, no path helper (`research` §Q6). Dependencies are 10 crates
  (`Cargo.toml:10-23`); none are fs/home-resolution crates.
- The central `Error` is a flat two-String struct — `{ message, source }`
  (`src/errors.rs:9-13`) — with exactly two conversions: `From<std::io::Error>`
  (source `"io"`, `errors.rs:39-46`) and `From<toml::ser::Error>` (source
  `"toml::ser"`, `errors.rs:48-55`). Rendering has no error-content branching:
  text prints colored `Display` to stderr + bell (`src/main.rs:36-40`), JSON
  prints `{"error":{"message","source"}}` to stdout (`src/main.rs:52-53`), exit 1
  (`src/main.rs:86-88`).
- Tests are coupled to the CWD write. Four integration tests assume the file
  lands in the CWD and trigger failures by making the *CWD* read-only
  (`tests/init_creates_file.rs:11-26,29-38,41-56,59-78`), and the in-process unit
  test `select_command_routes_init` pollsutes the runner CWD
  (`src/commands/mod.rs:70-78`). The only redirect mechanism available to tests
  is assert_cmd's `current_dir`; **no test ever sets an env var** (§Q5).

## Desired End State

- `init` accepts an optional `--config <PATH>` (long flag, `-c` short), typed as
  a path. When absent, the target defaults to the per-user config directory:
  `~/.config/orksorksorks.toml` on Unix (Linux+macOS) and
  `%APPDATA%\orksorksorks.toml` on Windows.
- The parent directory is created automatically if missing (typically
  `~/.config`).
- If the config directory cannot be resolved (no `$HOME`/no `%APPDATA%`, or no
  usable `XDG_CONFIG_HOME`), `init` fails with a clear error tagged
  `source: "config-dir"` instead of silently writing to the CWD.
- The success message reports the actual resolved path (not the bare
  `orksorksorks.toml`), so a redirected `XDG_CONFIG_HOME` is visible.
- Verified by the local gate (`./scripts/test.sh`: fmt → check → clippy
  `-D warnings` → nextest → forbidden-string gate) and by updated tests that pin
  *both* the default-directory behavior and the `--config` override.

## Patterns to Follow

### Match (good patterns in the codebase)

- **Central error chokepoint with tag-based conversions.** New errors follow the
  existing shape: a `From` impl (or `Error::new`) producing
  `{ source: "<lowercase-tag>", message: e.to_string() }`, exactly like
  `From<std::io::Error>` (`src/errors.rs:39-46`). Both render paths (text
  `errors.rs:26-35`, JSON `main.rs:52-53`) handle any new tag automatically —
  no switch-on-variant code exists or is needed.
- **Generate success/error strings through the color choke.** All user-facing
  strings go through `green_string`/`red_string`/`yellow_string`, whose
  `cfg!(test)` no-op (`src/format.rs:6-12`) is what lets tests assert ANSI-free
  output (`tests/json_output.rs:33`). The richer success message must still
  route through `green_string` (`format.rs:14-16`).
- **Edition-2024 dialect.** `std::`-prefixed module paths, `Ok`/`Err`
  constructors, `?` propagation (`src/commands/mod.rs:53-60`), `cfg!` for
  platform/test branching. No new idioms.
- **Test fixture discipline.** `tempfile::tempdir()` + `assert_cmd` +
  `predicates::str::contains` (`tests/init_creates_file.rs:7,12`), exact-content
  assertions (`:24-25`), and readonly setup/restore so `tempfile` can clean up
  (`:43-45,53-55`). Extend this with `cmd.env("XDG_CONFIG_HOME", …)` rather than
  inventing a new mechanism.

### Do NOT follow (anti-patterns found)

- **In-process handler calls in unit tests.** `select_command_routes_init` calls
  `init_command()` directly and writes into the runner CWD
  (`src/commands/mod.rs:70-78`). The redesign must change this test to exercise
  routing/output without touching a real file, not preserve the pollution.
- **Unused state.** The `bell_success`/`bell_failure` fields are set but never
  read (`src/main.rs:20-23` vs `:34,39`). Do not add more dead fields in the
  handler signature or result transport.
- **Inline, untestable env/file handling.** The hardcoded `"orksorksorks.toml"`
  (`src/commands/mod.rs:57`) is exactly the pattern being removed. All path
  resolution goes through one testable function; no ad-hoc `env::var` calls in
  `init_command`.

## Design Decisions

1. **Explicit path override**: add `--config <PATH>` (long, `-c` short,
   `clap::value_parser!(PathBuf)`) as the first field on the `Init` variant,
   typed `Option<PathBuf>`. Rationale: the task's "when no explicit path is
   passed" implies an override must exist; the variant is currently unit-shaped
   (`src/commands/mod.rs:39-42`) so this is the first per-variant arg, following
   clap 4.6.6's `value_parser!(PathBuf)` support (§Q1). When `None`, fall through
   to the default (Decision 2).

2. **Hand-rolled resolution, no new dependency**: a new module
   `src/config_dir.rs` with `pub fn config_file_path(explicit: Option<&Path>) ->
   Result<PathBuf, Error>`. Rationale: the crate is deliberately minimal
   (`Cargo.toml:10-23`), and the resolution is a handful of lines with a
   well-documented spec (§Q3). `etcetera`/`dirs` are rejected to preserve
   dependency discipline; we own the edge cases below explicitly.

3. **Universal config-dir convention**: `$XDG_CONFIG_HOME` (absolute only) →
   fallback `$HOME/.config` on all Unix including macOS; `%APPDATA%` on Windows.
   Rationale: matches the task's literal `~/.config/orksorksorks.toml` and keeps
   macOS/Linux identical, so the test matrix does not multiply. Native Apple
   dirs (`~/Library/Application Support`) are explicitly out.

   Resolution rules (per XDG Base Directory spec, §Q3): treat unset **or empty**
   `XDG_CONFIG_HOME` as unset; treat a **relative** `XDG_CONFIG_HOME` as invalid
   (ignore it, fall back to `$HOME/.config`); if `$HOME` is also unset (or
   `%APPDATA%` on Windows), return `Error::new("config-dir", …)`.

4. **Auto-create the parent directory**: `std::fs::create_dir_all(parent)` before
   `File::create`. Rationale: `~/.config` commonly does not exist; any failure
   already maps through `From<std::io::Error>` (`src/errors.rs:39-46`), needing
   no new error plumbing.

5. **New error tag `"config-dir"`** for the unresolvable-home case, via
   `Error::new("config-dir", "<clear message>")` following the constructor at
   `src/errors.rs:18-23`. Rationale: the flat error has no enum to extend; a
   distinct `source` tag keeps the JSON contract self-describing (tests already
   assert exact tags, `tests/init_creates_file.rs:72`). No silent CWD fallback.

6. **`init_command` signature change**: accept the resolved/override path and
   keep `select_command` as the sole router —
   `Commands::Init { config } => init_command(config)`. Layout: `select_command`
   resolves the path via `config_file_path` and passes it down, so routing stays
   a pure match (`src/commands/mod.rs:45-49`).

7. **Success message reports the resolved path**, e.g.
   `✓ Created <resolved path>`, still via `green_string`. Rationale: with an
   env-driven default, the old bare-filename message (`mod.rs:61`) is ambiguous
   about *where* the file landed. This is a test-contract change —
   `tests/init_creates_file.rs:37` must assert the new string.

8. **Test override via `XDG_CONFIG_HOME`**: set it on the spawned child with
   `cmd.env(...)` and point it at a fresh subdirectory of `tempdir()`. Rationale:
   tests exercise the real resolution path (no bespoke flag/env), and no new CLI
   surface is added. The four CWD-coupled integration tests get repointed:
   the readonly tests now make the *target directory's parent* read-only (or
   assert on a nonexistent-parent failure), and the in-process unit test is
   rewritten to not write into the runner CWD.

## What We're NOT Doing

- **No config *reading*.** Nothing in the crate reads `orksorksorks.toml` today
  (§Q6), and this task only relocates the write. No loader, no `Config::from`
  file path, no round-trip through the committed repo-root file.
- **No new dependencies.** Hand-rolled `std::env` + `std::fs` only; `etcetera`/
  `dirs`/`directories`/`home` stay out.
- **No native macOS/Windows dirs.** No `~/Library/Application Support`; Windows
  uses `%APPDATA%` (not the Known-Folder API) for consistency with the XDG-first
  choice.
- **No change to overwrite semantics.** `File::create` keeps O_TRUNC with no
  existence check, prompt, or backup (§Q2's documented posture is preserved).
- **No change to the output envelope or exit codes.** JSON shape, bell behavior,
  and exit 1 on error are untouched (`src/main.rs:36-40,52-53,86-88`).
- **No `cache_dir`/`data_dir`/`ProjectDirs` plumbing** and no new subcommands —
  the CLI surface remains `init` + `-j/--json` + `-c/--config`.
- **No platform-gated test matrix** (e.g. `#[cfg(target_os = "windows")]`) until
  Windows CI exists; the resolution logic is unit-tested, Windows integration
  behavior is documented but not asserted in CI.

## Open Risks

- **Windows is untested in CI.** The current workflows (`_reusable-test.yml`)
  run on a single OS. `%APPDATA%` can be absent in minimal/CI environments
  (§Q3), which would turn a previously-succeeding CWD write into a `"config-dir"`
  error. Mitigation: the fallback rules in Decision 3 are unit-tested; Windows is
  left as a documented, unasserted surface like the existing readonly tests
  (`tests/init_creates_file.rs:41-78` carry no os gating).
- **Env-driven surprise.** A user with `XDG_CONFIG_HOME` set (or `$HOME` on an
  unusual path) gets a write target they may not expect. Mitigated by Decision 7
  (message shows the resolved path).
- **macOS `XDG_CONFIG_HOME` is honored** by this design, matching the task, but
  slightly diverges from Apple platform idiom. Low risk; explicitly decided in
  Decision 3.
- **`create_dir_all` + `File::create` race** (e.g. symlinked parent swapped
  between calls) is theoretically possible but matches common tool behavior; not
  addressed beyond a single `create_dir_all`.
- **Exact string assertions are brittle.** The new success message and any
  `"config-dir"` message text will be pinned in tests; keep messages stable or
  update `tests/init_creates_file.rs` in the same change.
- **Committed repo-root `orksorksorks.toml`** becomes orphaned by the default
  change (still read by nothing, `tests/json_output.rs` runs in the repo root and
  exercises the overwrite path). Decision needed later on whether to delete it.