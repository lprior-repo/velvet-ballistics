# vb-9wg3 Final Evidence Decision

## Decision

PASS for scoped delivery.

## Basis

- Kani exact-width refinement: PASS, five harnesses over symbolic full-width domains.
- TLC BudgetArithmetic model: PASS, 166 generated states, 84 distinct states, depth 2.
- `vb_core` build: PASS.
- Black-hat review: PASS.
- Truth-serum audit: PASS.

## Limitations

- Manual TLA transcription map, not parser-backed TLA AST linkage.
- Workspace `moon ci`: `FAIL_OUT_OF_SCOPE_DIRTY_WORKTREE` due unrelated dirty checkout failures.
- No PO-030 full validation pipeline composition claim.
- No Gate 8 Verus or Miri claim.
