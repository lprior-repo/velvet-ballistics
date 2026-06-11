#!/usr/bin/env bash
set -euo pipefail

repo_root() {
  if git rev-parse --show-toplevel 2>/dev/null; then
    return 0
  fi
  if command -v jj >/dev/null 2>&1; then
    if jj root 2>/dev/null; then
      return 0
    fi
  fi
  pwd -P
}

ROOT="$(repo_root)"
cd "$ROOT"

mkdir -p target/gate-tools
rustc --edition=2024 scripts/check-test-integrity.rs -o target/gate-tools/check-test-integrity
target/gate-tools/check-test-integrity "$@"
