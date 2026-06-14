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
# Per-line allowlist: a line containing "<!-- ALLOW_HISTORICAL: <reason> -->"
# suppresses that same line. Reported as "allowlisted:" (informational).
#
# Block allowlist: "<!-- position-disclaimer -->" ... "<!-- /position-disclaimer -->"
# around a paragraph suppresses every match inside. Reported as "disclaimered:"
# (informational).
#
# Self-skip basenames: velvet-ballistics-MASTER.md, CHANGELOG.md, HISTORY.md,
# MIGRATION.md.
# Self-skip directories (and descendants): target, node_modules, .bead-progress,
# .evidence.
#
# Default scan surface (relative to repo root):
#   - README.md
#   - docs/**/*.md
#   - crates/**/README.md
#   - crates/vb_cli/**/*.md
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

mkdir -p target/gate-tools
BIN="$ROOT/target/gate-tools/check-product-positioning"
rustc --edition=2024 "$ROOT/scripts/check-product-positioning.rs" -o "$BIN"

"$BIN" "$@"
