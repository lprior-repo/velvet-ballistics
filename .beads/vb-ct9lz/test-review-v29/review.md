# Test Suite Review: vb-ct9lz

**Candidate:** vb-ct9lz (executor catalog uses shared artifact issuance helper)
**Actual source:** 6d6ee939a48d805c56d631e904854ad94c93e68a
**Review workspace:** /home/lewis/src/isoloated/vb-ct9lz-test-review-local3090-v26
**Author:** local3090 (independent review of author 5090 implementation)
**Verdict:** APPROVED

## Scope

This review covers the 15 new issuance tests added in 4 files under `crates/vb_storage/src/executor/`:
- `issuance_test_support.rs` (fixture module, 7 test functions in issuance_tests.rs)
- `issuance_tests.rs` (7 tests)
- `issuance_failure_tests.rs` (4 tests)
- `issuance_boundary_tests.rs` (4 tests)

Production code reviewed: `crates/vb_storage/src/executor/packed.rs` (submit_artifact using issue_next_artifact_sequence).

Test execution evidence: `ROOT_REGISTERED_ISSUANCE_V23_STORAGE_TEST.json` shows `cargo nextest run -p vb_storage --lib --all-features --locked` exited 0 in 3.565s. All 1658 storage tests pass (including the 15 new), zero skipped.

## Contract Parity

All behavioral guarantees from contract.md are tested:

1. **Non-zero accepted_at_seq (first issuance is seq 1):** `strict_submit_sequence_one`, `journaled_submit_sequence_one`, `relaxed_submit_sequence_one` — each asserts `accepted_at_seq.get() == 1`.
2. **Counter persists across close/reopen:** `journal_reopen_preserves_sequence` — drops journal, reopens same path, verifies counter at 1, submits second artifact at seq 2, verifies first artifact still readable.
3. **All catalog submissions use RuntimePolicy::Strict for helper storage:** Implementation hardcodes `RuntimePolicy::Strict` in `submit_artifact` call to `issue_next_artifact_sequence` (packed.rs line 60). Tests submit with all 3 admission policies and verify counter/state behavior is identical, confirming the helper policy is applied regardless of caller policy.
4. **Pure prepare_artifact unchanged (seq=0, unpersisted):** `prepare_produces_no_sequence` — iterates all 3 policies, asserts `accepted_at_seq.get() == 0`, counter remains None, journal.list_artifacts() empty.
5. **Public APIs unchanged:** Tests use existing `ExecutorCatalog::prepare_artifact`, `submit_artifact`, `verify_artifact` signatures.

## Required Scenarios

All 10 required scenarios are present:

1. **Parity across 3 admission profiles retaining Strict helper storage:** `strict_submit_sequence_one`, `journaled_submit_sequence_one`, `relaxed_submit_sequence_one` — all assert seq 1 and counter 1.
2. **Pure prepare returns seq=0 and unpersisted artifact:** `prepare_produces_no_sequence`.
3. **First 1/shared mixed producer sequence 1/2/3 with catalog/default workflow:** `sequential_submissions_increment` (catalog only, 1/2/3) and `mixed_default_and_catalog_submissions` (default at 1, catalog at 2, default at 3).
4. **Full record/digest verification:** `fixture.check` method calls `verify_artifact`, checks evidence policy, calls `verify_artifact_readback` which verifies record digest, decoded artifact equality, artifact digest consistency, and verification.digest match.
5. **Same path reopen prior record before followup:** `journal_reopen_preserves_sequence` — reopens, checks first artifact at seq 1, then submits second at seq 2, then checks first again.
6. **Injected precommit persist all 3/batch no counter or compiled records before retry 1:** `strict_persist_failure_is_precommit` (all 3 policies via `check_persist_precommit`) and `batch_commit_failure_is_precommit`. Both verify counter None and compiled_ir empty after failure, then retry succeeds at seq 1.
7. **Postcommit missing readback Corrupt with counter 1 and actual first record retained before followup 2:** `compiled_ir_readback_failure_is_postcommit` — injects readback failure, expects `ExecutorError::Corrupt`, verifies counter at 1, first record readable, followup succeeds at seq 2.
8. **Empty packs rejection no change:** `empty_packs_yield_pack_mismatch` — expects `PackMismatch`, counter None, compiled_ir empty.
9. **Malformed 7/9 exact error fields raw unchanged/empty compiled:** `malformed_counter_seven_bytes` and `malformed_counter_nine_bytes` — both use `check_malformed_counter` which asserts exact `MalformedKeyspaceRow` with prefix 0, expected_len 8, correct actual_len, raw bytes unchanged, compiled_ir empty.
10. **MAX overflow/MAX-1 then MAX record retained after overflow:** `counter_at_max_yields_sequence_overflow` and `counter_at_max_minus_one_allows_one_more` — both verify exact `SequenceOverflow`, counter state, and record persistence.

## Assertion Strength

All error assertions use `matches!` with exact variant and field matching:

```rust
matches!(&result,
    Err(ExecutorError::Artifact(JournalError::MalformedKeyspaceRow {
        prefix, expected_len, actual_len
    })) if *prefix == 0 && *expected_len == 8 && *actual_len == expected_actual_len)
```

Sequence numbers asserted with exact `assert_eq!`. Counter state asserted with exact `assert_eq!(read_counter(&journal)?, Some(n))` or `None`. Record state asserted with `journal.compiled_ir.iter().next().is_none()` or full readback verification.

The `fixture.check` method provides comprehensive verification: catalog verify_artifact, evidence policy check, and full readback with digest verification.

## Determinism

- Each test opens a fresh journal in a tempfile directory.
- No sleeps, no ignored tests, no shared mutable state between tests.
- Test hooks are deterministic (`fail_next_persist_for_test`, `fail_next_batch_commit_for_test`, `fail_next_compiled_ir_readback_for_test`).
- Counter writes use `journal.artifact_acceptance.insert` directly (no async).
- No broad mocks of domain queries; only specific journal failure injection hooks.

## Mutation Resistance

Deleting any branch/error logic would be caught:

- **Removing the `RuntimePolicy::Strict` hardcoded policy → tests WOULD fail.** The test `strict_persist_failure_is_precommit` (issuance_failure_tests.rs lines 15-21) calls `check_persist_precommit` with all three admission policies. `check_persist_precommit` (lines 23-42) injects `fail_next_persist_for_test()` (line 25) and expects `ExecutorError::Artifact(JournalError::StrictDurabilityFailed)` (lines 27-35). In `commit_batch` (artifact_acceptance.rs lines 151-155), the persist failure hook is only consumed when `policy == RuntimePolicy::Strict`. If `submit_artifact` (packed.rs line 60) passed the original caller policy instead of hardcoded Strict, then for Journaled/Relaxed policies the hook would not be consumed and `submit` would return `Ok(artifact)` instead of the expected error, causing the test to fail. v26's claim that this mutation would pass unnoticed was incorrect.
- Removing the counter increment → `sequential_submissions_increment` and `mixed_default_and_catalog_submissions` would fail.
- Removing the postcommit readback → `compiled_ir_readback_failure_is_postcommit` would fail. The test injects `fail_next_compiled_ir_readback_for_test()` (line 88) which causes `journal.compiled_ir(stamped.digest)` to return `Ok(None)` (source.rs lines 77-78). Without the readback code (packed.rs lines 62-69), `submit_artifact` would return `Ok(stamped)` directly, not `Err(ExecutorError::Corrupt)` as the test expects (line 91).
- Removing the MalformedKeyspaceRow length check → `malformed_counter_seven_bytes`/`nine_bytes` would fail.
- Removing the SequenceOverflow check → `counter_at_max_yields_sequence_overflow` would fail.
- Removing the precommit vs postcommit distinction → `batch_commit_failure_is_precommit` or `compiled_ir_readback_failure_is_postcommit` would fail.
- Removing empty packs check → `empty_packs_yield_pack_mismatch` would fail.

One branch not directly tested: the `checked_envelope` schema version check (`packed.base.schema_version != verification::EVIDENCE_VERSION`). However, this is a pre-existing check in the packed verification path, not part of this bead's change. The fixture always produces the current schema version.

## Code Size

- `issuance_tests.rs`: 140 lines
- `issuance_failure_tests.rs`: 125 lines
- `issuance_boundary_tests.rs`: 104 lines
- `issuance_test_support.rs`: 255 lines

All under 300 lines. All test functions under 25 logical lines. Helper functions are reasonably sized and focused.

## Corrections from v26

1. **Test count:** issuance_tests.rs has 7 test functions, not 6. Total remains 15 (7 + 4 + 4).
2. **Scenario count:** 10 required scenarios are covered, not 11 as v26 claimed.
3. **Mutation resistance claim corrected:** v26 stated that removing the hardcoded Strict policy would still pass and would require a missing durability test. This is incorrect — the existing test `strict_persist_failure_is_precommit` catches this mutation because it tests all three admission policies and the persist failure hook is only consumed when the storage policy is Strict (enforced in `commit_batch`). If the caller policy (Relaxed/Journaled) were passed instead of hardcoded Strict, the hook would not be consumed and the submission would return Ok instead of the expected error.
4. **Postcommit readback removal:** v26 claimed this would "get Artifact error instead of Corrupt." The correct behavior is that without the readback code, `submit_artifact` would return `Ok(stamped)` directly (since the issuance helper succeeded), not an Artifact error. The test catches this because it expects `Corrupt`.

## Findings

None. All contract guarantees tested. All required scenarios present. Assertion strength is high. Tests are deterministic and mutation-resistant.

## STATUS: APPROVED