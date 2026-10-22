# Artifact Producer Map — vb-0qwfq (Local 5090 Research)

## Current Producer Landscape

Five artifact production paths exist in the codebase. Three are pure/read-only, one is stamped (child vb-qz2yn), two remain unstamped.

### 1. Pure Prepare (seq=0, not persisted)
- **Source:** `crates/vb_storage/src/admission/verification.rs:18` — `pub fn prepare_compiled_artifact(workflow, policy, contracts)`
- **Behavior:** Validates, encodes IR, binds zero-sentinel digest, returns artifact with `accepted_at_seq = EventSeq::new(0)`. No journal writes. No counter interaction.
- **Consumers:** All submission paths call this first.
- **Status:** ✅ seq=0 is correct (not yet accepted). No change needed.

### 2. Default API Path (STAMPED — child vb-qz2yn)
- **Source:** `crates/vb_storage/src/admission.rs:341` — `pub fn submit_artifact(journal, workflow, policy)` → `submit_artifact_with_contracts`
- **Source:** `crates/vb_storage/src/admission.rs:351` — `pub fn submit_artifact_with_contracts(journal, workflow, policy, action_contracts)`
- **Flow:** `prepare_compiled_artifact(seq=0)` → `issue_next_artifact_sequence(journal, artifact, policy)` → `verify_artifact_persisted(journal, digest)`
- **Stamping:** Helper in `crates/vb_storage/src/journal/artifact_acceptance.rs` does RMW under `write_lock`: read counter, increment, stamp seq, rebind digest, encode, batch commit with policy durability.
- **Counter keyspace:** `artifact_acceptance/last_issued` (KEYSPACE_ARTIFACT_ACCEPTANCE = "artifact_acceptance").
- **Status:** ✅ Child completed.

### 3. CLI RunSubmission Path (UNSTAMPED — candidate)
- **Source:** `crates/velvet_ballistics/src/submit.rs:90` — calls `prepare_compiled_artifact(seq=0)` then builds `RunSubmission`
- **Source:** `crates/vb_storage/src/inputs/submission.rs:16` — `RunSubmission::new(source, artifact, inputs, header)`
- **Source:** `crates/vb_storage/src/inputs/submission.rs:140` — `FjallJournal::commit_run_submission(submission, policy)`
- **Flow:** `prepare_compiled_artifact(seq=0)` → `RunSubmission::new()` → `FjallJournal::commit_run_submission()` → `submission.commit(journal, policy)` → `submission.stage(batch)` (stages source, artifact, inputs, header, indexes in ONE batch)
- **Current seq behavior:** `RunSubmission::accepted_event()` at line 167 hardcodes `seq: EventSeq::new(0)`. RunHeaderRecord has `accepted_at_ms` (Unix timestamp), no sequence.
- **Atomicity constraint:** Stages 6 record types + events in one batch. Counter update would need to be included in this batch or done atomically.
- **Status:** ⏳ Unstamped. Complex due to multi-record batch.

### 4. ExecutorCatalog Packed Path (UNSTAMPED — candidate)
- **Source:** `crates/vb_storage/src/executor/packed.rs:12` — `ExecutorCatalog::submit_artifact(journal, workflow, policy, contracts, packs)`
- **Source:** `crates/vb_storage/src/executor/packed.rs:20` — `ExecutorCatalog::prepare_artifact(workflow, policy, contracts, packs)` (uses `prepare_with_profile(..., ExecutorProfile::Registered)`)
- **Flow:** `ExecutorCatalog::prepare_artifact(seq=0)` → `journal.put_compiled_ir(record)` → `journal.persist_strict()` → readback verification
- **Current seq behavior:** Artifact has `accepted_at_seq = EventSeq::new(0)`. No counter interaction. No seq stamping.
- **Atomicity constraint:** Single-artifact submission, already single-transaction via `persist_strict`. Counter update can be integrated cleanly.
- **Status:** ⏳ Unstamped. Clean minimal child scope.

### 5. Raw OwnedWriteBatch (not a producer, explicitly excluded)
- Direct `journal.batch()` usage with `put_compiled_ir`. No counter per contract.
- Status: Raw storage, not a public producer API.

## Counter Keyspace and Digest Rebinding

**Counter location:** `artifact_acceptance/last_issued` in FjallJournal (KEYSPACE_ARTIFACT_ACCEPTANCE).

**Current counter helper:** `crates/vb_storage/src/journal/artifact_acceptance.rs:191` — `pub(crate) fn issue_next_artifact_sequence(journal, artifact, policy)`
- Reads counter → increments → stamps seq → calls `bind_artifact_digest` (recomputes digest to include seq) → encodes → stages in owned batch with counter write → commits with policy durability.
- Returns stamped `AcceptedArtifact` with NEW digest (different from prepared artifact's seq=0 digest).
- **Key invariant:** Stamped artifact has different digest than prepared artifact. Callers must use the returned stamped artifact's digest for subsequent operations.

**Impact on ExecutorCatalog path:**
- Current: `artifact = prepare_artifact(seq=0, digest=D0)` → `put_compiled_ir(D0)` → verify `D0`
- With stamping: `prepared = prepare_artifact(seq=0, digest=D0)` → `issue_next_artifact_sequence(journal, prepared, policy)` → returns `stamped` with `seq=N, digest=D_N` → verify `D_N`
- The existing post-submission verification uses `artifact.digest` after the call, so it will naturally use the stamped digest. No breaking change.

## Minimal Child Scope Assessment

**Recommended next producer: ExecutorCatalog packed path (option 4)**

**Why ExecutorCatalog over RunSubmission:**
1. Single-artifact submission — counter RMW fits naturally in same transaction
2. Already calls `prepare_artifact` then persists — matches default path's prepare-then-stamp pattern
3. `issue_next_artifact_sequence` takes `(journal, artifact, policy)` and returns stamped artifact — direct drop-in replacement for `put_compiled_ir` + `persist_strict`
4. RunSubmission path stages 6+ record types; integrating counter requires either modifying helper to support custom batches or doing counter in separate transaction (atomicity risk)

**Proposed contract for child bead:**

```
Route ExecutorCatalog::submit_artifact through issue_next_artifact_sequence.

Changed files:
- crates/vb_storage/src/executor/packed.rs (submit_artifact implementation)

Behavior:
1. ExecutorCatalog::prepare_artifact still returns seq=0 artifact (unchanged)
2. ExecutorCatalog::submit_artifact calls issue_next_artifact_sequence(journal, prepared, policy)
3. Returns stamped artifact with incremented seq and rebound digest
4. Existing post-submission readback verification unchanged (uses stamped digest)

Preservation:
- prepare_compiled_artifact remains seq=0
- RunSubmission path unchanged (future child)
- Raw OwnedWriteBatch unchanged
- No new public API
- No raw counter setter
- Counter keyspace already created (shared with vb-qz2yn)
```

**Risks:**
1. Digest mismatch: If caller caches prepared artifact's digest and uses it after submission, verification fails. Mitigation: Always use returned stamped artifact's digest.
2. Counter collision: Both vb-qz2yn path and ExecutorCatalog path increment same counter. This is correct behavior (global sequence).
3. Policy compatibility: `issue_next_artifact_sequence` applies policy durability. ExecutorCatalog currently always calls `persist_strict()`. The helper applies SyncAll only for Strict policy. For Relaxed/Journaled, ExecutorCatalog would submit without strict persistence. This is a behavior change that matches the default path's behavior.

**Verification approach:**
- Behavioral test: Submit via ExecutorCatalog, verify seq increments globally across both paths
- Readback: Verify stamped digest roundtrips correctly
- Mixed-path: Submit via default path (seq=1), then ExecutorCatalog (seq=2), then default (seq=3)

## Sources

- Default path implementation: `crates/vb_storage/src/admission.rs:341-375`
- Counter helper: `crates/vb_storage/src/journal/artifact_acceptance.rs:191`
- ExecutorCatalog packed path: `crates/vb_storage/src/executor/packed.rs:47-69`
- RunSubmission path: `crates/velvet_ballistics/src/submit.rs:90-93`, `crates/vb_storage/src/inputs/submission.rs:140-148`
- Counter keyspace constant: `crates/vb_storage/src/constants.rs:32`
- Child contract: `.beads/vb-qz2yn/contract.md`
