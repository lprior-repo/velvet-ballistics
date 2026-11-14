/// Test store that provides a real executor catalog for Packed verification.
struct CatalogProvidingStore {
    catalog: Option<Arc<vb_storage::executor::ExecutorCatalog>>,
    artifact: Option<AcceptedArtifact>,
}

impl AcceptedArtifactStore for CatalogProvidingStore {
    fn load_accepted_artifact(
        &self,
        digest: vb_core::ids::WorkflowDigest,
    ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
        self.artifact
            .clone()
            .filter(|a| a.digest == digest)
            .ok_or(ArtifactEnvelopeError::ArtifactNotFound { digest })
    }

    fn executor_catalog(&self) -> Option<&vb_storage::executor::ExecutorCatalog> {
        self.catalog.as_deref()
    }
}

#[test]
fn present_catalog_with_tampered_pack_fails_with_concrete_executor_error() -> Result<(), String> {
    let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let journal =
        vb_storage::FjallJournal::open(tmp.path(), Some(vb_storage::FjallConfig::default()))
            .map_err(|e| e.to_string())?;

    let catalog = vb_storage::executor::ExecutorCatalog::run_registered_conformance()
        .map_err(|e| e.to_string())?;

    // Create a workflow with an action contract using matching IDs
    let action = ActionId::new(0);
    let cap = Capability::new("velvet.register.write".into(), action);
    let contract = vb_core::action::ActionContract {
        id: action,
        name: ActionName::new("recovery-packed").map_err(|e| e.to_string())?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([cap]),
    };

    let scope = vb_core::action_pack::CapabilityScopeSpec {
        namespace: 15,
        key: 29,
    };
    let pack = catalog
        .register_pack(
            &contract,
            scope,
            vb_core::action_pack::RecoveryClass::ReconcileReceipt,
        )
        .map_err(|e| e.to_string())?;

    let contracts = Box::new([contract]);

    // Build workflow with action0 using accessible test helper
    let cap = Capability::new("velvet.register.write".into(), ActionId::new(0));
    let artifact_with_caps =
        crate::admission::tests::evidence_test_support::accepted_artifact_with_caps(Box::new([
            cap,
        ]))
        .map_err(|e| e.to_string())?;
    let workflow = vb_storage::admission::verify_accepted_artifact(&artifact_with_caps)
        .map_err(|e| e.to_string())?;

    // Submit as Packed with genuine catalog and pack
    let artifact = catalog
        .submit_artifact(
            &journal,
            &workflow,
            RuntimePolicy::Strict,
            contracts.as_slice(),
            &[pack],
        )
        .map_err(|e| e.to_string())?;

    // Verification should succeed with correct pack
    let store = CatalogProvidingStore {
        catalog: Some(Arc::new(catalog.clone())),
        artifact: Some(artifact.clone()),
    };
    let result = verify_stored_artifact_detailed(
        &store,
        store
            .artifact
            .as_ref()
            .ok_or("expected artifact in store")?,
        Some(RuntimePolicy::Strict),
    );
    assert!(matches!(result, Ok(_)));

    // Now tamper: replace with a pack for a different action
    let other_action = ActionId::new(99);
    let other_cap = Capability::new("velvet.register.write".into(), other_action);
    let other_contract = vb_core::action::ActionContract {
        id: other_action,
        name: ActionName::new("other-action").map_err(|e| e.to_string())?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([other_cap]),
    };
    let other_pack = catalog
        .register_pack(
            &other_contract,
            scope,
            vb_core::action_pack::RecoveryClass::ReconcileReceipt,
        )
        .map_err(|e| e.to_string())?;

    let ArtifactEvidence::Packed(packed) = artifact.verification.evidence.clone() else {
        return Err(String::from("expected packed evidence"));
    };
    let mut tampered = artifact;
    tampered.verification.evidence =
        ArtifactEvidence::Packed(Box::new(vb_core::action_pack::PackedArtifactEvidence {
            base: packed.base.clone(),
            packs: Box::new([other_pack]),
        }));
    // Recompute artifact digest so check_envelope passes and reaches check_packs
    tampered.digest =
        vb_storage::admission::accepted_artifact_digest(&tampered).map_err(|e| e.to_string())?;
    tampered.verification.digest = tampered.digest;

    let store2 = CatalogProvidingStore {
        catalog: Some(Arc::new(catalog)),
        artifact: Some(tampered),
    };
    let result2 = verify_stored_artifact_detailed(
        &store2,
        store2
            .artifact
            .as_ref()
            .ok_or("expected artifact in store")?,
        Some(RuntimePolicy::Strict),
    );
    // Must fail with exact PackMismatch executor error (unit variant, no action field)
    match result2 {
        Err(StoredArtifactVerificationError::Executor(
            vb_storage::executor::ExecutorError::PackMismatch,
        )) => {
            // Correct: exact pack mismatch
        }
        other => {
            return Err(format!("expected exact PackMismatch, got {:?}", other));
        }
    }
    Ok(())
}
