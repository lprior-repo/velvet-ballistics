#!/usr/bin/env bash
# check-removed-crate-residue: scans the velvet-ballistics repository for
# ACTIVE residue of the removed release crates
# (master §32 / deferred-scope fence). The master contract states:
# "Removed crates: vb_codegen, vb_ui_model, and vb_ui_makepad... must not
# appear as active workspace members or current release gates". Companion
# UI surface: makepad-widgets, makepad-draw, and the bare `makepad` token.
#
# Banned tokens (precise phrase / substring match):
#   - "vb_codegen"       : exact substring
#   - "vb_ui_model"      : exact substring
#   - "vb_ui_makepad"    : exact substring
#   - "makepad-widgets"  : exact substring
#   - "makepad-draw"     : exact substring
#   - "makepad" (bare)   : word boundary (so "velvet-ballistics" and
#                          "makepad-2.0" do not false-match). "Makepad"
#                          (capitalised) is allowed.
#
# Per-line allowlist: a single line containing
# "# allow-removed-crate: <reason>" or "// allow-removed-crate: <reason>"
# suppresses the NEXT non-blank line; the scanner reports the suppression as
# "allowlisted:" (informational) but does not fail on it.
#
# Usage:
#   bash scripts/check-removed-crate-residue.sh                # full repo scan
#   bash scripts/check-removed-crate-residue.sh <path>        # single file/dir
#
# Exit 0 if active residue == 0, exit 1 otherwise.
set -euo pipefail

ROOT="$(pwd -P)"
if [[ ! -f "$ROOT/Cargo.toml" || ! -d "$ROOT/crates" ]]; then
  echo "InvalidInvocation: run from repository root" >&2
  exit 64
fi

mkdir -p target/gate-tools
rustc --edition=2024 scripts/check-removed-crate-residue.rs \
  -o target/gate-tools/check-removed-crate-residue

if [[ $# -gt 0 ]]; then
  target/gate-tools/check-removed-crate-residue "$@"
else
  target/gate-tools/check-removed-crate-residue
fi
