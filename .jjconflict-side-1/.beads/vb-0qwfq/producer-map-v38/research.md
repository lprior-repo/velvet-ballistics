# Artifact Producer Map — vb-0qwfq (Local 5090 Research)

**Status:** v38 draft corrected from actual source packets. Root research only; no implementation.

## Current Producer Landscape

Five artifact production paths exist. Three are pure/read-only (no persistence), one is stamped (child vb-qz2yn), two remain unstamped. Categorization by storage behavior, not arithmetic.

### Pure Prepare Paths (seq=0, not persisted)

**1a. Default Pure Prepare**
- **Source:** `crates/vb_storage/src/admission/verification.rs:18` — `pub fn prepare_compiled_artifact(workflow, policy, contracts)`
- **Behavior:** Validates, encodes IR, binds zero-sentinel digest, returns artifact with `accepted_at_seq = EventSeq::new(0)`. No journal writes. No counter interaction. Uses `ExecutorProfile::Legacy`.
- **Status:** ✅ Correct (seq=0, not yet accepted).

**1b. ExecutorCatalog Pure Prepare**
- **Source:** `crates/vb_storage/src/executor/packed.rs:14` — `pub fn prepare_artifact(&self, workflow, policy, contracts, packs)`
- **Behavior:** Checks registered executor authority and conformance, then calls `verification::prepare_with_profile(..., ExecutorProfile::Registered)`. Returns seq=0 artifact.
- **Status:** ✅ Correct.

### Stamped Submission Path (child vb-qz2yn)

**2. Default API Path (STAMPED)**
- **Source:** `crates/vb_storage/src/admission.rs:341` — `pub fn submit_artifact(journal, workflow, policy)` → `submit_artifact_with_contracts`
- **Source:** `crates/vb_storage/src/admission.rs:351` — `pub fn submit_artifact_with_contracts(journal, workflow, policy, action_contracts)`
- **Flow:** `prepare_compiled_artifact(seq=0)` → `issue_next_artifact_sequence(journal, artifact, policy)` → `verify_artifact_persisted(journal, stamped_digest)`
- **Stamping:** `crates/vb_storage/src/journal/artifact_acceptance.rs:191` — `pub(crate) fn issue_next_artifact_sequence(journal, artifact, policy)`. RMW under `write_lock`: read counter → increment → stamp seq → `bind_artifact_digest` (recomputes digest to include seq) → encode → batch commit with policy durability.
- **Counter keyspace:** `artifact_acceptance/last_issued` (KEYSPACE_ARTIFACT_ACCEPTANCE).
- **Durability:** Strict policy → SyncAll; Relaxed/Journaled → no fsync.
- **Status:** ✅ Child completed.

### Unstamped Submission Paths (candidates)

**3. CLI RunSubmission Path (UNSTAMPED)**
- **Source:** `crates/velvet_ballistics/src/submit.rs:90` — calls `prepare_compiled_artifact(seq=0)` then builds `RunSubmission`
- **Source:** `crates/vb_storage/src/inputs/submission.rs:16` — `RunSubmission::new(source, artifact, inputs, header)`
- **Source:** `crates/vb_storage/src/inputs/submission.rs:138` — `FjallJournal::commit_run_submission(submission, policy)`
- **Flow:** `prepare_compiled_artifact(seq=0)` → `RunSubmission::new()` → `commit_run_submission()` → `submission.commit(journal, policy)` → `submission.stage(batch)` (6 record types in ONE batch)
- **Current seq behavior:** `RunSubmission::accepted_event()` (line 117) hardcodes `seq: EventSeq::new(0)`. This is the per-run journal event sequence (first event in run's event log), NOT the artifact sequence. Different domain — must stay 0. `RunSubmission::admission_event()` (line 126) uses `seq: EventSeq::new(1)` and carries `artifact_digest: self.artifact.digest` (the seq=0 artifact digest).
- **Artifact:** Private `self.artifact` with seq=0 from pure prepare. No counter interaction. The stored artifact is never stamped.
- **Atomicity constraint:** Stages 6 record types (source, compiled_ir, run_inputs, run_header, workflow_index, status_index) + events under shared `write_lock`. Counter update would need to join this batch.
- **Status:** ⏳ Unstamped. Complex due to multi-record batch.

**4. ExecutorCatalog Packed Path (UNSTAMPED — recommended next child)**
- **Source:** `crates/vb_storage/src/executor/packed.rs:48` — `pub fn submit_artifact(&self, journal, workflow, policy, contracts, packs)`
- **Flow:** `self.prepare_artifact(seq=0)` → `journal.put_compiled_ir(record)` → `journal.persist_strict()` → readback verification → return original seq=0 artifact
- **Current seq behavior:** Artifact has `accepted_at_seq = EventSeq::new(0)`. No counter interaction. No seq stamping.
- **Durability:** ALWAYS calls `persist_strict()` regardless of `policy` parameter. Policy only affects admission verification, not storage durability.
- **Transaction structure:** `put_compiled_ir` and `persist_strict` are separate calls — not one atomic transaction. `persist_strict` provides durability for the preceding write.
- **Return value:** Original `artifact` (seq=0), not a stamped version.
- **Status:** ⏳ Unstamped. Clean minimal child scope.

**5. Raw OwnedWriteBatch (explicitly not a producer)**
- Direct `journal.batch()` usage with `put_compiled_ir`. No counter per contract.
- Status: Raw storage API, not a public producer.

## Counter Keyspace and Digest Rebinding

**Counter location:** `artifact_acceptance/last_issued` in FjallJournal.

**Counter helper:** `crates/vb_storage/src/journal/artifact_acceptance.rs:191` — `pub(crate) fn issue_next_artifact_sequence(journal, artifact, policy)`
- Reads counter → increments → stamps seq → `bind_artifact_digest` (recomputes digest to include seq) → encodes → stages in owned batch with counter write → commits with policy durability.
- Returns stamped `AcceptedArtifact` with NEW digest (different from prepared artifact's seq=0 digest).
- **Key invariant:** Stamped artifact has different digest than prepared artifact. Callers must use returned stamped artifact's digest for subsequent operations.
- **Durability control:** The `policy` argument controls ONLY batch durability (SyncAll for Strict, none otherwise). It does NOT affect the prepare/stamp logic.

**Impact on ExecutorCatalog path:**
- Current: `artifact = prepare_artifact(seq=0, digest=D0)` → `put_compiled_ir(D0)` → verify `D0`
- With stamping: `prepared = prepare_artifact(seq=0, digest=D0)` → `issue_next_artifact_sequence(journal, prepared, policy)` → returns `stamped` with `seq=N, digest=D_N` → verify `D_N`
- The existing post-submission verification uses `artifact.digest` after the call, so it will naturally use the stamped digest. No breaking change.

## Verification Against Actual Code

**`verify_expected_bindings` (verification.rs:123):**
- Compares: source_digest, policy_digest, ir, verification (after normalizing digest field), required_capabilities
- Does NOT compare `accepted_at_seq` directly
- Digest comparison covers seq implicitly (via `bind_artifact_digest`)
- Therefore: stamped packed verification works against actual code

**`checked_envelope` (packed.rs:117):**
- Validates: schema version, policy match, packs length, `artifact.digest == artifact.verification.digest`, `accepted_artifact_digest(artifact) == artifact.digest`
- Does NOT compare seq directly
- Therefore: check stamped packed verification against actual code works

## Policy vs Durability Separation

Two distinct concepts:

1. **Admission policy** (RuntimePolicy): Validates the artifact for runtime execution. Embedded in artifact's `policy_digest` and evidence. Determined by caller.

2. **Storage durability**: Whether writes are fsynced. Controlled by the `policy` argument to submission functions.

Current default path (`submit_artifact_with_contracts`): Uses caller's policy for BOTH admission and durability. Relaxed/Journaled submissions are not fsynced.

Current ExecutorCatalog path: Uses caller's policy for admission only, but ALWAYS fsyncs via `persist_strict()`. Storage durability is unconditional Strict regardless of policy.

**Design decision for child bead:** To preserve the packed path's unconditional strict durable ACK for ALL policy profiles, the child must call the counter helper with `RuntimePolicy::Strict` regardless of the caller's original policy. The artifact's admission policy remains the caller's original policy (set during prepare). This maintains the separation: storage guarantee from admission policy.

If the helper's types/validation prevent this (e.g., policy must match artifact policy), then a minimal private durability seam is needed — a private helper that accepts the counter stamping logic but uses unconditional Strict durability. No public counter setter. Do NOT weaken Relaxed/Journaled packed ack.

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
2. ExecutorCatalog::submit_artifact calls issue_next_artifact_sequence(journal, prepared, RuntimePolicy::Strict)
   - Note: RuntimePolicy::Strict for durability, prepared artifact retains original admission policy
3. Returns stamped artifact with incremented seq and rebound digest
4. Existing post-submission readback verification unchanged (uses stamped digest)

Preservation:
- prepare_compiled_artifact remains seq=0
- RunSubmission path unchanged (future child)
- Raw OwnedWriteBatch unchanged
- No new public API
- No raw counter setter
- Counter keyspace already created (shared with vb-qz2yn)
- Unconditional packed strict durable ACK preserved for ALL policy profiles
```

**Risks:**
1. Digest mismatch: If caller caches prepared artifact's digest and uses it after submission, verification fails. Mitigation: Always use returned stamped artifact's digest.
2. Counter collision: Both vb-qz2yn path and ExecutorCatalog path increment same counter. This is correct behavior (global sequence).
3. Policy type validation: If `issue_next_artifact_sequence` validates that policy matches artifact policy, direct reuse may fail. Requires checking actual type signatures. If blocked, propose minimal private durability seam.
4. Return value change: `submit_artifact` currently returns original seq=0 artifact. Must return stamped artifact instead. All callers must use returned value.

**Test requirements for child bead:**
- Corruption/overflow: Counter overflow at u64::MAX, malformed counter bytes
- Strict injected persist failure: Counter and artifact written atomically or neither
- Batch atomicity: Counter and artifact in same batch, no split commits
- Readback: Stamped digest roundtrips correctly through put_compiled_ir
- Reopen: Counter survives journal reopen
- Mixed default+catalog: Submit via default path (seq=1), ExecutorCatalog (seq=2), default (seq=3)
- Policy separation: Relaxed/Journaled admission policy artifacts get stamped with Strict durability
- Preserve packs/authority/proof: Packed evidence remains valid after stamping
- No false IO rollback: Failed persist doesn't roll back counter incorrectly

## Sources

- Default path implementation: `crates/vb_storage/src/admission.rs:341-375`
- Counter helper: `crates/vb_storage/src/journal/artifact_acceptance.rs:191`
- ExecutorCatalog packed path: `crates/vb_storage/src/executor/packed.rs:48-70`
- RunSubmission path: `crates/velvet_ballistics/src/submit.rs:90-93`, `crates/vb_storage/src/inputs/submission.rs:138-148`
- Pure prepare: `crates/vb_storage/src/admission/verification.rs:18`
- Counter keyspace constant: `crates/vb_storage/src/constants.rs:32`
- Child contract: `.beads/vb-qz2yn/contract.md`