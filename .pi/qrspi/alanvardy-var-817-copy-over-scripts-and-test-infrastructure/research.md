# Research Findings

Context: two Rust projects — mature CLI `tod` (`/Users/vardy/dev/tod`) with full toolchain, and a fresh near-empty crate (destination, this repo). Questions cover three tod tooling pieces (`scripts/test.sh`, `rust-toolchain.toml`, `codecov.yml`) and the destination baseline. All findings verified by targeted reads of the cited regions; one agent contradiction (build toolchain) resolved in favor of direct `rustup show` output below.

## Q1: test.sh mechanics and dependencies

### Findings
- `tod/scripts/test.sh` is 18 lines, `#!/usr/bin/env bash`, **no `set -e`**. It parses as exactly **two `&&`-statements**: Statement A = `test.sh:2-14`, Statement B = `test.sh:15-18`. A failure in A does not abort the script — Statement B always runs, so the script **always exits 0** (the last statement's exit code). The only hard-fail path is `exit 1` at `test.sh:13` (an `if` operand of A), reachable only when everything in A up to it succeeded.
- Ordered steps (all refs `tod/scripts/test.sh`):
  1. `:3` `cargo fmt --all` (rustfmt, all packages + path deps)
  2. `:5` `cargo check` (no flags)
  3. `:7` `cargo clippy --tests -- -D warnings` (external cargo-clippy; `--tests` includes test targets; `-D warnings`)
  4. `:9` `cargo nextest run` (external cargo-nextest, default profile, **no `--no-tests` flag**)
  5. `:10` echo `=== FORGOTTEN TODOS ===` — only if step 4 succeeded (same AND-list)
  6. `:11-14` `if rg -i -s -g '*.rs' 'TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:|TODO\s|todo\s' .; then exit 1; fi` — only if step 4 succeeded; `-i` ignore-case, `-g '*.rs'` limits to Rust files, `.` = whole repo; on match → `exit 1` (`:13`)
  7. `:15-18` `echo === SUCCESS ===` / `echo === CLEANING FILES ===` / `./scripts/testcfg_clean.sh` / `echo === Done ===.` — always run, independent of A
- **External tools and checks**: **no presence checks anywhere** in the script; the only acknowledgment is the comment `# Requires ripgrep` at `test.sh:11`. Required: cargo rustfmt, cargo-clippy (`:7`), cargo-nextest (`:9`), ripgrep (`:12`), plus coreutils find/wc/rm in the invoked script. If a tool is missing: clippy/nextest produce `error: no such subcommand` → chain breaks → failure **masked** (script still prints SUCCESS/Done, exit 0); missing `rg` → exit 127 → `if` false → guard **silently passes**. `docs/contributing.md:7` documents `cargo install cargo-nextest`; CI installs nextest via `taiki-e/install-action@nextest` (`e2e_todoist.yml:26`).
- **Only one other script invoked**: `./scripts/testcfg_clean.sh` at `test.sh:17`. It counts `find ./tests/ -name "*.testcfg" | wc -l` (`testcfg_clean.sh:6`), and if > 0 runs `rm ./tests/*.testcfg` (`:10`) else prints `No files to delete` (`:13`); no `set -e`, final echo ⇒ **always exits 0**. Missing `./tests/` dir is non-fatal (stderr `find: ./tests/: No such file or directory`, count = 0). tod currently has 0 `*.testcfg` files.
- **No CI workflow invokes `test.sh`** — `.github/workflows/*` replicate the steps inline; the only mention is prose in `.github/copilot-instructions.md:13`.
- **Empty-crate behavior** (empirically verified in a byte-identical `/tmp` sandbox on this machine — cargo 1.98.1, cargo-nextest, cargo-clippy, rg 15.2.0, no tests/):
  - `cargo fmt --all` → exit 0 (src/main.rs exists; would fail with `no targets specified in the manifest` only with zero targets)
  - `cargo check` → exit 0
  - `cargo clippy --tests -- -D warnings` → exit 0 (no lints; `--tests` adds nothing without test targets)
  - `cargo nextest run` → **exit 4, `error: no tests to run`** (hint: `use --no-tests to customize`) after `Starting 0 tests across 1 binary` — the only genuinely failing step
  - FORGOTTEN TODOS echo (`:10`) and rg guard (`:11-14`) → **skipped** (chain broke at nextest)
  - `:15-18` still print; **overall exit code 0** — the nextest failure is masked
  - rg guard, if reached: `Hello, world!` matches nothing → passes; **ripgrep 15.2.0 changed `-s` to mean `--case-sensitive`** (was `--no-messages` ≤14), which overrides `-i`; the pattern's explicit case alternates (`TODO:`/`todo:` etc.) are what catch strings
  - `./scripts/testcfg_clean.sh` → exit 0, `No files to delete`

## Q2: rust-toolchain.toml

### Findings
- `tod/rust-toolchain.toml` is 6 lines, exactly: comment `:1`, `[toolchain]` `:3`, `channel = "1.97.1"` `:4`, `components = ["clippy", "rustfmt"]` `:5`. **No `profile`, no `targets`, no other keys.** One copy in the repo; no legacy `rust-toolchain` file.
- **rustup consumption**: a `rust-toolchain.toml` at repo root acts as a directory override for all rustup/cargo/rustc invocations in that tree, taking precedence over the default toolchain; bare `1.97.1` maps to `1.97.1-<host-triple>`. `RUSTUP_TOOLCHAIN` is **never set** anywhere in tod (grep: no matches). No `.cargo/config.toml` override. CI uses `dtolnay/rust-toolchain@stable` with **no channel input** (only `components`) — e.g. `_reusable-lint.yml:23,36,50`, `_reusable-test.yml:52`, `ci-secure.yml:37,65` — so CI's effective toolchain is `stable`, not the pinned 1.97.1; `rustup toolchain install` with no channel appears in `release_linux.yml:93`, `release_macos.yml:40`, `release_windows.yml:40`.
- `tod/Cargo.toml`: `name = "tod"` (`:2`), `version = "0.18.0"` (`:3`), `edition = "2024"` (`:5`), **no `rust-version`** key anywhere.
- Edition 2024 requires rustc ≥ 1.85.0; pinned channel 1.97.1 satisfies it.
- **Destination**: `Cargo.toml` (6 lines): `[package]` `:1`, `name = "orksorksorks"` `:2`, `version = "0.1.0"` `:3`, `edition = "2024"` `:4`, empty `[dependencies]` `:6`. No `rust-version`. **No `rust-toolchain`/`rust-toolchain.toml` file** (find: none).
- Destination currently builds with **stable = rustc 1.98.1** (verified: `rustc --version` → `rustc 1.98.1 (48a229cea 2026-09-01)`; `rustup show` → active toolchain `stable-aarch64-apple-darwin` "because: it's the default toolchain"). Installed toolchains include `1.97.1-aarch64-apple-darwin` — present but **not active** in the destination.
- **Compatibility**: pinned 1.97.1 satisfies edition 2024 (≥ 1.85); destination has no MSRV beyond the edition, so no additional constraint. Destination's current default (1.98.1) also satisfies edition 2024.

## Q3: codecov.yml configuration and consumers

### Findings
- `tod/codecov.yml` (22 lines) configures exactly:
  - `coverage.ignore` (`codecov.yml:2-7`): five paths — `src/errors.rs` `:3`, `src/input.rs` `:4`, `src/format.rs` `:5`, `src/debug.rs` `:6`, `src/main.rs` `:7` — **all tod-specific, all verified to exist** in `tod/src/`
  - `coverage.status.project.default` (`codecov.yml:9-13`): `target: auto`, `threshold: 10%`, `base: auto`
  - `coverage.status.patch.default` (`codecov.yml:14-18`): `target: 50%`, `threshold: 2%`, `base: auto`
  - `comment` (`codecov.yml:19-21`): `layout: "diff, flags, files"`, `require_changes: false` — literally indented 4 spaces, i.e. nested as `coverage.status.comment` (sibling of project/patch), not `coverage.comment`
  - **Absent**: `flags` section, `after_n_builds`, `behavior`, `require_changes: true`, token, anything else
- **Production of coverage data — CI only, single producer** (`tod/.github/workflows/_reusable-test.yml`):
  - `:54` rustup component `llvm-tools-preview` via `dtolnay/rust-toolchain@stable` when `upload_coverage` input is true
  - `:57` `taiki-e/install-action@v2` installs `cargo-llvm-cov,nextest` (or just `nextest`)
  - `:61` Swatinem/rust-cache shared-key switches `coverage` vs `test`
  - `:67` `cargo llvm-cov nextest --profile ci --all-features --lcov --output-path lcov.info` (guarded by `if: ${{ inputs.upload_coverage }}` at `:66`); non-coverage path runs `cargo nextest run --profile ci --all-features` (`:65`)
  - Emitted file `lcov.info` at repo root: 15,731 lines, all **43 `SF:` entries absolute paths under `/Users/vardy/dev/tod/src/`** (`lcov.info:1` = `SF:/Users/vardy/dev/tod/src/cargo.rs`); zero entries for `crates/` or `bucket/`
  - `forge.yaml` has no coverage content (`forge.yaml:1-3` only schema comment, `variables: operating_agent: muse`, `model: gpt-5-mini`); **no Makefile** in tod; no coverage in `tod/scripts/`
- **Uploader**: `codecov/codecov-action@v7` at `_reusable-test.yml:68-74` — `token: ${{ secrets.CODECOV_TOKEN }}` (`:72`), `files: lcov.info` (`:73`), `disable_search: true` (`:74`). A second codecov-action (`:75-86`) uploads nextest JUnit test results (`files: target/nextest/ci/junit.xml`, `report_type: test_results`, `flags: nextest`). `CODECOV_TOKEN` declared as optional `workflow_call` secret at `_reusable-test.yml:31`.
  - **Callers**: only `ci-pr.yml:25-31` sets `upload_coverage: true` (plus `upload_codecov: true`, `skip_codecov_for_fork_prs: true`, `secrets: inherit`). `ci-push.yml:25-28` and `ci_dependabot.yml:27-30` set `upload_codecov: false`; `ci-release.yml:47-50` matrix but `upload_codecov: false` and no `upload_coverage` key (default false).
  - Public Codecov badge query token `9FBJK1SU0K` in `tod/README.md:2`.
- **Hardcoded layout assumptions**:
  - `codecov.yml` ignore paths assume a flat root-level `src/*.rs` layout — no `crates/` or `bucket/` paths (`codecov.yml:3-7`); note `src/tasks/format.rs` exists and is *not* ignored
  - `lcov.info` SF headers hardcode the absolute path prefix `/Users/vardy/dev/tod/src/...` — coverage scoped entirely to `src/`; workspace member `crates/tod-e2e/` and `bucket/` never instrumented
  - `target/nextest/ci/junit.xml` hardcoded at `_reusable-test.yml:76`
  - `lcov.info` output path hardcoded at `_reusable-test.yml:67` and gitignored at `tod/.gitignore:6` (also `tests/*.testcfg` at `.gitignore:9`, `tarpaulin-report.html` at `.gitignore:5`)
  - Current `tod/lcov.info` is an untracked/ignored local artifact (absolute dev-machine paths)

## Q4: Destination baseline and local conventions

### Findings
- **Destination inventory** (all paths in this repo):
  - `Cargo.toml:1-6` — `name = "orksorksorks"`, `version = "0.1.0"`, `edition = "2024"`, empty `[dependencies]`; **no** `description`, `license`, `repository`, `rust-version`; no `Cargo.lock` file
  - `src/main.rs:1-3` — `fn main() { println!("Hello, world!"); }`
  - `.gitignore:1` — `/target`
  - `linear-project.md:1` — `orksorksorks` (Linear project name); no README
  - `.pi/qrspi/alanvardy-var-817-copy-over-scripts-and-test-infrastructure/` — contains `task.md` and `questions.md` (QRSPI artifacts); **no `.pi/skills/`**
  - **Absent**: `.github/`, `forge.yaml`, `scripts/`, `rust-toolchain.toml`, `codecov.yml`, `AGENTS.md`, `Makefile`, `tests/`
  - Git: a worktree — `.git` is a gitfile pointing to `/Users/vardy/dev/orksorksorks/.git/worktrees/alanvardy-var-817-copy-over-scripts-and-test-infrastructure`; remote `origin https://github.com/alanvardy/orksorksorks.git`; branches `main`, `alanvardy-var-816-init`, current branch; 3 commits, clean tree, no tags
- **Conventions regarding `./scripts/test.sh`, toolchain, coverage**:
  - Home `~/.pi/agent/AGENTS.md:78-79` — "Run the project's `./scripts/test.sh` gate before committing — never declare work done while it fails" (repo-independent convention: gate script must live at repo root `./scripts/test.sh`)
  - `tod/AGENTS.md:6` — "No `dbg!`, `TODO`, `FIXME`, `DEBUG:`, or `FIXTURE:` strings anywhere in `.rs` files — `scripts/test.sh` greps for these and fails the build"; `tod/AGENTS.md:32` — "Run `scripts/test.sh` before committing — it covers `cargo fmt --check`, `cargo check`, `cargo clippy`, `cargo test`, and forbidden-string grep"
  - `tod/.pi/skills/testing/SKILL.md:22` — "`./scripts/test.sh` # Full verification: format, check, clippy, tests, forgotten-strings"; `:31` — "`./scripts/testcfg_clean.sh`"
  - Home skills referencing the gate: `delegation/SKILL.md:49`, `qrspi/SKILL.md:115`, `mutants/SKILL.md:145,150`
  - **Zero matches** for `toolchain` or `codecov` in any AGENTS.md or skill (home `~/.pi/agent/skills/*/SKILL.md`, `tod/.pi/skills/*/SKILL.md`, both AGENTS.md) — **no convention prescribes a location for `rust-toolchain.toml` or `codecov.yml`**; their placement is determined only by the standard rustup/Codecov behaviors (repo root)
  - `tod/.github/copilot-instructions.md:13` also references `scripts/test.sh` ("never prefix with `DEBUG:` as `scripts/test.sh` rejects that string")
- **No Makefile** in tod; `tod/forge.yaml` (`:1-3`) does not reference scripts, coverage, or cargo
- tod `scripts/` directory contents: `auto_release.sh, create_pr.sh, manual_test.sh, push_aur.sh, release.sh, setup_aur.sh, test.sh, testcfg_clean.sh`

## Cross-Cutting Observations
- **Gate semantics**: `test.sh` is a *developer* gate with no `set -e`, always exits 0, and only fails via the rg guard (`test.sh:13`) — which is unreachable until nextest passes (i.e., until real tests exist). A zero-test crate still "passes" the gate. CI does **not** use `test.sh`; it re-implements the cargo steps inline (`_reusable-test.yml:65-67`) with an exit-4-on-zero-tests nextest invocation.
- **Relative-script dependency**: `test.sh:17` calls `./scripts/testcfg_clean.sh` relative to the repo root, so the gate requires `scripts/` to live there; the home AGENTS.md (`~/.pi/agent/AGENTS.md:78-79`) likewise assumes `./scripts/test.sh`.
- **`tests/*.testcfg` convention**: `testcfg_clean.sh:6,10` and `tod/.gitignore:9` assume test-config fixtures named `*.testcfg` under `tests/` — currently zero exist in tod.
- **Coverage is CI-only and single-source**: produced at `_reusable-test.yml:67` and uploaded at `:73`; locally only an untracked `lcov.info` artifact exists. All tod-specific assumptions (ignore list, absolute `src/` SF paths) would not match a different crate layout.
- **Edition 2024 alignment**: destination `edition = "2024"` is satisfied by both the pinned tod toolchain (1.97.1) and the destination's default (stable 1.98.1) — no MSRV key exists in either manifest.

## Open Areas
- Whether Codecov honors `comment` nested under `status` (`codecov.yml:19`) — only literal structure was observed; effect unverified.
- `dtolnay/rust-toolchain@stable`'s internal handling of a repo `rust-toolchain.toml` is not vendored in tod; tod CI in practice uses `stable`.
- nextest's exit code 4 on zero tests verified only with the locally installed cargo-nextest version.
- Destination repo on GitHub has no CI configuration visible locally; whether `CODECOV_TOKEN` or other secrets exist in the GitHub settings is unknown.
- What a `*.testcfg` fixture contains and how test code is expected to consume it — no fixtures or test code exist in either repo today (tod's `tests/` dir has no `*.testcfg`).