# Phase 1 Output — Baseline freeze (no code)

Status: **COMPLETE** — no commit made (no code changes; baseline/ is artifact-only).

## scripts/test.sh (pristine tree)
PASSED. Final lines:

```
     Summary [   0.751s] 197 tests run: 197 passed, 0 skipped
=== TODO/FIXME/dbg gate ===
=== SUCCESS ===
```

## wc -l src/commands/mod.rs
`1459` — matches expectation.

## nextest summary → baseline/nextest.txt
```
        PASS [   0.008s] (195/197) ... git::tests::parse_branch_output_trims_trailing_newline
        PASS [   0.082s] (196/197) ... git::tests::current_branch_in_returns_configured_branch
        PASS [   0.095s] (197/197) ... git::tests::current_branch_in_detached_head_errors
────────────
     Summary [   0.752s] 197 tests run: 197 passed, 0 skipped
```

## baseline/ captures
```
total 56
branch.json      51 bytes   {"data":"alanvardy-var-981-refactor-orksorksorks"}
help.txt        899 bytes
nextest.txt     459 bytes
script-help.txt 445 bytes
step-help.txt   389 bytes
step-missing.json 156 bytes  {"error":{"message":"could not read config file at /nonexistent/orks.toml (specified via --config): No such file or directory (os error 2)","source":"io"}}
step-missing.txt 135 bytes
```
All captured with `cargo build --quiet` first; nonexistent-config invocations exit 1 as expected and capture the envelope/error cleanly.

## git status
```
?? .pi/orksorksorks/alanvardy-var-981-refactor-orksorksorks/
```
`git diff --stat` is empty — **no source edits**. Note: the artifact dir is excluded via `.ignore`, not `.gitignore` (repo `.gitignore` only has `/target`), so it shows as a single untracked directory — exactly the expected "only the new untracked baseline dir" state.

## Manual eye-check (for user confirmation)
`baseline/help.txt` lists all eight subcommand names: `init`, `branch`, `artifact_directory`, `step`, `model`, `thinking`, `prompt`, `script` (plus `help`).
The `LONG_VERSION` block is not in `--help` output; it is emitted by `--version` (`orksorksorks 0.1.0` + `build target: aarch64-apple-darwin` + `build profile: debug` + `build timestamp: 2026-09-12T20:39:47.218622+00:00`) — verified present.

## plan.md
All 5 Phase 1 automated checkboxes marked `[x]`; the Manual item left `[ ]` per instructions.

## Notes
- `.ignore` (not `.gitignore`) hides `.pi/orksorksorks`; status shows the artifact dir as untracked, matching "no source edits" intent.
- No commit: phase explicitly makes no code changes and baseline is artifact-only.