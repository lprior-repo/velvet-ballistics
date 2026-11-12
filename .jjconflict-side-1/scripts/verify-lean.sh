#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
PROOF_DIR="${LEAN_PROOF_DIR:-$ROOT/proofs/lean}"

if [ ! -d "$PROOF_DIR" ]; then
  if [ "${LEAN_REQUIRED:-0}" = "1" ]; then
    printf 'Lean proof directory is required but missing: %s\n' "$PROOF_DIR" >&2
    exit 1
  fi
  printf '[verify:lean] no Lean proof directory found at %s; skipped\n' "$PROOF_DIR"
  exit 0
fi

if [ ! -f "$PROOF_DIR/lakefile.lean" ] && [ ! -f "$PROOF_DIR/lakefile.toml" ]; then
  if [ "${LEAN_REQUIRED:-0}" = "1" ]; then
    printf 'Lean proof directory exists but has no lakefile: %s\n' "$PROOF_DIR" >&2
    exit 1
  fi
  printf '[verify:lean] no lakefile found in %s; skipped\n' "$PROOF_DIR"
  exit 0
fi

if ! command -v lake >/dev/null 2>&1; then
  printf 'lake is required for Lean proof verification but is unavailable.\n' >&2
  exit 1
fi

printf '[verify:lean] lake build in %s\n' "$PROOF_DIR"
cd "$PROOF_DIR"
lake build
