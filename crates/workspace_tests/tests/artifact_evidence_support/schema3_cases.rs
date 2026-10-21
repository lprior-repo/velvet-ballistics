use super::*;

#[test]
fn real_schema2_artifact_requires_migration_before_positional_ir_decode() -> TestResult {
    let bytes = include_bytes!("../../../../.beads/vb-9agmk/schema2-fixture/artifact.postcard");
    let artifact: vb_storage::AcceptedArtifact = postcard::from_bytes(bytes)?;
    let old_ir =
        include_bytes!("../../../../.beads/vb-9agmk/schema2-fixture/workflow-parts.postcard");
    assert_eq!(artifact.ir.as_slice(), old_ir);
    // These actual old bytes have no appended semantic_source_identity field.
    assert!(postcard::from_bytes::<vb_core::WorkflowParts>(old_ir).is_err());
    assert!(matches!(
        verify_accepted_artifact(&artifact),
        Err(JournalError::MigrationRequired { from: 2, to: 3 })
    ));
    assert!(matches!(
        verify_accepted_artifact_for_policy(&artifact, RuntimePolicy::Strict),
        Err(JournalError::MigrationRequired { from: 2, to: 3 })
    ));
    Ok(())
}

#[test]
fn schema3_roundtrip_preserves_source_identity_and_explicit_ir_only_none() -> TestResult {
    let compiled = workflow()?;
    let source_identity = compiled
        .to_parts()
        .semantic_source_identity
        .ok_or("source identity missing")?;
    let artifact = prepare_compiled_artifact(&compiled, RuntimePolicy::Strict, &[])?;
    let ArtifactEvidence::Verified(evidence) = &artifact.verification.evidence else {
        return Err("missing evidence".into());
    };
    assert_eq!(evidence.schema_version, 3);
    let restored = verify_accepted_artifact(&artifact)?;
    assert_eq!(
        restored.to_parts().semantic_source_identity,
        Some(source_identity)
    );
    assert_eq!(restored.digest(), compiled.digest());
    assert_eq!(postcard::to_allocvec(&restored.to_parts())?, artifact.ir);
    let mut parts = compiled.to_parts();
    parts.semantic_source_identity = None;
    let ir_only = canonicalize(parts)?;
    assert_ne!(ir_only.digest(), compiled.digest());
    let artifact = prepare_compiled_artifact(&ir_only, RuntimePolicy::Strict, &[])?;
    assert_eq!(
        verify_accepted_artifact(&artifact)?
            .to_parts()
            .semantic_source_identity,
        None
    );
    assert_eq!(verify_accepted_artifact(&artifact)?, ir_only);
    Ok(())
}

#[test]
fn schema3_rejects_retained_digest_after_semantic_source_identity_change() -> TestResult {
    let mut artifact = prepare_compiled_artifact(&workflow()?, RuntimePolicy::Strict, &[])?;
    let mut parts: vb_core::WorkflowParts = postcard::from_bytes(&artifact.ir)?;
    parts.semantic_source_identity = Some(WorkflowDigest::from_bytes([77; 32]));
    artifact.ir = postcard::to_allocvec(&parts)?;
    rebind(&mut artifact)?;
    assert!(matches!(
        verify_accepted_artifact(&artifact),
        Err(JournalError::ArtifactChecksumMismatch)
    ));
    Ok(())
}
