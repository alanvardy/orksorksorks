# Implementation Summary

## Commits
| Phase | Commit | Description |
|-------|--------|-------------|
| 1     | f9c8a88 | ConfigEnv + pure resolution core |
| 2     | 77009f5 | config_file_path_with_env — filename composition |
| 3     | a6a4f26 | thread env through select_command |
| 4     | 179a508 | full gate |

Completed in a prior run and resumed here; each phase was implemented, verified, and
committed sequentially. The resume checkpoint for the last stage (Phase 4, the full
gate) was re-run and is green.

## Automated Checks
- [x] `cargo nextest run --no-tests pass config_dir` — green (10 tests; superset of the plan's 7)
- [x] `rg -n 'set_var|remove_var' src/config_dir.rs` — no matches
- [x] `cargo clippy --tests -- -D warnings` — clean (no dead `resolve_config_dir`)
- [x] `cargo nextest run --no-tests pass commands` — all command tests green (26)
- [x] `rg -n 'set_var|remove_var' src/` — no matches anywhere in `src/`
- [x] `./scripts/test.sh` passes (fmt → check → clippy `-D warnings` → nextest 82/82 → forbidden-strings grep)
- [x] `cargo clippy --all-targets --all-features --locked -- -D warnings` passes (CI mirror)

## Manual Verification Items (from the plan)
- [ ] `cargo run -- init` with `XDG_CONFIG_HOME` set to an absolute dir writes `$XDG_CONFIG_HOME/orksorksorks.toml` (unchanged behavior)
- [ ] `cargo run -- init` with no `XDG_CONFIG_HOME` falls back to `$HOME/.config/orksorksorks.toml` (unchanged)
- [ ] `cargo run -- init` (no `--config`) still writes to the config dir and prints `✓ Created …` (unchanged)
- [ ] `cargo run -- init` and `cargo run -- step` behave identically to before on this host (spot-check with and without `XDG_CONFIG_HOME`/`HOME` set)

## Notes for Review (adaptations from the plan)
The base branch already carried the merged var-832 "path-in-config-error" work, so the
codebase had diverged from the plan's expectations. The implementation adapted while
preserving the plan's intent:

1. **Tuple returns.** `resolve_config_dir_with_env` and `config_file_path_with_env`
   return `(PathBuf, ConfigPathSource)` instead of a bare `PathBuf` (unlike the plan's
   snippets), because `ConfigPathSource` from var-832 is threaded through config-dir
   resolution. The `cfg!(windows)` switch lives in `ConfigEnv::from_env` (plan deviation 1
   honored); the core is platform-agnostic and `appdata_used_on_windows` runs on every host.
2. **`resolve_config_dir` / plain `config_file_path` no longer exist** — removed when they
   became dead (the shared `ConfigPathSource` threading made them obsolete before Phase 2
   per plan deviation 2). No dead-code warnings.
3. **Extra tests.** `config_dir` has 10 tests (plan expected 7): the 7 planned ones plus
   `config_path_source_display_pins_phrases` (set-source assertions added when adapting to
   the tuple return). All env-mutating tests were rewritten to inject `ConfigEnv`; no
   `set_var`/`remove_var` remains in `src/`, `tests/`, or `scripts/`.
4. `./scripts/test.sh` runs 82 tests total, all passing (was verified in Phase 4 and
   re-verified on resume).