#!/usr/bin/env bash
# scripts/check-no-dead-ir-duplicates.sh
# Bead: vb-dedup.* (DEDUP-9, DEDUP-10)
#
# CI gate that fails the build if:
#   1. Any of the 7 dead IR-type duplicate files (excised in bead series
#      `vb-dedup.1..7`) re-appear on disk.
#   2. Any tombstone `*.removed` / `*.bak` / `*.orig` files linger under
#      `crates/vb_core/src/`.
#   3. Any documentation or verification artifact still CITES a deleted
#      dead path. Catches defects like FINDING-R1 where
#      `docs/compiled-ir.md:26` (now fixed) cited the deleted
#      `crates/vb_core/src/nodes.rs`.
#
# Canonical sources of truth:
#   - `crates/vb_core/src/workflow/types.rs` (CompiledWorkflow, CompiledNode,
#     CompiledNodeKind, ExprProgram, ExprOp, AccessorProgram, ResourceContract,
#     WorkflowError, ExprBranch, SlotBranch, check_expr_stack_bound)
#   - `crates/vb_core/src/workflow/validation.rs` (validate_parts,
#     validate_budget, all `validate_*` helpers)
# Both are re-exported through `crates/vb_core/src/lib.rs:127-130`.
set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# ============================================================================
# (1) Dead path files must not exist on disk.
# ============================================================================

DEAD_PATHS=(
  "crates/vb_core/src/nodes.rs"
  "crates/vb_core/src/expressions.rs"
  "crates/vb_core/src/accessors.rs"
  "crates/vb_core/src/validation.rs"
  "crates/vb_core/src/validation"
  "crates/vb_core/src/compiled_workflow.rs"
  "crates/vb_core/src/compiled_workflow.rs.removed"
  "crates/vb_core/src/kani_resource_contract_validation_18_fields.rs"
)

FAILED=0
for path in "${DEAD_PATHS[@]}"; do
  if [[ -e "$path" ]]; then
    printf 'REGRESSION: dead IR-type duplicate re-appeared at %s\n' "$path" >&2
    printf '  See bead vb-dedup.* (DEDUP-1..7) in to-fix/13-dead-ir-deduplication-plan.md.\n' >&2
    printf '  Canonical locations: crates/vb_core/src/workflow/{types,validation}.rs.\n' >&2
    FAILED=1
  fi
done

# ============================================================================
# (2) No lingering tombstone files in the vb_core source tree.
# ============================================================================

TOMBSTONES="$(find crates/vb_core/src \( -name '*.removed' -o -name '*.bak' -o -name '*.orig' \) -print 2>/dev/null || true)"
if [[ -n "$TOMBSTONES" ]]; then
  printf 'REGRESSION: tombstone file(s) found under crates/vb_core/src:\n' >&2
  printf '%s\n' "$TOMBSTONES" >&2
  printf '  Clean up under bead vb-dedup.6.\n' >&2
  FAILED=1
fi

# ============================================================================
# (3) No documentation/verification artifact cites a deleted dead path.
#
# Defense in depth against FINDING-R1: a stale citation
# `docs/compiled-ir.md:26` referencing the excised `nodes.rs` would
# survive any amount of file-existence checking unless we scan the
# textual content of docs and verification artifacts. This block scans
# every file under the listed roots and top-level doc files for any
# string matching the dead-path pattern, then filters out allowlisted
# paths that legitimately document the deletion history.
# ============================================================================

printf 'Scanning docs/, verification/, and top-level doc files for stale dead-path citations...\n'

# Regex for any of the 7 dead paths (file-level only; we already check
# directory existence for `crates/vb_core/src/validation` in part 1).
STALE_REGEX='crates/vb_core/src/(nodes|expressions|accessors|validation|compiled_workflow|kani_resource_contract_validation_18_fields)\.rs'

# Paths to scan: either a single file (relative to repo root) or a
# directory recursively scanned for `*.md`, `*.rs`, `*.txt`, `*.jsonl`,
# `*.json` files. The `verification/` tree is `.rs`-heavy, the
# `docs/` tree is `.md`-heavy, the top-level entries mix both.
SCAN_TARGETS=(
  "docs/"
  "verification/"
  "proof-to-rust-map.md"
  "rust-refinement-obligations.jsonl"
  "velvet-ballistics-MASTER.md"
)

# Allowlist: paths that LEGITIMATELY reference the dead paths. These
# are historical audit evidence, the dedup plan itself, the script
# itself, and research artifacts that pre-date the dedup.
STALE_ALLOWLIST=(
  "to-fix/13-dead-ir-deduplication-plan.md"
  "to-fix/12-resource-contract-admission-gap.md"
  ".beads/vb-o5zb.5/black-hat-review.md"
  ".beads/vb-o5zb.5/closure-reconciliation-packet.md"
  "fuzz/research/pub-types-raw.txt"
  "fuzz/research/crate-inventory.md"
  "fuzz/research/pub-fn-raw.txt"
  "scripts/check-no-dead-ir-duplicates.sh"
  # The whole `docs/black-hat-review-2026-06-07/` directory is a
  # historical review packet; matches inside it are not actionable.
  "docs/black-hat-review-2026-06-07/"
)

# Helper: is a file path under an allowlisted prefix?
is_allowlisted() {
  local file="$1"
  for allowed in "${STALE_ALLOWLIST[@]}"; do
    if [[ "$file" == "$allowed" || "$file" == "$allowed"* ]]; then
      return 0
    fi
  done
  return 1
}

# Helper: collect the set of files to scan for a given target.
collect_files_for_target() {
  local target="$1"
  if [[ -d "$target" ]]; then
    # Recursive: pick up .md, .rs, .txt, .jsonl, .json. We do not
    # scan binary or .png/.svg assets.
    find "$target" -type f \
      \( -name '*.md' -o -name '*.rs' -o -name '*.txt' \
         -o -name '*.jsonl' -o -name '*.json' \) \
      -print 2>/dev/null || true
  elif [[ -f "$target" ]]; then
    printf '%s\n' "$target"
  fi
}

# Collect all candidate files first so we can report a stable, sorted list.
ALL_FILES=()
for target in "${SCAN_TARGETS[@]}"; do
  while IFS= read -r f; do
    [[ -n "$f" ]] && ALL_FILES+=("$f")
  done < <(collect_files_for_target "$target")
done

# Sort + deduplicate (rg would do this too, but we want a clean list).
if [[ ${#ALL_FILES[@]} -gt 0 ]]; then
  while IFS= read -r f; do
    SORTED_FILES+=("$f")
  done < <(printf '%s\n' "${ALL_FILES[@]}" | sort -u)
else
  SORTED_FILES=()
fi

# Scan each surviving file with rg. rg returns 0 on match, 1 on no match,
# 2 on error; we only treat 0 as a hit.
STALE_HITS=()
for file in "${SORTED_FILES[@]}"; do
  if is_allowlisted "$file"; then
    continue
  fi
  # `rg --quiet` suppresses normal output, exits 0 on match. We capture
  # the path; the offending lines will be reported by a second pass.
  if rg --quiet --fixed-strings "crates/vb_core/src/nodes.rs" "$file" 2>/dev/null \
     || rg --quiet --fixed-strings "crates/vb_core/src/expressions.rs" "$file" 2>/dev/null \
     || rg --quiet --fixed-strings "crates/vb_core/src/accessors.rs" "$file" 2>/dev/null \
     || rg --quiet --fixed-strings "crates/vb_core/src/validation.rs" "$file" 2>/dev/null \
     || rg --quiet --fixed-strings "crates/vb_core/src/compiled_workflow.rs" "$file" 2>/dev/null \
     || rg --quiet --fixed-strings "crates/vb_core/src/kani_resource_contract_validation_18_fields.rs" "$file" 2>/dev/null; then
    STALE_HITS+=("$file")
  fi
done

if [[ ${#STALE_HITS[@]} -gt 0 ]]; then
  printf 'REGRESSION: stale citation(s) of a deleted dead path:\n' >&2
  for hit in "${STALE_HITS[@]}"; do
    printf '  %s\n' "$hit" >&2
    # Show the offending lines for triage.
    while IFS= read -r line; do
      [[ -n "$line" ]] && printf '    %s\n' "$line" >&2
    done < <(rg -n --no-heading "$STALE_REGEX" "$hit" 2>/dev/null || true)
  done
  printf '  Fix by removing the citation or updating it to the canonical path:\n' >&2
  printf '    crates/vb_core/src/workflow/types.rs   (IR types)\n' >&2
  printf '    crates/vb_core/src/workflow/validation.rs (validation helpers)\n' >&2
  printf '  See FINDING-R1 in the post-14089545a black-hat review.\n' >&2
  FAILED=1
fi

if [[ "$FAILED" -ne 0 ]]; then
  printf 'check-no-dead-ir-duplicates: FAILED\n' >&2
  exit 1
fi

printf 'check-no-dead-ir-duplicates: OK (no dead IR-type duplicates, no tombstones, no stale citations)\n'
