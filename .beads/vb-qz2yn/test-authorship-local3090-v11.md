# vb-qz2yn Test Authorship Notes — local3090 v11

## Work Performed

Rewrote both acceptance sequence test files from scratch, fixing all rejected v10 issues:

### Failure Tests (`acceptance_sequence_failure_tests.rs`)

1. **strict_persist_failure_is_precommit_counter_unchanged**: Uses `fail_next_persist_for_test()` hook. Verifies `StrictDurabilityFailed` error. Retries submission and asserts seq 1 (proving counter unchanged).

2. **batch_commit_failure_is_precommit_no_state_change**: Uses `fail_next_batch_commit_for_test()` hook. Verifies `JournalError::Fjall(_)` (NOT the invented `JournalError::Fjall`). Retries and asserts seq 1.

3. **compiled_ir_readback_failure_is_postcommit_artifact_exists**: Uses `fail_next_compiled_ir_readback_for_test()` hook. Verifies `ArtifactMalformed` error. Proves counter advanced by submitting again and getting seq 2. Reads back the committed artifact using the public `compiled_ir(digest)` API and the actual `accepted_artifact_digest(&expected)` computation (NOT raw postcard+blake3 hash of workflow).

4. **invalid_workflow_canonical_digest_rejected_before_mutation**: Takes valid workflow parts, forges digest to `[0xFF; 32]`, reconstructs via `try_from_parts`. The constructor accepts it (it doesn't validate digest), so `submit_artifact` rejects it with `ArtifactChecksumMismatch`. Verifies counter unchanged by submitting valid workflow and getting seq 1.

5. **counter_overflow_at_u64_max**: Sets counter to `u64::MAX - 1` via `insert` (NOT the invented `.put()`). First submission succeeds at `u64::MAX`. Verifies counter is now `u64::MAX` via direct readback. Second submission fails with `SequenceOverflow`. Verifies first artifact still readable.

### Concurrency Tests (`acceptance_sequence_concurrency_tests.rs`)

Uses `Arc<Mutex<TestJournal>>` to share journal across 8 threads (FjallJournal not Send+Sync; internal `write_lock` provides serialization). Each thread submits and returns the full `AcceptedArtifact`. After joining all threads:
- Verifies exactly 8 artifacts with sequences 1..8 (sorted comparison)
- Verifies 8 unique digests
- For EACH artifact: reads back via `compiled_ir(artifact.digest)`, verifies record digest matches
- For EACH artifact: calls `verify_accepted_artifact_for_policy(artifact, Relaxed)`
- For EACH artifact: computes `accepted_artifact_digest(artifact)` and verifies it matches both `artifact.digest` and `artifact.verification.digest`
- Submits one more artifact to verify seq 9

### Fixed v10 Issues

- No longer invents `WorkflowParts` fields (uses `minimal_workflow()` helper)
- No longer uses raw postcard+blake3 hash as accepted artifact digest (uses `accepted_artifact_digest()`)
- No longer silently returns Ok when constructor rejects (uses `.map_err()` to fail test)
- Uses actual `JournalError::Fjall(_)` (not invented `JournalError::Fjall`)
- Uses `.insert()` not `.put()` on keyspace
- No empty loop pretending to verify records (actual full verification per artifact)
- Uses `iter().next().is_some()` pattern correctly for emptiness checks
- No `RuntimePolicy::Default` (uses `Relaxed`)
- All error propagation uses `.map_err(|e| e.to_string())?` for `Result<(), String>`
- Tests append exactly two `#[path=...]` mod hooks to `tests.rs`

## Verification Status

Tests written and committed to isolated workspace. NOT yet compiled or run — coordinator will execute verification. All APIs used are verified against actual production source in this workspace.

## Files Modified

- `crates/vb_storage/src/admission/acceptance_sequence_failure_tests.rs` (complete rewrite)
- `crates/vb_storage/src/admission/acceptance_sequence_concurrency_tests.rs` (complete rewrite)
- `crates/vb_storage/src/admission/tests.rs` (appended two mod hooks)
- `.beads/vb-qz2yn/test-authorship-local3090-v11.md` (this file)