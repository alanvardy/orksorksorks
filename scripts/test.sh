#!/usr/bin/env bash
set -euo pipefail
echo "=== FORMAT ===" &&
cargo fmt --all &&
echo "=== CHECK ===" &&
cargo check &&
echo "=== CLIPPY ===" &&
cargo clippy --tests -- -D warnings &&
echo "=== TEST ===" &&
cargo nextest run --no-tests=pass &&
echo "=== FORGOTTEN TODOS ===" &&
# Requires ripgrep
if rg -i -g '*.rs' 'TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:|TODO\s|todo\s' .; then
    exit 1
fi &&
echo "=== SUCCESS ===" &&
echo "=== Done ===."