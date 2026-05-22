#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

mkdir -p target/gate-tools
rustc --edition=2024 scripts/check-test-integrity.rs -o target/gate-tools/check-test-integrity
target/gate-tools/check-test-integrity "$@"
