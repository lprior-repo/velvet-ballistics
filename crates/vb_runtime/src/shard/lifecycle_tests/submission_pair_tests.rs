struct PreparedSubmissionPair {
    workflow: vb_core::workflow::CompiledWorkflow,
    contracts: Box<[vb_core::action::ActionContract]>,
    inputs: [(SlotIdx, SlotValue); 1],
    caps: vb_core::capability::CapabilitySet,
    artifact: vb_storage::admission::AcceptedArtifact,
    admission: crate::admission::RunAdmission,
}

fn prepare_submission_pair_fixture(
    shard: &mut Shard,
    run: RunId,
) -> crate::RuntimeResult<PreparedSubmissionPair> {
    let workflow = suspended_workflow().ok_or(RuntimeError::InvalidActionCompletion)?;
    let workflow =
        honest_digest_workflow(workflow).map_err(|_| RuntimeError::InvalidActionCompletion)?;
    let contracts = strict_submit_contracts();
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        &workflow,
        vb_core::policy::RuntimePolicy::Strict,
        &contracts,
    )?;
    let artifact = if let Some(journal) =
        crate::journal::RuntimeJournal::storage_journal(shard.journal.as_ref())
    {
        let accepted = vb_storage::submit_artifact_with_contracts(
            journal.as_ref(),
            &workflow,
            vb_core::policy::RuntimePolicy::Journaled,
            &contracts,
        )?;
        shard.policy = vb_core::policy::RuntimePolicy::Journaled;
        shard.artifact_store = crate::admission::StorageArtifactStore::shared(journal);
        accepted
    } else {
        shard.policy = vb_core::policy::RuntimePolicy::Strict;
        shard.artifact_store = std::sync::Arc::new(PreparedArtifactStore {
            artifact: artifact.clone(),
        });
        artifact
    };
    let inputs = [(SlotIdx::ZERO, SlotValue::I64(0))];
    let input_digest =
        vb_storage::inputs::compute_input_digest(&inputs).map_err(RuntimeError::from)?;
    let caps = strict_submit_caps();
    let admission = shard
        .build_admission_with_budget(run, &workflow, caps.clone(), None)?
        .ok_or(RuntimeError::InvalidActionCompletion)?
        .with_input_digest(vb_storage::RunInputBinding::Digest(input_digest));
    Ok(PreparedSubmissionPair {
        workflow,
        contracts,
        inputs,
        caps,
        artifact,
        admission,
    })
}

fn submit_pair_fixture(shard: &mut Shard, run: RunId) -> crate::RuntimeResult<()> {
    let prepared = prepare_submission_pair_fixture(shard, run)?;
    shard.handle_submit_with_inputs_and_contracts(
        run,
        prepared.workflow,
        &prepared.inputs,
        prepared.caps,
        &prepared.contracts,
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
    let mut shard = Shard::new_with_journal(small_config(), shared).expect("valid test configuration");
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
        queued_journal_fixture("vb-submission-pair-flush-", 4, 1)?;
    let mut shard = Shard::new_with_journal(small_config(), shared).expect("valid test configuration");
    let run = RunId::new(60_102);
    let prepared =
        prepare_submission_pair_fixture(&mut shard, run).map_err(|error| error.to_string())?;
    let workflow = prepared.workflow.clone();
    let expected_input_digest = vb_storage::inputs::compute_input_digest(&prepared.inputs)
        .map_err(|error| error.to_string())?;
    let expected_budget = vb_core::AggregateResourceBudget::from_workflow(&workflow)
        .map_err(|error| format!("budget fixture failed: {error:?}"))?;
    assert_eq!(
        prepared.admission.artifact_digest(),
        prepared.artifact.digest
    );
    assert_eq!(prepared.admission.run_id(), run);
    assert_eq!(prepared.admission.granted_capabilities(), &prepared.caps);
    assert_eq!(
        prepared.admission.policy(),
        vb_core::policy::RuntimePolicy::Journaled
    );
    assert_eq!(
        prepared.admission.input_digest(),
        vb_storage::RunInputBinding::Digest(expected_input_digest)
    );
    assert_eq!(
        prepared.admission.policy_pin(),
        vb_storage::RunPolicyPin::Pinned(prepared.artifact.policy_digest)
    );
    assert_eq!(prepared.admission.budget(), Some(expected_budget));
    let expected_admission = crate::journal::RuntimeJournal::convert_to_storage_event(
        shard.journal.as_ref(),
        RuntimeJournalEvent::RunAdmission {
            admission: prepared.admission.clone(),
        },
        vb_storage::EventSeq::new(1),
    )
    .map_err(|error| error.to_string())?;
    let expected_compatibility = crate::journal::RuntimeJournal::convert_to_storage_event(
        shard.journal.as_ref(),
        RuntimeJournalEvent::CompatibilityDecision {
            run,
            decision: prepared
                .admission
                .ir_compat_decision()
                .ok_or_else(|| String::from("current admission compatibility decision missing"))?,
        },
        vb_storage::EventSeq::new(2),
    )
    .map_err(|error| error.to_string())?;
    let result = shard.handle_submit_with_inputs_and_contracts(
        run,
        prepared.workflow,
        &prepared.inputs,
        prepared.caps,
        &prepared.contracts,
    );
    assert!(matches!(
        result,
        Err(RuntimeError::StorageJournalAppend { source })
            | Err(RuntimeError::AdmissionHeaderPersistenceFailed { source })
            if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
    ));
    assert_eq!(shard.active_run_count(), 1);
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    assert_eq!(
        shard.journal_sequence_for(run),
        vb_storage::EventSeq::new(4)
    );
    assert_eq!(
        queue
            .pending_profile_counts()
            .map_err(|error| error.to_string())?
            .journaled,
        4
    );
    assert!(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?
            .is_empty()
    );
    let state = shard
        .run_state_get(run)
        .ok_or("run state missing after current admission submission")?;
    assert_eq!(state.frame.read_slot(SlotIdx::ZERO), Ok(&SlotValue::I64(0)));
    assert_eq!(state.admission.as_ref(), Some(&prepared.admission));
    let expected_started = state
        .run_started_millis
        .ok_or("missing recorded run origin")?;
    let events_before_flush = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert!(events_before_flush.is_empty());
    assert_eq!(
        queue
            .pending_profile_counts()
            .map_err(|error| error.to_string())?
            .journaled,
        4
    );
    let expected_events = vec![
        vb_storage::JournalEvent::RunAccepted {
            run,
            seq: vb_storage::EventSeq::ZERO,
            workflow: workflow.digest(),
        },
        expected_admission,
        expected_compatibility,
        vb_storage::JournalEvent::RunBudgetOriginEvent {
            run,
            seq: vb_storage::EventSeq::new(3),
            started_millis: expected_started,
        },
    ];
    let report = adapter.flush_batch().map_err(|error| error.to_string())?;
    assert_eq!((report.drained, report.written), (4, 4));
    assert_eq!(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?,
        expected_events
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
        vb_storage::EventSeq::new(5)
    );
    let report = adapter.flush_batch().map_err(|error| error.to_string())?;
    assert_eq!((report.drained, report.written), (1, 1));
    let events = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(events.len(), 5);
    assert!(matches!(
        events.last(),
        Some(vb_storage::JournalEvent::RunCancelled { run: found, seq, .. })
            if *found == run && *seq == vb_storage::EventSeq::new(4)
    ));
    Ok(())
}

#[derive(Default)]
struct SubmissionSingletonJournal {
    calls: std::sync::atomic::AtomicUsize,
}

impl crate::journal::RuntimeJournal for SubmissionSingletonJournal {
    fn durability_profile(&self) -> Option<vb_storage::DurabilityProfile> {
        Some(vb_storage::DurabilityProfile::Volatile)
    }

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
    let mut shard = Shard::new_with_journal_and_artifact_store(small_config(), journal.clone(), crate::admission::AlwaysPresentArtifactStore::shared()).expect("valid test configuration");
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
    fn durability_profile(&self) -> Option<vb_storage::DurabilityProfile> {
        Some(vb_storage::DurabilityProfile::Volatile)
    }

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
    let mut shard = Shard::new_with_journal_and_artifact_store(small_config(), journal.clone(), crate::admission::AlwaysPresentArtifactStore::shared()).expect("valid test configuration");
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
        max_open_gates: 1024,
        policy: vb_core::policy::RuntimePolicy::Strict,
    };
    let store: crate::admission::SharedAcceptedArtifactStore =
        std::sync::Arc::new(PreparedArtifactStore { artifact });
    let mut shard = Shard::new_with_journal_and_artifact_store(strict_config, shared, store).expect("valid test configuration");
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
