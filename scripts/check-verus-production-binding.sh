#!/usr/bin/env bash
# scripts/check-verus-production-binding.sh
#
# ABSOLUTE CI GATE: rejects ANY Verus spec file that has `proof fn` but is not
# bound to production code via #[path = ".../crates/..."] or via #[path =
# ".../production_inner/..."]. NO EXCEPTIONS. NO BACKDOORS.
#
# Every Verus spec/proof artifact MUST be bound to production Rust code. There
# is no path where a proof can verify "internally consistent math" without
# binding to the actual production implementation. Either bind or delete.
#
# A spec is "production-bound" iff:
#   1. It declares `#[path = ".../crates/..."] mod production;` (direct production source)
#      OR `#[path = ".../production_inner/..."] mod production;` (verbatim mirror)
#   2. AND it has at least one `assume_specification[ production::... ](...)`
#      contract that attaches a spec contract to a production exec fn.
#
# Specs without both conditions fail this gate. There is no override.
#
# Author: enforced via `.moon/tasks/all.yml:verify-verus-production-binding`

set -uo pipefail

REPO_ROOT="${1:-$(git rev-parse --show-toplevel)}"
VERIFICATION_DIR="${REPO_ROOT}/verification/verus"

if [[ ! -d "${VERIFICATION_DIR}" ]]; then
    echo "ERROR: ${VERIFICATION_DIR} does not exist"
    exit 2
fi

# NO ALLOWED_EXCEPTIONS. NO BACKDOORS. NO OVERRIDES.
# If a spec cannot be bound to production, DELETE IT.
# If the spec proves pure math with no production connection, it has no value.

vacuum_files=()
weak_files=()
strong_files=()

while IFS= read -r -d '' file; do
    rel="${file#${REPO_ROOT}/}"

    # Skip extern_*.rs files (they are companion modules, not spec files)
    case "${rel}" in
        */extern_*.rs)
            continue
            ;;
        */production_inner/*)
            continue
            ;;
    esac

    # Detect whether file is a Verus spec (has proof fn)
    if ! grep -qE '^([[:space:]]*)(pub[[:space:]]*)?proof[[:space:]]+fn[[:space:]]' "${file}"; then
        continue  # not a spec with proofs, skip
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

    # Companion extern pattern: spec has `#[path = "extern_*.rs"]` + assume_specification
    if grep -qE '^\s*#\[path\s*=\s*"extern_[^"]*\.rs"' "${file}"; then
        extern_file=$(grep -oE 'extern_[a-zA-Z0-9_]+\.rs' "${file}" | head -1)
        if [[ -n "${extern_file}" && -f "${VERIFICATION_DIR}/${extern_file}" ]]; then
            if grep -qE '^\s*(pub[[:space:]]+)?assume_specification\[' "${file}"; then
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

    # Otherwise: VACUUM — fail this gate
    vacuum_files+=("${rel}")
done < <(find "${VERIFICATION_DIR}" -type f -name '*.rs' -print0)

echo "================================================================"
echo "  Verus production-binding audit (ABSOLUTE — no exceptions)"
echo "================================================================"
printf "  STRONG (direct crates/ binding): %d\n" "${#strong_files[@]}"
printf "  WEAK (production_inner/ mirror): %d\n" "${#weak_files[@]}"
printf "  VACUUM (no production binding):  %d\n" "${#vacuum_files[@]}"
echo

if (( ${#vacuum_files[@]} > 0 )); then
    echo "  VACUUM files — GOD RULE 2 VIOLATION:"
    for f in "${vacuum_files[@]}"; do
        echo "    - ${f}"
    done
    echo
    echo "  Every VACUUM file MUST be fixed. NO EXCEPTIONS."
    echo "  Options:"
    echo "    1. Bind via #[path = \".../crates/.../src/...rs\"] + assume_specification"
    echo "    2. Bind via #[path = \"production_inner/...rs\"] + assume_specification"
    echo "    3. DELETE the file"
    echo
    echo "  There is no fourth option. There is no override. There is no allowlist."
    echo "  Hand-written shadow types are NOT proof. They prove nothing."
    echo "  If your spec cannot bind to production, delete it."
fi

if (( ${#vacuum_files[@]} > 0 )); then
    exit 1
fi
exit 0