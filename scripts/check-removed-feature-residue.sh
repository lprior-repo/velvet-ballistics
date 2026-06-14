#!/usr/bin/env bash
# check-removed-feature-residue: scans the velvet-ballistics repository for
# ACTIVE residue of removed release features (master §41). The master contract
# states: "PGO, target-cpu=native, maxperf, and generated Rust benchmark
# workflows are removed. They do not block the current Backend / IR
# Interpreter Complete milestone and must not be current release gates."
# The companion quote: "generated and maxperf are removed and must not be
# current default or release features".
#
# Banned tokens (precise phrase/substring match):
#   - "target-cpu=native"  : exact substring
#   - "pgo"                : restricted to PGO active contexts:
#                            "pgo = ", "cargo pgo", "pgo-data", "RUSTC_PGO"
#   - "maxperf"            : as a feature identifier ([features] entry or
#                            --features maxperf)
#   - "generated"          : as a feature identifier ([features] entry or
#                            --features generated)
#
# Per-line allowlist: a single line containing "# allow-removed-feature: <r>"
# or "// allow-removed-feature: <r>" suppresses the NEXT non-blank line; the
# scanner reports the suppression as "allowlisted:" (informational) but does
# not fail on it.
#
# Usage:
#   bash scripts/check-removed-feature-residue.sh                # full repo scan
#   bash scripts/check-removed-feature-residue.sh <path>        # single file/dir
#
# Exit 0 if active residue == 0, exit 1 otherwise.
set -euo pipefail

ROOT="$(pwd -P)"
if [[ ! -f "$ROOT/Cargo.toml" || ! -d "$ROOT/crates" ]]; then
  echo "InvalidInvocation: run from repository root" >&2
  exit 64
fi

mkdir -p target/gate-tools
rustc --edition=2024 scripts/check-removed-feature-residue.rs \
  -o target/gate-tools/check-removed-feature-residue

if [[ $# -gt 0 ]]; then
  target/gate-tools/check-removed-feature-residue "$@"
else
  target/gate-tools/check-removed-feature-residue
fi
