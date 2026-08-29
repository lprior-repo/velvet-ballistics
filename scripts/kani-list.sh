#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd -P)"
OUTPUT_DIR="${KANI_LIST_DIR:-$ROOT/.evidence/kani-list}"

usage() {
  printf 'usage: bash scripts/kani-list.sh <package> [<package> ...]\n' >&2
  printf 'writes per-package kani-list JSON to KANI_LIST_DIR or .evidence/kani-list\n' >&2
  printf 'set KANI_FEATURES=feature1,feature2 to activate package features\n' >&2
  printf 'set KANI_DEFAULT_UNWIND=N to set CBMC loop-unwind bound for listed harnesses\n' >&2
}

if [ "$#" -eq 0 ]; then
  usage
  exit 2
fi

if ! cargo kani --version >/dev/null 2>&1; then
  printf 'cargo kani is required on PATH.\n' >&2
  exit 1
fi

mkdir -p -- "$OUTPUT_DIR"

metadata_file="$(mktemp)"
trap 'rm -f -- "$metadata_file"' EXIT
cargo metadata --no-deps --format-version 1 >"$metadata_file"

for package in "$@"; do
  manifest_path="$(python3 - "$metadata_file" "$package" <<'PY'
import json
import sys

metadata_path, requested = sys.argv[1:3]
with open(metadata_path, encoding="utf-8") as handle:
    metadata = json.load(handle)

matches = [pkg for pkg in metadata["packages"] if pkg["name"] == requested]
if len(matches) != 1:
    raise SystemExit(f"expected exactly one package named {requested!r}, found {len(matches)}")
print(matches[0]["manifest_path"])
PY
)"
  package_dir="$(dirname -- "$manifest_path")"
  target_file="$OUTPUT_DIR/$package.json"

  printf '[kani-list] package=%s dir=%s output=%s\n' "$package" "$package_dir" "$target_file"
  rm -f -- "$package_dir/kani-list.json"
  (
    cd "$package_dir"
    kani_args=()
    if [ -n "${KANI_FEATURES:-}" ]; then
      kani_args+=(--features "$KANI_FEATURES")
    fi
    if [ -n "${KANI_DEFAULT_UNWIND:-}" ]; then
      kani_args+=(--default-unwind "$KANI_DEFAULT_UNWIND")
    fi
    cargo kani "${kani_args[@]}" list --format json
  )
  if [ ! -s "$package_dir/kani-list.json" ]; then
    printf 'cargo kani list did not produce %s\n' "$package_dir/kani-list.json" >&2
    exit 1
  fi
  mv -f -- "$package_dir/kani-list.json" "$target_file"
  python3 -m json.tool "$target_file" >/dev/null
done

printf 'KANI_LIST_OK output_dir=%s packages=%s\n' "$OUTPUT_DIR" "$*"
