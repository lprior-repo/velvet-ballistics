# Hazard Analysis: Artifact Acceptance Sequence

## Temporal Hazards

### Sequence Leaked on Write Failure
- **Risk**: `accept_seq_allocate()` returns n, artifact write fails; n is never reused.
- **Impact**: Minor waste; sequence space is u64 (1.8e19 values).
- **Mitigation**: Acceptable by design; no lockback or reservation system needed.

### Counter Lost on Crash Before Batch Commit
- **Risk**: Counter read, incremented, but batch commit fails before return.
- **Impact**: Same value could be allocated twice by different callers.
- **Mitigation**: Fjall's batch commit is atomic; either the counter update is persisted or it isn't. On crash, the next `accept_seq_allocate()` reads the last persisted value and continues from there.

## Rust-Core Invariant Hazards

### Zero Sequence After Allocation
- **Risk**: Counter starts at 0; first allocation returns 1. If counter somehow resets to 0, seq=0 becomes a valid allocation again.
- **Impact**: Breaks the "nonzero after allocation" invariant.
- **Mitigation**: Counter only ever increments; no reset path. Initial value is 0 (not yet allocated), first allocation yields 1.

### Digest Recomputation Mismatch
- **Risk**: If seq is assigned after `bind_artifact_digest()`, the digest won't include the seq.
- **Impact**: `verify_accepted_artifact()` fails on stored artifacts.
- **Mitigation**: Allocate seq before preparing artifact; seq is set in the artifact struct before `bind_artifact_digest()` is called.

## Concurrency Hazards

### Race Between Counter Read and Write
- **Risk**: Two threads read counter=n, both increment to n+1, both write n+1; one allocation is lost.
- **Impact**: Duplicate sequence values.
- **Mitigation**: `write_lock` serializes all journal writes; only one `accept_seq_allocate()` executes at a time.

## Bounded State Hazards

### Counter Overflow
- **Risk**: Counter reaches u64::MAX.
- **Impact**: `SequenceOverflow` error; no more artifacts can be accepted.
- **Mitigation**: Typed error; requires schema migration to reset. Practically impossible.

### Counter Key Collision
- **Risk**: `SYSTEM_ACCEPT_SEQ_KEY` collides with future system keys.
- **Impact**: Counter read/write affects wrong value.
- **Mitigation**: Prefix `0x40` reserved for system keys; `0x01` assigned to counter. Documented in constants.

## Hostile Input Hazards

### Forged Acceptance Sequence
- **Risk**: Caller constructs artifact with fake high seq value, bypasses freshness check.
- **Impact**: Stale artifact appears fresh.
- **Mitigation**: Seq is part of artifact digest; `verify_accepted_artifact()` recomputes and compares. Forged seq changes digest → verification fails.

### Tampered Counter
- **Risk**: Direct manipulation of `system` keyspace counter.
- **Impact**: Sequence values jump or reset.
- **Mitigation**: Counter is internal to journal; no public API to set it directly. Requires Fjall-level access (single-process lock protects this).

## API/Release Hazards

### Backward Compatibility with Zero Seq Artifacts
- **Risk**: Existing stored artifacts have `accepted_at_seq=0`. New code treats them as valid.
- **Impact**: Freshness checks work differently for old vs new artifacts.
- **Mitigation**: Old artifacts with seq=0 are treated as "pre-sequence"; they pass freshness checks with floor=0 but fail with any nonzero floor. This is correct: they were accepted before sequence tracking existed.

### Relaxed Policy Sequence Expectations
- **Risk**: Tests expect Relaxed to skip everything including sequence allocation.
- **Impact**: Test failures if seq allocation is added to Relaxed path.
- **Mitigation**: Sequence allocation is a journal property, not a policy property. All policies get sequences. Tests must be updated to expect nonzero seq for Relaxed.
