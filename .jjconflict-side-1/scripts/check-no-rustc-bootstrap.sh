#!/usr/bin/env bash
# Reject RUSTC_BOOTSTRAP in scripts, config, CI, and moon task definitions.
#
# Per `docs/rust-governance.md`, RUSTC_BOOTSTRAP is forbidden because it
# silently enables nightly-only language features against the stable toolchain.
# This script mechanically fails CI if RUSTC_BOOTSTRAP appears anywhere it
# could be picked up by a moon task or shell script.
#
# Bead: vb-jbe4l.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

violations=0

scan_paths=(
  scripts
  .moon
  .github
  xtask
)

self_basename="$(basename "$0")"

scan_files() {
  local path="$1"
  if [ -d "$path" ]; then
    # Skip self: the linter must reference the forbidden token in its own
    # comments to describe what it rejects.
    rg -n --no-heading --color=never --glob '!'"$self_basename" 'RUSTC_BOOTSTRAP' "$path" || true
  fi
}

echo "[check-no-rustc-bootstrap] scanning for RUSTC_BOOTSTRAP references..."
for p in "${scan_paths[@]}"; do
  out="$(scan_files "$p")"
  if [ -n "$out" ]; then
    echo "$out"
    violations=$((violations + $(printf '%s\n' "$out" | wc -l)))
  fi
done

if [ "$violations" -gt 0 ]; then
  echo ""
  echo "[check-no-rustc-bootstrap] FAIL: $violations RUSTC_BOOTSTRAP reference(s) found."
  echo "Per docs/rust-governance.md, RUSTC_BOOTSTRAP is forbidden."
  echo "Use pinned nightly toolchain (rust-toolchain.toml) instead."
  exit 1
fi

echo "[check-no-rustc-bootstrap] OK: no RUSTC_BOOTSTRAP references found."