# STATE.md: vb-qi37.8

**bead_id**: vb-qi37.8
**title**: validate/compile: Prove and complete shared validation pipeline
**current_state**: scoped proof repair approved
**updated**: 2026-05-17

## Current Evidence State

| Area | State |
|------|-------|
| Gate 8 Kani source repair | COMPLETE |
| Gate 8 Kani reruns | PASS |
| StepState Kani rerun | PASS |
| StepState Verus rerun | PASS |
| BudgetArithmetic TLC rerun | PASS |
| JSONL validation | PASS |
| formal-verifier review | APPROVED |
| black-hat-reviewer review | APPROVED |
| truth-serum audit | APPROVED |

## Current Scope

The 2026-05-17 repair is scoped to current-tree proof evidence. It does not claim a refresh of every historical validation-pipeline obligation.

## Deferred Work

| Item | Status | Reason |
|------|--------|--------|
| `PO-004` Gate 8 Miri | `DEFERRED_GLOBAL` | Not rerun in current repair. |
| `PO-030` full pipeline Kani composition | `DEFERRED_GLOBAL` | Not proved by Gate 8 evidence. |
| Gate 8 Verus | `DEFERRED_GLOBAL` | Not run or claimed. |

## Raw Evidence

See `.beads/vb-qi37.8/verification-ledger.jsonl` for authoritative command evidence paths.
