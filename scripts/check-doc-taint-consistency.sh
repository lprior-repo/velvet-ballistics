#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$SCRIPT_DIR/.."

if ! command -v rustup >/dev/null 2>&1; then
    printf 'Missing required Rust tool: rustup\n' >&2
    exit 127
fi

rustup run nightly-2026-04-28 cargo run --quiet --locked --manifest-path "$ROOT_DIR/Cargo.toml" -p velvet-ballistics-workspace-tests --bin check-doc-taint-consistency -- "$@"
