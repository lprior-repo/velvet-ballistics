# Workflow Model: Artifact Acceptance

## Artifact Submission Flow

```
Callers: vb_cli run, vb_runtime admit, integration tests
    ↓
submit_artifact(journal, workflow, policy)
    ↓
journal.accept_seq_allocate()
    ↓ (atomic: read counter → increment → write counter → return next)
    ↓ returns EventSeq(n)
prepare_compiled_artifact(workflow, policy, contracts, seq=n)
    ↓ (computes digest including seq=n)
journal.put_compiled_ir(artifact)
    ↓
if Strict: journal.persist_strict()
    ↓
verify_artifact_persisted(artifact.digest)
    ↓
return AcceptedArtifact (with accepted_at_seq=n)
```

## Acceptance Sequence Allocation

```
accept_seq_allocate()
    ↓
acquire write_lock
    ↓
system.get(SYSTEM_ACCEPT_SEQ_KEY) → Option<u64>
    ↓ (None → current=0)
current = current + 1
    ↓ (checked_add; if overflow → SequenceOverflow)
batch = database.batch()
    ↓
batch.insert(system, SYSTEM_ACCEPT_SEQ_KEY, current.to_be_bytes())
    ↓
batch.commit()
    ↓
release write_lock
    ↓
return EventSeq(current)
```

## Run Admission with Freshness Check

```
admit_artifact_run_with_certificate_floor(store, policy, run_id, digest, caps, floor)
    ↓ (Strict/Journaled only)
store.load_accepted_artifact(digest)
    ↓
validate envelope (gates, flags, digest binding)
    ↓
if artifact.accepted_at_seq < floor:
    → ArtifactCertificateStale error
    ↓
validate capabilities
    ↓
return RunAdmission
```

## Crash and Reopen Semantics

- Counter persists in `system` keyspace across journal open/close.
- On reopen, `accept_seq_allocate()` reads the persisted counter and continues.
- If artifact write fails after sequence allocation, the sequence is "leaked" (not reused).
  This is acceptable: sequences are u64, allocation is rare relative to u64 space.
