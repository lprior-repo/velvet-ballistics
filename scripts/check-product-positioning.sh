#!/usr/bin/env bash
# check-product-positioning: enforce the velvet-ballistics Product Positioning
# Contract (velvet-ballistics-MASTER.md:29). Verbatim master quote:
# "Publicly, velvet-ballistics must not be described as a generic DAG runner,
# low-code graph editor, YAML-as-programming framework, Airflow replacement,
# or Temporal clone. Those frames hide the actual wedge and invite false
# comparisons."
#
# Banned phrases (case-insensitive substring match, see the .rs source for
# the canonical list):
#   - generic dag runner
#   - low-code graph editor
#   - yaml-as-programming
#   - yaml as programming
#   - airflow replacement
#   - airflow alternative
#   - temporal clone
#   - temporal alternative
#
# Matching normalizes Unicode, strips zero-width characters, and collapses
# hyphen/underscore/whitespace runs before applying the banned phrase scan.
#
# Block disclaimer: "<!-- position-disclaimer -->" ...
# "<!-- /position-disclaimer -->" only suppresses lines that also contain an
# explicit negation marker. Unbalanced blocks fail closed.
#
# Self-skip basenames: velvet-ballistics-MASTER.md, CHANGELOG.md, HISTORY.md,
# MIGRATION.md.
# Self-skip directories (and descendants): target, node_modules, .git,
# .beads, .dolt, .moon, .jj, .evidence, .bead-progress, and any directory
# starting with '.'
#
# Default scan surface (relative to repo root):
#   - *.md at the repository root
#   - README.md
#   - docs/**/*.md
#   - crates/**/README.md
#   - crates/vb_cli/**/*.md
#   - fuzz/*.md
#
# Usage:
#   bash scripts/check-product-positioning.sh                       # full repo scan
#   bash scripts/check-product-positioning.sh <path> [<path> ...]  # focused scan
#
# Exit 0 if active findings == 0, exit 1 otherwise.
set -euo pipefail

ROOT="$(pwd -P)"
if [[ ! -f "$ROOT/Cargo.toml" || ! -d "$ROOT/crates" ]]; then
  echo "InvalidInvocation: run from repository root" >&2
  exit 64
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/vb-check-product-positioning.XXXXXX")"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

cat >"$TMP_DIR/Cargo.toml" <<EOF
[package]
name = "vb-check-product-positioning"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
unicode-normalization = "0.1"
EOF

mkdir -p "$TMP_DIR/src"
cat >"$TMP_DIR/src/main.rs" <<EOF
#![forbid(unsafe_code)]

use std::process::ExitCode;

#[path = "$ROOT/scripts/check-product-positioning.rs"]
mod check_product_positioning;

fn main() -> ExitCode {
    check_product_positioning::main()
}
EOF

cargo run --quiet --manifest-path "$TMP_DIR/Cargo.toml" -- "$@"
