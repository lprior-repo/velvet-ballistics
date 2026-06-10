#!/usr/bin/env bash
# scripts/check-bench-registration.sh
# Bead: vb-scr01 — R1-A11 / R4-A11 missing check
#
# Asserts that every Rust file under `crates/<crate>/benches/*.rs` is
# actually wired into the corresponding crate's `Cargo.toml` via a
# `[[bench]]` entry. The Criterion harness relies on the bench entry
# to register a binary target; a `.rs` file dropped under `benches/`
# without a registration compiles but is never built or run.
#
# Counting model:
#   - For each `crates/<crate>` with a `benches/` subdirectory, walk
#     `benches/*.rs` and record the basename of each file (no `.rs`).
#   - For each such crate, parse `Cargo.toml` for every
#     `[[bench]]` table whose `name` is set. (The convention is
#     `name = "<basename>"` with `path` left implicit.)
#   - A bench file is "unregistered" iff its basename is not in the
#     crate's set of `[[bench]]` names.
#
# Threshold:
#   - Exits 0 when every `benches/*.rs` is registered.
#   - Exits 1 when one or more bench files are not registered. The
#     diagnostic lists the missing names and their crate.
#   - Exits 2 on usage or environment error.
#
# This script is read-only: it never modifies repository state. It
# does not require cargo; it parses Cargo.toml with `awk`-based key
# extraction (no dependency on `toml` crates).

set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"

if [[ ! -d "$ROOT/crates" ]]; then
  printf 'check-bench-registration: error: %s/crates is not a directory\n' "$ROOT" >&2
  exit 2
fi

# Read every `name = "..."` line that follows a `[[bench]]` directive
# in a Cargo.toml. This is a tolerant parser: it accepts the
# `name = "..."` form, ignoring the optional `path` and `harness` keys.
extract_bench_names() {
  local toml="$1"
  [[ -f "$toml" ]] || return 0
  awk '
    /^\[\[bench\]\]/ { in_bench = 1; next }
    /^\[/            { in_bench = 0; next }
    in_bench && /^[[:space:]]*name[[:space:]]*=/ {
      # strip leading whitespace, the literal "name", the "=", optional
      # surrounding double-quotes from the value, and any trailing
      # whitespace/comment.
      line = $0
      sub(/^[[:space:]]*name[[:space:]]*=[[:space:]]*"/, "", line)
      sub(/"[[:space:]]*$/, "", line)
      print line
    }
  ' "$toml"
}

printf 'check-bench-registration: scanning %s/crates for unregistered benches\n' "$ROOT" >&2

failed=0
total_crates=0
total_files=0
total_registered=0
total_unregistered=0

for crate_dir in "$ROOT"/crates/*/; do
  [[ -d "$crate_dir" ]] || continue
  crate="$(basename -- "$crate_dir")"
  benches_dir="$crate_dir/benches"
  toml="$crate_dir/Cargo.toml"

  [[ -d "$benches_dir" ]] || continue
  total_crates=$((total_crates + 1))

  # Build the set of registered bench names for this crate.
  declare -A registered
  while IFS= read -r name; do
    [[ -n "$name" ]] && registered["$name"]=1
  done < <(extract_bench_names "$toml")

  # Walk the bench files in lexical order for a stable report.
  unregistered=()
  while IFS= read -r bench_file; do
    [[ -n "$bench_file" ]] || continue
    bn="$(basename -- "$bench_file" .rs)"
    total_files=$((total_files + 1))
    if [[ -n "${registered[$bn]+set}" ]]; then
      total_registered=$((total_registered + 1))
    else
      unregistered+=("$bn")
      total_unregistered=$((total_unregistered + 1))
    fi
  done < <(find "$benches_dir" -maxdepth 1 -type f -name '*.rs' | sort)

  if [[ ${#unregistered[@]} -gt 0 ]]; then
    failed=1
    printf '  [FAIL] crate=%s unregistered_benches=%d:\n' "$crate" "${#unregistered[@]}" >&2
    for name in "${unregistered[@]}"; do
      printf '          - %s\n' "$name" >&2
    done
  fi

  unset registered
done

printf '\nSUMMARY: crates=%d bench_files=%d registered=%d unregistered=%d\n' \
  "$total_crates" "$total_files" "$total_registered" "$total_unregistered"

if [[ "$failed" -ne 0 ]]; then
  printf '\ncheck-bench-registration: FAILED — one or more benches/*.rs files lack a [[bench]] entry.\n' >&2
  printf '  Add a `[[bench]]\n  name = "<basename>"\n  harness = false\n` block to the crate'\''s Cargo.toml.\n' >&2
  exit 1
fi

printf '\ncheck-bench-registration: OK (every benches/*.rs is registered as a [[bench]] target)\n'
exit 0
