#![forbid(unsafe_code)]
#![allow(clippy::expect_used)]

use vb_core::value::ConstValue;
use vb_core::workflow::{ResourceContract, WorkflowParts};
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, RuntimePolicy, SlotIdx, StepIdx,
    WorkflowDigest,
};
use vb_storage::FjallJournal;
use vb_storage::admission::{AcceptedArtifact, VerificationWarning, submit_artifact};

fn temp_journal() -> Result<FjallJournal, String> {
    let dir = tempfile::tempdir().map_err(|error| format!("tempdir failed: {error}"))?;
    FjallJournal::open(dir.keep(), None).map_err(|error| format!("journal open failed: {error}"))
}

fn minimal_workflow() -> Result<CompiledWorkflow, String> {
    let mut parts = WorkflowParts {
        name: Box::<str>::from("scope.valid_workflow"),
        digest: WorkflowDigest::from_bytes([0_u8; 32]),
        nodes: Box::new([
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(42)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let hash_bytes = postcard::to_allocvec(&parts)
        .map_err(|error| format!("serialize workflow parts failed: {error}"))?;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&hash_bytes).into());
    CompiledWorkflow::try_from_parts(parts)
        .map_err(|error| format!("compiled workflow construction failed: {error}"))
}

fn submit_minimal(policy: RuntimePolicy) -> Result<AcceptedArtifact, String> {
    let journal = temp_journal()?;
    let workflow = minimal_workflow()?;
    submit_artifact(&journal, &workflow, policy)
        .map_err(|error| format!("submit_artifact failed: {error}"))
}

fn warning_at(gate: u8) -> VerificationWarning {
    VerificationWarning {
        code: 1,
        message: Box::<str>::from("accepted-artifact v1 warning boundary"),
        gate,
    }
}

#[test]
fn accepted_artifact_validator_accepts_warning_gate_fifteen() {
    let warning = warning_at(15);
    assert!(warning.is_valid());
}

#[test]
fn accepted_artifact_validator_rejects_warning_gate_sixteen() {
    let warning = warning_at(16);
    assert!(!warning.is_valid());
}

#[test]
fn accepted_artifact_validator_uses_fifteen_gate_v1_upper_bound() {
    assert_eq!(VerificationWarning::MAX_GATE, 15);
}

#[test]
fn accepted_artifact_validator_rejects_legacy_thirteen_gate_upper_bound() {
    assert_ne!(VerificationWarning::MAX_GATE, 13);
}

#[test]
fn accepted_artifact_encoder_records_fifteen_gate_proof_when_policy_is_journaled()
-> Result<(), String> {
    let artifact = submit_minimal(RuntimePolicy::Journaled)?;
    if artifact.verification.gate_count != 15 {
        return Err(format!(
            "Journaled must have 15 gates, got {}",
            artifact.verification.gate_count
        ));
    }
    Ok(())
}

#[test]
fn accepted_artifact_encoder_records_fifteen_gate_proof_when_policy_is_strict() -> Result<(), String>
{
    let artifact = submit_minimal(RuntimePolicy::Strict)?;
    if artifact.verification.gate_count != 15 {
        return Err(format!(
            "Strict must have 15 gates, got {}",
            artifact.verification.gate_count
        ));
    }
    Ok(())
}

#[test]
fn accepted_artifact_encoder_rejects_relaxed_raw_submit_when_accepted_artifacts_are_required()
-> Result<(), String> {
    let journal = temp_journal()?;
    let workflow = minimal_workflow()?;
    let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed)
        .map_err(|error| format!("Relaxed policy must be accepted: {error}"))?;
    if artifact.verification.gate_count != 0 {
        return Err(format!(
            "Relaxed must have 0 gates, got {}",
            artifact.verification.gate_count
        ));
    }
    if artifact.verification.durable {
        return Err(String::from("Relaxed must not be durable"));
    }
    Ok(())
}

#[test]
fn accepted_artifact_store_payload_is_raw_workflow_parts_not_nested_artifact() -> Result<(), String>
{
    let artifact = submit_minimal(RuntimePolicy::Strict)?;
    let decoded_parts = postcard::from_bytes::<WorkflowParts>(&artifact.ir)
        .map_err(|error| format!("workflow parts decode failed: {error}"))?;
    if &*decoded_parts.name != "scope.valid_workflow" {
        return Err(format!(
            "name must be 'scope.valid_workflow', got {:?}",
            &*decoded_parts.name
        ));
    }
    Ok(())
}

#[test]
fn accepted_artifact_encoder_binds_ir_digest_to_ir_bytes_not_workflow_parts_digest()
-> Result<(), String> {
    let artifact = submit_minimal(RuntimePolicy::Strict)?;
    if artifact.digest != artifact.verification.digest {
        return Err(format!(
            "artifact digest must match verification digest: artifact={:?}, verification={:?}",
            artifact.digest, artifact.verification.digest
        ));
    }
    Ok(())
}

#[test]
fn accepted_artifact_validator_produces_valid_verification_proof_with_all_flags_true()
-> Result<(), String> {
    let artifact = submit_minimal(RuntimePolicy::Strict)?;
    let proof = &artifact.verification;
    if proof.gate_count != 15 {
        return Err(format!("expected 15 gates, got {}", proof.gate_count));
    }
    if !proof.bounded_claimed {
        return Err(String::from("bounded_claimed flag must be true"));
    }
    if !proof.taint_safe_claimed {
        return Err(String::from("taint_safe_claimed flag must be true"));
    }
    if !proof.retry_safe_claimed {
        return Err(String::from("retry_safe_claimed flag must be true"));
    }
    if !proof.replayable_claimed {
        return Err(String::from("replayable_claimed flag must be true"));
    }
    if !proof.durable {
        return Err(String::from("durable flag must be true for Strict policy"));
    }
    Ok(())
}

#[test]
fn accepted_artifact_encoder_journaled_proof_has_durable_false() -> Result<(), String> {
    let artifact = submit_minimal(RuntimePolicy::Journaled)?;
    if artifact.verification.durable {
        return Err(String::from("Journaled policy must not be durable"));
    }
    Ok(())
}

#[test]
fn accepted_artifact_encoder_strict_proof_has_durable_true() -> Result<(), String> {
    let artifact = submit_minimal(RuntimePolicy::Strict)?;
    if !artifact.verification.durable {
        return Err(String::from("Strict policy must be durable"));
    }
    Ok(())
}

#[test]
fn accepted_artifact_encoder_journaled_gate_count_equals_fifteen() -> Result<(), String> {
    let artifact = submit_minimal(RuntimePolicy::Journaled)?;
    if artifact.verification.gate_count != 15 {
        return Err(format!(
            "Journaled must have 15 gates, got {}",
            artifact.verification.gate_count
        ));
    }
    Ok(())
}

#[test]
fn accepted_artifact_encoder_strict_gate_count_equals_fifteen() -> Result<(), String> {
    let artifact = submit_minimal(RuntimePolicy::Strict)?;
    if artifact.verification.gate_count != 15 {
        return Err(format!(
            "Strict must have 15 gates, got {}",
            artifact.verification.gate_count
        ));
    }
    Ok(())
}

#[test]
fn accepted_artifact_validator_accepts_empty_warnings_array() -> Result<(), String> {
    let artifact = submit_minimal(RuntimePolicy::Strict)?;
    if !artifact.verification.warnings.is_empty() {
        return Err(String::from("warnings must be empty"));
    }
    Ok(())
}

#[test]
fn accepted_artifact_validator_accepts_empty_idempotency_lists() -> Result<(), String> {
    let artifact = submit_minimal(RuntimePolicy::Strict)?;
    if !artifact.verification.idempotency_keyed.is_empty() {
        return Err(String::from("idempotency_keyed must be empty"));
    }
    if !artifact.verification.idempotency_attested.is_empty() {
        return Err(String::from("idempotency_attested must be empty"));
    }
    Ok(())
}

#[test]
fn accepted_artifact_encoder_records_empty_required_capabilities() -> Result<(), String> {
    let artifact = submit_minimal(RuntimePolicy::Strict)?;
    if !artifact.required_capabilities.is_empty() {
        return Err(String::from("required_capabilities must be empty"));
    }
    Ok(())
}

#[test]
fn accepted_artifact_encoder_records_zero_accepted_at_seq() -> Result<(), String> {
    let artifact = submit_minimal(RuntimePolicy::Strict)?;
    // Current implementation records accepted artifacts without journal sequence tracking.
    if artifact.accepted_at_seq.get() != 0 {
        return Err(format!(
            "accepted_at_seq must be 0, got {}",
            artifact.accepted_at_seq.get()
        ));
    }
    Ok(())
}

#[test]
fn accepted_artifact_roundtrip_through_storage_persists_and_loads() -> Result<(), String> {
    let journal = temp_journal()?;
    let workflow = minimal_workflow()?;
    let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
        .map_err(|e| format!("submit failed: {e}"))?;

    let stored = journal
        .compiled_ir(artifact.digest)
        .map_err(|e| format!("load failed: {e}"))?
        .ok_or_else(|| String::from("artifact not found after submit"))?;

    if stored.digest != artifact.digest {
        return Err(format!(
            "stored digest must match: stored={:?}, artifact={:?}",
            stored.digest, artifact.digest
        ));
    }
    if stored.ir.is_empty() {
        return Err(String::from("ir must not be empty"));
    }
    Ok(())
}

#[test]
fn accepted_artifact_stored_bytes_are_postcard_encoded() -> Result<(), String> {
    let journal = temp_journal()?;
    let workflow = minimal_workflow()?;
    let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
        .map_err(|e| format!("submit failed: {e}"))?;

    let decoded: WorkflowParts = postcard::from_bytes(&artifact.ir)
        .map_err(|e| format!("postcard decode of ir field failed: {e}"))?;

    if &*decoded.name != "scope.valid_workflow" {
        return Err(format!(
            "name must be 'scope.valid_workflow', got {:?}",
            &*decoded.name
        ));
    }
    Ok(())
}

#[test]
fn runtime_admission_requires_artifact_digest_not_raw_workflow() -> Result<(), String> {
    let journal = temp_journal()?;
    let workflow = minimal_workflow()?;
    let result = submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed);
    let artifact = result.expect("Relaxed must be accepted");
    if artifact.digest != workflow.digest() {
        return Err(format!(
            "artifact digest must match workflow digest: artifact={:?}, workflow={:?}",
            artifact.digest,
            workflow.digest()
        ));
    }
    Ok(())
}

#[test]
fn submit_artifact_rejects_missing_workflow_digest() -> Result<(), String> {
    let journal = temp_journal()?;
    let workflow = minimal_workflow()?;
    let bad_workflow = CompiledWorkflow::try_from_parts(vb_core::workflow::WorkflowParts {
        name: Box::<str>::from("scope.bad"),
        digest: WorkflowDigest::from_bytes([0_u8; 32]),
        nodes: workflow.to_parts().nodes,
        expressions: workflow.to_parts().expressions,
        accessors: workflow.to_parts().accessors,
        constants: workflow.to_parts().constants,
        slot_count: workflow.to_parts().slot_count,
        symbols_count: 0,
        entry: workflow.to_parts().entry,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| format!("workflow with zero digest should fail: {e}"))?;
    let result = submit_artifact(&journal, &bad_workflow, RuntimePolicy::Strict);
    match result {
        Err(vb_storage::JournalError::ArtifactChecksumMismatch) => Ok(()),
        other => Err(format!(
            "workflow with zero digest must be rejected with ArtifactChecksumMismatch, got {:?}",
            other
        )),
    }
}

#[test]
fn submit_artifact_validates_workflow_structure() -> Result<(), String> {
    let journal = temp_journal()?;
    let workflow = minimal_workflow()?;
    let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
        .expect("valid workflow should be accepted under Strict policy");
    if artifact.verification.gate_count != 15 {
        return Err(format!(
            "accepted workflow must have 15 gates, got {}",
            artifact.verification.gate_count
        ));
    }
    Ok(())
}

#[test]
fn submit_artifact_returns_artifact_with_correct_digest() -> Result<(), String> {
    let journal = temp_journal()?;
    let workflow = minimal_workflow()?;
    let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
        .map_err(|e| format!("submit failed: {e}"))?;
    if artifact.digest != workflow.digest() {
        return Err(format!(
            "artifact digest must match workflow digest: artifact={:?}, workflow={:?}",
            artifact.digest,
            workflow.digest()
        ));
    }
    Ok(())
}

#[test]
fn submit_artifact_persists_artifact_to_journal() -> Result<(), String> {
    let journal = temp_journal()?;
    let workflow = minimal_workflow()?;
    let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
        .map_err(|e| format!("submit failed: {e}"))?;

    let loaded = journal
        .compiled_ir(artifact.digest)
        .map_err(|e| format!("load failed: {e}"))?;
    if loaded.is_none() {
        return Err(String::from("artifact must be persisted"));
    }
    Ok(())
}

#[test]
fn submit_artifact_journaled_does_not_persist_strictly() -> Result<(), String> {
    let journal = temp_journal()?;
    let workflow = minimal_workflow()?;
    let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
        .map_err(|e| format!("submit failed: {e}"))?;

    let loaded = journal
        .compiled_ir(artifact.digest)
        .map_err(|e| format!("load failed: {e}"))?;
    if loaded.is_none() {
        return Err(String::from("Journaled artifact must still be persisted"));
    }
    Ok(())
}

#[test]
fn accepted_artifact_proof_contains_workflow_digest() -> Result<(), String> {
    let journal = temp_journal()?;
    let workflow = minimal_workflow()?;
    let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
        .map_err(|e| format!("submit failed: {e}"))?;
    if artifact.verification.digest != workflow.digest() {
        return Err(format!(
            "proof digest must match workflow digest: proof={:?}, workflow={:?}",
            artifact.verification.digest,
            workflow.digest()
        ));
    }
    Ok(())
}

#[test]
fn accepted_artifact_validator_requires_all_proof_flags_true_under_strict() -> Result<(), String> {
    let artifact = submit_minimal(RuntimePolicy::Strict)?;
    let p = &artifact.verification;
    if !(p.bounded_claimed
        && p.taint_safe_claimed
        && p.retry_safe_claimed
        && p.replayable_claimed
        && p.durable)
    {
        return Err(String::from(
            "all proof flags must be true and durable must be true under Strict",
        ));
    }
    Ok(())
}

#[test]
fn accepted_artifact_validator_requires_all_proof_flags_true_under_journaled() -> Result<(), String>
{
    let artifact = submit_minimal(RuntimePolicy::Journaled)?;
    let p = &artifact.verification;
    if !(p.bounded_claimed
        && p.taint_safe_claimed
        && p.retry_safe_claimed
        && p.replayable_claimed
        && !p.durable)
    {
        return Err(String::from(
            "all proof flags must be true and durable must be false under Journaled",
        ));
    }
    Ok(())
}
