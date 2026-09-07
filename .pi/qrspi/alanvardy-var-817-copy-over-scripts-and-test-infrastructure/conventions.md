# Conventions — shared factual appendix

Source-of-truth files: `tod/scripts/test.sh`, `tod/scripts/testcfg_clean.sh`, `tod/rust-toolchain.toml`, `tod/codecov.yml`, `tod/.github/workflows/_reusable-test.yml`, `tod/AGENTS.md`, `~/.pi/agent/AGENTS.md`, tod `.pi/skills/testing/SKILL.md`, destination `Cargo.toml`.

## Canonical commands

**Local developer gate (project verify — the only gate; no Makefile exists in either repo):**
```
./scripts/test.sh          # repo-root relative; tod/scripts/test.sh
```
Contents (use these when re-implementing individual steps):
| Step | Command | Ref |
|---|---|---|
| Format | `cargo fmt --all` | tod/scripts/test.sh:3 |
| Check | `cargo check` | tod/scripts/test.sh:5 |
| Clippy | `cargo clippy --tests -- -D warnings` | tod/scripts/test.sh:7 |
| Tests | `cargo nextest run` (bare; **no `--no-tests`**) | tod/scripts/test.sh:9 |
| Forbidden strings | `rg -i -s -g '*.rs' 'TODO:\|todo:\|FIXME\|fixme\|dbg!\|DEBUG:\|FIXTURE:\|TODO\s\|todo\s' .` → exit 1 on match | tod/scripts/test.sh:11-14 |
| Cleanup | `./scripts/testcfg_clean.sh` | tod/scripts/test.sh:17 |

**CI equivalents** (tod `.github/workflows/`; not via test.sh — steps are inlined):
- `cargo nextest run --profile ci --all-features` — `_reusable-test.yml:65`
- `cargo llvm-cov nextest --profile ci --all-features --lcov --output-path lcov.info` (coverage runs, PR-only) — `_reusable-test.yml:67`
- Toolchain in CI: `dtolnay/rust-toolchain@stable` (no channel input; `components: llvm-tools-preview` when coverage) — `_reusable-test.yml:52-54`; nextest via `taiki-e/install-action@v2` — `:57`
- Required rustup components for the pinned toolchain: `clippy`, `rustfmt` — tod/rust-toolchain.toml:5

## Test-suite inventory

**Destination crate (this repo) — no test infrastructure exists today:**
- No `tests/` dir, no test code in `src/main.rs:1-3` (Hello-world binary), no `Cargo.toml` `[dev-dependencies]`, no `Cargo.lock`.
- Consequence: the gate's `cargo nextest run` step fails (exit 4, `error: no tests to run`); all other steps pass trivially; overall gate still exits 0 (see Gotchas).

**tod (source of conventions) — current state relevant to copying:**
- `tests/` dir exists; currently contains **0** `*.testcfg` fixture files.
- `.testcfg` convention: fixtures named `*.testcfg` under `tests/`, deleted by `testcfg_clean.sh:6,10`, gitignored at `tod/.gitignore:9`.
- Coverage test runs: `cargo llvm-cov nextest --profile ci ...` (CI-only; requires `llvm-tools-preview` component + `cargo-llvm-cov`); emits root `lcov.info` (gitignored, `tod/.gitignore:6`).
- Workspace member `crates/tod-e2e/` exists but is **not covered** (no `SF:` entries in `lcov.info`; not in `codecov.yml` ignores).

**Platform gating:** CI test matrix is `os: ${{ fromJSON(inputs.os_matrix) }}` (`_reusable-test.yml:49-50`) — OS gating lives in caller inputs (`ci-pr.yml`, `ci-release.yml`); locally tests run on macOS (`aarch64-apple-darwin` rustup host). No `#if os(...)`/cfg-gating observed in the tooling files. Coverage upload is gated on `upload_coverage: true` — only `ci-pr.yml:25-31` sets it.

## Build / verify gotchas (verified)

- **`./scripts/test.sh` always exits 0** — two `&&`-statements (`test.sh:2-14`, `:15-18`), no `set -e`. Failing steps only suppress the rest of Statement A; `=== SUCCESS ===`/`=== CLEANING FILES ===`/`./scripts/testcfg_clean.sh`/`=== Done ===.` always run. Exit 1 only via the rg guard at `test.sh:13`, which is **unreachable until `cargo nextest run` succeeds**. Do not use test.sh's exit code for CI-like gating with zero tests.
- **`cargo nextest run` fails on zero tests** (exit 4, hint `use --no-tests to customize`); test.sh does not pass `--no-tests` (`test.sh:9`). CI's coverage run (`_reusable-test.yml:67`) would likewise fail without tests.
- **testcfg_clean.sh always exits 0** (`testcfg_clean.sh:13` final echo): missing `tests/` dir → stderr `find: ./tests/: No such file or directory`, still counts 0 and prints `No files to delete` (`testcfg_clean.sh:6,13`).
- **Ripgrep `-s` changed meaning in v15** (15.2.0 installed): `--no-messages` (≤14) → `--case-sensitive` (15), which overrides `-i` in the guard; the guard's explicit case alternates catch canonical strings.
- **No tool-presence checks** in `test.sh`; missing `rg` makes the guard silently pass; missing clippy/nextest prints `error: no such subcommand` and is masked by the exit-0 gate.
- **Toolchain resolution**: a repo-root `rust-toolchain.toml` overrides the default for everything under that tree; destination currently has none and uses **stable rustc 1.98.1** (`rustup show` — active, default). `1.97.1` is installed system-wide but not active in the destination. Edition 2024 (both manifests) requires ≥ 1.85 — satisfied by 1.97.1 and 1.98.1.
- **Coverage output path** `lcov.info` is hardcoded (`_reusable-test.yml:67,73`) and gitignored (`tod/.gitignore:6`); a stale untracked `tod/lcov.info` exists with absolute dev-machine `/Users/vardy/dev/tod/src/...` SF paths.
- **test.sh is not run by CI** — CI inlines the steps (`_reusable-test.yml:65-67`); `test.sh` is referenced only in AGENTS.md/skills/copilot-instructions (`tod/AGENTS.md:6,32`, `tod/.pi/skills/testing/SKILL.md:22,31`, `tod/.github/copilot-instructions.md:13`, `~/.pi/agent/AGENTS.md:78-79`, `delegation/SKILL.md:49`, `qrspi/SKILL.md:115`, `mutants/SKILL.md:145,150`).

## File-location conventions discovered

- Gate script must be at repo root `./scripts/test.sh` (invoked as `./scripts/test.sh`, `tod/scripts/test.sh:17` calls `./scripts/testcfg_clean.sh` relative to root; home `~/.pi/agent/AGENTS.md:78-79`).
- `rust-toolchain.toml` / `codecov.yml`: **no convention prescribes a location** — zero `toolchain`/`codecov` matches across home skills, tod skills, and both AGENTS.md. Standard rustup (repo-root override) and Codecov (repo-root config) behavior applies.
- Destination currently has **none** of: `scripts/`, `rust-toolchain.toml`, `codecov.yml`, `.github/`, `AGENTS.md`, `Makefile`.