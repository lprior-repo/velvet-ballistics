#!/usr/bin/env bash
set -euo pipefail

ROOT="$(pwd -P)"
if [[ ! -f "$ROOT/Cargo.toml" || ! -d "$ROOT/crates" ]]; then
  printf '%s\n' "InvalidInvocation: run from repository root" >&2
  exit 64
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if ! command -v rustup >/dev/null 2>&1; then
  printf 'Missing required Rust tool: rustup\n' >&2
  exit 127
fi
rustup run nightly-2026-04-28 cargo run --quiet --locked --manifest-path "$SCRIPT_DIR/../Cargo.toml" -p velvet-ballistics-workspace-tests --bin hotpath-scan -- "$@"
