fn submit_pair_fixture(shard: &mut Shard, run: RunId) -> crate::RuntimeResult<()> {
    let workflow = suspended_workflow().ok_or(RuntimeError::InvalidActionCompletion)?;
    shard.handle_submit_with_inputs_and_contracts(
        run,
        workflow,
        &[(SlotIdx::ZERO, SlotValue::I64(0))],
        crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        &crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0)),
    )
}

/// Contract for the suspended workflow's single Do action with a
/// verifier-valid capability name. The shared `contracts_through`
/// fixture names its capability `__contract_required__`, which the
/// artifact verifier's capability-name grammar rejects, so a Strict
/// submit needs its own contract.
fn strict_submit_contracts() -> Box<[vb_core::action::ActionContract]> {
    Box::new([vb_core::action::ActionContract {
        id: ActionId::new(0),
        name: vb_core::action::ActionName::from_static_infallible("test-action"),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: vb_core::action::Idempotency::DeterministicPure,
        side_effect: vb_core::action::SideEffect::None,
        retry_safety: vb_core::action::RetrySafety::Safe,
        required_capabilities: Box::new([vb_core::capability::Capability::new(
            "test.action".into(),
            ActionId::new(0),
        )]),
    }])
}

fn strict_submit_caps() -> vb_core::capability::CapabilitySet {
    vb_core::capability::CapabilitySet::from_grants(Box::new([
        vb_core::capability::Capability::new("test.action".into(), ActionId::new(0)),
    ]))
}

/// Replaces a fixture workflow's placeholder digest with the BLAKE3
/// digest of its zeroed-digest serialization, so artifact preparation
/// accepts it. The suspended-workflow fixture hardcodes a `[1; 32]`
/// digest that does not match its contents.
fn honest_digest_workflow(
    workflow: vb_core::workflow::CompiledWorkflow,
) -> Result<vb_core::workflow::CompiledWorkflow, String> {
    let mut parts = workflow.to_parts();
    parts.digest = vb_core::ids::WorkflowDigest::from_bytes([0; 32]);
    let canonical = postcard::to_allocvec(&parts).map_err(|error| error.to_string())?;
    parts.digest = vb_core::ids::WorkflowDigest::from_bytes(blake3::hash(&canonical).into());
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())
}

fn assert_submit_pair_refused(shard: &Shard, run: RunId) {
    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.counters().snapshot().runs_submitted, 0);
    assert_eq!(shard.journal_sequences.get(&run), None);
    assert_eq!(shard.pending_action_get(run), None);
    assert!(!shard.run_state_contains(run));
}

#[test]
fn submission_pair_one_free_queue_slot_publishes_neither_event() -> Result<(), String> {
    let (journal, queue, adapter, shared) =
        queued_journal_fixture("vb-submission-pair-refusal-", 2, 1)?;
    fill_queue_to_one_free_slot(&queue, 2, 60_100)?;
    let mut shard = Shard::new_with_journal_and_artifact_store(
        small_config(),
        shared,
        crate::admission::AlwaysPresentArtifactStore::shared(),
    );
    let run = RunId::new(60_101);
    shard
        .seed_journal_sequence(run, vb_storage::EventSeq::new(7))
        .map_err(|error| error.to_string())?;
    let counters = shard.counters().snapshot();
    let result = submit_pair_fixture(&mut shard, run);
    assert!(matches!(
        result,
        Err(RuntimeError::AdmissionHeaderPersistenceFailed { source })
            if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
    ));
    assert_submit_pair_refused(&shard, run);
    assert_eq!(shard.counters().snapshot(), counters);
    assert_eq!(
        queue
            .pending_profile_counts()
            .map_err(|error| error.to_string())?
            .journaled,
        1
    );
    assert!(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?
            .is_empty()
    );
    let report = adapter.flush_batch().map_err(|error| error.to_string())?;
    assert_eq!((report.drained, report.written), (1, 1));
    assert!(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?
            .is_empty()
    );
    Ok(())
}

#[test]
fn submission_pair_flush_is_indivisible_at_batch_size_one() -> Result<(), String> {
    let (journal, queue, adapter, shared) =
        queued_journal_fixture("vb-submission-pair-flush-", 2, 1)?;
    let mut shard = Shard::new_with_journal_and_artifact_store(
        small_config(),
        shared,
        crate::admission::AlwaysPresentArtifactStore::shared(),
    );
    let run = RunId::new(60_102);
    let workflow = require_workflow("submission pair", suspended_workflow())?;
    let expected_admission = shard
        .build_admission_with_budget(
            run,
            &workflow,
            crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        )
        .map_err(|error| error.to_string())?
        .ok_or("missing admission fixture")?;
    let result = submit_pair_fixture(&mut shard, run);
    assert!(matches!(
        result,
        Err(RuntimeError::StorageJournalAppend { source })
            if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
    ));
    assert_eq!(shard.active_run_count(), 1);
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    assert_eq!(
        shard.journal_sequence_for(run),
        vb_storage::EventSeq::new(2)
    );
    assert_eq!(
        queue
            .pending_profile_counts()
            .map_err(|error| error.to_string())?
            .journaled,
        2
    );
    assert!(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?
            .is_empty()
    );
    let report = adapter.flush_batch().map_err(|error| error.to_string())?;
    assert_eq!((report.drained, report.written), (2, 2));
    let expected_admission = crate::journal::RuntimeJournal::convert_to_storage_event(
        adapter.as_ref(),
        RuntimeJournalEvent::RunAdmission {
            admission: expected_admission,
        },
        vb_storage::EventSeq::new(1),
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?,
        vec![
            vb_storage::JournalEvent::RunAccepted {
                run,
                seq: vb_storage::EventSeq::ZERO,
                workflow: workflow.digest(),
            },
            expected_admission,
        ]
    );
    shard
        .append_journal_event(RuntimeJournalEvent::RunCancelled {
            run,
            reason: None,
            attempt: 1,
        })
        .map_err(|error| error.to_string())?;
    assert_eq!(
        shard.journal_sequence_for(run),
        vb_storage::EventSeq::new(3)
    );
    let report = adapter.flush_batch().map_err(|error| error.to_string())?;
    assert_eq!((report.drained, report.written), (1, 1));
    let events = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(events.len(), 3);
    assert!(matches!(
        events.last(),
        Some(vb_storage::JournalEvent::RunCancelled { run: found, seq, .. })
            if *found == run && *seq == vb_storage::EventSeq::new(2)
    ));
    Ok(())
}

#[derive(Default)]
struct SubmissionSingletonJournal {
    calls: std::sync::atomic::AtomicUsize,
}

impl crate::journal::RuntimeJournal for SubmissionSingletonJournal {
    fn append(&self, _event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    fn probe(&self) -> crate::RuntimeResult<()> {
        Ok(())
    }
}

#[test]
fn submission_pair_default_batch_refusal_has_no_singleton_fallback() -> Result<(), String> {
    let journal = std::sync::Arc::new(SubmissionSingletonJournal::default());
    let mut shard = Shard::new_with_journal_and_artifact_store(
        small_config(),
        journal.clone(),
        crate::admission::AlwaysPresentArtifactStore::shared(),
    );
    let run = RunId::new(60_103);
    let result = submit_pair_fixture(&mut shard, run);
    assert!(matches!(
        result,
        Err(RuntimeError::UnsupportedOperation {
            operation: "runtime_journal_batch_append"
        })
    ));
    assert_eq!(journal.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_submit_pair_refused(&shard, run);
    Ok(())
}

#[derive(Default)]
struct SubmissionRejectingBatchJournal {
    singleton_calls: std::sync::atomic::AtomicUsize,
    batch_calls: std::sync::atomic::AtomicUsize,
}

impl crate::journal::RuntimeJournal for SubmissionRejectingBatchJournal {
    fn append(&self, _event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
        self.singleton_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    fn append_sequenced_batch(
        &self,
        _events: &[RuntimeJournalEvent],
        _start_seq: vb_storage::EventSeq,
    ) -> crate::RuntimeResult<()> {
        self.batch_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Err(RuntimeError::from(vb_storage::JournalError::QueueFull))
    }

    fn probe(&self) -> crate::RuntimeResult<()> {
        Ok(())
    }
}

#[test]
fn submission_pair_explicit_batch_rejection_has_no_singleton_fallback() -> Result<(), String> {
    let journal = std::sync::Arc::new(SubmissionRejectingBatchJournal::default());
    let mut shard = Shard::new_with_journal_and_artifact_store(
        small_config(),
        journal.clone(),
        crate::admission::AlwaysPresentArtifactStore::shared(),
    );
    let run = RunId::new(60_104);
    let result = submit_pair_fixture(&mut shard, run);
    assert!(matches!(
        result,
        Err(RuntimeError::AdmissionHeaderPersistenceFailed { source })
            if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
    ));
    assert_eq!(
        journal
            .singleton_calls
            .load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    assert_eq!(
        journal
            .batch_calls
            .load(std::sync::atomic::Ordering::SeqCst),
        1
    );
    assert_submit_pair_refused(&shard, run);
    Ok(())
}

struct PreparedArtifactStore {
    artifact: vb_storage::admission::AcceptedArtifact,
}

impl crate::admission::AcceptedArtifactStore for PreparedArtifactStore {
    fn load_accepted_artifact(
        &self,
        _digest: vb_core::ids::WorkflowDigest,
    ) -> Result<vb_storage::admission::AcceptedArtifact, crate::admission::ArtifactEnvelopeError>
    {
        Ok(self.artifact.clone())
    }
}

#[test]
fn submission_journals_ir_compat_decision() -> Result<(), String> {
    let (journal, _queue, adapter, shared) =
        queued_journal_fixture("vb-compat-decision-submit-", 10, 10)?;
    let workflow = require_workflow("compat decision submit", suspended_workflow())?;
    let workflow = honest_digest_workflow(workflow)?;
    let contracts = strict_submit_contracts();
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        &workflow,
        vb_core::policy::RuntimePolicy::Strict,
        &contracts,
    )
    .map_err(|error| error.to_string())?;
    let strict_config = crate::shard::ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Strict,
    };
    let store: crate::admission::SharedAcceptedArtifactStore =
        std::sync::Arc::new(PreparedArtifactStore { artifact });
    let mut shard = Shard::new_with_journal_and_artifact_store(strict_config, shared, store);
    let run = RunId::new(60_105);
    shard
        .handle_submit_with_inputs_and_contracts(
            run,
            workflow,
            &[(SlotIdx::ZERO, SlotValue::I64(0))],
            strict_submit_caps(),
            &contracts,
        )
        .map_err(|error| error.to_string())?;
    adapter.flush_batch().map_err(|error| error.to_string())?;
    let decisions = vb_storage::compat::compat_decisions_for_run(&journal, run)
        .map_err(|error| error.to_string())?;
    assert_eq!(decisions.len(), 1);
    let decision = decisions[0];
    assert_eq!(
        decision.boundary,
        vb_storage::compat::CompatBoundary::IrVersion
    );
    assert_eq!(
        decision.class,
        vb_storage::compat::CompatibilityClass::Identical
    );
    assert_eq!(decision.decision_code.code(), 100);
    assert_eq!(
        decision.old_schema_digest,
        vb_storage::compat::schema_version_digest(vb_storage::compat::CURRENT_IR_VERSION)
    );
    assert_eq!(
        decision.new_schema_digest,
        vb_storage::compat::schema_version_digest(vb_storage::compat::CURRENT_IR_VERSION)
    );
    let events = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert!(matches!(
        events.get(2),
        Some(vb_storage::JournalEvent::CompatibilityDecisionEvent {
            run: found,
            seq,
            decision: found_decision,
        }) if *found == run
            && *seq == vb_storage::EventSeq::new(2)
            && *found_decision == decision
    ));
    Ok(())
}
