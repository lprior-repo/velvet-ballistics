# vb-3l40a: Contract validation boundary tests - review and repair

## Status
Tests fixed and verified. All 1909 vb_runtime lib tests pass.

## What was broken
Four shard boundary tests in `crates/vb_runtime/src/shard/lifecycle_tests/contract_validation_tests.rs` had insufficient assertions:

1. **Negative boundary tests (2):** Compared `Result<Vec<...>>` values via `assert_eq!` instead of unwrapping with `?` and comparing actual event vectors. Only checked `!run_state_contains` (not admitted) but not `!terminal_runs_contains` (not terminal).

2. **Positive boundary test 1 (setconst path):** Used `test_shard()` (no journal) so no durable evidence was verified. Only checked `terminal_runs_contains` without verifying actual `RunFinished` journal event or absence of `RunFailed`. Did not assert the `ActionCompletedEnvelope` output value bytes.

3. **Positive boundary test 2 (seeded input path):** Used `do_workflow` (Do-only, no Finish node) so the run could not reach a proper terminal state. Used `test_shard()` (no journal). Same missing evidence verification as test 1.

## What was fixed

### Negative boundary tests
- Changed `journal.snapshot()` to `journal.snapshot()?` to unwrap the Result and compare actual `Vec<RuntimeJournalEvent>` values.
- Added `assert!(!shard.terminal_runs_contains(run), "run must not be terminal")` to both negative tests.

### Positive boundary test 1 (shard_boundary_accepts_valid_contract_and_executes_do)
- Replaced `test_shard()` with `Shard::new_with_journal(...)` using real `VolatileRuntimeJournal`.
- Added journal event verification after tick 2:
  - Filter events for `RunFinished`, assert exactly 1 with correct run id and result slot.
  - Filter events for `RunFailed`, assert 0.
  - Filter for `ActionCompletedEnvelope`, assert exactly 1 with correct ticket, output slot, and serialized value bytes (`SlotValue::I64(100)`).

### Positive boundary test 2 (shard_boundary_accepts_valid_contract_with_seeded_input)
- Created new `input_slot_do_workflow(action_id)` helper: Do->Finish workflow with 2 slots, typed `input_slots` (`SlotIdx::ZERO` as `InputSlotKind::I64`). Does not alter the shared `do_workflow` used by other tests.
- Replaced `test_shard()` with real `VolatileRuntimeJournal`.
- Added same journal event verification: exactly 1 `RunFinished` (correct run/result slot), 0 `RunFailed`, exactly 1 `ActionCompletedEnvelope` with correct serialized value (`SlotValue::I64(200)`).

## Evidence
- Targeted 4 boundary tests: `.beads/vb-3l40a/evidence/admission-review-fix/outcomes.log` (4 passed, 0 failed)
- Full lib suite: `.beads/vb-3l40a/evidence/admission-review-fix/outcomes-lib.log` (1909 passed, 0 failed)

## Files changed
- `crates/vb_runtime/src/shard/lifecycle_tests/contract_validation_tests.rs` (test source only, no production edits)
