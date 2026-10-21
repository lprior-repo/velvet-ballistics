# vb-3l40a Independent Admission Test Suite Review

**Candidate SHA:** 713902f756ea825ff6500c838a087dbb98f25cf4
**Workspace:** /home/lewis/src/isoloated/velvet-ballistics-vb-3l40a-test-review-5090
**Filter:** `shard::lifecycle::tests` (lib tests only)
**Result:** 120 passed, 0 failed, 0 ignored, 1789 filtered out
**Full lib:** 1909 passed, 0 failed
**Evidence:** `.beads/vb-3l40a/evidence/independent-admission-review/`
**Date:** 2026-10-21

## Test Inventory

The admission validation tests span two files:
- `lifecycle_tests/contract_validation_tests.rs` (15 tests)
- `lifecycle_tests/chunk_001.rs` (2 tests in `vb_3l40a_contract_validation_tests` module)

Total: 17 dedicated admission validation tests.

### Positive Tests (7)
1. `validate_contracts_accepts_valid_do_contract` — valid Do contract accepted
2. `validate_contracts_accepts_with_capability` — capability requirement satisfied
3. `validate_contracts_accepts_reused_action_ids` — same action ID in multiple Do nodes
4. `validate_contracts_accepts_multiple_distinct_action_ids` — workflow with two distinct actions
5. `validate_contracts_accepts_unused_dense_entries` — extra contracts for unused actions allowed
6. `shard_boundary_accepts_valid_contract_and_executes_do` — real Do with input 42, output 100
7. `shard_boundary_accepts_valid_contract_with_seeded_input` — real Do with input 55, output 200

### Negative Tests (10)
8. `validate_contracts_rejects_fabricated_action_id` — ID 999 at index 0
9. `validate_contracts_rejects_duplicate_action_ids` — two contracts with ID 0
10. `validate_contracts_rejects_missing_capability` — required cap not granted
11. `validate_contracts_rejection_is_non_mutating` — run state unchanged on rejection
12. `validate_contracts_rejects_empty_action_name` — empty string name
13. `validate_contracts_rejects_missing_referenced_action` — no contract for action 10
14. `validate_contracts_rejects_sparse_registry` — ID 2 at index 1
15. `validate_contracts_duplicate_precedes_index_mismatch` — duplicate detected before index error
16. `shard_boundary_validates_fabricated_id_before_admission` — ID 999, journal unchanged
17. `shard_boundary_validates_fabricated_id_with_inputs_before_admission` — ID 777, journal unchanged
18. `validate_rejects_insufficient_capability` (chunk_001) — different cap granted
19. `validate_accepts_sufficient_capability` (chunk_001) — correct cap granted

## Findings

### Previous Issues Now Fixed

**Journal immutability in negative shard tests:** Tests 16 and 17 now attach recording journals, take snapshots before tick, and assert snapshots are equal after validation failure. The BLOCKER finding from the previous test review is resolved.

**Terminal outcome verification:** Tests 6 and 7 now verify:
- Exactly one `RunFinished` journal event with correct run ID and result slot
- Zero `RunFailed` journal events
- `ActionCompletedEnvelope` with correct serialized output values (100 and 200)
The MINOR findings about terminal membership without outcome verification are resolved.

**Empty action name test:** Test 12 now exists, using postcard deserialization to construct an empty ActionName. The MINOR finding is resolved.

**Multiple distinct action IDs test:** Test 4 now exists with a workflow referencing two distinct action IDs (0 and 1) and two corresponding contracts. The MINOR finding is resolved.

**Precedence test:** Test 15 verifies duplicate ID detection takes precedence over dense index mismatch. This addresses the coordinator's concern about documented vs actual precedence.

### Remaining Findings

**INFORMATIONAL: Missing out-of-bounds action index test**
No test covers a workflow referencing an action ID beyond the contract array length where the contract at that index has a different ID. Test 13 covers the case where `contracts.get(action_index)` returns None, but a test with a mismatched ID at a valid index (e.g., workflow references action 0 but contract at index 0 has ID 1) would exercise the `contract.id != action` branch in `validate_workflow_action_binding`.

**INFORMATIONAL: Test-only compilation drift (vb-eyij7 migration)**
Integration tests in `crates/vb_runtime/tests/vb_qi37_12_2_resume_error_propagation.rs` still have missing `input_slots` fields in `WorkflowParts` constructors. This is part of the vb-eyij7 migration compile repair, not this bead's scope. The bead's own tests and all 1909 lib tests compile and pass.

**INFORMATIONAL: vb-8iilq cfg(test) fabrication not changed**
The existing `cfg(test)` production replacement in runtime_tests.rs that replaces some submit paths with simpler non-contract variants is tracked as vb-8iilq and is not changed by this bead. The production with-contract tests bypass this fixture API change and test the actual admission validation logic.

## Test Mutation Thought Experiments

1. **Delete validation in submit handler:** Caught by tests 16 and 17 (journal immutability check). PASS.
2. **Validate after enqueue instead of before:** Caught by tests 16 and 17 (journal would show RunStarted event). PASS.
3. **Allow fabricated action IDs:** Caught by tests 8, 16, 17. PASS.
4. **Allow duplicate action IDs:** Caught by tests 9, 15. PASS.
5. **Allow empty action names:** Caught by test 12. PASS.
6. **Allow sparse registry:** Caught by test 14. PASS.
7. **Allow missing referenced action:** Caught by test 13. PASS.
8. **Allow missing capability:** Caught by tests 10, 18. PASS.
9. **Change error message for fabricated ID:** Would be caught by exact string matching in tests 8, 16, 17. PASS.
10. **Regression writing journal before validation:** Caught by tests 16 and 17 (snapshot comparison). PASS.
11. **Run fails instead of finishes after validation:** Caught by tests 6 and 7 (RunFinished count, RunFailed count). PASS.
12. **Wrong output value serialized:** Caught by tests 6 and 7 (postcard comparison of values 100 and 200). PASS.

## Resource Governance

Lib test filter `shard::lifecycle::tests` ran 120 tests in 0.15s. Full lib ran 1909 tests in 0.46s. No timeout issues. No expensive verifier commands. Bounded execution with 300s timeout. PASS.

## Disposition

STATUS: APPROVED

All previous BLOCKER and MINOR findings from the prior test suite review have been repaired. Negative shard boundary tests now verify journal immutability. Positive tests now verify actual output values and RunFinished events. Empty action name, multiple distinct action IDs, and precedence tests now exist. The two remaining findings are INFORMATIONAL only and do not affect test coverage adequacy.

The test suite provides comprehensive coverage of the admission contract validation logic with proper negative test coverage of the global duplicate > empty name > dense index > capability > binding error precedence.
