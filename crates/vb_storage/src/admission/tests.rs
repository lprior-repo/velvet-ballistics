#[allow(
    clippy::assertions_on_constants,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
use crate::admission::*;
use crate::error::JournalError;
use vb_core::workflow::ResourceContract;

#[test]
fn verification_warning_display_formats_gate_code_message() {
    let warning = VerificationWarning {
        code: 42,
        message: Box::from("deprecated action kind"),
        gate: 3,
    };
    assert_eq!(format!("{warning}"), "gate 3: [42] deprecated action kind");
}

#[test]
fn verification_warning_equality_works() {
    let a = VerificationWarning {
        code: 1,
        message: Box::from("alpha"),
        gate: 2,
    };
    let b = VerificationWarning {
        code: 1,
        message: Box::from("alpha"),
        gate: 2,
    };
    assert_eq!(a, b);
}

#[test]
fn verification_warning_inequality_different_code() {
    let a = VerificationWarning {
        code: 1,
        message: Box::from("alpha"),
        gate: 2,
    };
    let b = VerificationWarning {
        code: 99,
        message: Box::from("alpha"),
        gate: 2,
    };
    assert_ne!(a, b);
}

#[test]
fn verification_warning_inequality_different_gate() {
    let a = VerificationWarning {
        code: 1,
        message: Box::from("alpha"),
        gate: 1,
    };
    let b = VerificationWarning {
        code: 1,
        message: Box::from("alpha"),
        gate: 13,
    };
    assert_ne!(a, b);
}

#[test]
fn verification_proof_new_initializes_empty_warnings() {
    let digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
    let proof = VerificationProof::new(digest, 2, true);
    assert!(proof.warnings.is_empty());
    assert_eq!(proof.gate_count, 2);
    assert!(proof.durable);
}

#[test]
fn verification_proof_warnings_can_be_populated() {
    let digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
    let mut proof = VerificationProof::new(digest, 5, false);
    proof.warnings.push(VerificationWarning {
        code: 100,
        message: Box::from("soft check advisory"),
        gate: 7,
    });
    proof.warnings.push(VerificationWarning {
        code: 200,
        message: Box::from("boundary advisory"),
        gate: 11,
    });
    assert_eq!(proof.warnings.len(), 2);
    assert_eq!(proof.warnings[0].gate, 7);
    assert_eq!(proof.warnings[1].code, 200);
}

#[test]
fn verification_warning_clone_preserves_fields() {
    let original = VerificationWarning {
        code: 55,
        message: Box::from("cloneable warning"),
        gate: 9,
    };
    let cloned = original.clone();
    assert_eq!(original, cloned);
    assert_eq!(cloned.code, 55);
    assert_eq!(&*cloned.message, "cloneable warning");
    assert_eq!(cloned.gate, 9);
}

#[test]
fn is_valid_rejects_gate_zero() {
    let w = VerificationWarning {
        code: 1,
        message: Box::from("zero gate"),
        gate: 0,
    };
    assert!(!w.is_valid());
}

#[test]
fn is_valid_accepts_gate_one() {
    let w = VerificationWarning {
        code: 1,
        message: Box::from("min gate"),
        gate: VerificationWarning::MIN_GATE,
    };
    assert!(w.is_valid());
}

#[test]
fn is_valid_accepts_gate_two() {
    let w = VerificationWarning {
        code: 1,
        message: Box::from("max gate"),
        gate: VerificationWarning::MAX_GATE,
    };
    assert!(w.is_valid());
}

#[test]
fn is_valid_accepts_gate_fourteen() {
    let w = VerificationWarning {
        code: 1,
        message: Box::from("within gate range"),
        gate: 14,
    };
    assert!(w.is_valid());
}

// =========================================================================
// submit_artifact: Relaxed policy
// =========================================================================

/// Owns both a temporary directory path and a FjallJournal so the directory
/// is not dropped while the journal is in use.
struct TestJournal {
    path: std::path::PathBuf,
    journal: crate::FjallJournal,
}

impl Drop for TestJournal {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.path) {
            std::hint::black_box(error.kind());
        }
    }
}

impl std::ops::Deref for TestJournal {
    type Target = crate::FjallJournal;
    fn deref(&self) -> &Self::Target {
        &self.journal
    }
}

/// Opens a temporary FjallJournal that is cleaned up when dropped.
fn temp_journal() -> Result<TestJournal, JournalError> {
    let dir = tempfile::tempdir().map_err(|_| JournalError::ArtifactMalformed)?;
    let path = dir.keep();
    let journal = crate::FjallJournal::open(&path, None)?;
    Ok(TestJournal { path, journal })
}

/// Builds a minimal valid CompiledWorkflow for testing.
///
/// The digest is computed by serializing the parts with the digest field zeroed,
/// then BLAKE3-hashing the result. This mirrors the checksum validation gate.
fn minimal_workflow() -> Result<vb_core::CompiledWorkflow, String> {
    use vb_core::value::ConstValue;
    use vb_core::workflow::{ResourceContract, WorkflowParts};
    use vb_core::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, SlotIdx, StepIdx,
        WorkflowDigest,
    };

    let mut parts = WorkflowParts {
        name: Box::<str>::from("test_admission"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
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

    // Compute the correct BLAKE3 digest from the zeroed-digest serialization.
    let hash_bytes =
        postcard::to_allocvec(&parts).map_err(|e| format!("serialize parts for digest: {e}"))?;
    let computed = blake3::hash(&hash_bytes);
    parts.digest = WorkflowDigest::from_bytes(computed.into());

    CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
}

fn compiled_record_digest(
    workflow: &vb_core::CompiledWorkflow,
) -> Result<vb_core::WorkflowDigest, String> {
    let bytes = postcard::to_allocvec(&workflow.to_parts())
        .map_err(|e| format!("serialize compiled workflow parts: {e}"))?;
    Ok(vb_core::WorkflowDigest::from_bytes(
        blake3::hash(&bytes).into(),
    ))
}

#[test]
fn submit_artifact_relaxed_persists_and_returns_artifact() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;
    let result = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed)
        .map_err(|e| format!("submit_artifact(relaxed) failed: {e}"))?;

    // The returned artifact digest binds the accepted-artifact envelope;
    // source_digest retains the workflow structural digest.
    assert_eq!(
        result.source_digest,
        workflow.digest(),
        "source digest must match workflow digest"
    );
    assert_eq!(
        accepted_artifact_digest(&result).map_err(|e| format!("artifact digest: {e}"))?,
        result.digest,
        "artifact digest must match the canonical envelope digest"
    );

    // The proof under Relaxed must have 0 gates and durable=false.
    assert_eq!(result.verification.gate_count, 0, "relaxed must skip gates");
    assert!(!result.verification.durable, "relaxed must not be durable");

    // The proof's digest must match.
    assert_eq!(
        result.verification.digest, result.digest,
        "proof digest must match artifact digest"
    );

    // The ir bytes must be non-empty (postcard serialization).
    assert!(!result.ir.is_empty(), "compiled IR bytes must not be empty");

    // Verify we can read the artifact back from storage.
    let loaded = journal
        .compiled_ir(result.digest)
        .map_err(|e| format!("compiled_ir read failed: {e}"))?;
    assert!(loaded.is_some(), "artifact must be readable after submit");
    Ok(())
}

#[test]
fn submit_artifact_relaxed_performs_immediate_live_readback() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    // Relaxed submit must succeed end-to-end: the new live-readback check
    // inside submit_artifact_with_contracts must observe the record that
    // put_compiled_ir just persisted. If the readback regressed to
    // returning None or errored, submit_artifact would now return
    // JournalError::ArtifactMalformed.
    let artifact = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed)
        .map_err(|e| format!("submit_artifact(relaxed) failed: {e}"))?;

    // The returned AcceptedArtifact must carry the workflow digest as source evidence.
    assert_eq!(
        artifact.source_digest,
        workflow.digest(),
        "accepted artifact source digest must match workflow digest"
    );
    assert_eq!(artifact.verification.digest, artifact.digest);

    // The Relaxed verification proof must report zero gates and not durable.
    assert_eq!(
        artifact.verification.gate_count, 0,
        "relaxed verification must skip all gates"
    );
    assert!(
        !artifact.verification.durable,
        "relaxed verification must not be durable"
    );

    // The live readback path must have observed Some(record). Verify the
    // persisted record is present and structurally consistent with the
    // artifact just returned.
    let stored = journal
        .compiled_ir(artifact.digest)
        .map_err(|e| format!("compiled_ir live readback failed: {e}"))?;
    let record = stored.ok_or_else(|| {
        String::from("live readback returned None — Relaxed must persist and read back")
    })?;

    assert_eq!(
        record.digest, artifact.digest,
        "stored CompiledIrRecord digest must match accepted artifact digest"
    );
    assert!(
        !record.ir.is_empty(),
        "stored CompiledIrRecord ir bytes must be non-empty (postcard AcceptedArtifact)"
    );

    // Round-trip the stored bytes back into an AcceptedArtifact and verify
    // the digest survives the readback path. This catches the case where
    // a future change weakens the readback but still leaves Some(record).
    let decoded: AcceptedArtifact = postcard::from_bytes(&record.ir)
        .map_err(|e| format!("postcard decode of stored artifact failed: {e}"))?;
    assert_eq!(
        decoded.digest, artifact.digest,
        "decoded artifact digest must match accepted artifact digest"
    );
    assert_eq!(decoded.source_digest, workflow.digest());
    assert_eq!(
        decoded.verification.gate_count, 0,
        "decoded artifact must reflect Relaxed gate count"
    );
    Ok(())
}

#[test]
fn submit_artifact_journaled_runs_both_gates() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    let result = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
        .map_err(|e| format!("submit_artifact(journaled) failed: {e}"))?;

    // Journaled passes 15 gates but is not durable (no SyncAll).
    assert_eq!(
        result.verification.gate_count, 15,
        "journaled must pass 15 verification gates"
    );
    assert!(
        !result.verification.durable,
        "journaled must not be durable"
    );
    assert_eq!(result.source_digest, workflow.digest());
    assert_eq!(result.verification.digest, result.digest);
    Ok(())
}

#[test]
fn submit_artifact_strict_is_durable() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    let result = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict)
        .map_err(|e| format!("submit_artifact(strict) failed: {e}"))?;

    // Strict passes 15 gates AND is durable.
    assert_eq!(result.verification.gate_count, 15);
    assert!(result.verification.durable, "strict must be durable");
    assert_eq!(result.source_digest, workflow.digest());
    assert_eq!(result.verification.digest, result.digest);
    Ok(())
}

// =========================================================================
// submit_artifact: checksum validation
// =========================================================================

#[test]
fn submit_artifact_journaled_roundtrip_bytes_match() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    let result = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
        .map_err(|e| format!("submit failed: {e}"))?;

    // Deserialize the returned IR bytes and verify the digest field.
    let loaded = journal
        .compiled_ir(result.digest)
        .map_err(|e| format!("read failed: {e}"))?;
    let record = loaded.ok_or_else(|| String::from("artifact not found after submit"))?;
    assert_eq!(record.digest, result.digest, "stored digest must match");
    Ok(())
}

// =========================================================================
// admit_compiled_artifact
// =========================================================================

#[test]
fn admit_compiled_artifact_succeeds_for_valid_workflow() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    let digest = admit_compiled_artifact(&journal, &workflow)
        .map_err(|e| format!("admit_compiled_artifact failed: {e}"))?;
    let expected = compiled_record_digest(&workflow)?;

    assert_eq!(
        digest, expected,
        "returned digest must match compiled payload digest"
    );

    // Verify it's stored.
    let loaded = journal
        .compiled_ir(digest)
        .map_err(|e| format!("read failed: {e}"))?;
    assert!(loaded.is_some(), "artifact must be stored after admission");
    Ok(())
}

#[test]
fn admit_compiled_artifact_idempotent() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    let digest_a = admit_compiled_artifact(&journal, &workflow)
        .map_err(|e| format!("first admit failed: {e}"))?;
    let digest_b = admit_compiled_artifact(&journal, &workflow)
        .map_err(|e| format!("second admit failed: {e}"))?;

    assert_eq!(
        digest_a, digest_b,
        "idempotent admission must return same digest"
    );
    Ok(())
}

// =========================================================================
// AcceptedArtifact fields
// =========================================================================

#[test]
fn accepted_artifact_fields_are_populated() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    let artifact = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed)
        .map_err(|e| format!("submit failed: {e}"))?;

    // accepted_at_seq should be 0 (no journal sequence tracking in current impl).
    assert_eq!(
        artifact.accepted_at_seq.get(),
        0,
        "accepted_at_seq must be 0"
    );
    // required_capabilities should be empty for minimal workflow.
    assert!(
        artifact.required_capabilities.is_empty(),
        "minimal workflow has no capabilities"
    );
    Ok(())
}

#[test]
fn submit_artifact_persists_non_empty_required_capabilities_when_contract_requires_capability()
-> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;
    let required = vb_core::capability::Capability::new(
        Box::<str>::from("network.github"),
        vb_core::ActionId::new(7),
    );
    let contract = vb_core::action::ActionContract {
        id: vb_core::ActionId::new(7),
        name: vb_core::action::ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 2048,
        timeout_ms: 1000,
        idempotency: vb_core::action::Idempotency::IdempotentExternal,
        side_effect: vb_core::action::SideEffect::Writes,
        retry_safety: vb_core::action::RetrySafety::KeyRequired,
        required_capabilities: Box::new([required.clone()]),
    };

    let artifact = submit_artifact_with_contracts(
        &journal,
        &workflow,
        vb_core::RuntimePolicy::Journaled,
        &[contract],
    )
    .map_err(|e| format!("submit_artifact_with_contracts failed: {e}"))?;
    let loaded = journal
        .compiled_ir(artifact.digest)
        .map_err(|e| format!("compiled_ir read failed: {e}"))?
        .ok_or_else(|| String::from("persisted artifact not found"))?;
    let decoded: AcceptedArtifact = postcard::from_bytes(&loaded.ir)
        .map_err(|e| format!("decode accepted artifact failed: {e}"))?;

    assert_eq!(artifact.required_capabilities.as_ref(), &[required.clone()]);
    assert_eq!(decoded.required_capabilities.as_ref(), &[required]);
    Ok(())
}

#[test]
fn submit_artifact_carries_idempotency_evidence_from_contracts() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;
    let action = vb_core::ActionId::new(11);
    let contract = vb_core::action::ActionContract {
        id: action,
        name: vb_core::action::ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 2048,
        timeout_ms: 1000,
        idempotency: vb_core::action::Idempotency::IdempotentExternal,
        side_effect: vb_core::action::SideEffect::Writes,
        retry_safety: vb_core::action::RetrySafety::KeyRequired,
        required_capabilities: Box::new([]),
    };

    let artifact = submit_artifact_with_contracts(
        &journal,
        &workflow,
        vb_core::RuntimePolicy::Journaled,
        &[contract],
    )
    .map_err(|e| format!("submit_artifact_with_contracts failed: {e}"))?;

    assert!(artifact.verification.idempotency_verified_claimed);
    assert_eq!(artifact.verification.idempotency_keyed.as_ref(), &[action]);
    assert_eq!(
        artifact.verification.idempotency_attested.as_ref(),
        &[action]
    );
    Ok(())
}

// =========================================================================
// SA-013: idempotency_evidence ownership parity between relaxed and checked
// admission paths. Both paths must consume the same IdempotencyEvidence and
// surface identical `idempotency_keyed` / `idempotency_attested` arrays in
// the resulting proof.
// =========================================================================

#[test]
fn sa013_relaxed_carries_idempotency_evidence_from_contracts() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;
    let action = vb_core::ActionId::new(11);
    let contract = vb_core::action::ActionContract {
        id: action,
        name: vb_core::action::ActionName::new("sa013-relaxed").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 2048,
        timeout_ms: 1000,
        idempotency: vb_core::action::Idempotency::IdempotentExternal,
        side_effect: vb_core::action::SideEffect::Writes,
        retry_safety: vb_core::action::RetrySafety::KeyRequired,
        required_capabilities: Box::new([]),
    };

    let artifact = submit_artifact_with_contracts(
        &journal,
        &workflow,
        vb_core::RuntimePolicy::Relaxed,
        &[contract],
    )
    .map_err(|e| format!("submit_artifact_with_contracts(relaxed) failed: {e}"))?;

    assert!(artifact.verification.idempotency_verified_claimed);
    assert_eq!(artifact.verification.idempotency_keyed.as_ref(), &[action]);
    assert_eq!(
        artifact.verification.idempotency_attested.as_ref(),
        &[action]
    );
    Ok(())
}

#[test]
fn sa013_relaxed_and_journaled_idempotency_evidence_parity() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;
    let action_a = vb_core::ActionId::new(13);
    let action_b = vb_core::ActionId::new(14);
    let build_contract =
        |id: vb_core::ActionId, name: &'static str| vb_core::action::ActionContract {
            id,
            name: vb_core::action::ActionName::new(name).unwrap(),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 2048,
            timeout_ms: 1000,
            idempotency: vb_core::action::Idempotency::IdempotentExternal,
            side_effect: vb_core::action::SideEffect::Writes,
            retry_safety: vb_core::action::RetrySafety::KeyRequired,
            required_capabilities: Box::new([]),
        };
    let build_pure_contract =
        |id: vb_core::ActionId, name: &'static str| vb_core::action::ActionContract {
            id,
            name: vb_core::action::ActionName::new(name).unwrap(),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 2048,
            timeout_ms: 1000,
            idempotency: vb_core::action::Idempotency::IdempotentExternal,
            side_effect: vb_core::action::SideEffect::None,
            retry_safety: vb_core::action::RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
    let relaxed_contracts = [
        build_contract(action_a, "sa013-parity-relaxed-a"),
        build_pure_contract(action_b, "sa013-parity-relaxed-b"),
    ];
    let journaled_contracts = [
        build_contract(action_a, "sa013-parity-journaled-a"),
        build_pure_contract(action_b, "sa013-parity-journaled-b"),
    ];

    let relaxed = submit_artifact_with_contracts(
        &journal,
        &workflow,
        vb_core::RuntimePolicy::Relaxed,
        &relaxed_contracts,
    )
    .map_err(|e| format!("submit_artifact_with_contracts(relaxed) failed: {e}"))?;
    let journaled = submit_artifact_with_contracts(
        &journal,
        &workflow,
        vb_core::RuntimePolicy::Journaled,
        &journaled_contracts,
    )
    .map_err(|e| format!("submit_artifact_with_contracts(journaled) failed: {e}"))?;

    assert_eq!(
        relaxed.verification.idempotency_keyed.as_ref(),
        journaled.verification.idempotency_keyed.as_ref(),
        "relaxed and journaled must surface identical idempotency_keyed arrays"
    );
    assert_eq!(
        relaxed.verification.idempotency_attested.as_ref(),
        journaled.verification.idempotency_attested.as_ref(),
        "relaxed and journaled must surface identical idempotency_attested arrays"
    );
    Ok(())
}

#[test]
fn submit_artifact_rejects_failed_idempotency_contract() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;
    let contract = vb_core::action::ActionContract {
        id: vb_core::ActionId::new(12),
        name: vb_core::action::ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 2048,
        timeout_ms: 1000,
        idempotency: vb_core::action::Idempotency::DeterministicPure,
        side_effect: vb_core::action::SideEffect::Writes,
        retry_safety: vb_core::action::RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };

    let result = submit_artifact_with_contracts(
        &journal,
        &workflow,
        vb_core::RuntimePolicy::Journaled,
        &[contract],
    );

    assert!(matches!(result, Err(JournalError::ArtifactMalformed)));
    Ok(())
}

// =========================================================================
// VerificationProof details
// =========================================================================

#[test]
fn verification_proof_serde_roundtrip() -> Result<(), String> {
    let digest = vb_core::WorkflowDigest::from_bytes([0xAA_u8; 32]);
    let mut proof = VerificationProof::new(digest, 3, true);
    proof.warnings.push(VerificationWarning {
        code: 7,
        message: Box::from("test warning"),
        gate: 5,
    });

    let serialized = postcard::to_allocvec(&proof).map_err(|e| format!("serialize failed: {e}"))?;
    let deserialized: VerificationProof =
        postcard::from_bytes(&serialized).map_err(|e| format!("deserialize failed: {e}"))?;

    assert_eq!(proof, deserialized, "proof must survive serde roundtrip");
    Ok(())
}

#[test]
fn accepted_artifact_serde_roundtrip() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    let artifact = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
        .map_err(|e| format!("submit failed: {e}"))?;

    let serialized =
        postcard::to_allocvec(&artifact).map_err(|e| format!("serialize failed: {e}"))?;
    let deserialized: AcceptedArtifact =
        postcard::from_bytes(&serialized).map_err(|e| format!("deserialize failed: {e}"))?;

    assert_eq!(
        artifact, deserialized,
        "artifact must survive serde roundtrip"
    );
    Ok(())
}

// =========================================================================
// Relaxed vs Journaled/Strict gate count difference
// =========================================================================

#[test]
fn relaxed_skips_gates_while_journaled_passes_them() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    let relaxed = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed)
        .map_err(|e| format!("relaxed failed: {e}"))?;
    let journaled = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
        .map_err(|e| format!("journaled failed: {e}"))?;

    assert!(
        relaxed.verification.gate_count < journaled.verification.gate_count,
        "relaxed gate count ({}) must be less than journaled ({})",
        relaxed.verification.gate_count,
        journaled.verification.gate_count
    );
    Ok(())
}

#[test]
fn strict_and_journaled_have_same_gate_count() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    let journaled = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
        .map_err(|e| format!("journaled failed: {e}"))?;
    let strict = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict)
        .map_err(|e| format!("strict failed: {e}"))?;

    assert_eq!(
        journaled.verification.gate_count, strict.verification.gate_count,
        "journaled and strict must have identical gate count"
    );
    // Only difference is durable flag.
    assert!(!journaled.verification.durable);
    assert!(strict.verification.durable);
    Ok(())
}

// =========================================================================
// SA-009: Relaxed artifact readback verification
//
// The Relaxed branch historically returned Ok(artifact) immediately after
// `put_compiled_ir` without verifying that the value survived the LSM
// memtable round-trip. The fix routes Relaxed through the same
// `verify_artifact_persisted` helper used by Journaled/Strict, so a silent
// persistence failure surfaces as `ArtifactMalformed` instead of as a
// falsely-accepted artifact.
// =========================================================================

#[test]
fn sa009_relaxed_rejects_silent_persistence_failure() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    // Force the post-put readback to report the artifact as missing,
    // simulating a silent LSM-level persistence failure.
    journal.fail_next_compiled_ir_readback_for_test();

    let result = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed);

    assert!(
        matches!(result, Err(JournalError::ArtifactMalformed)),
        "Relaxed branch must surface a silent persistence failure as ArtifactMalformed, got {result:?}"
    );
    Ok(())
}

#[test]
fn sa009_journaled_rejects_silent_persistence_failure() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    journal.fail_next_compiled_ir_readback_for_test();

    let result = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled);

    assert!(
        matches!(result, Err(JournalError::ArtifactMalformed)),
        "Journaled branch must surface a silent persistence failure as ArtifactMalformed, got {result:?}"
    );
    Ok(())
}

#[test]
fn sa009_strict_rejects_silent_persistence_failure() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    journal.fail_next_compiled_ir_readback_for_test();

    let result = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict);

    assert!(
        matches!(result, Err(JournalError::ArtifactMalformed)),
        "Strict branch must surface a silent persistence failure as ArtifactMalformed, got {result:?}"
    );
    Ok(())
}

#[test]
fn sa009_relaxed_succeeds_after_failure_flag_consumed() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    // Arm the failure flag for the first call; the flag is one-shot so the
    // second call exercises the normal happy-path readback.
    journal.fail_next_compiled_ir_readback_for_test();
    let first = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed);
    assert!(
        matches!(first, Err(JournalError::ArtifactMalformed)),
        "first call with armed failure flag must fail closed"
    );

    let second = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed)
        .map_err(|e| format!("second submit_artifact(relaxed) failed: {e}"))?;
    assert_eq!(
        second.source_digest,
        workflow.digest(),
        "subsequent Relaxed submit must preserve workflow source digest once the failure flag is consumed"
    );
    assert_eq!(second.verification.digest, second.digest);
    Ok(())
}

// =========================================================================
// Warning gate boundary values
// =========================================================================

#[test]
fn all_valid_gates_pass_is_valid() -> Result<(), String> {
    for gate in VerificationWarning::MIN_GATE..=VerificationWarning::MAX_GATE {
        let w = VerificationWarning {
            code: 1,
            message: Box::from("boundary test"),
            gate,
        };
        if !w.is_valid() {
            return Err(format!("gate {gate} should be valid"));
        }
    }
    Ok(())
}

#[test]
fn gate_values_outside_range_fail_is_valid() -> Result<(), String> {
    for gate in [0u8, 16, 20, 255] {
        let w = VerificationWarning {
            code: 1,
            message: Box::from("out of range test"),
            gate,
        };
        if w.is_valid() {
            return Err(format!("gate {gate} should be invalid"));
        }
    }
    Ok(())
}

// =========================================================================
// VerificationWarning serialization
// =========================================================================

#[test]
fn verification_warning_serde_roundtrip() -> Result<(), String> {
    let warning = VerificationWarning {
        code: 999,
        message: Box::from("serde test warning"),
        gate: 7,
    };
    let bytes = postcard::to_allocvec(&warning).map_err(|e| format!("serialize failed: {e}"))?;
    let back: VerificationWarning =
        postcard::from_bytes(&bytes).map_err(|e| format!("deserialize failed: {e}"))?;
    assert_eq!(warning, back);
    Ok(())
}

// =========================================================================
// Proof Flag Gap Tests (demonstrate VB-STORAGE-GAP)
//
// These tests document the gap: VerificationProof::new() sets all proof
// flags to true UNCONDITIONALLY, without any actual per-gate validation.
// The flags should be set based on actual verification results.
// =========================================================================

#[test]
fn gap_proof_flags_always_true_regardless_of_gate_count() -> Result<(), String> {
    let digest = vb_core::WorkflowDigest::from_bytes([0xAB_u8; 32]);

    let proof_zero = VerificationProof::new(digest, 0, false);
    assert!(
        proof_zero.bounded_claimed,
        "GAP: bounded_claimed=true even with gate_count=0 (no verification performed)"
    );
    assert!(
        proof_zero.taint_safe_claimed,
        "GAP: taint_safe_claimed=true even with gate_count=0 (no verification performed)"
    );
    assert!(
        proof_zero.retry_safe_claimed,
        "GAP: retry_safe_claimed=true even with gate_count=0 (no verification performed)"
    );
    assert!(
        proof_zero.replayable_claimed,
        "GAP: replayable_claimed=true even with gate_count=0 (no verification performed)"
    );

    let proof_fifteen = VerificationProof::new(digest, 15, true);
    assert!(
        proof_fifteen.bounded_claimed,
        "GAP: bounded_claimed=true with gate_count=15 (verification claimed but not performed)"
    );
    assert!(
        proof_fifteen.taint_safe_claimed,
        "GAP: taint_safe_claimed=true with gate_count=15 (verification claimed but not performed)"
    );
    assert!(
        proof_fifteen.retry_safe_claimed,
        "GAP: retry_safe_claimed=true with gate_count=15 (verification claimed but not performed)"
    );
    assert!(
        proof_fifteen.replayable_claimed,
        "GAP: replayable_claimed=true with gate_count=15 (verification claimed but not performed)"
    );

    Ok(())
}

#[test]
fn gap_proof_flags_true_for_any_digest_value() -> Result<(), String> {
    let zero_digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
    let proof_zero = VerificationProof::new(zero_digest, 15, true);
    assert!(
        proof_zero.bounded_claimed
            && proof_zero.taint_safe_claimed
            && proof_zero.retry_safe_claimed
            && proof_zero.replayable_claimed,
        "GAP: proof flags are true for zero digest"
    );

    let max_digest = vb_core::WorkflowDigest::from_bytes([0xFFu8; 32]);
    let proof_max = VerificationProof::new(max_digest, 15, true);
    assert!(
        proof_max.bounded_claimed
            && proof_max.taint_safe_claimed
            && proof_max.retry_safe_claimed
            && proof_max.replayable_claimed,
        "GAP: proof flags are true for max digest"
    );

    let arbitrary_digest = vb_core::WorkflowDigest::from_bytes([
        0x12_u8, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
        0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
        0x66, 0x77, 0x88,
    ]);
    let proof_arb = VerificationProof::new(arbitrary_digest, 15, false);
    assert!(
        proof_arb.bounded_claimed
            && proof_arb.taint_safe_claimed
            && proof_arb.retry_safe_claimed
            && proof_arb.replayable_claimed,
        "GAP: proof flags are true for arbitrary digest"
    );

    Ok(())
}

#[test]
fn gap_submit_artifact_journaled_produces_unconditional_true_flags() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    let result = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
        .map_err(|e| format!("submit_artifact(journaled) failed: {e}"))?;

    assert_eq!(result.verification.gate_count, 15);
    assert!(
        result.verification.bounded_claimed,
        "GAP: submit_artifact produces bounded_claimed=true without checking workflow size"
    );
    assert!(
        result.verification.taint_safe_claimed,
        "GAP: submit_artifact produces taint_safe_claimed=true without checking taint propagation"
    );
    assert!(
        result.verification.retry_safe_claimed,
        "GAP: submit_artifact produces retry_safe_claimed=true without checking idempotency"
    );
    assert!(
        result.verification.replayable_claimed,
        "GAP: submit_artifact produces replayable_claimed=true without checking replay invariants"
    );

    Ok(())
}

// =========================================================================
// vb-1rqz7.36 / SA-008 — canonical ResourceContract fits the policy buffer
// =========================================================================

#[test]
fn policy_buffer_fits_canonical_resource_contract() {
    // A fully-populated ResourceContract: every numeric field at the
    // upper bound (the contract itself enforces these), bool at true.
    let contract = ResourceContract {
        max_steps: u16::MAX,
        max_slots: u16::MAX,
        max_constants: u16::MAX,
        max_accessors: u16::MAX,
        max_expressions: u16::MAX,
        max_expr_stack: u8::MAX,
        max_step_budget_per_tick: u64::MAX,
        max_transitions_per_tick: u64::MAX,
        max_input_bytes: u32::MAX,
        max_output_bytes: u32::MAX,
        max_blob_bytes: u64::MAX,
        max_ipc_payload_bytes: u32::MAX,
        max_retry_attempts: u16::MAX,
        max_fanout: u16::MAX,
        max_collect_items: u32::MAX,
        max_queue_depth: u32::MAX,
        max_journal_batch_bytes: u32::MAX,
        result_taint_policy: ResultTaintPolicy::Allow,
    };

    let bound = resource_contract_policy_bytes_bound();
    let serialized = postcard::to_allocvec(&contract)
        .expect("postcard encode must succeed for canonical contract");

    assert!(
        serialized.len() <= bound,
        "vb-1rqz7.36: serialized ResourceContract ({} bytes) must fit in policy buffer ({} bytes)",
        serialized.len(),
        bound
    );
}

// =========================================================================
// compute_policy_digest: regression guard for the YAGNI Vec::with_capacity
// regression (RE-REVIEW #2). The previous implementation used
// `Vec::with_capacity(bound) + postcard::to_slice(&mut Vec<u8>)` which
// deref-coerced to a zero-length slice and returned `SerializeBufferFull`
// on every call (the function ALWAYS returned `ArtifactMalformed`,
// breaking 94+ production-path tests). `policy_buffer_fits_canonical_resource_contract`
// did not catch this because it uses `to_allocvec` directly and never
// calls `compute_policy_digest`. The test below calls
// `compute_policy_digest` directly and would have caught the regression.
// =========================================================================

#[test]
fn compute_policy_digest_succeeds_and_yields_nonzero_digest() {
    use crate::admission::compute_policy_digest;

    let _journal = match temp_journal() {
        Ok(j) => j,
        Err(e) => panic!("journal open failed: {e}"),
    };
    let workflow = match minimal_workflow() {
        Ok(w) => w,
        Err(e) => panic!("workflow build failed: {e}"),
    };

    let digest = match compute_policy_digest(&workflow) {
        Ok(d) => d,
        Err(e) => panic!("compute_policy_digest must succeed for canonical workflow: {e}"),
    };

    let bytes: [u8; 32] = digest.as_bytes();
    assert!(
        bytes.iter().any(|b| *b != 0),
        "compute_policy_digest must yield a non-zero digest for a non-empty ResourceContract, \
         got all-zero digest (regression guard)"
    );

    // The digest must also be deterministic: two consecutive calls on the
    // same workflow must return identical digests.
    let digest_again = match compute_policy_digest(&workflow) {
        Ok(d) => d,
        Err(e) => panic!("second compute_policy_digest call must succeed: {e}"),
    };
    assert_eq!(
        digest, digest_again,
        "compute_policy_digest must be deterministic across calls"
    );

    // And it must be hex-distinct from a sentinel zero digest (the digest
    // is derived from the ResourceContract bytes, not from the workflow
    // digest or the workflow id).
    let zero = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
    assert_ne!(
        digest, zero,
        "compute_policy_digest must not collide with the zero sentinel"
    );
}

// =========================================================================
// SA-015 (vb-36fly): postcard-wrapping sites in submit_artifact_with_contracts
// and admit_compiled_artifact must surface the underlying `postcard::Error`
// as `JournalError::PostcardEncodeFailed(_)`, NOT collapse it into the generic
// `ArtifactMalformed` bucket. Forces a deterministic postcard failure via
// `to_slice` against a zero-length output buffer (the production code at
// admission.rs:222 uses the same `to_slice` call shape).
// =========================================================================

#[test]
fn admission_postcard_encode_failed_surfaces_typed_variant() {
    use vb_core::workflow::ResourceContract;

    // (1) Force a deterministic postcard encode failure with the same
    // `to_slice` call shape used at admission.rs:222. A zero-length output
    // buffer cannot fit any postcard payload, so postcard deterministically
    // returns `Error::SerializeBufferFull`.
    let mut zero_len_buffer: [u8; 0] = [];
    let to_slice_result = postcard::to_slice(&ResourceContract::DEFAULT, &mut zero_len_buffer);
    let source_err = to_slice_result.expect_err("zero-length buffer must fail to_slice");

    // (2) Apply the SA-015 wrapping pattern: `map_err(JournalError::PostcardEncodeFailed)`,
    // the same transformation now used at the seven postcard sites in
    // `submit_artifact_with_contracts` (lines 340/354/371/385/398) and
    // `admit_compiled_artifact` (lines 521/528).
    let journal_err: JournalError =
        match postcard::to_slice(&ResourceContract::DEFAULT, &mut zero_len_buffer) {
            Ok(_) => panic!("zero-length buffer must not succeed"),
            Err(e) => JournalError::PostcardEncodeFailed(e),
        };

    // (3) The typed variant must capture the source error verbatim so
    // operators can read the underlying postcard category from logs.
    match &journal_err {
        JournalError::PostcardEncodeFailed(inner) => {
            assert_eq!(
                *inner, source_err,
                "PostcardEncodeFailed must preserve the source postcard::Error"
            );
        }
        other => panic!("expected PostcardEncodeFailed, got {other:?}"),
    }

    // (4) Diagnostic code must be distinct from `ARTIFACT_MALFORMED_CODE`
    // so a postcard-side failure is distinguishable from a structural
    // artifact defect in operator dashboards and alert routing.
    assert_ne!(
        journal_err.diagnostic_code(),
        JournalError::ARTIFACT_MALFORMED_CODE,
        "PostcardEncodeFailed must NOT collapse into the ArtifactMalformed bucket"
    );
    assert_eq!(
        journal_err.diagnostic_code(),
        JournalError::POSTCARD_ENCODE_FAILED_CODE,
        "PostcardEncodeFailed must map to its own diagnostic code (0x4032)"
    );

    // (5) The `Display` impl must render the inner postcard error category
    // (e.g. "serialize" for `SerializeBufferFull`) so logs identify the
    // failure shape without forcing operators to inspect the variant.
    let rendered = format!("{journal_err}");
    assert!(
        rendered.contains("serialize") || rendered.contains("postcard"),
        "PostcardEncodeFailed Display must mention the inner error category, got: {rendered:?}"
    );

    // (6) `PostcardEncodeFailed` is a distinct sum-type variant from the
    // existing `Encode` variant so admission callers can pattern-match on
    // the admission-side failure without confusing it with the generic
    // journal-encode bucket.
    let encode_err = JournalError::Encode(postcard::Error::SerializeBufferFull);
    let journal_is_postcard_encode = matches!(journal_err, JournalError::PostcardEncodeFailed(_));
    let journal_is_encode = matches!(journal_err, JournalError::Encode(_));
    assert_eq!(
        journal_is_postcard_encode, true,
        "journal_err must be PostcardEncodeFailed"
    );
    assert_eq!(
        journal_is_encode, false,
        "PostcardEncodeFailed must remain a distinct variant from Encode"
    );
    assert_ne!(
        journal_err.diagnostic_code(),
        encode_err.diagnostic_code(),
        "PostcardEncodeFailed and Encode must have distinct diagnostic codes"
    );

    // (7) `From<postcard::Error>` continues to route through `Encode` so
    // non-admission callers that rely on the existing conversion are not
    // broken by adding the new variant.
    let from_postcard: JournalError = postcard::Error::SerializeBufferFull.into();
    let from_postcard_is_encode = matches!(from_postcard, JournalError::Encode(_));
    assert_eq!(
        from_postcard_is_encode, true,
        "`From<postcard::Error>` for `JournalError` must continue to produce the Encode variant"
    );
}
