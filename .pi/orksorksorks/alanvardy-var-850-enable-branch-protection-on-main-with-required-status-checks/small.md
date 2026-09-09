# Task

Enable branch protection on `main` for `alanvardy/orksorksorks` and align the repo's merge-mode settings with the documented `--rebase`-only convention. This is security remediation #2 of the audit: `gh api repos/alanvardy/orksorksorks/branches/main/protection` currently returns 404, so `main` is unprotected — force-pushes, review-less merges, and squash/merge-commit merges are all possible.

Run as admin (exact commands from the ticket):

```bash
gh api -X PUT repos/alanvardy/orksorksorks/branches/main/protection \
  -f required_status_checks[strict]=true \
  -f 'required_status_checks[checks][][context]=lint' \
  -f 'required_status_checks[checks][][context]=test' \
  -f enforce_admins=true \
  -f required_pull_request_reviews[required_approving_review_count]=1 \
  -f required_linear_history=false

gh api -X PATCH repos/alanvardy/orksorksorks \
  -f allow_squash_merge=false -f allow_merge_commit=false -f delete_branch_on_merge=true
```

Do not touch `allow_rebase_merge` (it must stay `true` — rebase-only is the convention). No repo files need changing.

Verify afterwards: (1) `gh api repos/alanvardy/orksorksorks/branches/main/protection` shows the required checks `lint`/`test`, strict, enforce_admins, 1 approving review, linear history off; (2) `gh api repos/alanvardy/orksorksorks` shows `allow_squash_merge=false`, `allow_merge_commit=false`, `allow_rebase_merge=true`, `delete_branch_on_merge=true`.

## Why SMALL

Single surface (GitHub repo config — zero source files touched), exact commands provided by the ticket (no design or sign-off needed), no schema/migration, no new subsystem, and verification is a few local read-only GETs. The one real unknown — will the `lint`/`test` required checks ever pass? — is already answered: the CI produces exactly those check contexts, so A–F all hold.

## Key files (if the recon found any)

- `.github/workflows/ci-pr.yml` — runs on `pull_request` to `main` with two jobs named `lint` and `test`; these job names ARE the check contexts the protection requires, and they reference the existing reusable workflows `_reusable-lint.yml` / `_reusable-test.yml`. No workflow changes needed.
- Verified current state: protection GET → 404 `Branch not protected`; repo settings `allow_squash_merge=true, allow_merge_commit=true, allow_rebase_merge=true, delete_branch_on_merge=false, default_branch=main`.
- Branch note: the only commit on this branch touches `DELETEME` (a placeholder, not part of the task — ignore it; do not merge it, do not rely on it).
- Caveat for implementation: GitHub's REST API accepts arbitrary required-check `context` names without validating them exist, so the guarantee that `lint`/`test` are real checks comes from `ci-pr.yml`, not from the API. `strict=true` means PRs must be up to date with `main` — expected.
- Add a regression-style check of the settings in the PR description or a short verify step if the small step wants a durable record; the protection state is GitHub server state, not repo code.