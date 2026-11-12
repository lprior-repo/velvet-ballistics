# Final Evidence Decision: vb-qi37.8

**bead_id**: vb-qi37.8
**decision_date**: 2026-05-17
**scope**: current-tree proof repair.

## Decision Summary

| Criterion | Verdict |
|-----------|---------|
| Gate 8 Kani harness coverage | PASS |
| StepState Kani parity | PASS |
| StepState Verus mirror | PASS |
| BudgetArithmetic TLC bounded model | PASS |
| Durable raw evidence paths | PASS |
| Traceability scoped to current evidence | PASS |
| `PO-030` overclaim prevention | PASS |
| Formal verifier review | APPROVED |
| Black-hat review | APPROVED |
| Truth-serum audit | APPROVED |

## Status

**APPROVED FOR SCOPED LANDING**

The current proof repair may land as scoped evidence. It does not close `PO-030` full pipeline composition.

## Deferred Items

| Item | Status | Reason |
|------|--------|--------|
| `PO-004` Gate 8 Miri | `DEFERRED_GLOBAL` | Not rerun in current repair. |
| `PO-030` full pipeline Kani composition | `DEFERRED_GLOBAL` | Not refreshed by Gate 8-only evidence. |
| Gate 8 Verus | `DEFERRED_GLOBAL` | Not run or claimed. |

## Sign-Off

| Role | Status | Date |
|------|--------|------|
| formal-verifier | APPROVED | 2026-05-17 |
| black-hat-reviewer | APPROVED | 2026-05-17 |
| truth-serum | APPROVED | 2026-05-17 |
