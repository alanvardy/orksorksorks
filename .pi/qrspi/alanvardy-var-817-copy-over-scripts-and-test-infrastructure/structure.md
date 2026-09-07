# Structure Outline

## Approach

Copy three tooling files from `tod` to the destination `orksorksorks` crate, with
targeted patches rather than rewrites: pin the toolchain to current stable, add
`set -euo pipefail` + `--no-tests` to the gate so it fails on real problems but
passes on a zero-test crate, and drop the tod-specific `ignore` section from
`codecov.yml`. Each file lands fully verified before the next is touched.

## Stage 1: Toolchain pin (`rust-toolchain.toml`)

What: Pin the compiler version for this crate so every subsequent cargo command
(fmt, check, clippy, nextest) runs under a known-good toolchain. This is the
foundation — all later verification steps depend on a reproducible compiler.

**Files**: `rust-toolchain.toml` (new, repo root)

**Key changes**:
- `[toolchain]` section with `channel = "1.98.1"` and `components = ["clippy", "rustfmt"]`
  — matches the active stable on this machine, satisfies edition 2024 (≥ 1.85)

**Contract**: Any `cargo`/`rustc`/`rustup` invocation in this directory tree uses
`1.98.1-aarch64-apple-darwin` instead of the system default. Consumed by: rustup
directory override — no explicit import by other files.

**Tests**: No programmatic test file. Verification is manual:
- `cargo check` exits 0 under the pinned toolchain
- `rustup show` (run from repo root) reports `active toolchain: 1.98.1-aarch64-apple-darwin (directory override)`

**Verify**:
```
cargo check                          # must pass (exit 0)
rustup show | grep "directory override"  # must show 1.98.1
```

---

## Stage 2: Developer gate (`scripts/test.sh`)

What: The `./scripts/test.sh` gate — format, check, clippy, nextest, and
forbidden-string grep — with `set -euo pipefail` so any failure stops the gate,
and `--no-tests` on nextest so the gate passes on a zero-test crate. Built on
Stage 1's toolchain pin.

**Files**: `scripts/test.sh` (new)

**Key changes** (from `tod/scripts/test.sh` baseline):
- Add `set -euo pipefail` at line 2 — any step failure now exits non-zero
- Drop `-s` from the rg invocation (`rg -i -g '*.rs' '...' .`) — ripgrep v15
  repurposed `-s` to `--case-sensitive`, which overrides `-i`
- `cargo nextest run --no-tests` — suppresses exit-4 on zero tests; harmless once
  tests exist
- Strip the `./scripts/testcfg_clean.sh` invocation — inert, no `.testcfg`
  fixtures exist, not in scope
- Keep the two-statement shape (`fmt→check→clippy→nextest→rg-guard` then
  `SUCCESS/Done` banner) but now both are guarded by `set -e`

**Contract**: `#!/usr/bin/env bash` script at `./scripts/test.sh`. Exit 0 = clean;
exit non-zero = problem found. Consumed by: developers via `./scripts/test.sh`,
`AGENTS.md` convention (`~/.pi/agent/AGENTS.md:78-79`).

**Tests**: Manual gate-run verification — happy path and one sad path:
1. Run `./scripts/test.sh` on the current (clean) crate → exit 0, all steps pass,
   `=== SUCCESS ===` / `=== Done ===.` printed
2. Inject `dbg!("test");` into `src/main.rs`, run gate → exit 1, forbidden-string
   guard catches it
3. Remove the injection, run gate → exit 0 again (proves the guard resets)

**Verify**:
```
./scripts/test.sh                   # exit 0 on clean crate
# After injecting dbg! into src/main.rs:
./scripts/test.sh                   # exit 1, grep catches dbg!
# After removing dbg!:
./scripts/test.sh                   # exit 0 again
```

---

## Stage 3: Coverage config (`codecov.yml`)

What: Codecov status thresholds and PR comment layout, adapted from `tod` with the
tod-specific `ignore` section dropped. This file is consumed by the Codecov cloud
service — not by local tooling — so verification is structural (valid YAML,
recognized keys). Full end-to-end verification requires CI wiring, which is out of
scope.

**Files**: `codecov.yml` (new, repo root)

**Key changes** (from `tod/codecov.yml` baseline):
- Drop `coverage.ignore` entirely (5 tod-specific paths; destination has nothing to ignore)
- Keep `coverage.status.project` with `target: auto, threshold: 10%`
- Keep `coverage.status.patch` with `target: 50%, threshold: 2%`
- Keep `comment` with `layout: "diff, flags, files"`, `require_changes: false`
  — nested under `coverage.status` matching tod's exact structure (not moved to
  `coverage.comment`)

**Contract**: YAML config at repo root. Consumed by: `codecov/codecov-action@v7`
during CI upload (future — not wired in this task). Validates structurally now;
threshold behavior verified when CI + Codecov integration is added later.

**Tests**: Structural validity check:
- File parses as valid YAML (`python3 -c 'import yaml; yaml.safe_load(open(...))'`
  or equivalent)
- Expected keys present: `coverage.status.project`, `coverage.status.patch`,
  `comment.layout`, `comment.require_changes`
- No `ignore` key present

**Verify**:
```
# Valid YAML
python3 -c "import yaml; yaml.safe_load(open('codecov.yml'))"  # exits 0, no error

# Key presence
grep -c 'project:' codecov.yml     # ≥ 1
grep -c 'patch:' codecov.yml       # ≥ 1
grep -c 'layout:' codecov.yml      # ≥ 1
grep -c 'ignore:' codecov.yml      # 0 (must be absent)
```

---

## Testing Checkpoints

| After stage | What must be green | Command |
|---|---|---|
| 1 — Toolchain | `cargo check` passes; rustup shows directory override | `cargo check && rustup show \| grep "directory override"` |
| 2 — Gate | Gate exits 0 on clean crate; catches injected forbidden string | `./scripts/test.sh && echo "PASS"` then inject/remove cycle |
| 3 — Coverage | Valid YAML; expected keys present; no `ignore` | `python3 -c "import yaml; yaml.safe_load(open('codecov.yml'))"` |

## Cross-Cutting Notes

- **No test code is added** — the task copies infrastructure, not tests. The gate
  passes via `--no-tests` until real tests are written later. This is a deliberate
  design decision, not a gap.
- **All three files are repo-root artifacts** — no `src/` changes, no `Cargo.toml`
  changes, no dependency additions. The existing `src/main.rs` (Hello World) is
  untouched except during the Stage 2 sad-path injection test.
- **CI wiring is out of scope** — `codecov.yml` is placed now as a baseline config;
  the `codecov/codecov-action` upload step and `lcov.info` generation are future
  work. The file is structurally verified but not end-to-end tested against the
  Codecov service.
- **`codecov.yml` comment nesting**: `tod` places `comment` under `coverage.status`
  (sibling of `project`/`patch`), not under `coverage` directly. This is preserved
  as-is. If Codecov ignores the nested placement, PR comments won't appear but
  status checks still work. Low-impact, matches tod exactly.
