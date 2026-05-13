# vb-qi37.2.1 STATE

- Current State: 9 (Test Review — REJECTED, routing back to State 8)
- Title: runtime: Define aggregate resource budget model
- Parent: vb-qi37.2
- Priority: P0
- Blocking: vb-qi37.2.2, vb-qi37.2.3, vb-qi37.2.4
- Attempt: 1 of 7

## State 9 Summary

`test-reviewer` ran Mode 2 (Suite Inquisition) and produced `test-suite-review.md` with **STATUS: REJECTED**.

### Test Review Outcome

**VERDICT: REJECTED**

Tier 0 (Static):
- 16 LETHAL findings: weak `assert!(result.is_err())` assertions without exact variant checks in `aggregate_budget_vb_qi37_2_1.rs`
- 5 LETHAL findings: missing test coverage for `from_workflow` (behaviors 1-7)
- 6 LETHAL findings: missing test coverage for `from_whole_workflow_budget` (behaviors 8-13)
- 18 LETHAL findings: missing test coverage for `validate_aggregate_budget` (behaviors 14-31)
- 7 LETHAL findings: missing test coverage for `admit_run_with_budget` (behaviors 30-36)
- 2 MAJOR findings: missing field assertions in fits_within tests, coverage below thresholds

Tier 1 (Execution): PASS — 1717 tests passed, 0 failed, 0 flaky
Tier 2 (Coverage): FAIL — budget.rs 76.02% line / 72.50% branch (target ≥90%)
Tier 3 (Mutation): SKIP — timed out

### LETHAL Findings Detail

**16 weak assertions** (L-1) in `aggregate_budget_vb_qi37_2_1.rs`:
- Lines 490, 531, 572: add overflow without exact variant
- Lines 800, 841, 923, 964, 1005, 1046, 1087, 1128: subtract underflow without exact variant
- Lines 1266, 1349, 1386, 1423, 1460: fits_within capacity without exact variant/fields

**5 missing behavior groups** (L-2 through L-5):
- `from_workflow`: 0 tests (need 7)
- `from_whole_workflow_budget`: 0 tests (need 6)
- `validate_aggregate_budget`: 0 tests (need 18+)
- `admit_run_with_budget`: 0 tests (need 7 or waiver)

## Repair Routing

**Routing: State 9 → State 8 (Test Writer)**

`test-repair-guide.md` is written. Test-writer must:
1. Fix all 16 weak assertions following the pattern at lines 189-196
2. Add 7 `from_workflow` tests
3. Add 6 `from_whole_workflow_budget` tests
4. Add 18+ `validate_aggregate_budget` tests
5. Add or waive `admit_run_with_budget` tests

## Blocking Status

This bead BLOCKS:
- `vb-qi37.2.2` — aggregate budget enforcement at tick admission
- `vb-qi37.2.3` — aggregate budget release on finish/fail/cancel
- `vb-qi37.2.4` — aggregate budget audit journal integration

## Next Action

Re-run State 8 (Test Writer) to address `test-repair-guide.md` findings. Then re-submit for State 9 review.
