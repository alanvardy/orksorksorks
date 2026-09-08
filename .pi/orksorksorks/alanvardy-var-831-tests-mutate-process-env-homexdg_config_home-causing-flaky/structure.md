# Structure Outline

## Approach

Inject ambient environment into config-path resolution through an explicit `ConfigEnv` struct, following the codebase's existing `current_branch()`/`current_branch_in(dir)` split: each layer adds a public explicit-state function plus a thin ambient-reading wrapper, and unit tests call the explicit variant directly. Result: zero `unsafe { set_var/remove_var }` in the entire test suite, `APPDATA` covered, and the same tests green under the unchanged gate.

Layers are strictly horizontal, bottom-up — each is independently testable and its tests are green before the next begins. This design decomposes cleanly; no cross-cutting changes need stubbing.

---

## Stage 1: `ConfigEnv` + pure resolution core

Delivers the bottom-most layer: a `ConfigEnv` type and a pure `resolve_config_dir_with_env` function whose behavior is fully determined by its argument. Its green tests prove XDG-absolute/fallback/HOME/APPDATA/error resolution with no process-env reads.

**Files**: `src/config_dir.rs`

**Key changes**:
- `pub(crate) struct ConfigEnv { pub xdg_config_home: Option<OsString>, pub home: Option<OsString>, pub appdata: Option<OsString> }` — new type (`use std::ffi::OsString;`)
- `impl Default for ConfigEnv` — all fields `None` (clean test slate; no ambient reads)
- `impl ConfigEnv { pub(crate) fn from_env() -> Self }` — reads `std::env::var_os` per field
- `pub(crate) fn resolve_config_dir_with_env(env: &ConfigEnv) -> Result<PathBuf, Error>` — body moved verbatim from today's `resolve_config_dir()`; each `var_os` call becomes a field read; same `"config-dir"` error tag
- `fn resolve_config_dir() -> Result<PathBuf, Error>` — stays module-private; body becomes `resolve_config_dir_with_env(&ConfigEnv::from_env())`

**Tests** (rewrite 5 in `src/config_dir.rs:48-114`; add 1):
- `absolute_xdg_is_used` → `resolve_config_dir_with_env(&ConfigEnv { xdg_config_home: Some("/tmp/ork-cfg".into()), ..Default::default() })`
- `unset_xdg_falls_back_to_home_dot_config` → `ConfigEnv { home: Some("/home/ork".into()), ..Default::default() }`
- `empty_xdg_falls_back_to_home` → `ConfigEnv { xdg_config_home: Some("".into()), home: Some("/home/ork".into()), ..Default::default() }`
- `relative_xdg_falls_back_to_home` → `ConfigEnv { xdg_config_home: Some("relative/dir".into()), home: Some("/home/ork".into()), ..Default::default() }`
- `unresolved_home_yields_config_dir_error` → `ConfigEnv::default()`
- `appdata_used_on_windows` (new, sad-path coverage for a previously untested branch) → `ConfigEnv { appdata: Some("C:\\Users\\ork\\AppData\\Roaming".into()), ..Default::default() }`, asserts raw return without absolute check
- `explicit_path_passthrough` — **unchanged** (calls `config_file_path(Some(...))`, bypasses env)

**Verify**: `cargo nextest run --no-tests pass config_dir` → 7 green; `rg -n 'set_var|remove_var' src/config_dir.rs` → no matches.

---

## Stage 2: `config_file_path_with_env` — filename composition

Adds the explicit-env variant of the public entry point so higher layers (and tests) can inject `ConfigEnv` without touching `resolve_config_dir` directly. Green tests prove the public passthrough behavior is preserved through the new wrapper.

**Files**: `src/config_dir.rs`

**Key changes**:
- `pub(crate) fn config_file_path_with_env(explicit: Option<&Path>, env: &ConfigEnv) -> Result<PathBuf, Error>` — `Some` returns `path.to_path_buf()` unchanged; `None` returns `resolve_config_dir_with_env(env).map(|d| d.join(FILE_NAME))`
- `pub fn config_file_path(explicit: Option<&Path>) -> Result<PathBuf, Error>` — **public signature unchanged**; body becomes `config_file_path_with_env(explicit, &ConfigEnv::from_env())`

**Tests**: `explicit_path_passthrough` — unchanged and still green. The 6 Stage 1 tests target `resolve_config_dir_with_env` directly and remain green (independent of this layer).

**Verify**: `cargo nextest run --no-tests pass config_dir` → 7 green.

---

## Stage 3: thread env through `select_command`

Adds `select_command_with_env` so command routing accepts injected env. This eliminates the last env-mutating test in the suite (`select_command_routes_init`) and is the layer `main.rs` already depends on indirectly.

**Files**: `src/commands/mod.rs` (import `crate::config_dir::ConfigEnv`)

**Key changes**:
- `fn select_command_with_env(cli: &Cli, env: &ConfigEnv) -> Result<String, Error>` — same match arms as today; only the `Init { config }` and `Step { config }` arms change: `crate::config_dir::config_file_path_with_env(config.as_deref(), env)?` (replacing `config_file_path(config.as_deref())?`). `Branch`/`ArtifactDirectory` arms unchanged.
- `pub fn select_command(cli: &Cli) -> Result<String, Error>` — **public signature unchanged**; body becomes `select_command_with_env(cli, &ConfigEnv::from_env())`
- `src/main.rs` — **unchanged** (still calls `select_command`, now the wrapper)

**Tests** (in `src/commands/mod.rs`):
- `select_command_routes_init` (`:177-190`) — delete the `unsafe { std::env::set_var("XDG_CONFIG_HOME", temp.path()) }` block; call `select_command_with_env(&cli, &ConfigEnv { xdg_config_home: Some(temp.path().into()), ..Default::default() })`. Assertions on the created `orksorksorks.toml` and success message unchanged; tempdir still auto-cleans on drop.
- All other ~29 tests — unchanged; they pass explicit `--config` paths (bypassing resolution) or exercise arms that ignore env, so ambient env is irrelevant.

**Verify**: `cargo nextest run --no-tests pass commands` → all green; `rg -n 'set_var|remove_var' src/` → no matches anywhere.

---

## Stage 4: full gate

Confirms zero regressions across the whole suite after the refactor and that the forbidden-string/clippy gates still hold.

**Files**: none (verification only).

**Verify**: `./scripts/test.sh` passes (fmt → check → clippy `-D warnings` → nextest → forbidden-strings grep). Optional CI-mirror check: `cargo clippy --all-targets --all-features --locked -- -D warnings`.

---

## Testing Checkpoints

| After Stage | Must be green before advancing |
|---|---|
| 1 | `cargo nextest run --no-tests pass config_dir` — 7 tests; no `set_var`/`remove_var` in `config_dir.rs` |
| 2 | `cargo nextest run --no-tests pass config_dir` — 7 tests |
| 3 | `cargo nextest run --no-tests pass commands` — all command tests; no `set_var`/`remove_var` in `src/` |
| 4 | `./scripts/test.sh` — full local gate |

Resume rule: if context resets mid-implementation, re-run the checkpoint for the last completed stage and only continue once it is green.
