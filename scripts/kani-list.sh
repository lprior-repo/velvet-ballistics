#!/usr/bin/env bash
# scripts/kani-list.sh — per-package Kani harness discovery.
#
# Per master §77.9: the root invocation `cargo kani list --format json`
# is forbidden because it requires the Kani toolchain on PATH and emits
# a wire format we do not control. This wrapper enumerates harnesses by
# scanning `crates/<package>/src/verification/kani/` plus the
# top-level `crates/<package>/src/**/*kani*.rs` shims, and reports the
# `kani::Arbitrary`/`kani::any` substrate status for each harness.
#
# Usage: bash scripts/kani-list.sh <package> [<package> ...]
#
# Environment:
#   KANI_LIST_DIR     output directory (default: .evidence/kani-list)
#   KANI_FEATURES     comma-separated cargo features to require on every
#                     harness file (for example `kani-trace-ring`).
#
# Exit codes:
#   0  success, JSON written for every package
#   1  internal scanner error
#   2  usage / missing package directory

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd -P)"
OUTPUT_DIR="${KANI_LIST_DIR:-$ROOT/.evidence/kani-list}"

usage() {
  printf 'usage: bash scripts/kani-list.sh <package> [<package> ...]\n' >&2
  printf 'writes per-package kani-list JSON to KANI_LIST_DIR or .evidence/kani-list\n' >&2
  printf 'KANI_LIST_DIR=path overrides the output directory.\n' >&2
}

if [ "$#" -eq 0 ]; then
  usage
  exit 2
fi

mkdir -p -- "$OUTPUT_DIR"

scan_failed=0
for package in "$@"; do
  package_dir="$ROOT/crates/$package"
  target_file="$OUTPUT_DIR/$package.json"

  if [ ! -d "$package_dir" ]; then
    printf 'package not found: %s (expected %s)\n' "$package" "$package_dir" >&2
    exit 2
  fi

  printf '[kani-list] package=%s dir=%s output=%s\n' "$package" "$package_dir" "$target_file"

  required_features="${KANI_FEATURES:-}"

  if ! KANI_LIST_REQUIRED_FEATURES="$required_features" \
       python3 - "$package_dir" "$target_file" <<'PY'
import json
import os
import re
import sys

package_dir, target_file = sys.argv[1:3]
required_features = [
    feature.strip()
    for feature in os.environ.get("KANI_LIST_REQUIRED_FEATURES", "").split(",")
    if feature.strip()
]

PROOF_ATTR_RE = re.compile(
    r"#\s*\[\s*kani\s*::\s*proof\b[^\]]*\]\s*fn\s+([A-Za-z_][A-Za-z0-9_]*)"
)
KANI_ARBITRARY_IMPL_RE = re.compile(
    r"\bimpl\b[^{}]*\bArbitrary\b[^;{}]*\bfor\b"
)

harness_path_globs = [
    os.path.join(package_dir, "src", "verification", "kani"),
    os.path.join(package_dir, "src"),
]
discovered = []
for root_dir in harness_path_globs:
    if not os.path.isdir(root_dir):
        continue
    for dirpath, _dirs, files in os.walk(root_dir):
        for filename in sorted(files):
            if not filename.endswith(".rs"):
                continue
            basename = filename[:-3]
            if not basename.startswith("kani") and not basename.endswith("_kani"):
                continue
            full_path = os.path.join(dirpath, filename)
            with open(full_path, encoding="utf-8") as handle:
                source = handle.read()

            proof_matches = list(PROOF_ATTR_RE.finditer(source))
            if not proof_matches:
                continue

            arbitrary_present = bool(KANI_ARBITRARY_IMPL_RE.search(source))
            any_present = "kani::any" in source or "kani :: any" in source
            cfg_kani = bool(re.search(r"#!\[cfg\(kani\)\]|#\[cfg\(kani\)\]", source))
            has_feature_gate = bool(
                re.search(r"feature\s*=\s*\"[^\"]*kani[^\"]*\"", source)
            )

            for match in proof_matches:
                harness_name = match.group(1)
                if arbitrary_present:
                    substrate_status = "kani_arbitrary_impl"
                elif any_present:
                    substrate_status = "kani_any_only"
                else:
                    substrate_status = "no_input_generator"

                discovered.append(
                    {
                        "harness": harness_name,
                        "path": os.path.relpath(full_path, start=os.getcwd()),
                        "kani_arbitrary_status": substrate_status,
                        "cfg_kani": cfg_kani,
                        "feature_gated": has_feature_gate,
                        "required_features": required_features,
                    }
                )

payload = {
    "package": os.path.basename(package_dir.rstrip(os.sep)),
    "package_dir": os.path.relpath(package_dir, start=os.getcwd()),
    "discovery_root": os.path.relpath(package_dir, start=os.getcwd()) + "/src",
    "harness_count": len(discovered),
    "harnesses": discovered,
}
os.makedirs(os.path.dirname(target_file), exist_ok=True)
with open(target_file, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY
  then
    scan_failed=1
    continue
  fi

  python3 -m json.tool "$target_file" >/dev/null
done

if [ "$scan_failed" -ne 0 ]; then
  printf 'KANI_LIST_FAILED packages=%s\n' "$*" >&2
  exit 1
fi

printf 'KANI_LIST_OK output_dir=%s packages=%s\n' "$OUTPUT_DIR" "$*"
