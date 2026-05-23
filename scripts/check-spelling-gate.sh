#!/usr/bin/env bash
# check-spelling-gate.sh
# Mechanical gate: rejects new velvet-ballistics (wrong) spelling in active code/docs.
# Canonical spelling is velvet-ballastics (correct).
#
# Allowlisted path patterns (skip these entirely):
#   - .beads/            — Bead artifacts and CI output (not source)
#   - .jj/               — JJ internal working-copy state
#   - target/            — Build artifact directories
#   - tests/             — Test clippy is not strict
#   - benches/           — Bench clippy is not strict
#   - velvet-ballistics-MASTER.md — The master contract file itself
#   - check-spelling-gate.sh — This script (self-referential)
#
# Allowlisted content patterns (in OTHER files):
#   - velvet-ballistics-MASTER.md     — reference to the master contract file
#   - /home/.*/velvet-ballistics/   — source checkout path (external migration artifact)
#   - FORBIDDEN_FEATURE_NAMES        — the check script uses wrong spelling as a forbid-tag
#   - '"velvet-ballistics" is invalid' — rule statement in AGENTS.md
#
# Usage: bash scripts/check-spelling-gate.sh
# Exit 0: gate passes (no violations)
# Exit 1: gate fails (violations found)

set -euo pipefail

ROOT="$(pwd -P)"

echo "=== Spelling Gate: velvet-ballistics vs velvet-ballastics ===" >&2

count=0

# Collect all files containing the wrong spelling (recursive, with includes)
# GNU grep-compatible: --include works with -r
mapfile -t files < <(
    rtk grep -rl --include='*.rs' \
                 --include='*.toml' \
                 --include='*.yaml' \
                 --include='*.yml' \
                 --include='*.md' \
                 --include='*.sh' \
                 --include='*.py' \
                 'velvet-ballistics' "$ROOT" 2>/dev/null || true
)

for file in "${files[@]}"; do
    # Path-based exclusions — skip these entirely
    case "$file" in
        *'/.beads/'*) continue ;;
        *'/.jj/'*) continue ;;
        *'/.evidence/'*) continue ;;
        *'/evidence/'*) continue ;;
        *'/target/'*) continue ;;
        *'/target_nosccache/'*) continue ;;
        *'/target_debug_clean/'*) continue ;;
        *'/target_clean/'*) continue ;;
        */tests/*) continue ;;
        */benches/*) continue ;;
        */velvet-ballistics-MASTER.md) continue ;;
        */check-spelling-gate.sh) continue ;;
        *'/BIG-ASS-TESTING-TO-FIX.md') continue ;;
        *'final-'*) continue ;;
        *'proof-repair-'*) continue ;;
        *'black-hat-review-'*) continue ;;
        # naming_scan defines LEGACY_* constants with wrong spelling as VALUES (intentional detection data)
        */naming_scan/*) continue ;;
        # Test files in src/ (*_tests.rs) — test clippy is not strict
        *'_tests.rs') continue ;;
    esac

    # For each remaining file, get matching lines
    # GNU grep format: linenum:content
    while IFS= read -r line; do
        linenum="${line%%:*}"
        linecontent="${line#*:}"

        # Allowlist content patterns:
        # 1. Reference to the master file itself
        if [[ "$linecontent" == *'velvet-ballistics-MASTER.md'* ]]; then
            continue
        fi
        # 2. Source checkout path (external migration artifact)
        if [[ "$linecontent" == *'/home/'*'/velvet-ballistics/'* ]]; then
            continue
        fi
        # 3. FORBIDDEN_FEATURE_NAMES in check scripts
        if [[ "$linecontent" == *'FORBIDDEN_FEATURE_NAMES'* ]]; then
            continue
        fi
        # 4. Rule text that states "velvet-ballistics is invalid" (AGENTS.md rule statement)
        if [[ "$linecontent" == *'velvet-ballistics` is invalid'* ]]; then
            continue
        fi
        # 5. Dolt remote URL (external system, can't be changed)
        if [[ "$linecontent" == *'dolthub.com/'*'velvet-ballistics'* ]]; then
            continue
        fi
        # 6. Test data in schema.rs/schema_tests.rs: wrong spelling as version string in test assertions
        if [[ "$linecontent" == *'velvet-ballistics/v2'* ]]; then
            continue
        fi

        echo "VIOLATION: $file:$linenum: wrong spelling 'velvet-ballistics' (use 'velvet-ballastics')" >&2
        count=$((count + 1))
    done < <(rtk grep -n --include='*.rs' \
                       --include='*.toml' \
                       --include='*.yaml' \
                       --include='*.yml' \
                       --include='*.md' \
                       --include='*.sh' \
                       --include='*.py' \
                       'velvet-ballistics' "$file" 2>/dev/null || true)
done

echo "=== Spelling Gate complete: $count violations ===" >&2

if [[ $count -gt 0 ]]; then
    echo "" >&2
    echo "Hint: The canonical spelling is 'velvet-ballastics'." >&2
    echo "Allowlisted path patterns (excluded entirely):" >&2
    echo "  - .beads/ (bead artifacts and CI output)" >&2
    echo "  - .jj/ (JJ internal state)" >&2
    echo "  - target/ (build artifacts)" >&2
    echo "  - tests/ and benches/ (test/bench clippy is not strict)" >&2
    echo "  - velvet-ballistics-MASTER.md (master contract file)" >&2
    echo "Allowlisted content patterns:" >&2
    echo "  - velvet-ballistics-MASTER.md (reference to master file)" >&2
    echo "  - /home/.*/velvet-ballistics/ (source checkout path, migration artifact)" >&2
    echo "  - FORBIDDEN_FEATURE_NAMES (spelling used as forbid-tag)" >&2
    echo "  - 'velvet-ballistics' is invalid (rule statement)" >&2
    exit 1
fi

exit 0
