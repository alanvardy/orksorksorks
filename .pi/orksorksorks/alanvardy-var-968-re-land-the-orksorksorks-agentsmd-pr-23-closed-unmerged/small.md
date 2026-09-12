# Task

Land the orksorksorks repo `AGENTS.md` on this branch and open a PR, so agents running orksorksorks get project instructions from `main` (currently no `AGENTS.md` exists there — the previous attempt, PR #23 on branch `alanvardy-var-854-core-docs-readme-license-contributing-security-changelog`, was closed unmerged on 2026-09-09).

Source: copy the file from the existing branch — `git show alanvardy-var-854-core-docs-readme-license-contributing-security-changelog:AGENTS.md` (~120 lines) — then apply these changes:

1. **Stack line + the real name.** It is `orksorksorks`, not `orksworksorks` (agents typed the typo; the `write` tool then created `.pi/orksworksorks` parents, silently mis-routing the pipeline).
2. **Ticket-worktree subsection.** Branch/worktree layout: ticket branches are worktrees `~/dev/alanvardy-var-<n>-<slug>`. Include the `DELETEME` bootstrap marker — never commit it; it must not reach `main` (it already did once via docs merge `bc53ab8e`). Artifact directory is `.pi/orksorksorks/<branch>/`.
3. **`.ignore` visibility note.** This file has its own `.ignore` (separate from `.gitignore`).
4. **`### Closing a ticket`** — ordered teardown: `gh pr ready` (check `isDraft` before merging) → `gh pr merge --rebase --delete-branch` → verify `mergedAt` → `git -C <root> worktree remove` → `branch -D` → archive the Linear ticket. Circuit breaker: never `git worktree remove` the directory you are standing in — `Working directory does not exist` is fatal for the whole session (bash validates session cwd before spawning; `cd <root> &&` does not recover it).
5. **Language identity** — this is a Rust project (`Cargo.toml`, `src/`, `tests/`).

Leanness budget (net ≈ −2 lines): shrink the existing `§Environment variables` (~10 lines) and `§CI equivalents` (~7 lines) sections to one-line pointers at the README/CONTRIBUTING. Note: those docs exist only on the `alanvardy-var-854-…` branch, not on `main` — point at their paths anyway; they land separately.

Do not duplicate home conventions — link to `~/.pi/agent/AGENTS.md` for shell/commit/editing rules instead of restating them.

Open / update the PR for this branch with the change. Run the repo's test gate (`./scripts/test.sh`) before committing.

## Why SMALL
Single file (AGENTS.md), content already written on the `854` branch; no schema/API change, no shared-code risk, no design decision, no tests — all of A–F hold.

## Key files
- `AGENTS.md` — to create at repo root; source of truth: `git show alanvardy-var-854-core-docs-readme-license-contributing-security-changelog:AGENTS.md`
- Reference (link, don't copy): `~/.pi/agent/AGENTS.md` for home conventions
- Beware commit `bc53ab8e` (previous `DELETEME` leak) — verify the committed file contains no `DELETEME` marker before pushing