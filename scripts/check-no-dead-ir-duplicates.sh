#!/usr/bin/env bash
# scripts/check-no-dead-ir-duplicates.sh
# Bead: vb-dedup.* (DEDUP-9)
#
# CI gate that fails the build if any of the 7 dead IR-type duplicate files
# (excised in bead series `vb-dedup.1..7`) re-appear, OR if any tombstone
# `*.removed` / `*.bak` / `*.orig` files linger under `crates/vb_core/src/`.
#
# Canonical sources of truth:
#   - `crates/vb_core/src/workflow/types.rs` (CompiledWorkflow, CompiledNode,
#     CompiledNodeKind, ExprProgram, ExprOp, AccessorProgram, ResourceContract,
#     WorkflowError, ExprBranch, SlotBranch, check_expr_stack_bound)
#   - `crates/vb_core/src/workflow/validation.rs` (validate_parts,
#     validate_budget, all `validate_*` helpers)
# Both are re-exported through `crates/vb_core/src/lib.rs:124-127`.
set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

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

# Also reject any lingering tombstone files in the vb_core source tree.
TOMBSTONES="$(find crates/vb_core/src \( -name '*.removed' -o -name '*.bak' -o -name '*.orig' \) -print 2>/dev/null || true)"
if [[ -n "$TOMBSTONES" ]]; then
  printf 'REGRESSION: tombstone file(s) found under crates/vb_core/src:\n' >&2
  printf '%s\n' "$TOMBSTONES" >&2
  printf '  Clean up under bead vb-dedup.6.\n' >&2
  FAILED=1
fi

if [[ "$FAILED" -ne 0 ]]; then
  printf 'check-no-dead-ir-duplicates: FAILED\n' >&2
  exit 1
fi

printf 'check-no-dead-ir-duplicates: OK (no dead IR-type duplicates, no tombstones)\n'
