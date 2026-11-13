#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd -P)"
OUTPUT_DIR="${KANI_LIST_DIR:-$ROOT/.evidence/kani-list}"

usage() {
  printf 'usage: bash scripts/kani-list.sh <package> [<package> ...]\n' >&2
  printf 'writes per-package kani-list JSON to KANI_LIST_DIR or .evidence/kani-list\n' >&2
  printf 'set KANI_FEATURES=feature1,feature2 to activate package features\n' >&2
}

if [ "$#" -eq 0 ]; then
  usage
  exit 2
fi

if ! cargo kani --version >/dev/null 2>&1; then
  printf 'cargo kani is required on PATH.\n' >&2
  exit 1
fi

source "$SCRIPT_DIR/kani-json-helper.sh"
helper_bin="$(kani_json_helper_build)"

mkdir -p -- "$OUTPUT_DIR"

mkdir -p -- "$ROOT/target/native-tools/tmp"
metadata_file="$(mktemp "$ROOT/target/native-tools/tmp/kani-metadata.XXXXXXXX")"
trap 'rm -f -- "$metadata_file"' EXIT
cargo metadata --no-deps --format-version 1 >"$metadata_file"

for package in "$@"; do
  manifest_path="$("$helper_bin" manifest-path "$metadata_file" "$package")"
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
    cargo kani "${kani_args[@]}" list --format json
  )
  if [ ! -s "$package_dir/kani-list.json" ]; then
    printf 'cargo kani list did not produce %s\n' "$package_dir/kani-list.json" >&2
    exit 1
  fi
  mv -f -- "$package_dir/kani-list.json" "$target_file"
  "$helper_bin" validate-json "$target_file"
done

printf 'KANI_LIST_OK output_dir=%s packages=%s\n' "$OUTPUT_DIR" "$*"
