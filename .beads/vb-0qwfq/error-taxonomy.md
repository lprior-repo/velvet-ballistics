# Error Taxonomy: Artifact Acceptance Sequence

## Journal Errors (vb_storage::JournalError)

### SequenceOverflow
- **When**: Counter reaches `EventSeq::MAX` (u64::MAX).
- **Cause**: More than 2^64 artifacts accepted; effectively impossible in practice.
- **Recovery**: None; requires counter reset or schema migration.
- **Typed**: Yes, new variant.

### WriteLockPoisoned
- **When**: `write_lock` mutex is poisoned (another thread panicked while holding it).
- **Cause**: Panic in concurrent `accept_seq_allocate` or other journal write operation.
- **Recovery**: Journal must be recreated; no partial state.

### Fjall IO Errors (existing variants)
- `JournalError::Fjall(Box<dyn std::error::Error>)` — underlying Fjall operation failed.
- `JournalError::KeyCapacity` — counter value too large to encode.
- `JournalError::PostcardEncodeFailed` — counter serialization failed.
- All propagate from `batch.commit()` or `system.get()`.

### ArtifactChecksumMismatch (existing, now enforced)
- **When**: Artifact envelope digest does not match expected (includes seq mismatch).
- **Cause**: Artifact modified after acceptance; sequence tampered.
- **Triggered by**: `verify_accepted_artifact` recomputing and comparing digest.

## Runtime Admission Errors (vb_runtime::AdmissionError)

### ArtifactCertificateStale (existing, now meaningful)
- **When**: `artifact.accepted_at_seq < required_at_least`.
- **Cause**: Artifact accepted before caller's freshness floor.
- **Fields**: `digest`, `accepted_at_seq`, `required_at_least`.
- **Previously**: Could never trigger with seq=0 sentinel and nonzero floor; now functional.

### ArtifactNotFound / ArtifactEvidenceInvalid / etc.
- Existing admission envelope validation errors; unchanged.

## Error Propagation

```
accept_seq_allocate fails → submit_artifact returns Err(JournalError)
    → caller (vb_cli, runtime, test) sees journal error

submit_artifact fails → caller sees Err(JournalError)
    → vb_cli exits with error code
    → runtime's admit_artifact_run sees storage error
```
