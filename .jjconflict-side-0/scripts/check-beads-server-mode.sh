#!/usr/bin/env bash
set -euo pipefail

metadata=".beads/metadata.json"

fail() {
  printf 'beads server-mode check failed: %s\n' "$1" >&2
  exit 1
}

[[ -f "$metadata" ]] || fail "$metadata is missing"

grep -Eq '"backend"[[:space:]]*:[[:space:]]*"dolt"' "$metadata" \
  || fail "$metadata must keep backend set to dolt"

grep -Eq '"dolt_mode"[[:space:]]*:[[:space:]]*"server"' "$metadata" \
  || fail "$metadata must keep dolt_mode set to server"

if grep -Eq '"dolt_mode"[[:space:]]*:[[:space:]]*"embedded"' "$metadata"; then
  fail "embedded Dolt mode is forbidden; use server mode only"
fi

if grep -Eq '"dolt_server_port"' "$metadata"; then
  fail "do not pin dolt_server_port in metadata; bd manages .beads/dolt-server.port"
fi

if [[ -e ".beads/embeddeddolt" ]]; then
  fail ".beads/embeddeddolt exists; remove it before running bd"
fi

printf 'beads server-mode check passed\n'
