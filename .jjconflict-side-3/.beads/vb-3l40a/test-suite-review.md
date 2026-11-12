**Bead**: vb-3l40a
**State**: 6
**Reviewer**: test-reviewer (local5090)
**Source checkout**: /home/lewis/src/isoloated/velvet-ballistics-vb-3l40a-test-review-5090
**Review snapshot**: c89940bd vs baseline da633fe0
**Review scope**: Recovery and contract validation tests only

**STATUS: REJECTED**

---

## PHASE 1: Tests Compiled and Executed

**Command**: `RUSTC_WRAPPER= CARGO_BUILD_JOBS=2 timeout 300 cargo test -p vb_runtime recovery_context_tests --lib -- --test-threads=1`
**Result**: 4 passed, 0 failed, 1873 filtered out
**Tests executed**:
- `shard::lifecycle::recovery_context_tests::recover_handles_open_ask_boundary` ... ok
- `shard::lifecycle::recovery_context_tests::recover_handles_pending_action_boundary` ... ok
- `shard::lifecycle::recovery_context_tests::recover_rejects_malformed_command` ... ok
- `shard::lifecycle::recovery_context_tests::recover_seeds_nonzero_journal_sequence` ... ok

**Command**: `RUSTC_WRAPPER= CARGO_BUILD_JOBS=2 timeout 300 cargo test -p vb_runtime contract_validation_tests --lib -- --test-threads=1`
**Result**: 4 passed, 0 failed, 1873 filtered out
**Tests executed**:
- `shard::lifecycle::contract_validation_tests::validate_contracts_accepts_well_formed_contracts` ... ok
- `shard::lifecycle::contract_validation_tests::validate_contracts_rejects_duplicate_action_ids` ... ok
- `shard::lifecycle::tests::vb_3l40a_contract_validation_tests::validate_accepts_sufficient_capability` ... ok
- `shard::lifecycle::tests::vb_3l40a_contract_validation_tests::validate_rejects_insufficient_capability` ... ok

**Full suite**: `cargo test -p vb_runtime --lib` → 1877 passed, 0 failed (confirms previous claim)

---

## PHASE 2: Assertion Strength Analysis

### recovery_context_tests (4 tests)

| Test | Assertion Type | Strength | Issue |
|------|----------------|----------|-------|
| `recover_rejects_malformed_command` | `is_err()` | WEAK | Only checks error occurred; doesn't verify the specific error variant or message. Would pass for ANY recovery error. |
| `recover_seeds_nonzero_journal_sequence` | None (no assert) | NONE | Calls `seed_journal_sequence` but doesn't verify the sequence was actually stored or is nonzero. Test passes if method returns Ok, regardless of value. |
| `recover_handles_pending_action_boundary` | `is_err()` + `matches!` | MEDIUM | Good: asserts RunNotFound error variant. But only tests fail-closed path, not successful reinjection. |
| `recover_handles_open_ask_boundary` | `assert_eq` | TRIVIAL | Just checks boundary kind constant. No runtime behavior tested. |

### contract_validation_tests (2 tests in chunk_001.rs)

| Test | Assertion Type | Strength | Issue |
|------|----------------|----------|-------|
| `validate_contracts_rejects_duplicate_action_ids` | `is_err()` + variant match + string check | GOOD | Asserts specific error variant and checks error message contains "duplicate". |
| `validate_contracts_accepts_well_formed_contracts` | `is_ok()` | WEAK | Only checks success; doesn't verify any state was created or validated. |

### vb_3l40a_contract_validation_tests (2 tests in lifecycle_tests)

| Test | Assertion Type | Strength | Issue |
|------|----------------|----------|-------|
| `validate_rejects_insufficient_capability` | `is_err()` + variant match + field check | GOOD | Asserts AdmissionCapabilityDenied with specific required capability name "storage". |
| `validate_accepts_sufficient_capability` | `is_ok()` | WEAK | Only checks success; doesn't verify state or capabilities. |

---

## PHASE 3: Mutation Thought Experiments

### CRITICAL: Empty action contracts after recovery

**Mutation**: Change `action_contracts: artifact.action_contracts.clone()` back to `action_contracts: Box::new([])` in `handle_recover` (line 481).

**Would any test fail?** NO. All 8 new tests would still pass:
- `recover_rejects_malformed_command`: Tests wrong workflow_digest, not action contracts → PASS
- `recover_seeds_nonzero_journal_sequence`: Tests seed call returns Ok → PASS
- `recover_handles_pending_action_boundary`: Tests reinject error path → PASS
- `recover_handles_open_ask_boundary`: Trivial constant check → PASS
- Contract validation tests: Test admission path, not recovery → PASS

**Conclusion**: The critical bug that the code review identified (recovery discarding action contracts) is NOT caught by any test. The fix is in production code but unverified by the test suite.

### Other mutations

| Mutation | Caught? | By which test? |
|----------|---------|----------------|
| Remove capability validation loop | YES | `validate_rejects_insufficient_capability` |
| Remove duplicate action ID check | YES | `validate_contracts_rejects_duplicate_action_ids` |
| Remove RunNotFound error for reinject | YES | `recover_handles_pending_action_boundary` |
| Remove journal sequence seeding | NO | No test verifies journal state |
| Return wrong workflow digest comparison | YES | `recover_rejects_malformed_command` |
| Change collect_states to empty after recovery | NO | No test inspects recovered collect_states |

---

## PHASE 4: Missing Coverage for Recovery Semantics

The following recovery behaviors are NOT tested:

1. **Action contract restoration**: No test verifies that `artifact.action_contracts` are actually restored to the run state after recovery. The fix is in production code but unverified.

2. **Recovery + action execution**: No test recovers a run and then actually drives it to execute actions. This is the exact path where the original bug would manifest in production.

3. **Workflow digest verification**: `recover_rejects_malformed_command` tests wrong workflow_digest, but the test provides a different artifact_digest than workflow_digest — it's unclear if this tests the intended path. The test comment says "malformed" but doesn't specify what aspect is malformed.

4. **Collect states restoration**: No test verifies that `command.collect_states` are actually restored to the run state. The test only calls seed_journal_sequence separately.

5. **Boundary handling for live runs**: `recover_handles_pending_action_boundary` tests the error path for non-live runs, but doesn't test successful reinjection for a live run.

6. **Open ask boundary behavior**: `recover_handles_open_ask_boundary` is trivial — it just checks a constant. No test verifies what happens when recovering a run with an open ask boundary (should resume at ask step, no side effects).

---

## PHASE 5: Unwrap/Expect Analysis

**Test setup unwraps** (acceptable — tests fail loudly if setup is wrong):
- `RunFrame::new(...).unwrap()` (line 833)
- `seed_journal_sequence(...).unwrap()` (line 867)
- `ActionName::new(name).expect("test name")` (line 895)

**Error extraction unwraps** (acceptable):
- `result.unwrap_err()` for error variant matching

No production-style unwrap in test logic.

---

## Findings (Ordered by Severity)

| Finding | Severity | File:Line | Status |
|---------|----------|-----------|--------|
| No test catches empty action contracts after recovery | CRITICAL | test suite | open |
| `recover_seeds_nonzero_journal_sequence` has no assertion | HIGH | chunk_001.rs:866 | open |
| No test exercises recovery + action execution path | CRITICAL | test suite | open |
| `recover_rejects_malformed_command` assertion is weak (is_err only) | MEDIUM | chunk_001.rs:845 | open |
| No test verifies collect_states restoration | HIGH | test suite | open |
| Open ask boundary test is trivial constant check | MEDIUM | chunk_001.rs:931 | open |

### [FINDING-001]: No test catches empty action contracts after recovery

**Severity**: CRITICAL
**Location**: Entire test suite
**Problem**: The fix for the original bug (recovery discarding action contracts) is in production code (`action_contracts: artifact.action_contracts.clone()`), but no test verifies this. If the fix regresses to `Box::new([])`, all 8 new tests would still pass.

**Evidence**: Mutation thought experiment shows all tests pass with empty action contracts.

**Required Fix**: Add a test that:
1. Creates a workflow with actions and action contracts
2. Starts a run
3. Crashes/suspends the run
4. Recovers the run using `handle_recover`
5. Verifies the recovered run state has non-empty action contracts matching the artifact

### [FINDING-002]: No test exercises recovery + action execution path

**Severity**: CRITICAL
**Location**: Entire test suite
**Problem**: The original bug caused recovered runs to fail when executing actions. No test recovers a run and then drives it to actually execute an action to completion.

**Required Fix**: Add an integration test that:
1. Creates a workflow with at least one action
2. Starts a run and drives it to a suspension point
3. Recovers the run
4. Drives the recovered run to execute the pending action
5. Verifies action completion and correct final state

### [FINDING-003]: `recover_seeds_nonzero_journal_sequence` has no assertion

**Severity**: HIGH
**Location**: `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:866-870`
**Problem**: The test calls `seed_journal_sequence` but doesn't assert anything about the result. It would pass even if the method did nothing.

**Required Fix**: Add an assertion that verifies the journal sequence was actually stored. This might require exposing a getter or using an internal check.

### [FINDING-004]: No test verifies collect_states restoration

**Severity**: HIGH
**Location**: Entire test suite
**Problem**: The recovery command includes `collect_states`, but no test verifies that these are actually restored to the run state after recovery.

**Required Fix**: Add a test that:
1. Creates a recovery command with non-empty collect_states
2. Recovers the run
3. Inspects the run state to verify collect_states were restored

### [FINDING-005]: `recover_rejects_malformed_command` assertion is weak

**Severity**: MEDIUM
**Location**: `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:845`
**Problem**: Only asserts `is_err()`, not the specific error variant. Would pass for any recovery error, including the wrong error path.

**Required Fix**: Match on the specific RuntimeError variant (likely `Recovery` with digest mismatch message).

### [FINDING-006]: Open ask boundary test is trivial

**Severity**: MEDIUM
**Location**: `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:928-932`
**Problem**: Just asserts a constant (`RecoveredRunBoundary::none().kind() == RecoveredRunBoundaryKind::None`). No runtime behavior tested.

**Required Fix**: Replace with a test that actually recovers a run with an open ask boundary and verifies the run resumes at the ask step without side effects.

---

## Quality Gates

| Gate | Result | Evidence |
|------|--------|----------|
| 8 new tests compile and pass | ✅ | Actual execution confirmed |
| Full suite 1877 tests pass | ✅ | Actual execution confirmed |
| All new tests have meaningful assertions | ❌ | 3 of 8 have weak or no assertions |
| Recovery + action execution path tested | ❌ | Not tested |
| Mutation resistance verified | ❌ | Empty action contracts would pass all tests |

---

## Verdict

**STATUS: REJECTED**

### Summary
The 8 new tests compile, run, and pass, which is a positive step. However, the test suite has critical gaps that leave the core recovery bug unverified:

1. **No test would catch the original bug regressing.** The exact mutation that caused the bug (empty action contracts after recovery) passes all 8 new tests.
2. **No test exercises the full recovery + action execution path.** The code review identified this as the critical missing test.
3. **Several tests have weak or no assertions.** Tests that just check `is_ok()` or have no assertion at all provide minimal coverage.

The contract validation tests are well-written and provide good coverage for the admission path. But the recovery tests are insufficient to verify the critical fix.

### Required Before Landing
1. Add a test that recovers a run and verifies action contracts are non-empty and match the artifact
2. Add a test that recovers a run and drives it to execute pending actions
3. Strengthen `recover_seeds_nonzero_journal_sequence` with a real assertion
4. Add a test for collect_states restoration
5. Strengthen `recover_rejects_malformed_command` with specific error variant matching

Without these tests, the implementation fix is unverified and the regression risk is high.
