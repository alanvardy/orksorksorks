#!/usr/bin/env bash
set -euo pipefail

# Reclaim age-expired build/test caches before building. No-op when the shared
# helper is not installed, so CI and other machines are unaffected.
command -v disk-clean >/dev/null 2>&1 && disk-clean || true

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
