# Test Suite Review: vb-2bok — Durability Gate

**Bead:** vb-2bok
**Current State:** 10 (test-suite-review — re-run)
**Next Gate:** 11 (Landing)
**Review Mode:** Suite Inquisition (Tier 0–3)

---

## VERDICT: REJECTED

### Tier 0 — Static
[PASS] Banned pattern scan — no `assert!(result.is_ok())` or `assert!(result.is_err())` in vb_2bok
[PASS] Silent error discard — no `let _ =` or `.ok()` in vb_2bok test bodies
[PASS] Ignored tests — none found
[PASS] Sleep in tests — none found
[PASS] Test naming violations — no `fn test_` helper patterns in vb_2bok
[PASS] Shared mutable state — none found
[PASS] Mock interrogation — none found
[PASS] Integration test purity — uses public API only
[PASS] Error variant completeness — all 14 contract error codes have exact-variant assertions
[FAIL] **Loops in test bodies (Rule 2 — LETHAL): 9 loops found**
[FAIL] **Unused imports causing clippy failure: 4 import statements**

### Tier 1 — Execution
[PASS] nextest: 2623 passed (consistent across 1-thread and 8-thread runs)
[PASS] Ordering probe: consistent
[FAIL] **Clippy: 4 unused import warnings in vb_2bok_durability_gate_tests.rs — LETHAL per `-D warnings`**
[N/A] Insta: not present

### Tier 2 — Coverage
[PASS] Line coverage: 94.86% overall (target ≥90%)
[PASS] Branch coverage: 0 branches recorded (not a meaningful failure)
**Note:** Coverage not blocking since Tier 1 failed

### Tier 3 — Mutation
**NOT RUN** — Tier 1 failed, mutation testing skipped per fail-fast pipeline

---

## LETHAL FINDINGS

### 1. Loops in Test Bodies (Rule 2 — LETHAL)
**File:** `crates/vb_storage/src/vb_2bok_durability_gate_tests.rs`

Per Holzmann Rule 2: "No loops in test bodies. Period. A test with a loop is a test with hidden logic. It is no longer a straight-line proof."

Nine loop statements found inside `#[test]` functions:

| Line | Loop | Test Function |
|------|------|---------------|
| 197 | `for policy in [Relaxed, Journaled, Strict]` | `submit_artifact_all_policies_set_correct_digest` |
| 221 | `for policy in [Relaxed, Journaled, Strict]` | `submit_artifact_all_policies_return_nonempty_ir` |
| 629 | `for byte in corrupt.iter_mut().skip(HEADER_BYTES)` | `decode_rejects_valid_header_with_corrupt_payload` |
| 1025 | `for policy in [Relaxed, Journaled, Strict]` | `artifact_digest_equals_workflow_digest` |
| 1058 | `for event in &events` | `events_for_run_returns_ascending_sequences` |
| 1068 | `for (i, event) in replayed.iter().enumerate()` | `events_for_run_returns_ascending_sequences` |
| 1547 | `for i in 0u64..5` | `bdd_event_replay_returns_ascending_sequences` |
| 1562 | `for (i, event) in replayed.iter().enumerate()` | `bdd_event_replay_returns_ascending_sequences` |
| 1692 | `for byte in corrupt.iter_mut().skip(HEADER_BYTES)` | `bdd_corrupted_payload_detected_by_blake3` |

**Fix required:** Replace each loop with `rstest` parameterized fixtures or proptest invariants. For example:
- Replace `for policy in [Relaxed, Journaled, Strict]` with `#[rstest] fn submit_artifact_policy #[case(RuntimePolicy::Relaxed)] #[case(RuntimePolicy::Journaled)] ...`
- Replace iteration loops with proptest invariants that assert the property holds for arbitrary valid inputs

### 2. Unused Imports (Tier 1 — LETHAL)
**File:** `crates/vb_storage/src/vb_2bok_durability_gate_tests.rs`

These cause `cargo clippy --tests --all-features -- -D warnings` to fail with exit code 101:

- **Line 17:** `AcceptedArtifact` imported but never used (line 871 uses full path `crate::admission::AcceptedArtifact`)
- **Line 20:** `MAGIC_COMPILED_ARTIFACT`, `MAGIC_WORKFLOW_SOURCE`, `MAX_BLOB_BYTES`, `MAX_COMPILED_IR_BYTES`, `MAX_WORKFLOW_SOURCE_BYTES` imported but never used
- **Line 26:** `CompiledIrRecord` imported but never used
- **Line 30:** `WorkflowId` imported but never used

**Fix required:** Either remove the unused imports or use them (for `AcceptedArtifact`, change line 871 to use the imported name instead of the full path).

---

## PREVIOUS REVIEW STATUS

The previous State 10 review identified the same 9 loops and 4 unused imports. The user claimed repairs were applied ("dead imports removed", "for loops replaced", "23 new tests added"). **Investigation confirms the repairs were NOT applied to vb_2bok_durability_gate_tests.rs.** The loops and unused imports remain at the exact same lines.

---

## MANDATE

Before resubmission, the following MUST be fixed:

### Must Fix (LETHAL blockers):

1. **Remove 4 unused imports** from `crates/vb_storage/src/vb_2bok_durability_gate_tests.rs`:
   - Line 17: Remove `AcceptedArtifact` or change line 871 to use the import
   - Line 20: Remove `MAGIC_COMPILED_ARTIFACT`, `MAGIC_WORKFLOW_SOURCE`, `MAX_BLOB_BYTES`, `MAX_COMPILED_IR_BYTES`, `MAX_WORKFLOW_SOURCE_BYTES`
   - Line 26: Remove `CompiledIrRecord`
   - Line 30: Remove `WorkflowId`

2. **Eliminate 9 loops in test bodies** in `crates/vb_storage/src/vb_2bok_durability_gate_tests.rs`:
   - Lines 197, 221, 1025: Replace `for policy in [...]` with `#[rstest]` parameterized tests
   - Lines 629, 1692: Replace `for byte in corrupt.iter_mut()...` with proptest invariants over byte indices
   - Lines 1058, 1068, 1547, 1562: Replace iteration with proptest invariants or `rstest` with enumerated cases

3. **Verify clippy passes** after fixes:
   ```bash
   cargo clippy -p vb_storage --tests --all-features -- -D warnings
   ```
   Must exit 0.

### After Fixes:
- Re-run ALL tiers from Tier 0
- Full re-review, not just the failing tier
- Every fix must be verified by running the full pipeline

---

## SUMMARY

The vb_2bok_durability_gate_tests.rs file has NOT been repaired since the previous State 10 review. The same 9 Holzmann Rule 2 violations (loops in test bodies) and 4 clippy unused import warnings remain. The test suite passes execution (2623 tests) and coverage (94.86%) but cannot be approved due to LETHAL static analysis failures.
