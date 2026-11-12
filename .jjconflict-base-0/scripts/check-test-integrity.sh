#!/usr/bin/env bash
set -euo pipefail

if ROOT="$(git rev-parse --show-toplevel 2>/dev/null)"; then
  :
elif ROOT="$(jj workspace root 2>/dev/null)"; then
  :
else
  ROOT="$PWD"
fi
cd "$ROOT"

mkdir -p target/gate-tools
rustc --edition=2024 scripts/check-test-integrity.rs -o target/gate-tools/check-test-integrity
target/gate-tools/check-test-integrity "$@"
