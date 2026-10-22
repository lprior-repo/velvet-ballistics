# Test Authorship: vb-qz2yn (Local3090 v10)

## Files Written

1. `crates/vb_storage/src/admission/acceptance_sequence_failure_tests.rs`
2. `crates/vb_storage/src/admission/acceptance_sequence_concurrency_tests.rs`

These are standalone test modules intended to be wired into the `admission::tests` module by the root local integration. They use `super::{minimal_workflow, temp_journal}` to access the parent module's private test helpers.

## Failure Tests (5)

### 1. strict_persist_failure_is_precommit_counter_unchanged
- Injects `fail_next_persist_for_test()` on a fresh journal
- Submits with `RuntimePolicy::Strict`
- Expects `JournalError::StrictDurabilityFailed`
- Verifies counter key is `None` (no artifact issued)
- Verifies `compiled_ir` keyspace is empty
- Retries the same submission successfully at seq 1
- Proves: strict persist failure is precommit; counter is not advanced

### 2. batch_commit_failure_is_precommit_no_state_change
- Injects `fail_next_batch_commit_for_test()` on a fresh journal
- Submits with `RuntimePolicy::Strict`
- Expects `JournalError::Fjall(_)`
- Verifies counter key is `None`
- Verifies `compiled_ir` keyspace is empty
- Retries successfully at seq 1
- Proves: batch commit failure is precommit; no state mutation

### 3. compiled_ir_readback_failure_is_postcommit_artifact_exists
- Injects `fail_next_compiled_ir_readback_for_test()` on a fresh journal
- Submits with `RuntimePolicy::Strict`
- Expects `JournalError::ArtifactMalformed` (detected after commit)
- Verifies counter is `Some(1)` — counter advanced despite readback failure
- Reads back the compiled IR record by workflow digest to prove it exists
- Follows up with a valid submission at seq 2
- Proves: readback failure is postcommit; no rollback occurs; artifact is durable

### 4. invalid_workflow_canonical_digest_rejected_before_mutation
- Builds a workflow with a forged (invalid) canonical digest
- Either `CompiledWorkflow::try_from_parts` rejects it, or `submit_artifact` rejects it with `JournalError::ArtifactChecksumMismatch`
- Verifies counter key is `None` in both cases
- Submits a valid workflow successfully at seq 1
- Proves: invalid digest is caught before any counter or artifact mutation

### 5. counter_overflow_at_u64_max
- Sets counter directly to `u64::MAX - 1` in the `artifact_acceptance` keyspace
- Submits successfully at seq `u64::MAX`
- Verifies counter is now `u64::MAX`
- Submits again; expects `JournalError::SequenceOverflow`
- Verifies the first artifact is still readable from the `compiled_ir` keyspace
- Proves: overflow is caught with a typed error; no wraparound; prior artifacts remain readable

## Concurrency Test (1)

### six.eight_concurrent_submissions_get_unique_sequences_1_through_8
- Shares a single `FjallJournal` across 8 threads via `Arc`
- Each thread submits the same workflow with `RuntimePolicy::Default`
- All 8 thread joins are checked for errors
- Collects all results; all must succeed
- Verifies sequences are unique and form the set {1, 2, 3, 4, 5, 6, 7, 8}
- Verifies all 8 artifact digests are unique (different `accepted_at_seq` changes the digest)
- Verifies final counter is 8
- Verifies `compiled_ir` keyspace contains exactly 8 records
- Proves: the `write_lock` Mutex correctly serializes concurrent submissions; counter is atomic and monotonic

## Integration Status

- **Not wired**: These test files are not yet included in `admission/tests.rs` via `#[path = ...]` module declarations. The root local integration task will add:
  ```rust
  #[path = "acceptance_sequence_failure_tests.rs"]
  mod acceptance_sequence_failure_tests;

  #[path = "acceptance_sequence_concurrency_tests.rs"]
  mod acceptance_sequence_concurrency_tests;
  ```

- **Not compiled**: The tests have not been compiled or run in this isolated workspace. The root integration task must run `moon ci` or equivalent to compile and execute them.

- **Production unchanged**: No production code, existing tests, or other workspace files were modified.

## API Used

All tests use the actual public API of the `vb_storage` crate:
- `crate::admission::submit_artifact(journal, workflow, policy)`
- `crate::FjallJournal::open(path, config)`
- `FjallJournal` test hooks: `fail_next_persist_for_test()`, `fail_next_batch_commit_for_test()`, `fail_next_compiled_ir_readback_for_test()`
- `FjallJournal` keyspace fields: `artifact_acceptance`, `compiled_ir`
- `vb_core::RuntimePolicy::{Strict, Relaxed, Default}`
- `crate::error::JournalError` variants

The tests use parent-module private helpers via `super::{minimal_workflow, temp_journal}` as specified.