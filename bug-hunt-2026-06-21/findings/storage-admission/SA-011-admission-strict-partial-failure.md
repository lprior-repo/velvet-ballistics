# SA-011: `submit_checked_artifact_with_evidence` calls `persist_strict()` then `verify_persisted_artifact_present()` — partial failure leaves artifact persisted but not verified

- **Severity**: Low
- **Category**: correctness
- **Location**: `crates/vb_storage/src/admission/flow.rs:100-106`
- **Confidence**: confirmed

## Description

After `persist_accepted_artifact_ir` commits the artifact to the LSM, the strict path calls `journal.persist_strict()` (fsync) and then `verify_persisted_artifact_present`. If the verify call returns Err (e.g., the journal's `compiled_ir` reader transiently fails), the function returns Err to the caller — but the artifact has already been written and fsynced. The caller has no way to know whether the artifact is in storage, and any retry of `submit_artifact_with_contracts` will re-run the full validation pipeline against the now-existing record.

## Evidence

```rust
// crates/vb_storage/src/admission/flow.rs:100-106
    let artifact = accepted_artifact(workflow, ir_bytes, proof, required_capabilities)?;
    persist_accepted_artifact_ir(journal, &artifact)?;       // <-- commit
    if durable {
        journal.persist_strict()?;                           // <-- fsync
    }
    verify_persisted_artifact_present(journal, workflow.digest())?;  // <-- can fail post-commit
    Ok(artifact)
```

`verify_persisted_artifact_present` (`crates/vb_storage/src/admission/persistence.rs:34-46`) wraps `journal.compiled_ir(digest)` and converts any error to `ArtifactMalformed`. A transient Fjall read error after a successful write therefore surfaces as `ArtifactMalformed` even though the artifact is fine.

## Adversarial Check

The verify step exists to catch the rare case where `put_compiled_ir` returned Ok but the row is not readable (Fjall memtable flush race, KV-separation lag, etc.). When verify fails for a transient reason, the artifact IS durably stored — the caller's `Err` is misleading. The caller is likely to retry, which re-runs validation against the just-written record (probably succeeding) but re-fsyncs, re-allocates IR bytes, and re-hashes the workflow. The pattern is correct in the limit (eventually consistent) but produces misleading errors and redundant work in the transient-failure case. Worse, the `ArtifactMalformed` error code conflates "artifact never written" with "verify transiently failed" — operator response differs for the two cases.

## Suggested Fix

Distinguish the post-write verify error from a real malformed artifact:

```rust
let artifact = accepted_artifact(workflow, ir_bytes, proof, required_capabilities)?;
persist_accepted_artifact_ir(journal, &artifact)?;
if durable { journal.persist_strict()?; }
verify_persisted_artifact_present(journal, workflow.digest())
    .map_err(|_| JournalError::AcceptedArtifactVerifyFailed { digest: workflow.digest() })?;
Ok(artifact)
```

This documents that the artifact was written but could not be verified, so the operator knows to inspect the journal directly.
