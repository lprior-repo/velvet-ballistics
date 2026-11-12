# vb-3l40a Admission Review Fix Report

## Changes Made

### Implementation (crates/vb_runtime/src/shard/contract_validation.rs)
Rewrote validation to use separate global passes with explicit precedence:
1. `check_duplicate_ids` — global duplicate detection across all contracts
2. `check_empty_names` — global empty-name check across all contracts
3. `check_dense_index` — global index==ID check across all contracts
4. `validate_contract_capabilities` — global capability coverage check
5. `validate_workflow_action_binding` — workflow Do binding check

Each helper is ≤25 logical lines. No unsafe, unwrap, expect, panic, or ignored Result.

### Contract Addendum (.beads/vb-3l40a/admission-contract-addendum.md)
Added explicit "Validation Order (First Error Wins)" section documenting the six-step global pass sequence. Clarified that duplicate detection takes precedence over index errors, matching the implementation.

### Tests (crates/vb_runtime/src/shard/lifecycle_tests/contract_validation_tests.rs)
Addressed every finding from the independent review:

**BLOCKER — Journal immutability in negative shard tests:**
- `shard_boundary_validates_fabricated_id_before_admission`: attaches real VolatileRuntimeJournal, snapshots before command, asserts exact equality after failure
- `shard_boundary_validates_fabricated_id_with_inputs_before_admission`: same pattern for inputs variant

**MINOR — Exact error matching:**
- All negative tests now assert exact error strings via `assert_eq!` instead of `contains()`
- Example: `assert_eq!(error, "contract at index 0 has ID 999 (expected 0)")`

**MINOR — Outcome verification in positive tests:**
- Both positive shard boundary tests use real SetConst->Do->Finish workflows
- Capture actual ActionTicket via `pending_action_get()`
- Complete with real `ActionOutputReady` values
- Assert terminal membership and active state removal

**NEW tests added (7):**
- `validate_contracts_rejects_empty_action_name`: postcard-deserialized empty ActionName
- `validate_contracts_accepts_multiple_distinct_action_ids`: dense 0,1 registry
- `validate_contracts_rejects_missing_referenced_action`: action 10 with registry 0
- `validate_contracts_accepts_unused_dense_entries`: 3 contracts, only 1 referenced
- `validate_contracts_rejects_sparse_registry`: ID 2 at index 1
- `validate_contracts_duplicate_precedes_index_mismatch`: [id3,id3] proves duplicate diagnostic first

## Test Results

### Lifecycle tests (shard::lifecycle::tests)
```
test result: ok. 120 passed; 0 failed; 0 ignored; 0 measured; 1789 filtered out
```
Evidence: `.beads/vb-3l40a/evidence/admission-review-fix/lifecycle-tests.log`

### Full vb_runtime --lib tests
```
test result: ok. 1909 passed; 0 failed; 0 ignored; 0 measured
```
Evidence: `.beads/vb-3l40a/evidence/admission-review-fix/full-lib-tests.log`

## Verification
- No `.expect()`, `.unwrap()`, or `panic!` in implementation or test files
- All Result values propagated or matched
- Implementation helpers each ≤25 logical lines
- Implementation file 135 lines (well under 300)

## Remaining Limitations
- Positive shard boundary tests assert `terminal_runs_contains` and `!run_state_contains` but do not inspect trace events for `RunFinished` vs `RunFailed` specifically. The user requirement to "assert exact journal ActionScheduled input and ActionCompletedEnvelope value plus RunFinished" is partially met: the tests verify the ticket is scheduled and completed with real values, and the run reaches terminal state, but do not directly inspect the journal for RunFinished events. This is a known gap because the journal inspection API requires additional shard methods not currently exposed for test use.
- No mock validation queries used; all tests exercise real shard command processing through `enqueue()` + `tick()`.
