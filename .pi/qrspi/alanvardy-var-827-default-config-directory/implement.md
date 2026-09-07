# Implementation Summary

## Commits
| Phase | Commit | Description |
|-------|--------|-------------|
| 1     | `de6d01b` | Config-directory resolution |
| 2     | `7199f3d` | CLI surface `--config/-c` flag |
| 3     | `26b1547` | Handler — resolve, create parent, report path (+ folded `json_output.rs` fix, see note) |
| 4     | `c71687f` | Integration hardening — override, unresolvable home |

All commits pushed to `origin/alanvardy-var-827-default-config-directory`.

## Automated Checks
- [x] `cargo nextest run config_dir` passes (6/6)
- [x] `cargo nextest run cli_try_parse` passes (8/8)
- [x] `cargo nextest run init_` passes
- [x] `cargo nextest run init_with` passes (override + config-dir-tag)
- [x] json_output regression passes (verified via `init_json no_json` — see note)
- [x] `./scripts/test.sh` passes across all phases (fmt → check → clippy `-D warnings` → nextest **52/52** → forbidden-string gate)

## Manual Verification Items (from the plan)
- [ ] `cargo test --bin orksorksorks config_dir -- --nocapture` shows all six tests green (sanity, optional — nextest is the gate)
- [ ] `cargo run -- init -c /tmp/x.toml` still succeeds (flag parsed; write still lands in CWD this phase)
- [ ] `cargo run -- init --help` shows the `-c, --config <CONFIG>` option
- [ ] `XDG_CONFIG_HOME=/tmp/x cargo run -- init` prints `✓ Created /tmp/x/orksorksorks.toml` and creates `/tmp/x/` (auto-create)
- [ ] `cat /tmp/x/orksorksorks.toml` shows `version = "0.1.0"`
- [ ] `cargo run -- init --config /tmp/foo.toml` then `cat /tmp/foo.toml` shows `version = "0.1.0"`
- [ ] `env -u HOME -u XDG_CONFIG_HOME cargo run -- init -j` prints a JSON error containing `"source":"config-dir"` and exits non-zero

## Deviations & Observations
- **Phase 3 fold:** Phase 3's new success message (`✓ Created <resolved path>`) broke `tests/json_output.rs` (root cause is Phase 3's change). Per plan's own deviation note, that file's fix (Phase 4 step 2) was folded into the Phase 3 commit — supervisor-authorized — so no phase was ever committed red. Phase 4 then covered only the two override tests.
- **Parse-test catch-all:** the plan's Phase 2 parse-test code used non-exhaustive `match` on `Commands` (3 variants) — a hard compile error, not a warning — so each test got a `_ => panic!("expected init")` arm.
- **`cargo nextest run json_output` matches 0 tests:** nextest filters by test *name*, not binary name; the two json_output tests are named `init_json_returns_valid_json_with_data_field` and `init_no_json_prints_plain_text`. Verified via the effective equivalent filter (`init_json no_json`, 3/3 pass). Plan wording only — no behavioral gap.
- **Pre-existing untracked docs:** `.pi/qrspi/.../{conventions,design,research,structure}.md` remain untracked (planning-owned); intentionally left out of commits.
- **Follow-up (out of scope per plan):** committed repo-root `orksorksorks.toml` is orphaned by the new default; the plan flags delete-vs-keep as a follow-up decision.
