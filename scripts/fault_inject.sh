#!/usr/bin/env bash
# scripts/fault_inject.sh — thin shell wrapper around the deterministic
# runtime/journal fault injection engine (vb-wy33p.12).
#
# Usage:
#   scripts/fault_inject.sh                     # run all fault injection tests
#   scripts/fault_inject.sh fault_injection_determinism
#   scripts/fault_inject.sh --list
#
# The engine itself is a pure Rust library exposed at `xtask::fault_inject`
# (see `xtask/src/fault_inject/mod.rs`). This wrapper just builds the test
# binary that exercises the engine and invokes it, so contributors can
# run the deterministic fault-injection scenarios from the shell without
# remembering the cargo incantation.
#
# Exit codes follow the underlying `cargo test` binary: 0 = success,
# non-zero = test failure.

set -euo pipefail

readonly SCRIPT_NAME="scripts/fault_inject.sh"
readonly CRATE="velvet-ballistics-workspace-tests"
readonly TEST_TARGET="fault_injection_tests"

usage() {
    cat <<USAGE
$SCRIPT_NAME — vb-wy33p.12 deterministic fault injection

USAGE:
    $SCRIPT_NAME                    Run every fault_injection_* test.
    $SCRIPT_NAME <filter>           Run only the tests matching <filter>.
    $SCRIPT_NAME --list             List the fault injection test names.
    $SCRIPT_NAME --help             Print this help.

ENVIRONMENT:
    CARGO_TARGET_DIR   Cargo target directory (defaults to ./target).
USAGE
}

list_tests() {
    grep -hoE 'fn (fault_injection_[A-Za-z0-9_]+)' \
        crates/workspace_tests/tests/${TEST_TARGET}.rs \
        | sed 's/^fn //' \
        | sort -u
}

if [ "$#" -eq 0 ]; then
    : # fall through, run cargo test with no filter
fi

case "${1:-}" in
    --help|-h)
        usage
        exit 0
        ;;
    --list|-l)
        list_tests
        exit 0
        ;;
esac

# 1. Build the test binary (--no-run produces the executable without
#    invoking it, so we can locate it deterministically).
cargo test \
    --workspace \
    -p "$CRATE" \
    --test "$TEST_TARGET" \
    --all-features \
    --no-run

# 2. Locate the freshly built binary under the cargo target directory.
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
DEPS_DIR="$TARGET_DIR/debug/deps"

if [ ! -d "$DEPS_DIR" ]; then
    echo "$SCRIPT_NAME: deps directory $DEPS_DIR not found" >&2
    exit 1
fi

BIN="$(find "$DEPS_DIR" -maxdepth 1 -type f -executable \
        -name "${TEST_TARGET}-*" \
        -newermt '@0' \
        -printf '%T@ %p\n' 2>/dev/null \
        | sort -n | tail -1 | awk '{print $2}')"

if [ -z "$BIN" ]; then
    # Fallback: take the most recently modified executable.
    BIN="$(find "$DEPS_DIR" -maxdepth 1 -type f -executable \
            -name "${TEST_TARGET}-*" \
            -printf '%T@ %p\n' \
            | sort -n | tail -1 | awk '{print $2}')"
fi

if [ -z "$BIN" ]; then
    echo "$SCRIPT_NAME: $TEST_TARGET binary not found in $DEPS_DIR" >&2
    exit 1
fi

# 3. Invoke the binary with the user-supplied filter (or no filter, which
#    runs every fault injection test). Libtest accepts positional
#    arguments as substring filters over test names.
echo "$SCRIPT_NAME: invoking $BIN $*"
exec "$BIN" "$@" --test-threads=1