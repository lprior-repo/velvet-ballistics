#!/usr/bin/env bash
set -euo pipefail

ROOT="$(pwd -P)"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"

mkdir -p "$ROOT/target/gate-tools"
rustc --edition=2024 "$SCRIPT_DIR/check-source-length.rs" -o "$ROOT/target/gate-tools/check-source-length"
"$ROOT/target/gate-tools/check-source-length"
