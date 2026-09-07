# Implementation Summary

## Commits
| Phase | Commit | Description |
|-------|--------|-------------|
| 1     | `e4f951c` | Toolchain pin (rust-toolchain.toml) |
| 2     | `101c981` | Developer gate (scripts/test.sh) |
| 3     | `d1acd40` | Coverage config (codecov.yml) |

All three commits pushed to `origin/alanvardy-var-817-copy-over-scripts-and-test-infrastructure`. One commit per phase.

## Automated Checks
- [x] `cargo check` exits 0 under the pinned 1.98.1 toolchain (Phase 1)
- [x] `rustup show` reports the 1.98.1 directory override / "overridden by" pin (Phase 1)
- [x] `./scripts/test.sh` exits 0 on the clean crate (`=== SUCCESS ===` / `=== Done ===.`) (Phase 2)
- [x] Injecting `dbg!("test");` → gate exits 1, forbidden-string rg guard catches it (Phase 2)
- [x] After removing the injection → gate exits 0 again (Phase 2)
- [x] Gate fails on a real problem: compile error in `src/main.rs` → exits 101, no SUCCESS banner (main-agent-added check; see deviation note)
- [x] `codecov.yml` parses as valid YAML (Phase 3)
- [x] `codecov.yml` has `project:`, `patch:`, `layout:` keys; no `ignore:` key (Phase 3)
- [x] Final cross-check: gate exits 0, pin intact, YAML parses, no `src/`/`Cargo.toml` changes (`git diff cf01f01..HEAD -- src/ Cargo.toml` empty)

## Approved Deviations (user-confirmed)
1. **Phase 2 — `cargo nextest run --no-tests` → `--no-tests=pass`**: installed nextest 0.9.143 requires a value for `--no-tests <ACTION>`; `pass` is the plan's stated intent (gate passes on zero tests).
2. **Phase 2 — `fi` → `fi &&`**: with `set -e`, the `echo "=== SUCCESS ==="` statement outside the AND-chain ran unconditionally and reset the exit status to 0, so fmt/check/clippy/test failures printed SUCCESS and exited 0 (verified empirically). Joining the banner into the chain makes the gate fail on real problems while keeping the plan's two-AND-chain shape. All Phase 2 checks re-verified after the fix.
3. **rustup verification wording**: rustup 1.29.1 prints `overridden by '.../rust-toolchain.toml'`, not the older `(directory override)` phrasing; the `active toolchain` subcommand is `active-toolchain`. Same intent, adapted wording throughout plan.md.

## Manual Verification Items (from the plan — pending user confirmation)
- [ ] **Phase 1** `rust-toolchain.toml` exists at repo root with `channel = "1.98.1"` and `components = ["clippy", "rustfmt"]`
- [ ] **Phase 1** `rustup show active-toolchain` prints `1.98.1-aarch64-apple-darwin (overridden by '.../rust-toolchain.toml')`
- [ ] **Phase 2** `scripts/test.sh` has `set -euo pipefail` at line 2
- [ ] **Phase 2** script has `cargo nextest run --no-tests=pass` (not bare `cargo nextest run`)
- [ ] **Phase 2** script has `rg -i -g` (no `-s` flag)
- [ ] **Phase 2** script does NOT invoke `testcfg_clean.sh` or print `=== CLEANING FILES ===`
- [ ] **Phase 3** `codecov.yml` has `coverage.status.project` with `target: auto`, `threshold: 10%`
- [ ] **Phase 3** `codecov.yml` has `coverage.status.patch` with `target: 50%`, `threshold: 2%`
- [ ] **Phase 3** `codecov.yml` has `comment.layout` and `comment.require_changes: false`
- [ ] **Phase 3** no `coverage.ignore` section anywhere in `codecov.yml`

## Observations / Open Items (out of scope, for the owner to decide)
- **`Cargo.lock`**: `cargo` regenerates an untracked `Cargo.lock` on every invocation (the crate is a binary — Cargo convention would commit it; the plan expects exactly 3 new files so it was left untracked and removed for the cross-check). Decide: commit it or add to `.gitignore`. This is a follow-up decision, not part of this plan.
- **Codecov `comment` nesting**: `comment` is nested under `coverage.status` (copied from tod verbatim). Codecov docs show `coverage.comment`; if the nested placement is ignored, PR comments may not render. Status checks still work. Unchanged to match tod exactly — revisit when CI wiring is added.
- **Toolchain pin vs. future CI**: a `dtolnay/rust-toolchain@stable` CI step (as tod uses) would ignore the `rust-toolchain.toml` pin. Acceptable; local dev gets the pin.
