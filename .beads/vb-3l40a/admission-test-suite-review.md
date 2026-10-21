# vb-3l40a Admission Test Suite Review

**CurrentSHA:** 1b0fe1f87c3a897f6873d02b524e4bfe84aa350c
**Filter:** `shard::lifecycle::tests` (114 tests)
**Result:** 114 passed, 0 failed, 0 ignored, 1789 filtered out
**Evidence:** `.beads/vb-3l40a/evidence/admission-review/lifecycle.log`

## Test Inventory

The admission validation tests span two files:
- `lifecycle_tests/contract_validation_tests.rs` (11 tests)
- `lifecycle_tests/chunk_001.rs` (2 tests in `vb_3l40a_contract_validation_tests` module)

Total: 13 dedicated admission validation tests.

### Positive Tests
1. `validate_contracts_accepts_valid_do_contract`
2. `validate_contracts_accepts_with_capability`
3. `validate_contracts_accepts_reused_action_ids`
4. `shard_boundary_accepts_valid_contract_and_executes_do`
5. `shard_boundary_accepts_valid_contract_with_seeded_input`
6. `validate_accepts_sufficient_capability` (chunk_001)

### Negative Tests
7. `validate_contracts_rejects_fabricated_action_id`
8. `validate_contracts_rejects_duplicate_action_ids`
9. `validate_contracts_rejects_missing_capability`
10. `validate_contracts_rejection_is_non_mutating`
11. `shard_boundary_validates_fabricated_id_before_admission`
12. `shard_boundary_validates_fabricated_id_with_inputs_before_admission`
13. `validate_rejects_insufficient_capability` (chunk_001)

## Findings

### BLOCKER: Missing Journal Immutability Checks in Negative Shard Tests
**Tests:** `shard_boundary_validates_fabricated_id_before_admission`,
`shard_boundary_validates_fabricated_id_with_inputs_before_admission`

These tests check `!shard.run_state_contains(run)` to verify no run was
admitted, but do not attach a recording journal or assert no journal events
were written. A regression that writes journal events before validation
would pass these tests. The admission contract addendum requires "no
journal events...created on failure."

**Severity:** BLOCKER
**Evidence:** Test source lines 419, 469 — only `run_state_contains` check,
no journal assertion.

### MINOR: Substring Error Matching
**Tests:** `validate_contracts_rejects_fabricated_action_id`,
`shard_boundary_validates_fabricated_id_before_admission`,
`shard_boundary_validates_fabricated_id_with_inputs_before_admission`

These tests use `error.contains("999")` or `error.contains("777")` for
error message matching. Substring checks are fragile — a future error
message refactor that removes the numeric ID from the string would pass
the test. Exact error variant/field matching is preferred.

**Severity:** MINOR
**Evidence:** Test source lines 163, 414, 464.

### MINOR: Terminal Membership Without Outcome Verification
**Tests:** `shard_boundary_accepts_valid_contract_and_executes_do`,
`shard_boundary_accepts_valid_contract_with_seeded_input`

These tests check `shard.terminal_runs_contains(run)` to verify the run
completed, but terminal membership alone does not distinguish RunFinished
from RunFailed. A regression that fails the run after validation would
pass these tests. Verifying the output value or inspecting trace events
for RunFinished would strengthen these tests.

**Severity:** MINOR
**Evidence:** Test source lines 497, 594 — only `terminal_runs_contains`
check, no outcome verification.

### MINOR: Missing Empty Action Name Test
The production code in `validate_contract_ids` checks for empty action
names and rejects them with `ContractValidationError { error: "action
name is empty" }`. No test covers this case.

**Severity:** MINOR
**Evidence:** Production code line 47, no corresponding test.

### MINOR: Missing Multiple Action ID Test
No test covers a workflow with multiple distinct action IDs requiring
multiple distinct contracts. The `validate_contracts_accepts_reused_action_ids`
test uses the same action ID twice, but a test with different action IDs
would verify the binding check across multiple actions.

**Severity:** MINOR
**Evidence:** No test with multiple distinct action IDs.

### INFORMATIONAL: Missing Out-of-Bounds Action Index Test
No test covers a workflow referencing an action ID beyond the contract
array length (e.g., action ID 10 with only 1 contract). The production
code handles this with `contracts.get(action_index)` returning None, but
no test exercises this path.

**Severity:** INFORMATIONAL

## Test Mutation Thought Experiments

1. **Delete validation in submit handler:** Would be caught by
   `shard_boundary_validates_fabricated_id_before_admission` (run state
   check). PASS.

2. **Validate after enqueue instead of before:** Would NOT be caught by
   current tests because `run_state_contains` would still be false if
   validation fails before the run is created. However, `tick()` returns
   an error, so the test still passes. This is a false positive — the
   validation timing is not actually verified. FAIL (BLOCKER above).

3. **Change error message for fabricated ID:** Would be caught by
   substring check in `validate_contracts_rejects_fabricated_action_id`.
   PASS (but fragile, see MINOR above).

4. **Allow empty action names:** Would NOT be caught by any test. FAIL
   (MINOR above).

5. **Accept duplicate action IDs:** Would be caught by
   `validate_contracts_rejects_duplicate_action_ids`. PASS.

6. **Accept missing capability:** Would be caught by
   `validate_contracts_rejects_missing_capability`. PASS.

## Resource Governance

The test filter `shard::lifecycle::tests` ran 114 tests in 0.31 seconds
with no timeout issues. No expensive verifier commands were used. Bounded
execution with 300s timeout. PASS.

## Disposition

STATUS: REJECTED

The missing journal immutability checks in the negative shard boundary
tests are a BLOCKER finding. These tests do not verify the admission
contract addendum's requirement that "no journal events are created on
failure." A regression that writes journal events before validation would
pass these tests.

Additional MINOR findings (substring error matching, terminal membership
without outcome verification, missing empty name test, missing multiple
action ID test) should also be addressed.

Test execution evidence is real and the 114 tests pass deterministically.
The filter ran >0 actual tests as required.
