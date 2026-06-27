#!/usr/bin/env bash
# scripts/check-verus-production-binding.sh
#
# HARD CI GATE: rejects any Verus spec file that has `proof fn` but is not bound
# to production code via #[path = ".../crates/..."] or via #[path = "production_inner/..."].
#
# A spec is "production-bound" iff:
#   1. It declares `#[path = ".../crates/..."] mod production;` (direct production source)
#      OR `#[path = ".../production_inner/..."] mod production;` (verbatim mirror)
#   2. AND it has at least one `assume_specification[ production::... ](...)`
#      contract that attaches a spec contract to a production exec fn.
#
# Specs without both conditions are VACUUM (GOD RULE 2 violation) and this script
# exits non-zero to fail the moon task and CI gate.
#
# Author: enforced via `.moon/tasks/all.yml:verify-verus-production-binding`

set -uo pipefail

REPO_ROOT="${1:-$(git rev-parse --show-toplevel)}"
VERIFICATION_DIR="${REPO_ROOT}/verification/verus"

if [[ ! -d "${VERIFICATION_DIR}" ]]; then
    echo "ERROR: ${VERIFICATION_DIR} does not exist"
    exit 2
fi

# Map of allowed exceptions (files explicitly granted model-only / pure-math
# exemption by an ADR/PO/ledger). Add a row ONLY when the binding work is
# permanently offloaded to Kani/Flux/proptest/fuzz lanes (per AGENTS.md).
#
# Format: "<relpath>|<reason>"
ALLOWED_EXCEPTIONS=(
    "choose_proofs.vr|PO-VERUS-XXXX: model-only; live binding is Kani/Flux (see proof-review.md)"
)

# Convert ALLOWED_EXCEPTIONS array into a lookup
declare -A allowed
for entry in "${ALLOWED_EXCEPTIONS[@]}"; do
    allowed["${entry%%|*}"]="${entry##*|}"
done

vacuum_files=()
weak_files=()
strong_files=()

while IFS= read -r -d '' file; do
    rel="${file#${REPO_ROOT}/}"

    # Skip allowed exceptions
    if [[ -n "${allowed[${rel}]:-}" ]]; then
        continue
    fi

    # Detect whether file is a Verus spec (has proof fn or verus! block)
    if ! grep -qE '^([[:space:]]*)(pub[[:space:]]*)?(proof[[:space:]]+fn|fn[[:space:]]+main)[[:space:]]' "${file}"; then
        continue  # not a spec, skip
    fi

    # STRONG: has direct #[path] to crates/ + assume_specification bridge
    if grep -qE '^\s*#\[path\s*=\s*"[^"]*crates/' "${file}" \
       && grep -qE '^\s*(pub[[:space:]]+)?assume_specification\[' "${file}"; then
        strong_files+=("${rel}")
        continue
    fi

    # WEAK: binds via production_inner/* mirror + assume_specification bridge
    if grep -qE '^\s*#\[path\s*=\s*"[^"]*production_inner/' "${file}" \
       && grep -qE '^\s*(pub[[:space:]]+)?assume_specification\[' "${file}"; then
        weak_files+=("${rel}")
        continue
    fi

    # Also check if spec binds via extern_*.rs companion (the common pattern):
    # spec file has `#[path = "extern_*.rs"]` or imports from extern
    if grep -qE '^\s*#\[path\s*=\s*"extern_[^"]*\.rs"\]' "${file}"; then
        # Find the extern file referenced
        extern_file=$(grep -oE 'extern_[a-zA-Z0-9_]+\.rs' "${file}" | head -1)
        if [[ -n "${extern_file}" && -f "${VERIFICATION_DIR}/${extern_file}" ]]; then
            # Check if spec uses assume_specification
            if grep -qE '^\s*(pub[[:space:]]+)?assume_specification\[' "${file}"; then
                # Strong if the extern itself has #[path] to crates/
                if grep -qE '^\s*#\[path\s*=\s*"[^"]*crates/' "${VERIFICATION_DIR}/${extern_file}" \
                   || grep -qE '^\s*#\[path\s*=\s*"[^"]*production_inner/' "${VERIFICATION_DIR}/${extern_file}"; then
                    if grep -qE '^\s*#\[path\s*=\s*"[^"]*crates/' "${VERIFICATION_DIR}/${extern_file}"; then
                        strong_files+=("${rel}")
                    else
                        weak_files+=("${rel}")
                    fi
                    continue
                fi
            fi
        fi
    fi

    # Otherwise: VACUUM
    vacuum_files+=("${rel}")
done < <(find "${VERIFICATION_DIR}" -type f \( -name '*.rs' -o -name '*.vr' \) -print0)

echo "================================================================"
echo "  Verus production-binding audit"
echo "================================================================"
printf "  STRONG (direct crates/ binding): %d\n" "${#strong_files[@]}"
printf "  WEAK (production_inner/ mirror): %d\n" "${#weak_files[@]}"
printf "  VACUUM (no production binding):  %d\n" "${#vacuum_files[@]}"
echo

if (( ${#vacuum_files[@]} > 0 )); then
    echo "  VACUUM files (GOD RULE 2 violation — proof without production binding):"
    for f in "${vacuum_files[@]}"; do
        echo "    - ${f}"
    done
    echo
    echo "  Either:"
    echo "    1. Bind via #[path = \".../crates/.../src/...rs\"] + assume_specification"
    echo "    2. Bind via #[path = \"production_inner/...rs\"] + assume_specification"
    echo "    3. DELETE the file"
    echo "    4. Add an entry to ALLOWED_EXCEPTIONS in scripts/check-verus-production-binding.sh"
    echo "       with a PO-XXXX reference and a Kani/Flux/proptest offload rationale."
fi

if (( ${#vacuum_files[@]} > 0 )); then
    exit 1
fi
exit 0