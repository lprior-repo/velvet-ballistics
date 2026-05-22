#!/usr/bin/env bash
set -euo pipefail

ROOT="$(pwd -P)"
if [[ ! -f "$ROOT/Cargo.toml" || ! -d "$ROOT/crates" ]]; then
  echo "InvalidInvocation: run from repository root" >&2
  exit 64
fi

mkdir -p target/gate-tools
rustc --edition=2024 scripts/check-workspace-assertions.rs -o target/gate-tools/check-workspace-assertions
target/gate-tools/check-workspace-assertions
