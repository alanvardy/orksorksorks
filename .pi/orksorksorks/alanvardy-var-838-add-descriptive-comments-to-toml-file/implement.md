# Implementation Summary

Branch: `alanvardy-var-838-add-descriptive-comments-to-toml-file` — PR #17 (draft)

## Commits

| Phase | Commit | Description |
|-------|--------|-------------|
| 1     | `7b08168` | Template asset — single source of truth (data layer) |
| 2     | `1528f35` | Write-safety guard — init becomes non-clobbering |
| 3     | `e3f3d22` | Template emission — init writes the commented template |
| 4     | — (no repo commit) | Live config — user-driven manual edit in `/Users/vardy/dev/dotfiles` (separate repo) |

All three repo commits are pushed; local == remote == `e3f3d22`.

## Automated Checks

- [x] `cargo nextest run config` passes — 75 tests incl. new `config::tests::template_deserializes_to_default` (proves `templates/default.toml` is valid TOML deserializing to `Config::default()`)
- [x] `./scripts/test.sh` passes after Phase 1 (fmt / check / clippy `-D warnings` / nextest / forbidden-strings)
- [x] `cargo nextest run init_creates_file` suite passes — new `init_refuses_to_overwrite_existing_file` exits 1 with `"source":"config-exists"` and pre-existing bytes unchanged; all 6 existing init tests still green. (Note: the literal command `cargo nextest run init_creates_file` exits "no tests to run" in this repo because nextest's filter matches test *function* names, not file suites; the suite was verified via the full `./scripts/test.sh` run and individual function filters.)
- [x] `./scripts/test.sh` passes after Phase 2 and Phase 3 — 193 of 193 nextest tests
- [x] Byte-exact assertions in `tests/init_creates_file.rs` now compare against `include_str!("../templates/default.toml")`, so `init` output and test expectation share one source of truth

## Manual Verification Items (from the plan)

- [ ] **Phase 1**: Visually confirm `templates/default.toml` renders a header, two live keys, and four commented example sections with dash banners in steps → models → scripts → prompts order
- [ ] **Phase 2**: `cargo run -- init` against a path that already exists prints `Error from config-exists:` with an "already exists; not overwriting" message and exits nonzero; the file is untouched
- [ ] **Phase 3**: `cargo run -- init` with a fresh `XDG_CONFIG_HOME`, then `cat` the emitted file: it is the full commented template, not the 42-byte default

### Phase 4 — user-driven (out-of-repo, `/Users/vardy/dev/dotfiles`)

The plan marks Phase 4 as a manual, user-driven edit — no repo code or tests involved, and the commit is made in the dotfiles repo, separate from this checkout's PR. Not performed here.

**File**: `/Users/vardy/dev/dotfiles/orksorksorks/orksorksorks.toml` (1269 lines, git-managed in the dotfiles repo)

Planned change (line numbers verified against the live file):
1. Insert a header comment block at the top (above `version = "0.1.0"`, line 1) matching Stage 1's header (describes the file, points at `src/config.rs` as the schema, notes steps → models → scripts → prompts order).
2. Insert `#`-dash banners before each section: before first `[[steps]]` (line 4), first `[[models]]` (line 53), first `[[scripts]]` (line 65), first `[[prompts]]` (line 99).
3. Normalize the stray leading space on the two existing dividers: ` # ---` → `# ---` at lines 63 and 97.
4. Hard constraints: no line removed, no trailing `key = value # note` comments, no existing step/model/script/prompt content modified (single-space-only blank lines at 64/89 left as-is).

Verification:
- [ ] `python3 -c "import tomllib,sys; tomllib.load(open(sys.argv[1],'rb'))" /Users/vardy/dev/dotfiles/orksorksorks/orksorksorks.toml` parses without error
- [ ] `git diff` in the dotfiles repo shows only added comment lines plus the two normalized dividers; existing content byte-identical except divider whitespace
- [ ] Commit is made in the dotfiles repo (user drives this)

## Notes / Observations

- **DELETEME scratch file**: the working tree contained a dangling deletion of the tracked scratch file `DELETEME` from an earlier aborted attempt of this same ticket. During the required rebase onto `origin/main` it conflicted (both sides had added it); it was resolved to upstream's copy, so this PR contains **no** `DELETEME` delta. The stale remote branch tip (`4e83fae`) was dropped during rebase and `--force-with-lease` pushed.
- The plan's literal `cargo nextest run init_creates_file` / `cargo nextest run config` commands are function-name filters in this repo; the `config` one works (function prefix `config`), the file name one does not. Full gate is `./scripts/test.sh`.