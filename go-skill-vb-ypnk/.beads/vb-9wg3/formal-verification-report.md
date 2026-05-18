# vb-9wg3 Formal Verification Report

## Decision

PASS for the scoped vb-9wg3 obligation: the reviewed Rust projection of the BudgetArithmetic four-limb abstraction now has exact-width Kani evidence against Rust integer domains, and the existing TLC model still passes.

## Evidence Summary

| Obligation | Lane | Status | Evidence |
| --- | --- | --- | --- |
| BudgetArithmetic word encoding is exact for Rust `u64` | Kani | PASS | `.beads/vb-9wg3/evidence/kani-budget-arithmetic-refinement.out` |
| BudgetArithmetic word ordering matches Rust `u64` ordering | Kani | PASS | `.beads/vb-9wg3/evidence/kani-budget-arithmetic-refinement.out` |
| BudgetArithmetic add abstraction matches Rust `checked_add` | Kani | PASS | `.beads/vb-9wg3/evidence/kani-budget-arithmetic-refinement.out` |
| BudgetArithmetic subtract abstraction matches Rust `checked_sub` | Kani | PASS | `.beads/vb-9wg3/evidence/kani-budget-arithmetic-refinement.out` |
| BudgetArithmetic field width maxima match Rust `u16`/`u32` | Kani | PASS | `.beads/vb-9wg3/evidence/kani-budget-arithmetic-refinement.out` |
| BudgetArithmetic finite state model invariants | TLC | PASS | `.beads/vb-9wg3/evidence/tlc-budget-arithmetic.out` |
| Normal `vb_core` build | Cargo | PASS | `.beads/vb-9wg3/evidence/cargo-check-vb-core.out` |
| TLA transcription review map | Manual proof audit | PASS | `.beads/vb-9wg3/tla-transcription-map.md` |
| Workspace CI | Moon | FAIL_OUT_OF_SCOPE_DIRTY_WORKTREE | `.beads/vb-9wg3/evidence/moon-ci.out` |

## Important Limits

- This is not full validation pipeline composition.
- This is not Gate 8 Verus.
- This is not Gate 8 Miri.
- This is not a mechanically linked proof over the TLA source AST; it is a reviewed Rust projection of the TLA limb equations checked against Rust integer semantics, with a source hash and transcription map.
- `moon ci` is not green in this dirty checkout because of failures outside the vb-9wg3 touched files.
