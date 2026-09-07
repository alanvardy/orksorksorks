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
