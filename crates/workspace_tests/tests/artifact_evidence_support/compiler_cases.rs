use super::*;

#[test]
fn legacy_constructor_cannot_mint_verification_from_caller_claims() -> TestResult {
    for gates in [0, 15, u8::MAX] {
        for durable in [false, true] {
            let proof =
                VerificationProof::new(WorkflowDigest::from_bytes([42; 32]), gates, durable);
            assert_eq!((proof.gate_count, proof.durable), (0, false));
            assert_eq!(
                (
                    proof.bounded_claimed,
                    proof.taint_safe_claimed,
                    proof.retry_safe_claimed,
                    proof.idempotency_verified_claimed,
                    proof.replayable_claimed
                ),
                (false, false, false, false, false)
            );
            assert_eq!(proof.evidence, ArtifactEvidence::Unverified);
        }
    }
    Ok(())
}

#[test]
fn compiler_candidate_binds_actual_source_and_exact_payload_with_named_checks() -> TestResult {
    let candidate = vb_compile::compile_verified_artifact(SOURCE, &[])?;
    assert_eq!(
        candidate.source_provenance(),
        SourceProvenance::CompiledSourceSpec(WorkflowDigest::from_bytes(
            blake3::hash(SOURCE).into()
        ))
    );
    assert_eq!(
        candidate.payload_digest(),
        WorkflowDigest::from_bytes(blake3::hash(candidate.payload()).into())
    );
    assert_ne!(candidate.payload_digest(), candidate.workflow().digest());
    assert_eq!(
        candidate.checks(),
        [
            CompilerCheck::WorkflowReconstruction,
            CompilerCheck::SharedIrGatesWithContracts,
            CompilerCheck::IdempotencyDeclarations,
            CompilerCheck::CanonicalWorkflowDigest,
            CompilerCheck::WholeWorkflowBudget
        ]
    );
    assert_eq!(candidate.budget().max_total_steps, 2);
    assert_eq!(candidate.contracts(), &[]);
    let parts: vb_core::WorkflowParts = postcard::from_bytes(candidate.payload())?;
    assert_eq!(parts, candidate.workflow().to_parts());
    Ok(())
}

#[test]
fn source_only_comment_changes_provenance_while_ir_identity_is_stable() -> TestResult {
    let mut source = SOURCE.to_vec();
    source.extend_from_slice(b"# provenance distinction\n");
    let first = vb_compile::compile_verified_artifact(SOURCE, &[])?;
    let second = vb_compile::compile_verified_artifact(&source, &[])?;
    assert_eq!(first.payload(), second.payload());
    assert_ne!(first.source_provenance(), second.source_provenance());
    assert_eq!(
        vb_compile::verify_compiled_artifact(first.workflow(), &[])?.source_provenance(),
        SourceProvenance::CompiledIrOnly
    );
    Ok(())
}

#[test]
fn compiler_rejects_stale_workflow_identity() -> TestResult {
    let mut parts = workflow()?.to_parts();
    parts.digest = WorkflowDigest::from_bytes([8; 32]);
    let stale = vb_core::CompiledWorkflow::try_from_parts(parts)?;
    match vb_compile::verify_compiled_artifact(&stale, &[]) {
        Err(ArtifactVerificationError::DigestMismatch {
            binding,
            claimed,
            computed,
        }) => {
            assert_eq!(binding, CompilationBinding::Workflow);
            assert_eq!(claimed, stale.digest());
            assert_ne!(claimed, computed);
        }
        other => return Err(format!("expected exact stale workflow error, got {other:?}").into()),
    }
    Ok(())
}
