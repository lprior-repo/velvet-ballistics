#!/usr/bin/env bash
# moon-task-coverage-audit wrapper: compiles the Rust source, runs self-test, then full audit.
set -euo pipefail

ROOT="$(pwd -P)"
if [[ ! -f "$ROOT/Cargo.toml" || ! -d "$ROOT/.moon" ]]; then
  echo "InvalidInvocation: run from repository root" >&2
  exit 64
fi

mkdir -p target/gate-tools
rustc --edition=2024 scripts/check-moon-task-coverage.rs -o target/gate-tools/check-moon-task-coverage
target/gate-tools/check-moon-task-coverage --self-test
target/gate-tools/check-moon-task-coverage
