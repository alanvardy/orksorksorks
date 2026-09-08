# Design Discussion

## Current State

Config path resolution centers on `resolve_config_dir()` (`src/config_dir.rs:24-46`), a
private fn that reads three process-global env vars (`XDG_CONFIG_HOME`, `HOME`, `APPDATA`)
via `std::env::var_os`. The public entry point is `config_file_path(explicit: Option<&Path>)`
(`src/config_dir.rs:16-22`): `Some` passes through; `None` calls `resolve_config_dir()`.

Five of six `config_dir::tests` (`src/config_dir.rs:48-114`) mutate the process env with
`unsafe { set_var/remove_var }` and never restore it (research Q2 table). One test in
`commands/mod.rs` (`select_command_routes_init`, `:177-190`) also sets `XDG_CONFIG_HOME`
permanently. Under nextest's process-per-test model these leaks are harmless (each test is a
separate OS process), but the suite is written as if it shares a process — no restore
discipline, no RAII guard, no fixture. The mutations are invisible to nextest but would
produce order-dependent failures under a shared-process harness.

The codebase already has a dependency-injection convention (research Q4):
`current_branch()` wraps `current_branch_in(dir)`, `parse_branch_output` is pure,
`determine_step` takes explicit `Config` + dir string. That pattern is not extended to env
reads in `config_dir.rs` — `resolve_config_dir` is private and takes no params.

`APPDATA` (`src/config_dir.rs:33-37`, Windows-only) has zero test coverage (research Q6).

## Desired End State

1. **Zero `unsafe { set_var/remove_var }` in the entire test suite.** All 6 mutating tests
   (`config_dir.rs`: 5 tests + `commands/mod.rs`: `select_command_routes_init`) are
   rewritten to pass explicit env values into the resolution functions.

2. **`resolve_config_dir` gains an explicit-state variant** following the
   `current_branch()`/`current_branch_in(dir)` pattern. A `ConfigEnv` struct holds the three
   optional env values; the private wrapper reads ambient env into a `ConfigEnv`.

3. **`config_file_path` threads env through** so tests can inject without touching the
   process environment. Production callers pass `ConfigEnv::from_env()`.

4. **`select_command` threads env through** so `select_command_routes_init` can inject a
   tempdir-backed `XDG_CONFIG_HOME` without `set_var`.

5. **`APPDATA` branch gets a unit test** — trivial after injection, no Windows host needed.

6. **Gate passes unchanged.** `./scripts/test.sh` succeeds with the same tests (rewritten,
   not removed), no forbidden strings, no clippy warnings.

## Patterns to Follow

| Pattern | Source | Apply to |
|---|---|---|
| Public explicit-state fn + thin private/env-reading wrapper | `git::current_branch()` → `current_branch_in(dir)` (`src/git.rs:8-10,17-25`) | `resolve_config_dir_with_env(env)` + `resolve_config_dir()` |
| Pure decision fn, unit-tested with literals | `parse_branch_output(exit_ok, stdout, stderr)` (`src/git.rs:28-36,95-116`) | Logic inside `resolve_config_dir_with_env` — tests pass `ConfigEnv` literals |
| `Option<&Path>` injection for bypass | `config_file_path(explicit: Option<&Path>)` (`src/config_dir.rs:16-22`) | Keep existing signature; add env param variant |
| Tempdir fixture for disk-backed assertions | `tempfile::tempdir()` in `select_command_routes_init` (`src/commands/mod.rs:178`) | Same test, but env injected via `ConfigEnv` instead of `set_var` |
| `Error::new("config-dir", ...)` for resolution failures | `src/config_dir.rs:43` | Keep; `resolve_config_dir_with_env` returns the same error tag |

**Patterns NOT to follow:**
- `unsafe { std::env::set_var/remove_var }` — the thing we're removing. Only `absolute_xdg_is_used` (`src/config_dir.rs:54→57`) restores; that pattern is fragile and incompatible with shared-process test runners.
- `serial_test::serial` — a documented no-op under nextest (research Q5); adds a dep for zero benefit in this project's execution model.
- The `APPDATA` branch's raw-return-without-absolute-check (`src/config_dir.rs:35-37`) — existing behavior preserved as-is (not a design change), but now tested for correctness.

## Design Decisions

1. **`ConfigEnv` struct for env injection** (Q1 Option B).
   New type in `src/config_dir.rs`:
   ```rust
   pub(crate) struct ConfigEnv {
       pub xdg_config_home: Option<OsString>,
       pub home: Option<OsString>,
       pub appdata: Option<OsString>,
   }
   ```
   with `ConfigEnv::from_env()` reading `std::env::var_os` for each field. `pub(crate)` — visible to `commands/mod.rs` for `select_command` threading but not leaked to integration tests (binary-only crate, no lib target).
   Chosen over individual `Option<&OsStr>` params because it documents which vars are consumed at the type level and keeps fn signatures compact across the call chain (`config_file_path` → `resolve_config_dir` → `select_command`).

2. **Fix all 6 env-mutating tests** (Q2 Option A).
   Every `unsafe { set_var/remove_var }` call in the test suite is removed. `config_dir::tests` (5 tests) switch to `resolve_config_dir_with_env(&ConfigEnv { ... })`. `select_command_routes_init` switches to `select_command_with_env(..., &ConfigEnv { xdg_config_home: Some(temp.path().into()), ... })`. The `absolute_xdg_is_used` test loses its restore step (`:57`) because there's nothing to restore.

3. **Add `APPDATA` coverage now** (Q3 Option A).
   A new unit test `appdata_used_on_windows` passes `ConfigEnv { appdata: Some(...), ..default() }` and asserts the raw-return behavior. No `#[cfg(windows)]` gate — the explicit fn is platform-agnostic; the `cfg!(windows)` branch in production (`src/config_dir.rs:33`) is a compile-time switch in the wrapper, but the injected path tests the logic directly regardless of host OS.

4. **Thread env through `select_command`** (Q4 Option A).
   `select_command_with_env(..., env: &ConfigEnv)` is the explicit variant; `select_command(...)` wraps it with `ConfigEnv::from_env()`. The env flows only to `config_file_path` calls in the `Init` and `Step` arms (`src/commands/mod.rs:69-70,75-76`). Tests that pass an explicit `--config` path (bypassing resolution) can pass a dummy `ConfigEnv::default()`.

5. **Retain `config_file_path`'s public signature.**
   Add `config_file_path_with_env(explicit: Option<&Path>, env: &ConfigEnv)` as the explicit variant; `config_file_path(explicit)` becomes the thin wrapper calling it with `ConfigEnv::from_env()`. Matches `current_branch`/`current_branch_in` split exactly.

6. **`resolve_config_dir` stays module-private; `resolve_config_dir_with_env` is `pub(crate)`.**
   `config_file_path_with_env` is the primary entry point for tests. `resolve_config_dir_with_env` is exposed for `config_dir::tests` direct unit testing but not wider than the crate.

## What We're NOT Doing

- NOT adding a `serial_test` dependency or nextest `[test-groups]` — process-per-test already isolates; this task adds injection so serialization is moot.
- NOT changing the `APPDATA` branch behavior (raw return, no absolute check) — coverage only.
- NOT creating a `ConfigEnv::default()` with real env reads — `Default` returns `None` for all fields so tests get a clean slate; `from_env()` is the ambient read.
- NOT modifying integration tests in `tests/` — they already use `cmd.env()`/`cmd.env_remove()` on child processes (`assert_cmd`), which is the correct isolation pattern.
- NOT converting the crate to a lib+bin layout — the binary-only structure is intentional and not in scope.
- NOT adding new dev-dependencies — `ConfigEnv` is a plain struct with zero new crates.
- NOT removing any test — same test count, same assertions, zero `unsafe` env blocks.

## Open Risks

1. **`select_command` threading depth.** `select_command` currently takes a `clap::ArgMatches` ref. Adding an `env` param is straightforward, but the test `select_command_routes_init` constructs matches via `Command::try_get_matches_from(...)` — that construction path is unaffected. Risk is low; `ArgMatches` is orthogonal to env.

2. **`ConfigEnv::from_env()` at compile time.** The wrapper calls `std::env::var_os` at runtime — same as today. No new failure modes introduced. The `cfg!(windows)` gate on `appdata` in the wrapper means `from_env()` unconditionally reads `APPDATA` on all platforms; on non-Windows it'll be `None` (var doesn't exist), which is harmless and matches today's behavior (the cfg gate prevents consulting it, not reading it).

3. **`APPDATA` test semantics.** The test asserts "raw return" behavior. If someone later adds an absolute-path check to the `APPDATA` branch (symmetry with XDG), this test will flag the change. That's intentional — the test documents current behavior, not desired behavior.

4. **`select_command_routes_init` fixture.** The test currently creates a tempdir, sets `XDG_CONFIG_HOME` to it, and relies on `tempfile::tempdir()` auto-cleanup on drop. After the rewrite, it passes `ConfigEnv { xdg_config_home: Some(temp.path().into()), ..default() }` to `select_command_with_env`. Same tempdir lifetime, same cleanup — just injected instead of env-mutated.
