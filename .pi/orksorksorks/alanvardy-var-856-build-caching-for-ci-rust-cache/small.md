# Task

Add `Swatinem/rust-cache` to the `_reusable-lint.yml` and `_reusable-test.yml` reusable workflows in CI to enable build caching.

## Why SMALL
Single-module CI configuration change affecting only 2 files; follows an existing pattern; no schema/API/UI changes; no unknowns; no design decision needed.

## Key files (if the recon found any)
- `_reusable-lint.yml`
- `_reusable-test.yml`
