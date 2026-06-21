# SA-009: Relaxed-policy admission skips the post-write presence check that Journaled/Strict perform

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_storage/src/admission/flow.rs:73-86` (relaxed) vs `crates/vb_storage/src/admission/flow.rs:88-107` (checked)
- **Confidence**: confirmed

## Description

`submit_checked_artifact_with_evidence` ends with a `verify_persisted_artifact_present(journal, workflow.digest())?` call (line 105) that confirms the artifact is durable before returning. The relaxed path `submit_relaxed_artifact_with_evidence` omits this check entirely, so a silent Fjall write failure under relaxed policy produces an `AcceptedArtifact` returned to the caller that is not actually in the journal.

## Evidence

```rust
// crates/vb_storage/src/admission/flow.rs:73-107
fn submit_relaxed_artifact_with_evidence(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    required_capabilities: Box<[vb_core::capability::Capability]>,
    idempotency_evidence: &IdempotencyEvidence,
) -> Result<AcceptedArtifact, JournalError> {
    let ir_bytes = validate_workflow_artifact_bytes(workflow)?;
    let mut proof = VerificationProof::new(workflow.digest(), 0, false);
    proof.idempotency_keyed = idempotency_evidence.keyed.clone();
    proof.idempotency_attested = idempotency_evidence.attested.clone();
    let artifact = accepted_artifact(workflow, ir_bytes, proof, required_capabilities)?;
    persist_accepted_artifact_ir(journal, &artifact)?;
    Ok(artifact)                                            // <-- no verify_persisted_artifact_present
}

fn submit_checked_artifact_with_evidence(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    policy: vb_core::RuntimePolicy,
    required_capabilities: Box<[vb_core::capability::Capability]>,
    idempotency_evidence: IdempotencyEvidence,
) -> Result<AcceptedArtifact, JournalError> {
    let ir_bytes = validate_workflow_artifact_bytes(workflow)?;
    let durable = policy == vb_core::RuntimePolicy::Strict;
    let mut proof = VerificationProof::new(workflow.digest(), ADMISSION_GATE_COUNT, durable);
    proof.idempotency_keyed = idempotency_evidence.keyed;
    proof.idempotency_attested = idempotency_evidence.attested;
    let artifact = accepted_artifact(workflow, ir_bytes, proof, required_capabilities)?;
    persist_accepted_artifact_ir(journal, &artifact)?;
    if durable { journal.persist_strict()?; }
    verify_persisted_artifact_present(journal, workflow.digest())?;       // <-- only here
    Ok(artifact)
}
```

`persist_accepted_artifact_ir` calls `journal.put_compiled_ir(&record)` (`crates/vb_storage/src/admission/persistence.rs:23`). If Fjall reports Ok but the row is not actually readable (e.g., torn write absorbed by the LSM, KV-separation race, slot reuse), relaxed-policy callers will hand back an `AcceptedArtifact` whose digest does not resolve on read.

## Adversarial Check

The relaxed policy by definition tolerates looser durability — it skips `persist_strict` (no fsync), accepting that the bytes may not survive a crash. But "may not survive a crash" is different from "may not be readable immediately after the function returns Ok". The verify-presence check is about read-back consistency within the live process, not about durability. Removing it from the relaxed path conflates the two: a write that returned Ok but is not readable in the same process is a Fjall bug or a corruption signal, and should be caught regardless of policy. The contract documented at the top of the file (`flow.rs:14-24`) lists "Persistence: store the artifact" as step 5 with no policy carve-out.

## Suggested Fix

Hoist `verify_persisted_artifact_present(journal, workflow.digest())?` out of `submit_checked_artifact_with_evidence` and into `submit_artifact_for_policy` so both branches run it:

```rust
fn submit_artifact_for_policy(...) -> Result<AcceptedArtifact, JournalError> {
    let artifact = match policy {
        vb_core::RuntimePolicy::Relaxed => submit_relaxed_artifact_with_evidence(...)?,
        vb_core::RuntimePolicy::Journaled | vb_core::RuntimePolicy::Strict =>
            submit_checked_artifact_with_evidence(...)?,
        _ => return Err(JournalError::ArtifactMalformed),
    };
    verify_persisted_artifact_present(journal, workflow.digest())?;
    Ok(artifact)
}
```
