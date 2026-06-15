#!/usr/bin/env bash
# check-cold-adapter-isolation: runtime-core cold-path scanner.
# Uses the xtask command so Rust source parsing stays on syn and the
# shell wrapper stays lint-only.
set -euo pipefail

ROOT="$(pwd -P)"
if [[ ! -f "$ROOT/Cargo.toml" || ! -d "$ROOT/crates" ]]; then
  echo "InvalidInvocation: run from repository root" >&2
  exit 64
fi

env RUSTC_WRAPPER= cargo run --quiet --locked -p xtask -- cold-adapter-isolation "$@"
