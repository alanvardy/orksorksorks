#!/usr/bin/env bash
set -euo pipefail

echo "=== cargo install (release, locked) ==="
cargo install --path . --locked

echo "=== SUCCESS: orksorksorks installed to ~/.cargo/bin ==="