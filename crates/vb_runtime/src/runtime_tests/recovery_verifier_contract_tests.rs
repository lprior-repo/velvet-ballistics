// Recovery verifier contract tests: strict runtime rejects journaled artifact
// at the detailed verifier boundary, producing exact JournalError text in the
// RuntimeError::Recovery payload. Also validates positive control and genuine
// Packed artifact with present executor catalog.
//
// File is include!d into runtime::tests, which provides its own imports.
// This module is wrapped in a private child module to avoid import conflicts
// and to inherit parent imports via use super::*.

mod recovery_verifier_contract_tests {
    use super::*;
    use crate::admission::StorageArtifactStore;
    use crate::admission::{verify_stored_artifact, verify_stored_artifact_detailed};
    use crate::journal::StorageRuntimeJournal;
    use crate::runtime::RecoveryArtifactWorkflow;
    use vb_core::Capability;
    use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
    use vb_core::action_pack::{CapabilityScopeSpec, RecoveryClass};
    use vb_storage::admission::{AcceptedArtifact, submit_artifact_with_contracts};

    /// Build a minimal honest-digest journaled artifact using the accessible
    /// finished_workflow() parent fixture.
    fn build_honest_journaled_artifact(
        journal: &Arc<vb_storage::FjallJournal>,
    ) -> Result<AcceptedArtifact, Box<dyn std::error::Error>> {
        // Use accessible parent fixture
        let workflow = finished_workflow().ok_or("failed to build finished workflow fixture")?;
        // Compute honest digest
        let mut parts = workflow.to_parts();
        parts.digest = vb_core::WorkflowDigest::from_bytes([0; 32]);
        let canonical = postcard::to_allocvec(&parts).map_err(|e| format!("serialize: {}", e))?;
        parts.digest = vb_core::WorkflowDigest::from_bytes(blake3::hash(&canonical).into());
        let honest = vb_core::CompiledWorkflow::try_from_parts(parts)
            .map_err(|e| format!("recompile: {}", e))?;

        Ok(
            submit_artifact_with_contracts(journal, &honest, RuntimePolicy::Journaled, &[])
                .map_err(|e| format!("submit: {}", e))?,
        )
    }

    /// Build a strict honest-digest artifact for the packed test.
    /// Uses a genuine Do0 workflow (not action-free) so the contract can
    /// resolve at execution time.
    fn build_honest_strict_artifact(
        journal: &Arc<vb_storage::FjallJournal>,
        catalog: &vb_storage::executor::ExecutorCatalog,
    ) -> Result<AcceptedArtifact, Box<dyn std::error::Error>> {
        // Use genuine Do7 workflow fixture, then mutate to Do0
        let wf = action_then_finish_workflow()
            .ok_or("failed to build action_then_finish workflow fixture")?;
        let mut parts = wf.to_parts();

        // Mutate the first node from Do7 to Do0 via checked match
        let first_node = parts
            .nodes
            .first_mut()
            .ok_or("expected at least one node in workflow")?;
        if let CompiledNodeKind::Do { action, .. } = &mut first_node.kind {
            *action = ActionId::new(0);
        } else {
            return Err("expected Do node at entry".into());
        }

        // Honest-digest normalization
        parts.digest = vb_core::WorkflowDigest::from_bytes([0; 32]);
        let canonical = postcard::to_allocvec(&parts).map_err(|e| format!("serialize: {}", e))?;
        parts.digest = vb_core::WorkflowDigest::from_bytes(blake3::hash(&canonical).into());
        let honest = vb_core::CompiledWorkflow::try_from_parts(parts)
            .map_err(|e| format!("recompile: {}", e))?;

        let action = ActionId::new(0);
        let cap = Capability::new("velvet.register.write".into(), action);
        let contract = ActionContract {
            id: action,
            name: ActionName::new("recovery-packed")?,
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

        let scope = CapabilityScopeSpec {
            namespace: 15,
            key: 29,
        };
        let pack = catalog
            .register_pack(&contract, scope, RecoveryClass::ReconcileReceipt)
            .map_err(|e| format!("register_pack: {}", e))?;

        let contracts = Box::new([contract]);
        Ok(catalog
            .submit_artifact(
                journal,
                &honest,
                RuntimePolicy::Strict,
                contracts.as_slice(),
                &[pack],
            )
            .map_err(|e| format!("submit_artifact: {}", e))?)
    }

    /// Strict runtime rejects Journaled artifact at the detailed verifier
    /// boundary. Policy mismatch produces exact JournalError::ArtifactMalformed
    /// text in the RuntimeError::Recovery payload, not the coarse wrapper.
    #[test]
    fn strict_runtime_rejects_journaled_artifact_at_detailed_verifier()
    -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let journal = Arc::new(vb_storage::FjallJournal::open(
            tmp.path(),
            Some(vb_storage::FjallConfig::default()),
        )?);

        let artifact = build_honest_journaled_artifact(&journal)?;

        // Verify journal has the artifact before runtime load
        let pre_stored = journal
            .compiled_ir(artifact.digest)?
            .ok_or("artifact not found in journal before runtime load")?;
        assert_eq!(pre_stored.digest, artifact.digest);

        // Verify unused RunId has empty prefix (no events) using events_for_run_full
        let unused_run = RunId::new(999999);
        let pre_events = journal.events_for_run_full(unused_run)?;
        assert!(
            pre_events.is_empty(),
            "unused RunId should have no events before load"
        );

        let runtime = Runtime::new(
            NonZeroUsize::MIN,
            ShardConfig {
                command_queue_capacity: 16,
                trace_capacity: 16,
                step_budget_per_tick: 4,
                max_active_runs: 4,
                max_open_gates: 1024,
                policy: RuntimePolicy::Strict,
            },
            StorageRuntimeJournal::shared_strict(Arc::clone(&journal)),
        );

        // Capture pre-state
        let pre_counters = runtime.counters_snapshot();
        let pre_pending = runtime.pending_boundary_snapshot(8);

        let result =
            runtime.load_recovery_artifact_and_workflow(Arc::clone(&journal), artifact.digest);

        assert!(matches!(
            result,
            Err(RuntimeError::Recovery { error })
                if error == vb_storage::JournalError::ArtifactMalformed.to_string()
        ));

        // Verify no mutation
        let post_counters = runtime.counters_snapshot();
        let post_pending = runtime.pending_boundary_snapshot(8);
        assert_eq!(pre_counters.runs_submitted, post_counters.runs_submitted);
        assert_eq!(pre_counters.runs_completed, post_counters.runs_completed);
        assert_eq!(pre_counters.runs_failed, post_counters.runs_failed);
        assert_eq!(pre_counters.steps_executed, post_counters.steps_executed);
        assert_eq!(pre_pending, post_pending);

        // Verify journal state unchanged: artifact still there, unused RunId still empty
        let post_stored = journal
            .compiled_ir(artifact.digest)?
            .ok_or("artifact missing from journal after failed load")?;
        // CompiledIrRecord derives PartialEq: compare full record (digest + ir bytes)
        assert_eq!(
            pre_stored, post_stored,
            "journal compiled_ir record must be unchanged by failed load"
        );
        let post_events = journal.events_for_run_full(unused_run)?;
        assert_eq!(
            pre_events, post_events,
            "unused RunId event list must be unchanged by failed load"
        );
        // Verify no live run installation
        assert!(matches!(
            runtime.inspect_run_state(unused_run),
            Err(crate::RuntimeError::RunNotFound)
        ));
        Ok(())
    }

    /// Positive control: matching Journaled policies load successfully.
    /// No RuntimeError::Recovery produced when artifact and runtime policies agree.
    #[test]
    fn journaled_runtime_loads_journaled_artifact_at_detailed_verifier()
    -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let journal = Arc::new(vb_storage::FjallJournal::open(
            tmp.path(),
            Some(vb_storage::FjallConfig::default()),
        )?);

        let artifact = build_honest_journaled_artifact(&journal)?;

        // Verify journal has the artifact before runtime load
        let pre_stored = journal
            .compiled_ir(artifact.digest)?
            .ok_or("artifact not found in journal before runtime load")?;
        assert_eq!(pre_stored.digest, artifact.digest);

        // Verify unused RunId has empty prefix (no events) using events_for_run_full
        let unused_run = RunId::new(999998);
        let pre_events = journal.events_for_run_full(unused_run)?;
        assert!(
            pre_events.is_empty(),
            "unused RunId should have no events before load"
        );

        // Derive expected full workflow independently via admission verifier
        let expected_workflow = vb_storage::admission::verify_accepted_artifact(&artifact)?;

        let runtime = Runtime::new(
            NonZeroUsize::MIN,
            ShardConfig {
                command_queue_capacity: 16,
                trace_capacity: 16,
                step_budget_per_tick: 4,
                max_active_runs: 4,
                max_open_gates: 1024,
                policy: RuntimePolicy::Journaled,
            },
            StorageRuntimeJournal::shared_journaled(Arc::clone(&journal)),
        );

        // Capture pre-state
        let pre_counters = runtime.counters_snapshot();
        let pre_pending = runtime.pending_boundary_snapshot(8);

        let result =
            runtime.load_recovery_artifact_and_workflow(Arc::clone(&journal), artifact.digest);
        let RecoveryArtifactWorkflow {
            artifact: loaded_artifact,
            workflow: loaded_workflow,
        } = result?;
        // Compare full loaded artifact against original artifact
        assert_eq!(
            loaded_artifact, artifact,
            "loaded artifact must match submitted artifact"
        );
        // Compare loaded workflow against independently verified expected workflow
        assert_eq!(
            loaded_workflow, expected_workflow,
            "loaded workflow must match independently verified expected workflow"
        );
        // Verify journal state unchanged by load
        let post_counters = runtime.counters_snapshot();
        let post_pending = runtime.pending_boundary_snapshot(8);
        assert_eq!(pre_counters.runs_submitted, post_counters.runs_submitted);
        assert_eq!(pre_counters.runs_completed, post_counters.runs_completed);
        assert_eq!(pre_counters.runs_failed, post_counters.runs_failed);
        assert_eq!(pre_counters.steps_executed, post_counters.steps_executed);
        assert_eq!(pre_pending, post_pending);

        // Verify journal state unchanged: artifact still there, unused RunId still empty
        let post_stored = journal
            .compiled_ir(artifact.digest)?
            .ok_or("artifact missing from journal after successful load")?;
        // CompiledIrRecord derives PartialEq: compare full record (digest + ir bytes)
        assert_eq!(
            pre_stored, post_stored,
            "journal compiled_ir record must be unchanged by load"
        );
        let post_events = journal.events_for_run_full(unused_run)?;
        assert_eq!(
            pre_events, post_events,
            "unused RunId event list must be unchanged by load"
        );
        // Verify no live run installation
        assert!(matches!(
            runtime.inspect_run_state(unused_run),
            Err(crate::RuntimeError::RunNotFound)
        ));
        Ok(())
    }

    /// Genuine Packed artifact with present executor catalog passes through
    /// detailed verifier. Uses actual catalog construction and pack registration
    /// APIs, not fake field assignments.
    #[test]
    fn present_catalog_valid_packed_artifact_passes_detailed_verifier()
    -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let journal = Arc::new(vb_storage::FjallJournal::open(
            tmp.path(),
            Some(vb_storage::FjallConfig::default()),
        )?);

        let catalog = vb_storage::executor::ExecutorCatalog::run_registered_conformance()?;
        let artifact = build_honest_strict_artifact(&journal, &catalog)?;
        let store =
            StorageArtifactStore::with_catalog(Arc::clone(&journal), Some(Arc::new(catalog)));

        // Verify through both coarse and detailed paths with the store
        let public = verify_stored_artifact(&store, &artifact, Some(RuntimePolicy::Strict));
        let detailed =
            verify_stored_artifact_detailed(&store, &artifact, Some(RuntimePolicy::Strict));
        let public_workflow = public?;
        let detailed_workflow = detailed?;
        // Full identity: both must return identical workflows
        assert_eq!(
            public_workflow, detailed_workflow,
            "coarse and detailed verification must yield identical workflows"
        );
        // CompiledWorkflow::digest() corresponds to AcceptedArtifact::source_digest
        assert_eq!(
            public_workflow.digest(),
            artifact.source_digest,
            "coarse verification digest must match artifact source_digest"
        );
        assert_eq!(
            detailed_workflow.digest(),
            artifact.source_digest,
            "detailed verification digest must match artifact source_digest"
        );
        Ok(())
    }
}
