#!/usr/bin/env bash
set -euo pipefail

ROOT="$(pwd -P)"
if [[ ! -f "$ROOT/Cargo.toml" || ! -d "$ROOT/crates" ]]; then
  echo "InvalidInvocation: run from repository root" >&2
  exit 64
fi

mkdir -p target/gate-tools
rustup run nightly-2026-04-28 rustc --edition=2024 scripts/check_panic_surface.rs \
  -o target/gate-tools/check-panic-surface
exec target/gate-tools/check-panic-surface
