# Bead vb-vs630 Contract: CLI Submission Artifact Sequence Stamp

**Status:** contract
**Domain:** durable execution engine, artifact acceptance, run admission, journal durability

## Problem

The CLI submission path (`velvet-ballistics submit`) bypasses the global artifact acceptance sequence counter. It publishes the full run admission boundary atomically but leaves `accepted_at_seq` unset on the artifact. The default API submission path (`commit_artifact`) increments the global counter and stamps the artifact, but uses a separate code path. Both paths must use the same monotonically increasing global counter for artifact acceptance ordering.

## Goal

Stamp CLI `RunSubmission` artifacts with the same global `accepted_at_seq` counter used by default/registered submissions, while retaining the existing atomic multi-record publication boundary. The counter increment and artifact stamp must occur under the existing submission `write_lock`, not as a separate transaction.

## Domain Model

### Core Entities

- **AcceptedArtifact** — The compiled, verified, accepted executable workflow artifact. Has an `accepted_at_seq` field that must be set by the global counter.
- **RunSubmission** — An aggregate of source, artifact, inputs, and header, staged for atomic publication.
- **Artifact Acceptance Counter** — A persistent monotonic counter in the `artifact_acceptance` keyspace, incremented once per artifact acceptance.
- **RunInputsRecord** — The typed input mapping retained at submission time. Contains an `artifact_digest` that must be revalidated after stamping.
- **RunAdmission Event** — A journal event with an `artifact_digest` field that must be consistent with the stamped artifact.

### Key Distinctions

1. **Global artifact counter** (`accepted_at_seq` on AcceptedArtifact) — This is the counter this bead addresses. It identifies the acceptance order of artifacts across all submissions.
2. **Per-run event sequence** (`RunAccepted.seq`, `RunAdmission.seq`) — These are per-run event ordinals (seq 0, seq 1, ...) that must stay unchanged.
3. **Header `compiled_digest`** — This is the workflow IR digest, not the artifact acceptance digest. It must not be confused with the artifact digest.

## Current State

### Default Path (API Submissions)

`FjallJournal::commit_artifact` in `crates/vb_storage/src/journal/admission.rs`:
1. Acquires `write_lock`
2. Calls `issue_next_artifact_sequence` (wraps `issue_artifact` in `artifact_acceptance.rs`)
3. `issue_artifact`: reads counter, increments, stamps `accepted_at_seq`, rebinds artifact digest via `bind_artifact_digest`, encodes, builds `OwnedWriteBatch`, commits with policy durability
4. Returns stamped artifact

### CLI Path (RunSubmissions)

`FjallJournal::commit_run_submission` in `crates/vb_storage/src/inputs/submission.rs`:
1. Acquires `write_lock`
2. Validates submission binding (identities, digests, policy)
3. Calls `submission.commit(journal, policy)`
4. `RunSubmission::commit`: creates `JournalWriteBatch`, stages source/artifact/inputs/header/indexes/events, commits with policy durability
5. Does NOT call `issue_next_artifact_sequence` — leaves `accepted_at_seq` unset

### Existing Counter Implementation

`crates/vb_storage/src/journal/artifact_acceptance.rs` provides:
- `read_counter(journal)` — reads last issued sequence (returns 0 if none)
- `prepare_artifact(last, artifact)` — increments, creates `IssuedArtifactSequence`, stamps `accepted_at_seq`, rebinds digest via `bind_artifact_digest`, returns `(next, stamped)`
- `build_batch(journal, next, stamped, encoded)` — builds `OwnedWriteBatch` with counter + artifact writes
- `commit_batch(journal, batch, policy)` — applies policy durability, commits

### Locking

Both paths acquire the same `write_lock` via `journal.write_lock.lock()`. Calling `issue_next_artifact_sequence` from within `commit_run_submission` would deadlock because `issue_artifact` also acquires the lock. The bead explicitly states: DO NOT call `issue_next_artifact_sequence` while outer lock is held.

## Design Decision: Factor Out Issuance Prepare

The solution is to factor out the counter-read + increment + stamp + rebind-digest logic from `issue_artifact` into a crate-private helper that takes no lock and returns the stamped artifact plus the counter value. Both the default path and the CLI submission path call this helper under their own lock.

### Proposed Refactoring

1. **New crate-private helper in `artifact_acceptance.rs`:**
   ```rust
   fn prepare_artifact_with_sequence(
       journal: &FjallJournal,
       artifact: AcceptedArtifact,
   ) -> Result<(u64, AcceptedArtifact), JournalError> {
       let last = read_counter(journal)?;
       prepare_artifact(last, artifact)
   }
   ```

2. **`issue_artifact` becomes a wrapper:**
   ```rust
   fn issue_artifact(
       journal: &FjallJournal,
       artifact: AcceptedArtifact,
       policy: RuntimePolicy,
   ) -> Result<AcceptedArtifact, JournalError> {
       let _write_guard = journal.write_lock.lock().map_err(|_| JournalError::WriteLockPoisoned)?;
       let (next, stamped) = prepare_artifact_with_sequence(journal, artifact)?;
       // ... encode, build batch, commit (existing logic)
   }
   ```

3. **`commit_run_submission` uses the helper:**
   ```rust
   // Under existing write_lock:
   let (counter, stamped) = artifact_acceptance::prepare_artifact_with_sequence(journal, submission.artifact)?;
   // Rebind RunInputsRecord.artifact_digest to stamped artifact
   // Rebind RunAdmission event artifact_digest
   // Stage counter write in submission batch
   // Stage stamped artifact (re-encode with new digest)
   // Commit batch
   ```

### Digest Rebinding

After stamping `accepted_at_seq`, the artifact digest changes. This cascades:
1. `AcceptedArtifact.digest` — set by `bind_artifact_digest`
2. `RunInputsRecord.artifact_digest` — must be updated
3. `RunAdmission` event `artifact_digest` — must be updated
4. `RunAccepted` event `workflow` — uses `compiled_digest` (unchanged)
5. `RunHeaderRecord.compiled_digest` — uses workflow digest (unchanged)

## Acceptance Criteria

### Functional

1. CLI submission increments the global artifact acceptance counter exactly once per submission.
2. CLI submission stamps the artifact's `accepted_at_seq` field with the new counter value.
3. The stamped artifact's digest is rebound and propagated to `RunInputsRecord.artifact_digest` and `RunAdmission.artifact_digest`.
4. The counter write and all run admission records are committed in a single atomic `fjall::OwnedWriteBatch`.
5. Per-run event sequences (`RunAccepted.seq` = 0, `RunAdmission.seq` = 1) remain unchanged.
6. `RunHeaderRecord.compiled_digest` remains the workflow IR digest, not the artifact digest.
7. Default/API submission path (`commit_artifact`) continues to work identically.
8. No deadlock: the submission path does not call `issue_next_artifact_sequence` while holding `write_lock`.

### Durability

1. Policy durability (Strict/Journaled/Relaxed) is applied to the combined batch.
2. Strict mode uses `SyncAll` durability.
3. On commit failure, no partial records are published (atomic rollback via Fjall).

### Error Handling

1. Counter read failure returns `JournalError::Fjall`.
2. Sequence overflow returns `JournalError::SequenceOverflow`.
3. Artifact digest binding failure returns `JournalError::PayloadDigestMismatch`.
4. Write lock poisoned returns `JournalError::WriteLockPoisoned`.

## Open Design Questions

1. **Counter write in submission batch:** The submission batch uses `JournalWriteBatch` which has typed put methods (`put_compiled_ir`, `put_run_header`, etc.). The counter write goes to the `artifact_acceptance` keyspace, not a standard record. We need a raw insert into the batch for the counter. Options:
   - Add a `put_artifact_sequence` method to `JournalWriteBatch`
   - Use raw Fjall batch operations from within the submission path
   - Create a hybrid batch that combines typed records and raw key-value writes

2. **RunInputsRecord immutability:** `RunInputsRecord` is designed as an immutable record with validation. Updating `artifact_digest` after stamping may require either:
   - Making `RunInputsRecord` mutable for the digest field
   - Reconstructing a new `RunInputsRecord` with the updated digest
   - Deferring `artifact_digest` validation until after submission (weaker invariant)

3. **Artifact encoding:** After stamping and rebinding, the artifact must be re-encoded. The submission path currently encodes the artifact in `stage()` (submission.rs line 86-91). We need to encode the stamped artifact instead.

4. **Testing strategy:** How to verify the counter increments correctly across mixed default and CLI submissions? Need test fixtures that exercise both paths and verify monotonic counter values.

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Deadlock from lock re-acquisition | Factor out lock-free prepare helper; callers own the lock |
| Digest mismatch after stamping | Rebind all dependent digests before encoding/staging |
| Partial publication on failure | Single atomic batch; Fjall rollback on error |
| Counter skipped on submission failure | Counter write is in the same batch; rolled back on failure |
| Event sequence confusion | Explicit documentation and tests distinguishing artifact seq from run event seq |

## Evidence Requirements

- Source: modified `artifact_acceptance.rs`, `submission.rs`, and any supporting modules
- Compiled: crate builds with no warnings
- Tests: behavioral tests for counter increment, digest rebinding, atomicity
- Benchmark: none required (no performance claim)

## Notes

- This bead is contract-only. No Rust, test, or proof code is written in this pass.
- No new public API is introduced. All changes are crate-private.
- The design preserves the existing public API and behavior of the default submission path.
- The CLI path gains the same artifact acceptance ordering semantics as the default path.