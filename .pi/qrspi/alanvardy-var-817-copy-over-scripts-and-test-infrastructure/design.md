# Design Discussion

## Current State

The destination crate (`orksorksorks`) is a near-empty Rust binary — `src/main.rs` prints
"Hello, world!", `Cargo.toml` has `edition = "2024"` and no dependencies (research Q4).
There is no `scripts/`, `rust-toolchain.toml`, `codecov.yml`, `tests/`, `.github/`, or
`AGENTS.md` — three commits on a clean `main` branch.

The source project (`tod`) has mature developer tooling at its repo root:
- `scripts/test.sh` — a two-AND-chain gate script (no `set -e`) running fmt, check,
  clippy, nextest, and a forbidden-string grep guard (research Q1)
- `rust-toolchain.toml` — pins `channel = "1.97.1"` with `clippy` + `rustfmt` components
  (research Q2)
- `codecov.yml` — 22 lines configuring coverage ignores (5 tod-specific paths),
  status thresholds, and comment layout (research Q3)

The home `~/.pi/agent/AGENTS.md:78-79` convention expects every project to have a
`./scripts/test.sh` gate — run it before committing and never declare work done
while it fails (research Q4 cross-cutting).

## Desired End State

Three files at the destination repo root, adapted from tod to fit the current crate:

1. **`scripts/test.sh`** — Developer gate script that fails on real problems but passes
   on a zero-test crate. Steps: format, check, clippy, nextest (with `--no-tests`),
   forbidden-string grep guard.

2. **`rust-toolchain.toml`** — Pins `stable` (currently 1.98.1) with `clippy` +
   `rustfmt` components, matching what this machine already runs by default.

3. **`codecov.yml`** — Status thresholds and comment layout from tod, with the
   tod-specific `ignore` section dropped entirely.

**NOT included** (per user directive): release scripts, CI workflows, test fixtures,
git hooks, commit/version tooling, `testcfg_clean.sh`, the `tests/*.testcfg` fixture
convention, or an `lcov.info` ignore entry.

## Patterns to Follow

### Adopt from tod (with citations)
- **Two-AND-chain gate structure** — `tod/scripts/test.sh:2-14` (Statement A: fmt →
  check → clippy → nextest → forbidden-strings) followed by `tod/scripts/test.sh:15-18`
  (Statement B: SUCCESS banner). Keeps the familiar shape.
- **Gate steps and flags** — `cargo fmt --all` (`tod/scripts/test.sh:3`), `cargo check`
  (`:5`), `cargo clippy --tests -- -D warnings` (`:7`), forbidden-string rg pattern
  (`:11-14`). These are the standard tod verify steps.
- **Toolchain pin with components** — `tod/rust-toolchain.toml:3-5`: `[toolchain]`
  section with `channel` and `components = ["clippy", "rustfmt"]`.
- **Codecov status thresholds** — `tod/codecov.yml:9-18`: `project` at `target: auto,
  threshold: 10%` and `patch` at `target: 50%, threshold: 2%`. Sensible defaults.
- **Codecov comment layout** — `tod/codecov.yml:19-21`: `layout: "diff, flags, files"`,
  `require_changes: false`.

### Divergences (with rationale)
- **Add `set -euo pipefail`** — tod has none; this makes the gate fail on any step
  failure instead of always exiting 0 (`conventions.md` gotchas). Combined with
  `--no-tests` on nextest, the gate still passes on a zero-test crate.
- **Drop `rg -s`** — ripgrep v15 changed `-s` from `--no-messages` to
  `--case-sensitive`, overriding `-i`. The explicit case alternates in the pattern
  already handle case. Drop `-s` so `-i` works as intended.
- **`cargo nextest run --no-tests`** — tod uses bare `cargo nextest run` which exits 4
  on zero tests. With `set -e` now active, `--no-tests` lets the gate pass until real
  tests are added. The flag is harmless once tests exist.
- **No `testcfg_clean.sh` invocation** — tod's `test.sh:17` calls it; the script is
  inert and the fixture convention has zero instances. Dropped from the gate script.
- **No `ignore` section in codecov.yml** — tod's 5 ignored paths are all
  tod-specific (`tod/codecov.yml:3-7`). The destination has nothing to ignore yet.
- **Channel pinned to current stable (`1.98.1`)** — tod pins `1.97.1` but this
  machine's default is `1.98.1` (research Q2). Both satisfy edition 2024. Pin to
  what's already active to avoid an unnecessary toolchain download.

## Design Decisions

1. **test.sh: copy then patch minimally** — Start from tod's script and apply targeted
   fixes (`set -euo pipefail`, drop `-s`, `--no-tests`, strip `testcfg_clean.sh` line).
   Keeps the familiar shape and steps while fixing the known issues. A full rewrite
   would lose the two-AND-chain convention that the AGENTS.md ecosystem references.

2. **rust-toolchain.toml: pin to current stable (1.98.1)** — Matches the active
   toolchain on this machine, avoids a download, and still satisfies edition 2024.
   Components (`clippy`, `rustfmt`) match tod's convention exactly.

3. **codecov.yml: drop the ignore section** — The tod ignore list is 100% tod-specific
   paths. The status thresholds and comment layout are general-purpose and form a
   reasonable baseline. Ignores can be added later as the crate grows.

4. **testcfg_clean.sh: skip entirely** — Not in the task scope, no `.testcfg` fixtures
   exist in either repo, and the script is inert. Including it would add maintenance
   surface for a convention that has never been used.

5. **nextest: use `--no-tests`** — The gate now has `set -e`, so a nextest exit 4
   would block commits on a crate with no tests. `--no-tests` lets the gate pass while
   still running fmt/check/clippy/forbidden-strings. The flag is harmless once tests
   exist — it just suppresses the "no tests" error.

## What We're NOT Doing

- **NOT** copying CI workflows (`.github/workflows/*`) — the task explicitly excludes
  them. CI can be added later if needed.
- **NOT** copying release scripts, git hooks, commit/version tooling, or `Makefile` —
  explicitly ruled out.
- **NOT** bringing over the `tests/*.testcfg` fixture convention or `testcfg_clean.sh`.
- **NOT** adding an `lcov.info` entry to `.gitignore` — coverage generation is CI-only
  and not yet set up.
- **NOT** adding `AGENTS.md`, README, or any other non-tooling files — out of scope.
- **NOT** adding test code — the task copies infrastructure, not tests. The gate
  passes with `--no-tests` on zero tests.
- **NOT** setting up Codecov upload CI — the `codecov.yml` config is the only
  coverage artifact; CI wiring comes later.

## Open Risks

- **`cargo nextest run --no-tests` behavior**: verified on the locally installed
  version (research Q1). Future nextest versions could change the flag or exit code.
  Low risk — the flag is explicitly documented.
- **Codecov `comment` nesting**: tod's `codecov.yml:19` places `comment` under
  `coverage.status` (alongside `project`/`patch`), not under `coverage` directly.
  This may be unintentional — Codecov docs show `coverage.comment`. If Codecov
  ignores the nested placement, PR comments won't appear. Low impact — status checks
  still work, and this matches tod's exact config.
- **Toolchain pin vs. CI**: if CI is added later using `dtolnay/rust-toolchain@stable`
  (as tod does), it will ignore the `rust-toolchain.toml` pin and use whatever
  `stable` resolves to. Acceptable — local dev gets reproducibility via the pin; CI
  gets the latest stable. This matches tod's own behavior (research Q2).
- **Forbidden-string grep on new Rust editions**: the pattern covers specific strings
  (`dbg!`, `TODO:`, `FIXME`, etc.) but won't catch new debug macros or conventions
  that emerge. The pattern can be updated as needed — it's a single line in test.sh.