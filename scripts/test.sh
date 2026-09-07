#!/usr/bin/env bash
set -euo pipefail

# ── Test-gate lock ────────────────────────────────────────────────────────────────
# Serialize against other pi instances running the heavy test gate on this machine
# (they share one OS user). Two+ concurrent build+test gates can exhaust RAM (OOM).
# Acquires an exclusive flock(2) on a shared lock file via /usr/bin/lockf and waits
# (up to PI_TEST_LOCK_TIMEOUT seconds) for it; the lock is held for the whole run
# and released automatically on exit, even if this process is killed. The lock is
# never broken while held by another process. All projects should use the same
# default path so their gates serialize against each other. Bypass with
# PI_TEST_NO_LOCK=1, or automatically under CI (each CI job is its own runner).
if [[ -z "${PI_TEST_LOCK_HELD:-}" && -z "${PI_TEST_NO_LOCK:-}" && -z "${CI:-}" ]]; then
    PI_TEST_LOCK="${PI_TEST_LOCK:-$HOME/.cache/pi/test-gate.lock}"
    PI_TEST_LOCK_TIMEOUT="${PI_TEST_LOCK_TIMEOUT:-3600}"
    export PI_TEST_LOCK_HELD=1
    echo "==> Waiting for test-gate lock ($PI_TEST_LOCK)…"
    exec /usr/bin/lockf -k -t "$PI_TEST_LOCK_TIMEOUT" "$PI_TEST_LOCK" bash "$0" "$@"
fi

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