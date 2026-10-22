# vb-qz2yn: Artifact Acceptance Sequence Counter

## Scope

Implement a durable, monotonically-increasing artifact acceptance sequence counter backed by a dedicated `artifact_acceptance` Fjall keyspace. This counter provides per-artifact issuance ordering for downstream ledger, replay, and audit use cases.

**In scope:** submission APIs (`submit_artifact`, `submit_artifact_with_contracts`), the new keyspace lifecycle, counter read-modify-write semantics under the existing journal write lock.

**Out of scope:** `prepare_compiled_artifact` remains an unpersisted, seq-0 API. CLI RunSubmission and ExecutorCatalog paths are separate follow-up beads. Per-run EventSeq allocation, record schemas, and evidence/profile structures remain unchanged.

## Requirements

1. **Dedicated keyspace:** Create `artifact_acceptance` keyspace in `crates/vb_storage/src/constants.rs`. Profile: Hot. This is NOT a reserved hash in `compiled_ir`.

2. **Single counter key:** One fixed internal key `last_issued` (constant name `ARTIFACT_LAST_ISSUED_KEY`) stores the last issued sequence as an 8-byte big-endian `u64`. Absent value means `0` (initial). First issuance returns `1`.

3. **Overflow and malformed handling:**
   - Counter read with wrong byte length → `JournalError::MalformedKeyspaceRow` before any mutation.
   - Checked increment overflows `u64::MAX` → `JournalError::SequenceOverflow` before any mutation.

4. **Encapsulation:**
   - Issued sequence is a private typed value; no public setter or `caller_at_seq` API.
   - `AcceptedArtifact.accepted_at_seq` is stamped by the implementation.
   - Public `submit_artifact` and `submit_artifact_with_contracts` signatures are unchanged.

5. **Submission flow:**
   1. Prepare and validate artifact via `prepare_compiled_artifact` (unchanged).
   2. Journal private helper acquires `write_lock`.
   3. Read current counter from `artifact_acceptance` keyspace.
   4. Checked increment; stamp `accepted_at_seq` on artifact.
   5. Rebind both `artifact.digest` and `verification.digest` using existing binding logic (digest changes because seq is now embedded).
   6. Serialize validated `CompiledIrRecord`.
   7. Single `FjallOwnedWriteBatch` stages BOTH the counter update and the artifact record.
   8. Commit batch with existing policy durability (Strict → SyncAll, Relaxed → no explicit sync).
   9. Existing readback check still runs.

6. **Atomicity:**
   - Counter and artifact persist in the same batch commit.
   - Pre-commit validation/overflow/encoding errors stage no durable writes.
   - Commit or readback IO failure may leave unknown already-committed state; error is propagated. No claim that all `Err` rolls back.

7. **Locking:**
   - Counter RMW protected by existing `write_lock` (same as other journal writes).
   - Helper must not recurse into any path that reacquires `write_lock`.

8. **Preserved invariants:**
   - Compiled IR size and record digest checks unchanged.
   - Cold profile and action contract bindings unchanged.
   - Existing test fault hooks (`fail_next_persist`, `fail_next_compiled_ir_readback`, `fail_next_batch_commit`) still surface proper errors.

## Files to Change

- `crates/vb_storage/src/constants.rs` — new keyspace name and counter key
- `crates/vb_storage/src/journal/mod.rs` — new keyspace declared/probed
- `crates/vb_storage/src/journal/core.rs` — new keyspace field, declared_keyspaces array, probe_health
- `crates/vb_storage/src/journal/readonly.rs` — declared_keyspaces array
- `crates/vb_storage/src/journal/artifact_acceptance.rs` — new: counter logic
- `crates/vb_storage/src/admission.rs` — integration into submission flow
- `crates/vb_storage/src/admission/verification.rs` — only if essential binding visibility needed
