# Bead vb-vs630 Contract: CLI Submission Artifact Sequence Stamp

**Status:** contract
**Domain:** durable execution engine, artifact acceptance, run admission, journal durability

## Problem

CLI submission (`velvet-ballistics submit`) bypasses the global artifact acceptance sequence counter. `submit_artifact`/`submit_artifact_with_contracts` in `admission.rs` call `issue_next_artifact_sequence` which increments the counter and stamps `accepted_at_seq`. `commit_run_submission` in `inputs/submission.rs` does not — it leaves `accepted_at_seq` at its prepare-time sentinel value. Both paths must use the same monotonic counter for artifact acceptance ordering.

## Goal

Stamp CLI `RunSubmission` artifacts with the same global `accepted_at_seq` counter used by default submissions, retaining the existing atomic multi-record publication boundary. Counter increment and artifact stamp occur under the existing submission `write_lock`, not as a separate transaction.

## Current State

### Default Path
`submit_artifact_with_contracts` in `crates/vb_storage/src/admission.rs`:
1. `prepare_compiled_artifact` — validates, builds artifact with seq=0 sentinel
2. `issue_next_artifact_sequence` (in `journal/artifact_acceptance.rs`) — acquires write_lock, reads counter, increments, stamps seq, rebinds `artifact.digest` and `artifact.verification.digest` via `bind_artifact_digest`, encodes, stages counter+artifact in `fjall::OwnedWriteBatch`, commits with policy durability, returns stamped artifact
3. `verify_artifact_persisted` — readback verification

### CLI Path
`FjallJournal::commit_run_submission` in `crates/vb_storage/src/inputs/submission.rs`:
1. `submission.validate_policy(policy)` — BEFORE lock
2. Acquires `journal.write_lock`
3. Checks duplicate run (header/inputs/events exist) — BEFORE commit
4. `submission.commit(journal, policy)` — `RunSubmission::commit` takes `&self`, creates `JournalWriteBatch`, stages source/artifact/inputs/header/indexes/events, commits
5. Does NOT call `issue_next_artifact_sequence` — `accepted_at_seq` remains at prepare-time sentinel

### Key Distinctions
- **Global artifact counter** (`accepted_at_seq` on AcceptedArtifact) — this bead's target
- **Per-run event sequence** (RunAccepted.seq=0, RunAdmission.seq=1) — unchanged
- **Header `compiled_digest`** — workflow IR digest, not artifact acceptance digest, unchanged

## Design: Factor Out Issuance Prepare

Factor counter-read + increment + stamp + rebind from `issue_artifact` into a crate-private lock-free helper. Both paths call this helper under their own lock.

### Proposed Changes

1. **New `pub(crate)` helper in `artifact_acceptance.rs`:**
   `fn prepare_artifact_with_sequence(journal, artifact) -> Result<(u64, AcceptedArtifact), JournalError>` — reads counter, calls `prepare_artifact`, returns (next_counter, stamped_artifact). No lock.

2. **`issue_artifact` wraps the helper:** acquires lock, calls helper, proceeds with encode/batch/commit (existing logic unchanged).

3. **`commit_run_submission` uses the helper:** under existing lock, calls helper, rebinds `RunInputsRecord.artifact_digest` and `RunAdmission.artifact_digest`, stages counter write in submission batch via a new crate-private `JournalWriteBatch` method, re-encodes stamped artifact, commits.

### Digest Rebinding Cascade
After stamping, artifact digest changes. Four fields must be consistent:
1. `AcceptedArtifact.digest` — set by `bind_artifact_digest`
2. `AcceptedArtifact.verification.digest` — set by `bind_artifact_digest`
3. `RunInputsRecord.artifact_digest` — must be updated
4. `RunAdmission` event `artifact_digest` — must be updated

### Batching Constraint
`JournalWriteBatch.inner` is `pub(super)` — accessible only within the batch module. `inputs/submission.rs` cannot access `inner` directly. Proposed: add crate-private counter staging method to `JournalWriteBatch` using existing byte-limit/aborted patterns.

### Borrowing Constraint
`commit_run_submission` takes `&RunSubmission` and `RunSubmission::commit` takes `&self`. To stamp the artifact, an owned checked clone/rebind scratch is needed under the lock (or bounded helper), since `submission.artifact` cannot be moved out of a borrowed reference. The clone rebinds `artifact.digest` in the private copy while retaining all input/workflow/source/policy fields.

### No Lock Re-acquisition
`issue_next_artifact_sequence` acquires its own lock. Calling it from within `commit_run_submission` (which already holds the lock) would deadlock. The factored helper must be lock-free.

## Acceptance Criteria

### Functional
1. CLI submission increments global artifact acceptance counter exactly once per submission.
2. CLI submission stamps `accepted_at_seq` with the new counter value.
3. Stamped artifact digest is rebound and propagated to `RunInputsRecord.artifact_digest` and `RunAdmission.artifact_digest`.
4. Counter write and all run admission records committed in single atomic batch.
5. Per-run event sequences (RunAccepted.seq=0, RunAdmission.seq=1) unchanged.
6. `RunHeaderRecord.compiled_digest` remains workflow IR digest.
7. Default/API path (`submit_artifact`) continues identically.
8. No deadlock: submission path does not call locked `issue_next_artifact_sequence`.

### Durability
1. Policy durability (Strict/Journaled/Relaxed) applied to combined batch.
2. Strict mode uses SyncAll durability.
3. Atomic publication: no partial multi-record state. On commit Err, Fjall batch may already have committed (IO uncertainty) — Err does not guarantee no-commit.

### Error Handling (Exact Actual Errors)
1. Malformed counter row: `JournalError::MalformedKeyspaceRow { prefix: 0, expected_len: 8, actual_len: <actual> }`
2. Sequence overflow: `JournalError::SequenceOverflow`
3. Write lock poisoned: `JournalError::WriteLockPoisoned`
4. Duplicate run: `JournalError::RunAlreadyExists`
5. Fjall IO errors: `JournalError::Fjall(...)`
6. Postcard encoding: `JournalError::PostcardEncodeFailed`
7. Digest binding failures: actual `bind_artifact_digest` errors propagate; `JournalError::PayloadDigestMismatch` only for binding identity failures, not all failures

### Precommit vs Postcommit Errors
- Precommit (no mutation, safe to retry): validation, overflow, malformed, injected precommit test hooks
- Postcommit (uncertain IO ack boundary): Fjall IO during/after commit — counter may have advanced even if Err returned

### Policy Acceptance
All three policies accepted: Relaxed, Journaled, Strict. Relaxed writes NO acceptance events. Journaled/Strict write events at seq 0/1.

### Mixed Submission Ordering
Counter increments monotonically across mixed default and CLI submissions. Duplicate-run rejected before counter publication.

## Open Design Questions

1. Counter write method name/signature on `JournalWriteBatch` (e.g., `put_artifact_sequence(u64)`); must use existing limits/aborted pattern.
2. Checked clone approach: clone `RunSubmission` under lock and mutate clone, or bounded helper taking ownership of rebind scratch.
3. Artifact re-encoding: must re-encode stamped artifact after `bind_artifact_digest`.

## Risks

| Risk | Mitigation |
|------|------------|
| Deadlock from lock re-acquisition | Factor out lock-free prepare helper |
| Digest mismatch after stamping | Rebind all four dependent fields before encoding/staging |
| Partial publication on failure | Single atomic batch; Fjall rollback on error (with IO uncertainty caveat) |
| Counter skipped on submission failure | Counter write in same batch |

## Evidence Requirements
- Source: modified `artifact_acceptance.rs`, `submission.rs`, `batch/types.rs` (or equivalent)
- Compiled: crate builds with no warnings
- Tests: behavioral tests for counter increment, digest rebinding, atomicity, mixed submissions

Contract-only; no Rust/test/proof code in this pass. No new public API. Default submission path unchanged.