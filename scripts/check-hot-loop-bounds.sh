#!/usr/bin/env bash
set -euo pipefail

ROOT="$(pwd -P)"
if [[ ! -f "$ROOT/Cargo.toml" || ! -d "$ROOT/crates" ]]; then
  echo "InvalidInvocation: run from repository root" >&2
  exit 64
fi

mkdir -p target/gate-tools
rustc --edition=2024 scripts/check-hot-loop-bounds.rs -o target/gate-tools/check-hot-loop-bounds
target/gate-tools/check-hot-loop-bounds --self-test
target/gate-tools/check-hot-loop-bounds
