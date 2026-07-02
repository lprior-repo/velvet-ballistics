#!/usr/bin/env bash
# Fuzz minimization wrapper.
#
# Cargo's TOML parser cannot simultaneously accept:
#   [package.metadata]
#   cargo-fuzz = true
# and:
#   [package.metadata.cargo-fuzz]
#   sancov_timeout = 60
#
# Therefore libfuzzer minimization options are passed via command-line.
set -euo pipefail

TARGET="${1:-}"
if [[ -z "$TARGET" ]]; then
    echo "Usage: $0 <target> [extra-args...]"
    echo "Example: $0 journal_event"
    exit 1
fi
shift || true

cargo fuzz run "$TARGET" \
    --target x86_64-unknown-linux-gnu \
    -- \
    -len_control=1 \
    -minimize_contribs=1 \
    "$@"
