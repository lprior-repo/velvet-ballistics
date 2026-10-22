# Artifact Acceptance Sequence Contract v3

Bug: `AcceptedArtifact.accepted_at_seq` is always 0 at admission
(GAP-007). The existing freshness floor check
`admit_artifact_run_with_certificate_floor`
(crust/vb_runtime/src/admission/parts/chunk_005_admit_core.rs:123)
rejects every real artifact when `required_at_least > 0`.

## Fix Design

`prepare_compiled_artifact` stays unchanged: it produces an
unpersisted checked envelope with `accepted_at_seq = 0`.

`submit_artifact` and `submit_artifact_with_contracts` change. After
calling `prepare_compiled_artifact`, they call a new private helper
`stamp_artifact_accepted(journal, artifact)` that:

1. Acquires the journal's `write_lock`.
2. Reads the journal-wide artifact counter from a reserved key in the
   `compiled_ir` keyspace (key = `[0x02][blake3("vb:artifact_counter")]`).
   Absent counter means LAST_ISSUED = 0; first stamp = 1.
3. Computes `next = last_issued.checked_add(1)`; on None returns
   `JournalError::SequenceOverflow`.
4. Sets `artifact.accepted_at_seq = next`.
5. Recomputes both digest bindings via the existing
   `bind_artifact_digest(&mut artifact)`.
6. Encodes the stamped artifact to postcard bytes.
7. Stages TWO ops in one `OwnedWriteBatch`: the counter key mapped to
   `next`, and the `CompiledIrRecord` for the stamped artifact.
8. Commits the batch. If the workflow policy is Strict, uses
   `StrictDurability` (SyncAll) before returning Ok. If commit errors
   at any stage, neither the counter nor the artifact is made durable.
9. Returns the stamped `AcceptedArtifact`.

`verify_expected_bindings` already does not compare `accepted_at_seq`
(verification.rs:127-137), so verification of a stamped artifact
against a freshly prepared seq-0 envelope remains valid.

Counter key constant: new `KEY_ARTIFACT_COUNTER` in `constants.rs`
(`compiled_ir` keyspace, reserved digest bytes =
`blake3("vb:artifact_counter")` so it can never collide with a real
content-addressed artifact). No new keyspace.

## Requirements

- Public `submit_artifact` and `submit_artifact_with_contracts`
  signatures unchanged; callers receive an artifact with a nonzero
  `accepted_at_seq` from the first submission.
- Counter increment and artifact persistence are in the same Fjall
  write batch. If either fails before commit, nothing is mutated.
- Counter read and next-computation are inside the `write_lock` hold,
  preventing two concurrent submissions from stamping the same seq.
- `SequenceOverflow` is returned for u64::MAX counter (reused error).
- No new public setter for the counter; the journal's internal
  `write_lock` is the sole serialization mechanism.
- Strict policy: batch uses SyncAll before Ok. Journaled/Relaxed
  preserve actual policy behavior and documented data-loss windows.
- Counter and artifact have atomic visibility (same batch). Physical
  durability is the policy's existing claim; this design does not
  promise IO-rollback or blanket crash consistency beyond what the
  write batch already provides.
- Rejected workflow (validation/encoding error before the batch
  commit) does not allocate a seq.
- Legacy seq-0 envelopes (old artifacts in the store) keep current
  read/floor semantics; nonzero stamps are only meaningful for
  same-journal acceptance order.

## Acceptance Scenarios

1. First `submit_artifact` call: returned artifact has
   `accepted_at_seq = 1`.
2. Second `submit_artifact` call: returned artifact has
   `accepted_at_seq = 2` (monotonically increasing).
3. Two concurrent `submit_artifact` calls: each gets a unique seq
   (1 and 2 in some order), no duplicates.
4. Journal closed, reopened, `submit_artifact` called: counter reads
   the persisted LAST_ISSUED and allocates LAST_ISSUED + 1.
5. Counter at u64::MAX: `submit_artifact` returns
   `SequenceOverflow`; no artifact or counter mutation.
6. Corrupt counter bytes: counter read error returns a typed error
   (e.g., `PostcardDecodeFailed`); no seq allocation.
7. Artifact validation error: `SequenceOverflow` is never returned;
   validation errors are returned unchanged.
8. Freshness floor = 0: all artifacts admitted (legacy compatibility).
9. Freshness floor = N: artifacts with seq < N rejected, seq >= N
   admitted.
10. Freshness floor > any issued seq: all artifacts rejected
    (floor is too high).
