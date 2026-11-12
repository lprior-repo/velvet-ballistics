#!/usr/bin/env bash
set -euo pipefail

ROOT="$(pwd -P)"
if [[ ! -f "$ROOT/Cargo.toml" || ! -d "$ROOT/crates" || ! -f "$ROOT/xtask/src/forbidden_scan.rs" ]]; then
  printf '%s\n' "InvalidInvocation: run from repository root" >&2
  exit 64
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/vb-forbidden-scan.XXXXXX")"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

cat >"$TMP_DIR/Cargo.toml" <<EOF
[package]
name = "vb-forbidden-scan-runner"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
anyhow = "1"
regex = "1"
EOF

mkdir -p "$TMP_DIR/src"
cat >"$TMP_DIR/src/main.rs" <<EOF
#![forbid(unsafe_code)]

#[path = "$ROOT/xtask/src/forbidden_scan.rs"]
mod forbidden_scan;

fn main() -> anyhow::Result<()> {
    forbidden_scan::cmd_forbidden_scan(None, None)
}
EOF

cargo run --quiet --manifest-path "$TMP_DIR/Cargo.toml"
