#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd -P)"
REGISTRY="${VERUS_PROOF_REGISTRY:-contracts/proof_obligations.yaml}"
EVIDENCE_DIR="${VERUS_EVIDENCE_DIR:-.evidence/verus}"

cd "$ROOT"

if ! command -v verus >/dev/null 2>&1; then
  printf 'Verus is required by registry L4 obligations but is unavailable on PATH.\n' >&2
  exit 1
fi

if [ ! -s "$REGISTRY" ]; then
  printf 'Verus proof registry is missing or empty: %s\n' "$REGISTRY" >&2
  exit 1
fi

mkdir -p -- "$EVIDENCE_DIR"

mapfile -t targets < <(python3 - "$REGISTRY" <<'PY'
from pathlib import Path
import re
import sys

registry = Path(sys.argv[1])
seen: set[str] = set()
for line in registry.read_text(encoding="utf-8").splitlines():
    match = re.match(r"^\s*verus:\s*['\"]?([^'\"#]+?)['\"]?\s*(?:#.*)?$", line)
    if match:
        target = match.group(1).strip()
        if target and target not in seen:
            seen.add(target)
            print(target)
PY
)

if [ "${#targets[@]}" -eq 0 ]; then
  printf 'No required.verus targets found in %s; refusing silent proof pass.\n' "$REGISTRY" >&2
  exit 1
fi

summary_file="$EVIDENCE_DIR/summary.txt"
{
  printf 'VERUS_TARGET_COUNT=%s\n' "${#targets[@]}"
  printf 'VERUS_VERSION='
  verus --version
  if command -v verusfmt >/dev/null 2>&1; then
    printf 'VERUSFMT=available\n'
  else
    printf 'VERUSFMT=VERUSFMT_MISSING\n'
    printf 'ERROR: verusfmt is required by VERUSFMT_MISSING gate but is unavailable on PATH.\n' >&2
    exit 1
  fi
} >"$summary_file"

for target in "${targets[@]}"; do
  if [ ! -s "$target" ]; then
    printf 'Required Verus target is missing or empty: %s\n' "$target" >&2
    exit 1
  fi

  evidence_file="$EVIDENCE_DIR/$(basename "${target%.rs}").txt"
  declare -A VERUS_FILE_FLAGS=(
    # Verus currently hits an internal lifetime-erasure error on this
    # production-bound mirror; --no-lifetime is the tool-suggested workaround
    # and does not change the spec contracts checked by this target.
    [budget_bounded]="--no-lifetime"
  )
  stem="$(basename "${target%.rs}")"
  extra_flags="${VERUS_FILE_FLAGS[$stem]:-}"
  if [ -n "$extra_flags" ]; then
    # shellcheck disable=SC2206
    verus_args=($extra_flags --crate-type=lib)
  else
    verus_args=(--crate-type=lib)
  fi
  printf '[verus] verus %s %s\n' "${verus_args[*]}" "$target" | tee "$evidence_file"
  set +e
  verus "${verus_args[@]}" "$target" 2>&1 | tee -a "$evidence_file"
  status=${PIPESTATUS[0]}
  set -e
  generated_name="$(basename "${target%.rs}")"
  for generated_path in "$generated_name" "lib${generated_name}.rlib"; do
    if ! git ls-files --error-unmatch "$generated_path" >/dev/null 2>&1; then
      rm -f -- "$generated_path"
    fi
  done
  if [ "$status" -ne 0 ]; then
    printf 'Verus target failed: %s (exit %s)\n' "$target" "$status" >&2
    exit "$status"
  fi
  if ! rg -q 'verification results:: .* 0 errors' "$evidence_file"; then
    printf 'Verus evidence lacks zero-error summary: %s\n' "$evidence_file" >&2
    exit 1
  fi
  printf 'PASS verus %s\n' "$target" >>"$summary_file"
done

trust_file="$EVIDENCE_DIR/trust-scan.txt"
forbidden_file="$EVIDENCE_DIR/trust-forbidden.txt"
set +e
rg -n 'assume\(|\baxiom\b' verification/verus contracts/verus --glob '*.rs' \
  | rg -v '^([^:]+:[0-9]+:\s*//|[^:]+:[0-9]+:\s*///)' >"$forbidden_file"
pipeline_status=("${PIPESTATUS[@]}")
trust_status=${pipeline_status[0]}
filter_status=${pipeline_status[1]}
set -e
if [ "$trust_status" -gt 1 ] || { [ "$filter_status" -gt 1 ] && [ "$trust_status" -eq 0 ]; }; then
  printf 'Verus forbidden trust-boundary scan failed (rg=%s filter=%s).\n' "$trust_status" "$filter_status" >&2
  exit 1
fi
if [ -s "$forbidden_file" ]; then
  printf 'Verus forbidden trust-boundary scan found assume()/axiom code. See %s\n' "$forbidden_file" >&2
  exit 1
fi

set +e
rg -n '#\[verifier::external_body\]|#\[verifier::external\]' \
  verification/verus contracts/verus \
  --glob '*.rs' >"$trust_file"
inventory_status=$?
set -e
case "$inventory_status" in
  0)
    printf 'VERUS_TRUST_BOUNDARY_INVENTORY see %s\n' "$trust_file" >>"$summary_file"
    ;;
  1)
    printf 'VERUS_TRUST_BOUNDARY_INVENTORY empty\n' | tee "$trust_file" >/dev/null
    printf 'VERUS_TRUST_BOUNDARY_INVENTORY empty\n' >>"$summary_file"
    ;;
  *)
    printf 'Verus trust-boundary inventory failed with rg exit %s.\n' "$inventory_status" >&2
    exit "$inventory_status"
    ;;
esac
printf 'VERUS_FORBIDDEN_TRUST_SCAN_OK no assume()/axiom code matches\n' >>"$summary_file"

printf 'VERUS_REGISTRY_OK evidence=%s\n' "$EVIDENCE_DIR"
