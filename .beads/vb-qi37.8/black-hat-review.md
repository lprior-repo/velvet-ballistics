# Black-Hat Review: vb-qi37.8

**bead_id**: vb-qi37.8
**reviewed**: 2026-05-17
**scope**: current-tree proof repair for Gate 8 Kani, StepState Kani/Verus, BudgetArithmetic TLC, and PO-030 deferral hygiene.

## Status

**APPROVED**

The black-hat reviewer re-reviewed the corrected artifacts and returned `APPROVE` after traceability was narrowed and durable Verus/TLC raw evidence files were added.

## Reviewed Artifacts

| Artifact | Result |
|----------|--------|
| `crates/vb_validate/src/kani_gate_08_accessor.rs` | PASS |
| `.beads/vb-qi37.8/proof-evidence.md` | PASS |
| `.beads/vb-qi37.8/formal-verification-report.md` | PASS |
| `.beads/vb-qi37.8/verification-ledger.jsonl` | PASS |
| `.beads/vb-qi37.8/traceability-matrix.jsonl` | PASS |
| `.beads/vb-qi37.8/evidence/verus-step-state-machine.out` | PASS |
| `.beads/vb-qi37.8/evidence/tlc-budget-arithmetic.out` | PASS |

## Review Findings

| Check | Verdict |
|-------|---------|
| Gate 8 harness names match source and evidence | PASS |
| Gate 8 Kani raw paths are durable and current | PASS |
| StepState Kani evidence has raw path and cover evidence | PASS |
| StepState Verus has durable raw output | PASS |
| BudgetArithmetic TLC has durable raw output | PASS |
| `PO-030` full pipeline composition remains deferred | PASS |
| Gate 8 Verus is not claimed | PASS |

## Non-Claims

| Item | Status | Reason |
|------|--------|--------|
| Full validation pipeline Kani composition (`PO-030`) | `DEFERRED_GLOBAL` | Not refreshed by Gate 8-only evidence. |
| Gate 8 Verus | `DEFERRED_GLOBAL` | No Gate 8 Verus proof was run or claimed. |
| Historical all-gate Miri/proptest/test coverage | Not in current repair scope | Current artifacts only claim rerun evidence listed in the ledger. |

## Verdict

**BLACK-HAT VERDICT: APPROVED**

Approval is scoped to the corrected current-tree proof repair. It is not approval to treat Gate 8 Kani evidence as full-pipeline composition proof.
