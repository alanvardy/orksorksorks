# Recon Report

## Files to Create
- `.github/dependabot.yml` (new file)

## Pattern Conventions
- YAML files use 2-space indentation
- GitHub Actions workflows use reusable workflow pattern with `_reusable-*.yml` and `ci-pr.yml`
- Actions use pinned versions (e.g., `actions/checkout@v4`, `dtolnay/rust-toolchain@stable`)

## Test Commands
```bash
#!/usr/bin/env bash
set -euo pipefail

echo "=== fmt ==="
cargo fmt --all

echo "=== check ==="
cargo check

echo "=== clippy ==="
cargo clippy --tests -- -D warnings

echo "=== nextest ==="
cargo nextest run --no-tests pass

echo "=== TODO/FIXME/dbg gate ==="
if rg -i -g '*.rs' 'TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:' .; then
    exit 1
fi

echo "=== SUCCESS ==="
```

## Dependencies
- Cargo: `clap`, `colored`, `serde`, `serde_json`, `tempfile`, `tokio`, `toml`
- GitHub Actions: `cargo` ecosystem only (no separate github-actions config needed)

## Notes
- The task mentions both `cargo` and `github-actions` ecosystems
- Dependabot can handle both in a single config file using `packages` and `github-actions` sections
- Weekly interval (`schedule.interval: weekly`)
- Open-pull-requests-limit should be capped (e.g., 10)