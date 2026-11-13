#!/usr/bin/env bash
# **PO-vb-hbav-033**: CI error exhaustiveness check script.
#
# Compares fuzz oracle function bodies against production error enum definitions.
# Exits 0 when every checked oracle body mentions every current production
# variant. Exits non-zero when variants are missing or inputs cannot be parsed.
#
# Usage: bash scripts/check-error-exhaustiveness.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$SCRIPT_DIR/.."

if ! command -v rustup >/dev/null 2>&1; then
    printf 'Missing required Rust tool: rustup\n' >&2
    exit 127
fi

rustup run nightly-2026-04-28 cargo run --quiet --locked --manifest-path "$ROOT_DIR/Cargo.toml" -p velvet-ballistics-workspace-tests --bin check-error-exhaustiveness -- "$ROOT_DIR"
