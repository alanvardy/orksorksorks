# Implementation Plan

## Overview

Copy three tooling files from `tod` to this near-empty Rust crate, with targeted patches: pin the toolchain to current stable (1.98.1), fix the gate script to fail on real problems but pass on a zero-test crate, and drop tod-specific coverage ignores. No `src/` or `Cargo.toml` changes.

## Phase 1: Toolchain pin (`rust-toolchain.toml`)

Pin the compiler to 1.98.1 (the active stable on this machine) with clippy + rustfmt components. All subsequent cargo commands use this toolchain via rustup directory override.

### Changes

#### 1. Create `rust-toolchain.toml`
**File**: `rust-toolchain.toml` (repo root)
**Action**: create

```toml
[toolchain]
channel = "1.98.1"
components = ["clippy", "rustfmt"]
```

This is the same structure as `tod/rust-toolchain.toml` with the channel bumped to match the machine's default stable (1.98.1 instead of 1.97.1). 1.98.1 satisfies edition 2024 (≥ 1.85). No `profile`, `targets`, or other keys.

### Verification

#### Automated
- [x] `cargo check` exits 0 (proves the pinned toolchain can build the crate)
- [x] `rustup show` (run from repo root) reports `active toolchain: 1.98.1-aarch64-apple-darwin (directory override)`
  - Command: `rustup show | grep "directory override" | grep "1.98.1"`

#### Manual
- [ ] File exists at repo root with correct channel and components
- [ ] `rustup show active toolchain` prints `1.98.1-aarch64-apple-darwin (directory override for '/Users/vardy/dev/alanvardy-var-817-copy-over-scripts-and-test-infrastructure')`

---

## Phase 2: Developer gate (`scripts/test.sh`)

Adapt `tod/scripts/test.sh` with four targeted patches: add `set -euo pipefail` so failures stop the gate, drop `-s` from the ripgrep invocation, pass `--no-tests` to nextest, and strip the `testcfg_clean.sh` invocation. Two-AND-chain shape preserved.

### Changes

#### 1. Create `scripts/` directory and `scripts/test.sh`
**File**: `scripts/test.sh`
**Action**: create (requires `mkdir -p scripts` first)

```bash
#!/usr/bin/env bash
set -euo pipefail
echo "=== FORMAT ===" &&
cargo fmt --all &&
echo "=== CHECK ===" &&
cargo check &&
echo "=== CLIPPY ===" &&
cargo clippy --tests -- -D warnings &&
echo "=== TEST ===" &&
cargo nextest run --no-tests &&
echo "=== FORGOTTEN TODOS ===" &&
# Requires ripgrep
if rg -i -g '*.rs' 'TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:|TODO\s|todo\s' .; then
    exit 1
fi
echo "=== SUCCESS ===" &&
echo "=== Done ===."
```

**Changes from `tod/scripts/test.sh` baseline (see design.md divergences):**

| # | Change | Rationale |
|---|---|---|
| 1 | `set -euo pipefail` added at line 2 | Gate now exits non-zero on any step failure instead of always exiting 0 |
| 2 | `rg -i -s` → `rg -i` (drop `-s`) | ripgrep v15 repurposed `-s` to `--case-sensitive`, overriding `-i`; explicit case alternates in the pattern handle case |
| 3 | `cargo nextest run` → `cargo nextest run --no-tests` | With `set -e`, bare nextest exits 4 on zero tests; `--no-tests` lets the gate pass until tests are written |
| 4 | Strip `./scripts/testcfg_clean.sh` and `=== CLEANING FILES ===` | Inert script, no `.testcfg` fixtures exist, not in scope |
| 5 | Keep two-AND-chain shape (`fmt→check→clippy→nextest→rg-guard` then `SUCCESS/Done`) but now `set -e` guards both | Preserves familiar structure; `set -e` is the behavioral fix |

#### 2. No other files
No `src/` changes. No `Cargo.toml` changes. No `testcfg_clean.sh` created.

### Verification

#### Automated
- [ ] `./scripts/test.sh` exits 0 on the clean crate (all steps pass, `=== SUCCESS ===` / `=== Done ===.` printed)
- [ ] Inject `dbg!("test");` into `src/main.rs`, run `./scripts/test.sh` → exits 1, forbidden-string grep catches it
- [ ] Remove the injection, run `./scripts/test.sh` → exits 0 again (proves the guard resets)

#### Manual
- [ ] Script has `set -euo pipefail` at line 2
- [ ] Script has `cargo nextest run --no-tests` (not bare `cargo nextest run`)
- [ ] Script has `rg -i -g` (no `-s` flag)
- [ ] Script does NOT invoke `testcfg_clean.sh` or print `=== CLEANING FILES ===`

---

## Phase 3: Coverage config (`codecov.yml`)

Adapt `tod/codecov.yml` by dropping the tod-specific `ignore` section. Keep status thresholds and comment layout. Consumed by Codecov cloud service; structural verification only at this stage.

### Changes

#### 1. Create `codecov.yml`
**File**: `codecov.yml` (repo root)
**Action**: create

```yaml
coverage:
  status:
    project:
      default:
        target: auto
        threshold: 10%
        base: auto
    patch:
      default:
        target: 50%
        threshold: 2%
        base: auto
    comment:
      layout: "diff, flags, files"
      require_changes: false
```

**Changes from `tod/codecov.yml` baseline:**

| # | Change | Rationale |
|---|---|---|
| 1 | Drop `coverage.ignore` block (`tod/codecov.yml:2-7` — 5 tod-specific paths) | Destination has nothing to ignore; all 5 paths are tod-specific and don't exist here |
| 2 | Keep `coverage.status.project` with `target: auto, threshold: 10%, base: auto` | Sensible defaults |
| 3 | Keep `coverage.status.patch` with `target: 50%, threshold: 2%, base: auto` | Sensible defaults |
| 4 | Keep `comment` nested under `coverage.status` with `layout: "diff, flags, files"`, `require_changes: false` | Matches tod's exact structure; if Codecov ignores the nested placement, status checks still work |

### Verification

#### Automated
- [ ] File parses as valid YAML: `python3 -c "import yaml; yaml.safe_load(open('codecov.yml'))"` exits 0 with no error
- [ ] Expected keys present:
  ```bash
  grep -c 'project:' codecov.yml   # ≥ 1
  grep -c 'patch:' codecov.yml     # ≥ 1
  grep -c 'layout:' codecov.yml    # ≥ 1
  ```
- [ ] No `ignore:` key present:
  ```bash
  grep -c 'ignore:' codecov.yml    # 0 (must be absent)
  ```

#### Manual
- [ ] File has `coverage.status.project` with `target: auto`, `threshold: 10%`
- [ ] File has `coverage.status.patch` with `target: 50%`, `threshold: 2%`
- [ ] File has `comment.layout` and `comment.require_changes: false`
- [ ] No `coverage.ignore` section anywhere in the file

---

## Final Cross-Check

After all three phases are complete:

- [ ] `./scripts/test.sh` exits 0 (proves toolchain pin + gate work together)
- [ ] `rustup show | grep "directory override" | grep "1.98.1"` still passes (pin intact)
- [ ] `python3 -c "import yaml; yaml.safe_load(open('codecov.yml'))"` still exits 0
- [ ] `git status` shows exactly 3 new files (no `src/` or `Cargo.toml` changes):
  - `rust-toolchain.toml`
  - `scripts/test.sh`
  - `codecov.yml`

## What this plan intentionally excludes

- No CI workflows, release scripts, git hooks, Makefile, or AGENTS.md
- No `testcfg_clean.sh` — the gate was adapted to not call it
- No test code or test fixtures — this is infrastructure-only; the gate passes via `--no-tests`
- No `Cargo.toml` or `src/main.rs` changes (except temporary `dbg!` injection for verification)
- No Codecov CI wiring — `codecov.yml` is the config baseline only; upload steps come later
