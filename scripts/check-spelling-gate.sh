#!/usr/bin/env bash
# check-spelling-gate.sh
# Mechanical gate: rejects legacy title-case product spelling in active code/docs.
# The canonical product/binary/package/bead-rig spelling is velvet-ballistics;
# Rust crate/module and bead database identifiers remain velvet_ballistics.
#
# Allowlisted path patterns (skip these entirely):
#   - .beads/            — Bead artifacts and CI output (not source)
#   - .jj/               — JJ internal working-copy state
#   - .evidence/         — Repository-root raw evidence artifacts only
#   - evidence/          — Repository-root benchmark/release evidence only
#   - target/            — Build artifact directories
#   - fixtures/          — Repository-root negative/positive gate fixtures
#   - .forge/            — Historical architecture-forge migration artifacts
#   - .bead-progress/    — Historical bead progress evidence
#   - ws-batch-*/        — Historical batch workspace snapshots
#   - tests/             — Test clippy is not strict
#   - benches/           — Bench clippy is not strict
#   - velvet-ballistics-MASTER.md — The master contract file itself
#   - check-spelling-gate.sh — This script (self-referential)
#
# Allowlisted content patterns (in OTHER files):
#   - velvet-ballistics-MASTER.md     — reference to the master contract file
#   - /home/.*/velvet-ballistics/     — source checkout path (external migration artifact)
#   - Legacy-name documentation that quotes the legacy title-case spelling
#   - ADR review regexes that search for legacy spellings
#
# Usage: bash scripts/check-spelling-gate.sh
# Exit 0: gate passes (no violations)
# Exit 1: gate fails (violations found)

set -euo pipefail

ROOT="$(pwd -P)"
FORBIDDEN_TOKEN="Velvet Ballastics"
CANONICAL_REPLACEMENT="velvet-ballistics"

if [[ -n "${MOON_TASK_ID:-}" ]] && command -v grep >/dev/null 2>&1; then
    SEARCH_TOOL=(grep)
elif command -v rtk >/dev/null 2>&1; then
    SEARCH_TOOL=(rtk grep)
elif [[ -n "${HOME:-}" && -x "${HOME}/.local/share/mise/shims/rtk" ]]; then
    SEARCH_TOOL=("${HOME}/.local/share/mise/shims/rtk" grep)
elif [[ -n "${HOME:-}" && -x "${HOME}/.cargo/bin/rtk" ]]; then
    SEARCH_TOOL=("${HOME}/.cargo/bin/rtk" grep)
elif command -v grep >/dev/null 2>&1; then
    SEARCH_TOOL=(grep)
else
    echo "FATAL: no spelling search backend available (rtk or grep required)" >&2
    exit 2
fi

run_search() {
    "${SEARCH_TOOL[@]}" "$@"
}

run_search_to_file() {
    local output_file="$1"
    shift

    if run_search "$@" >"$output_file"; then
        return 0
    fi

    local status=$?
    if [[ "$status" -eq 1 ]]; then
        return 1
    fi

    return 2
}

scrub_allowed_occurrences() {
    local scrubbed="$1"
    local span
    local legacy_backtick_span="\`${FORBIDDEN_TOKEN}\`"

    scrubbed="${scrubbed//${FORBIDDEN_TOKEN}-MASTER.md/}"
    scrubbed="${scrubbed//${FORBIDDEN_TOKEN}\/v2/}"

    if [[ "$scrubbed" == *"Legacy names such as ${legacy_backtick_span}"* ]]; then
        scrubbed="${scrubbed//${legacy_backtick_span}/}"
    fi

    if [[ "$scrubbed" == *'rg -n "'* && "$scrubbed" == *"${FORBIDDEN_TOKEN}|"* ]]; then
        scrubbed="${scrubbed//${FORBIDDEN_TOKEN}|/}"
    fi

    while [[ "$scrubbed" =~ /home/[^[:space:]]*/${FORBIDDEN_TOKEN}/ ]]; do
        span="${BASH_REMATCH[0]}"
        scrubbed="${scrubbed/"$span"/}"
    done

    while [[ "$scrubbed" =~ https?://[^[:space:]]*dolthub[.]com/[^[:space:]]*${FORBIDDEN_TOKEN}[^[:space:]]* ]]; do
        span="${BASH_REMATCH[0]}"
        scrubbed="${scrubbed/"$span"/}"
    done

    printf '%s' "$scrubbed"
}

line_has_unexcused_token() {
    local linecontent="$1"
    local scrubbed
    scrubbed="$(scrub_allowed_occurrences "$linecontent")"
    [[ "$scrubbed" == *"$FORBIDDEN_TOKEN"* ]]
}

echo "=== Spelling Gate: $FORBIDDEN_TOKEN vs $CANONICAL_REPLACEMENT ===" >&2

count=0

# Collect all files containing the forbidden spelling (recursive, with includes).
# GNU grep-compatible: --include works with -r. Search errors fail closed.
files=()
file_list_tmp="$(mktemp)"
if run_search_to_file "$file_list_tmp" \
    -rl --include='*.rs' \
        --include='*.toml' \
        --include='*.yaml' \
        --include='*.yml' \
        --include='*.md' \
        --include='*.sh' \
        --include='*.py' \
        "$FORBIDDEN_TOKEN" "$ROOT"; then
    mapfile -t files < "$file_list_tmp"
elif [[ "$?" -eq 1 ]]; then
    files=()
else
    rm -f "$file_list_tmp"
    echo "FATAL: spelling recursive search failed" >&2
    exit 2
fi
rm -f "$file_list_tmp"

for file in "${files[@]}"; do
    rel_file="$file"
    case "$file" in
        "$ROOT"/*) rel_file="${file#"$ROOT"/}" ;;
    esac

    # Path-based exclusions — skip only repository-root artifact trees or
    # non-production test/bench/source-policy fixtures. Do not match arbitrary
    # parent workspace names or nested docs/src directories.
    case "$rel_file" in
        .beads/*) continue ;;
        .jj/*) continue ;;
        .evidence/*) continue ;;
        evidence/*) continue ;;
        .forge/*) continue ;;
        .bead-progress/*) continue ;;
        ws-batch-*/*) continue ;;
        fixtures/*) continue ;;
        target/*) continue ;;
        target_nosccache/*) continue ;;
        target_debug_clean/*) continue ;;
        target_clean/*) continue ;;
        tests/*|*/tests/*) continue ;;
        benches/*|*/benches/*) continue ;;
        velvet-ballistics-MASTER.md) continue ;;
        scripts/check-spelling-gate.sh) continue ;;
        BIG-ASS-TESTING-TO-FIX.md) continue ;;
        # naming_scan defines LEGACY_* constants with wrong spelling as VALUES (intentional detection data)
        naming_scan/*|*/naming_scan/*) continue ;;
        # Test files in src/ (*_tests.rs) — test clippy is not strict
        *'_tests.rs') continue ;;
        */tests.rs) continue ;;
    esac

    # For each remaining file, get matching lines. Search errors fail closed.
    # GNU grep format: linenum:content
    line_list_tmp="$(mktemp)"
    if run_search_to_file "$line_list_tmp" \
        -n --include='*.rs' \
           --include='*.toml' \
           --include='*.yaml' \
           --include='*.yml' \
           --include='*.md' \
           --include='*.sh' \
           --include='*.py' \
           "$FORBIDDEN_TOKEN" "$file"; then
        true
    elif [[ "$?" -eq 1 ]]; then
        rm -f "$line_list_tmp"
        continue
    else
        rm -f "$line_list_tmp"
        echo "FATAL: spelling per-file search failed: $file" >&2
        exit 2
    fi

    while IFS= read -r line; do
        linenum="${line%%:*}"
        linecontent="${line#*:}"

        # Occurrence-scoped allowlist patterns. Only the matching span is
        # excused; an extra forbidden token on the same line remains active.
        if ! line_has_unexcused_token "$linecontent"; then
            continue
        fi

        echo "VIOLATION: $file:$linenum: wrong spelling '$FORBIDDEN_TOKEN' (use '$CANONICAL_REPLACEMENT')" >&2
        count=$((count + 1))
    done < "$line_list_tmp"
    rm -f "$line_list_tmp"
done

echo "=== Spelling Gate complete: $count violations ===" >&2

if [[ $count -gt 0 ]]; then
    echo "" >&2
    echo "Hint: Replace legacy product spelling with '$CANONICAL_REPLACEMENT' or document an exact allowlisted migration artifact." >&2
    echo "HZ-DRIFT-001: legacy title-case product spelling remains blocked in active files." >&2
    echo "Allowlisted path patterns (excluded entirely):" >&2
    echo "  - .beads/ (bead artifacts and CI output)" >&2
    echo "  - .jj/ (JJ internal state)" >&2
    echo "  - .evidence/ and evidence/ at workspace root only (evidence artifacts)" >&2
    echo "  - .forge/, .bead-progress/, and ws-batch-*/ (historical migration artifacts)" >&2
    echo "  - fixtures/ (gate fixtures)" >&2
    echo "  - target/ (build artifacts)" >&2
    echo "  - tests/ and benches/ (test/bench clippy is not strict)" >&2
    echo "  - velvet-ballistics-MASTER.md (master contract file)" >&2
    echo "Allowlisted content patterns:" >&2
    echo "  - velvet-ballistics-MASTER.md (reference to master file)" >&2
    echo "  - /home/.*/velvet-ballistics/ (source checkout path, migration artifact)" >&2
    echo "  - Legacy names such as '$FORBIDDEN_TOKEN' (legacy-name documentation)" >&2
    echo "  - ADR review regexes that search for '$FORBIDDEN_TOKEN'" >&2
    exit 1
fi

exit 0
