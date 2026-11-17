use crate::journal::VolatileRuntimeJournal;
use crate::recovery::RecoveredRunBoundary;
use crate::shard::types::RecoverRunCommand;

fn action_recovery_test_contract()
-> Result<vb_core::action::ActionContract, Box<dyn std::error::Error>> {
    let name = vb_core::action::ActionName::new("recovery-action")?;
    Ok(vb_core::action::ActionContract {
        id: vb_core::ids::ActionId::new(0),
        name,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: vb_core::action::Idempotency::DeterministicPure,
        side_effect: vb_core::action::SideEffect::None,
        retry_safety: vb_core::action::RetrySafety::Safe,
        required_capabilities: Box::new([]),
    })
}

fn action_recovery_contract_store(
    workflow: &vb_core::workflow::CompiledWorkflow,
    contract: &vb_core::action::ActionContract,
) -> Result<
    (
        vb_core::ids::WorkflowDigest,
        crate::admission::SharedAcceptedArtifactStore,
    ),
    Box<dyn std::error::Error>,
> {
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        workflow,
        vb_core::policy::RuntimePolicy::Relaxed,
        std::slice::from_ref(contract),
    )?;
    Ok((artifact.digest, action_recovery_artifact_store(artifact)))
}

fn action_recovery_artifact_store(
    artifact: vb_storage::admission::AcceptedArtifact,
) -> crate::admission::SharedAcceptedArtifactStore {
    #[derive(Debug)]
    struct ActionRecoveryContractStore(vb_storage::admission::AcceptedArtifact);

    impl crate::admission::AcceptedArtifactStore for ActionRecoveryContractStore {
        fn load_accepted_artifact(
            &self,
            artifact_digest: vb_core::ids::WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, crate::admission::ArtifactEnvelopeError>
        {
            if artifact_digest != self.0.digest {
                return Err(crate::admission::ArtifactEnvelopeError::ArtifactNotFound {
                    digest: artifact_digest,
                });
            }
            Ok(self.0.clone())
        }
    }

    std::sync::Arc::new(ActionRecoveryContractStore(artifact))
}

fn action_recovery_current_admission(
    run: vb_core::ids::RunId,
    workflow: &vb_core::workflow::CompiledWorkflow,
    artifact: &vb_storage::admission::AcceptedArtifact,
    policy: vb_core::policy::RuntimePolicy,
) -> Result<crate::admission::RunAdmission, Box<dyn std::error::Error>> {
    let inputs = [(
        vb_core::ids::SlotIdx::ZERO,
        vb_core::value::SlotValue::I64(0),
    )];
    let budget = vb_core::AggregateResourceBudget::from_workflow(workflow)?;
    let digest = vb_storage::inputs::compute_input_digest(&inputs)?;
    Ok(crate::admission::RunAdmission::with_budget(
        artifact.digest,
        run,
        vb_core::CapabilitySet::from_grants(artifact.required_capabilities.clone()),
        policy,
        budget,
    )
    .with_input_digest(vb_storage::RunInputBinding::Digest(digest))
    .with_policy_pin(vb_storage::RunPolicyPin::Pinned(artifact.policy_digest)))
}

fn current_recovery_header_accounting(
    run: vb_core::ids::RunId,
    workflow: &vb_core::workflow::CompiledWorkflow,
    admission: &crate::admission::RunAdmission,
    started_millis: u64,
) -> Result<(vb_storage::EventSeq, Box<[u16]>, u64), Box<dyn std::error::Error>> {
    let mut events = vec![
        vb_storage::JournalEvent::RunAccepted {
            run,
            seq: vb_storage::EventSeq::ZERO,
            workflow: workflow.digest(),
        },
        vb_storage::JournalEvent::RunAdmissionBudget {
            run,
            seq: vb_storage::EventSeq::new(1),
            artifact_digest: admission.artifact_digest(),
            input_digest: admission.input_digest().clone(),
            policy_pin: admission.policy_pin(),
            granted_capabilities: admission.granted_capabilities().clone(),
            policy: admission.policy(),
            budget: admission
                .budget()
                .ok_or("current admission budget missing")?,
        },
    ];
    let mut next_seq = 2u64;
    if let Some(decision) = admission.ir_compat_decision() {
        events.push(vb_storage::JournalEvent::CompatibilityDecisionEvent {
            run,
            seq: vb_storage::EventSeq::new(next_seq),
            decision,
        });
        next_seq = next_seq.checked_add(1).ok_or("header sequence overflow")?;
    }
    events.push(vb_storage::JournalEvent::RunBudgetOriginEvent {
        run,
        seq: vb_storage::EventSeq::new(next_seq),
        started_millis,
    });
    next_seq = next_seq.checked_add(1).ok_or("header sequence overflow")?;
    let journal_bytes = events.iter().try_fold(0u64, |total, event| {
        total
            .checked_add(crate::journal::storage_event_bytes(event)?)
            .ok_or(crate::RuntimeError::EncodeFailed)
    })?;
    Ok((
        vb_storage::EventSeq::new(next_seq),
        crate::shard::helpers::new_action_attempts(workflow.node_count()),
        journal_bytes,
    ))
}

#[test]
fn recover_restores_action_contracts_schedules_completes_and_finishes()
-> Result<(), Box<dyn std::error::Error>> {
    let run_id = vb_core::ids::RunId::new(1);

    let do_node = vb_core::workflow::node(
        vb_core::ids::StepIdx::ZERO,
        vb_core::workflow::CompiledNodeKind::Do {
            action: vb_core::ids::ActionId::new(0),
            input: vb_core::ids::SlotIdx::new(0),
        },
        Some(vb_core::ids::SlotIdx::new(1)),
        Some(vb_core::ids::StepIdx::new(1)),
    );
    let finish_node = vb_core::workflow::node(
        vb_core::ids::StepIdx::new(1),
        vb_core::workflow::CompiledNodeKind::Finish {
            result: vb_core::ids::SlotIdx::new(1),
        },
        None,
        None,
    );
    let parts = vb_core::workflow::WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("recovery-test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([do_node, finish_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        input_slots: Box::from([vb_core::CompiledInputSlot {
            slot: vb_core::SlotIdx::ZERO,
            kind: vb_core::InputSlotKind::Any,
            constraint: vb_core::CompiledInputConstraint::required(),
        }]),
    };

    let contract = action_recovery_test_contract()?;
    let workflow =
        honest_digest_workflow(vb_core::workflow::CompiledWorkflow::try_from_parts(parts)?)?;
    let workflow_digest = workflow.digest();
    let (artifact_digest, store) = action_recovery_contract_store(&workflow, &contract)?;
    assert_ne!(artifact_digest, workflow_digest);
    let accepted_artifact = store.load_accepted_artifact(artifact_digest)?;
    let admission = action_recovery_current_admission(
        run_id,
        &workflow,
        &accepted_artifact,
        vb_core::policy::RuntimePolicy::Relaxed,
    )?;
    let started_millis =
        crate::shard::helpers::wall_clock_millis_since_epoch().ok_or("clock unavailable")?;
    let (next_seq, action_attempts, journal_bytes) =
        current_recovery_header_accounting(run_id, &workflow, &admission, started_millis)?;

    let config = crate::shard::ShardConfig {
        command_queue_capacity: 64,
        trace_capacity: 64,
        step_budget_per_tick: 4,
        max_active_runs: 8,
        max_open_gates: 1024,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared_journal = journal.clone();
    let mut shard = crate::shard::Shard::new_with_journal_and_artifact_store(config, shared_journal, store.clone()).expect("valid test configuration");

    let mut frame = vb_core::frame::RunFrame::new(run_id, vb_core::ids::StepIdx::ZERO, 2, 2)?;
    frame.write_slot(
        vb_core::ids::SlotIdx::new(0),
        vb_core::value::SlotValue::I64(0),
    )?;

    let command = RecoverRunCommand::new(
        run_id,
        frame,
        artifact_digest,
        workflow_digest,
        next_seq,
        crate::primitives::collect::CollectStates::new(),
        RecoveredRunBoundary::none(),
    )
    .with_validated_admission(admission)
    .with_recovered_accounting(
        Some(started_millis),
        Some(action_attempts),
        Some(journal_bytes),
    );

    // Phase 1: Drive recovery — restores action contracts and schedules Do
    shard.handle_recover(command)?;

    // Verify action contracts were restored from the artifact (exact match)
    let state = shard
        .run_state_get(run_id)
        .ok_or("run state not found after recovery; recovery failed")?;
    let expected_artifact = store.load_accepted_artifact(artifact_digest)?;
    assert_eq!(
        state.action_contracts, expected_artifact.action_contracts,
        "recovered run action_contracts must exactly match artifact"
    );

    // The explicit historical receipt carries the exact admitted input,
    // artifact, policy pin, and workflow-derived budget into recovery.
    let admission = state
        .admission
        .as_ref()
        .ok_or("recovery must preserve current historical admission")?;
    assert_eq!(
        admission.run_id(),
        run_id,
        "recovered admission must carry the correct run identity"
    );
    assert_eq!(
        admission.artifact_digest(),
        artifact_digest,
        "recovered admission must carry the correct artifact digest"
    );
    assert_eq!(
        admission.policy_pin(),
        vb_storage::RunPolicyPin::Pinned(accepted_artifact.policy_digest),
        "recovery must preserve the pinned accepted artifact policy"
    );
    assert_eq!(
        admission.policy(),
        vb_core::policy::RuntimePolicy::Relaxed,
        "recovered admission must carry the current shard policy"
    );
    assert_eq!(
        admission.granted_capabilities(),
        &vb_core::CapabilitySet::from_grants(expected_artifact.required_capabilities.clone()),
        "recovered admission must grant exactly the artifact's required capabilities"
    );

    // Now prove that when a genuine failed-state checkpoint IS supplied via the
    // public .with_retry_checkpoint() builder, and the checkpoint carries a
    // different policy pin than the artifact, the shard rejects it with the
    // exact admission_policy_pin_mismatch error. This exercises the real
    // checkpoint capture, frame hydration, and policy pin validation path.
    let original_state_clone = state.clone();
    assert_raw_checkpoint_wrong_pin_rejected(
        &mut shard,
        original_state_clone,
        &journal,
        artifact_digest,
        &accepted_artifact,
    )?;

    // Verify the Do action was scheduled with the correct ticket fields
    let ticket = shard
        .pending_action_get(run_id)
        .ok_or("pending action not found after recovery; Do action was not scheduled")?;
    assert_eq!(ticket.run, run_id);
    assert_eq!(ticket.step, vb_core::ids::StepIdx::ZERO);
    assert_eq!(ticket.action, vb_core::ids::ActionId::new(0));
    assert_eq!(ticket.attempt, 1);

    // Phase 2: Complete the action using the REAL API (ShardCommand::ActionCompleted)
    let output = vb_core::action::ActionOutputReady {
        output_slot: vb_core::ids::SlotIdx::new(1),
        value: vb_core::value::SlotValue::I64(42),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 2,
    };
    shard.enqueue(crate::shard::ShardCommand::ActionCompleted { ticket, output })?;

    // Drive to completion: ActionCompleted → Finish → RunFinished
    let ticked = shard.tick()?;
    assert!(
        ticked,
        "tick should return true after processing the action completion"
    );

    // Verify run finished
    assert!(
        shard.terminal_runs_contains(run_id),
        "run should be in terminal state after action completion and finish"
    );
    assert!(
        !shard.run_state_contains(run_id),
        "run state should be removed after terminal completion"
    );

    // Phase 3: Verify real journal evidence
    let events = journal.snapshot()?;

    // When action contracts are present, the runtime uses the new
    // ActionCompletedEnvelope format (atomic durable envelope).
    let action_completed_count = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                crate::journal::RuntimeJournalEvent::ActionCompleted { .. }
                    | crate::journal::RuntimeJournalEvent::ActionCompletedEnvelope { .. }
            )
        })
        .count();
    assert_eq!(
        action_completed_count, 1,
        "exactly one action completion journal event expected (Completed or Envelope)"
    );

    // Exactly one RunFinished event
    let run_finished_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, crate::journal::RuntimeJournalEvent::RunFinished { .. }))
        .collect();
    assert_eq!(
        run_finished_events.len(),
        1,
        "exactly one RunFinished journal event expected"
    );
    // Verify the RunFinished event has the correct run id and result slot
    assert!(matches!(run_finished_events.first(),
            Some(crate::journal::RuntimeJournalEvent::RunFinished { run, result, attempt: 1})
                if *run == run_id && *result == vb_core::ids::SlotIdx::new(1)));
    let completed: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            crate::journal::RuntimeJournalEvent::ActionCompletedEnvelope {
                ticket,
                output,
                value,
                ..
            } => Some((*ticket, *output, value.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        completed,
        vec![(
            ticket,
            vb_core::ids::SlotIdx::new(1),
            postcard::to_allocvec(&vb_core::value::SlotValue::I64(42))?
        )]
    );

    // Phase 4: Duplicate completion must NOT produce a second RunFinished
    // After the run is terminal, a duplicate ActionCompleted is rejected
    // with an error (InvalidActionCompletion or RunNotFound), not silently
    // accepted. Verify the error is thrown and journal is unchanged.
    shard.enqueue(crate::shard::ShardCommand::ActionCompleted {
        ticket,
        output: vb_core::action::ActionOutputReady {
            output_slot: vb_core::ids::SlotIdx::new(1),
            value: vb_core::value::SlotValue::I64(99),
            taint: vb_core::value::Taint::Clean,
            encoded_len: 2,
        },
    })?;
    // tick() should return Err because the duplicate completion targets
    // a run that has already finished (terminal)
    let tick_result = shard.tick();
    assert_eq!(tick_result, Err(crate::RuntimeError::RunNotFound));

    let events2 = journal.snapshot()?;
    assert_eq!(
        events2, events,
        "duplicate completion must preserve the entire journal"
    );
    let run_finished_count_after_dup = events2
        .iter()
        .filter(|e| matches!(e, crate::journal::RuntimeJournalEvent::RunFinished { .. }))
        .count();
    assert_eq!(
        run_finished_count_after_dup, 1,
        "duplicate completion must not add second RunFinished"
    );

    Ok(())
}

fn assert_raw_checkpoint_wrong_pin_rejected(
    shard: &mut crate::shard::Shard,
    original: crate::shard::types::RunState,
    journal: &VolatileRuntimeJournal,
    artifact_digest: vb_core::ids::WorkflowDigest,
    accepted_artifact: &vb_storage::admission::AcceptedArtifact,
) -> Result<(), Box<dyn std::error::Error>> {
    let run_id = original.frame.run_id();

    // Record the complete original state before any probe.
    let original_pending = shard.pending_boundary_snapshot(0, 8);
    let original_journal = journal.snapshot()?;

    // Clone the recovered state to build a failed fixture. The original frame
    // is Running, so we must genuinely transition it to Failed via the real
    // frame API — not pretend a non-failed frame can be checkpointed.
    let mut failed = original.clone();
    failed.frame.mark_failed(vb_core::ids::StepIdx::ZERO)?;

    // Forge a policy pin that differs from the artifact's actual policy digest.
    // The original recovered admission has an Unverified raw pin (no digest).
    // Use a fabricated digest to guarantee the forged pin mismatches the real
    // computed policy digest, proving the checkpoint pin validation path.
    let wrong_pin = vb_core::ids::WorkflowDigest::from_bytes([44u8; 32]);
    assert_ne!(
        wrong_pin,
        vb_storage::admission::compute_policy_digest(&original.workflow)?,
        "forged pin must differ from the workflow's actual policy digest"
    );
    let forged_admission = failed
        .admission
        .as_ref()
        .ok_or("failed fixture must carry an admission record")?
        .clone()
        .with_policy_pin(vb_storage::RunPolicyPin::Pinned(wrong_pin));
    failed.admission = Some(forged_admission);

    // Capture a genuinely valid checkpoint from the failed state. This exercises
    // the real RunCheckpointV1::capture path with its admission validation.
    let checkpoint = crate::run_checkpoint::RunCheckpointV1::capture(
        run_id,
        artifact_digest,
        core::num::NonZeroU16::new(2).ok_or("invalid checkpoint attempt number")?,
        vb_core::ids::StepIdx::ZERO,
        1,
        &failed,
    )?;

    // Hydrate a valid retry frame from the checkpoint for the recovery command.
    let retry_frame = vb_core::frame::RunFrame::from_checkpoint_v1(checkpoint.frame.clone())
        .map_err(|error| format!("retry frame fixture failed: {error:?}"))?;

    // Build the public recovery command with the retry checkpoint attached via
    // the exact .with_retry_checkpoint() builder path that production uses.
    let command = RecoverRunCommand::new(
        run_id,
        retry_frame,
        artifact_digest,
        original.workflow.digest(),
        shard
            .journal_seq_get(run_id)
            .ok_or("recovered run must retain its next journal sequence")?,
        crate::primitives::collect::CollectStates::new(),
        crate::recovery::RecoveredRunBoundary::none(),
    )
    .with_validated_admission(
        failed
            .admission
            .clone()
            .ok_or("failed checkpoint must retain current admission")?,
    )
    .with_recovered_accounting(
        original.run_started_millis,
        Some(original.action_attempts.clone()),
        Some(original.journal_bytes_written),
    )
    .with_retry_checkpoint(checkpoint);

    // Occupied-run rejection takes precedence over command validation and must
    // leave the existing owner untouched, even when the command has a bad pin.
    let result = shard.handle_recover(command.clone());
    assert_eq!(
        result,
        Err(crate::RuntimeError::RunAlreadyExists),
        "occupied shard must reject duplicate recovery before authority validation"
    );

    // The failed probe must leave the shard's state, pending boundaries, and
    // journal exactly unchanged. Only the cloned failed fixture mutated; the
    // real shard's recovered run is untouched.
    assert_eq!(
        shard.run_state_get(run_id),
        Some(&original),
        "shard run state must be unchanged after rejected checkpoint probe"
    );
    assert_eq!(
        shard.pending_boundary_snapshot(0, 8),
        original_pending,
        "pending boundaries must be unchanged after rejected checkpoint probe"
    );
    assert_eq!(
        journal.snapshot()?,
        original_journal,
        "journal must be unchanged after rejected checkpoint probe"
    );

    // Probe the same forged authority on an unoccupied shard so duplicate
    // rejection cannot mask the original exact policy-pin validation oracle.
    let pin_config = crate::shard::ShardConfig {
        command_queue_capacity: 64,
        trace_capacity: 64,
        step_budget_per_tick: 4,
        max_active_runs: 8,
        max_open_gates: 1024,
        policy: shard.policy,
    };
    let pin_journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let mut pin_shard = crate::shard::Shard::new_with_journal_and_artifact_store(
        pin_config,
        pin_journal.clone(),
        shard.artifact_store.clone(),
    )?;
    let pin_pending = pin_shard.pending_boundary_snapshot(0, 8);
    let pin_journal_before = pin_journal.snapshot()?;
    assert_eq!(
        pin_shard.handle_recover(command),
        Err(crate::RuntimeError::RecoveryCannotResume {
            reason: "admission_policy_pin_mismatch".to_string()
        }),
        "fresh shard must reject retry checkpoint with mismatched policy pin"
    );
    assert_eq!(pin_shard.run_state_get(run_id), None);
    assert_eq!(pin_shard.pending_boundary_snapshot(0, 8), pin_pending);
    assert_eq!(pin_journal.snapshot()?, pin_journal_before);

    // Second probe: admission policy mismatch (Strict configured vs Relaxed
    // actual shard policy) with a valid pinned receipt. The pin check
    // passes, but the configured policy in the admission record does not match
    // the shard's actual runtime policy. This should be rejected with
    // admission_policy_mismatch.
    // Use a fresh shard so the run is not already tracked (avoids RunAlreadyExists
    // masking the policy admission check).
    let mut failed_policy = original.clone();
    failed_policy
        .frame
        .mark_failed(vb_core::ids::StepIdx::ZERO)?;

    let strict_admission = action_recovery_current_admission(
        run_id,
        &original.workflow,
        &accepted_artifact,
        vb_core::policy::RuntimePolicy::Strict,
    )?;
    failed_policy.admission = Some(strict_admission);

    let policy_checkpoint = crate::run_checkpoint::RunCheckpointV1::capture(
        run_id,
        artifact_digest,
        core::num::NonZeroU16::new(2).ok_or("invalid checkpoint attempt number")?,
        vb_core::ids::StepIdx::ZERO,
        1,
        &failed_policy,
    )?;

    let policy_retry_frame =
        vb_core::frame::RunFrame::from_checkpoint_v1(policy_checkpoint.frame.clone())
            .map_err(|error| format!("policy retry frame fixture failed: {error:?}"))?;

    let policy_command = RecoverRunCommand::new(
        run_id,
        policy_retry_frame,
        artifact_digest,
        original.workflow.digest(),
        shard
            .journal_seq_get(run_id)
            .ok_or("recovered run must retain its next journal sequence")?,
        crate::primitives::collect::CollectStates::new(),
        crate::recovery::RecoveredRunBoundary::none(),
    )
    .with_validated_admission(
        failed_policy
            .admission
            .clone()
            .ok_or("policy checkpoint must retain current admission")?,
    )
    .with_recovered_accounting(
        original.run_started_millis,
        Some(original.action_attempts.clone()),
        Some(original.journal_bytes_written),
    )
    .with_retry_checkpoint(policy_checkpoint);

    // Fresh shard with matching Relaxed policy, same config as original.
    let fresh_config = crate::shard::ShardConfig {
        command_queue_capacity: 64,
        trace_capacity: 64,
        step_budget_per_tick: 4,
        max_active_runs: 8,
        max_open_gates: 1024,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let fresh_journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let mut fresh_shard = crate::shard::Shard::new_with_journal_and_artifact_store(fresh_config, fresh_journal.clone(), shard.artifact_store.clone()).expect("valid test configuration");

    // Capture fresh shard pre-state: empty pending and journal.
    let fresh_pending = fresh_shard.pending_boundary_snapshot(0, 8);
    let fresh_journal_snapshot = fresh_journal.snapshot()?;

    // The fresh shard must reject because the admission's configured policy (Strict)
    // differs from the actual shard policy (Relaxed), while the pinned artifact
    // digest remains valid.
    let policy_result = fresh_shard.handle_recover(policy_command);
    assert_eq!(
        policy_result,
        Err(crate::RuntimeError::RecoveryCannotResume {
            reason: "admission_policy_mismatch".to_string()
        }),
        "fresh shard must reject retry checkpoint with admission policy mismatch"
    );

    // The rejected policy probe must leave the fresh shard with no run state
    // and empty pending/journal.
    assert_eq!(
        fresh_shard.run_state_get(run_id),
        None,
        "fresh shard run state must be empty after rejected policy mismatch probe"
    );
    assert_eq!(
        fresh_shard.pending_boundary_snapshot(0, 8),
        fresh_pending,
        "fresh shard pending boundaries must be unchanged after rejected policy mismatch probe"
    );
    assert_eq!(
        fresh_journal.snapshot()?,
        fresh_journal_snapshot,
        "fresh shard journal must be unchanged after rejected policy mismatch probe"
    );

    // The original shard's state must also remain unchanged from the first probe.
    assert_eq!(
        shard.run_state_get(run_id),
        Some(&original),
        "original shard run state must be unchanged after both checkpoint probes"
    );
    assert_eq!(
        shard.pending_boundary_snapshot(0, 8),
        original_pending,
        "original shard pending boundaries must be unchanged after both checkpoint probes"
    );
    assert_eq!(
        journal.snapshot()?,
        original_journal,
        "original shard journal must be unchanged after both checkpoint probes"
    );

    // Positive control: a valid failed checkpoint with matching admission,
    // correct artifact identity, and genuine policy provenance must recover
    // successfully. This proves the checkpoint machinery works when fixtures
    // are internally consistent, not just that invalid fixtures are rejected.
    let mut valid_failed = original.clone();
    valid_failed
        .frame
        .mark_failed(vb_core::ids::StepIdx::ZERO)?;
    let valid_admission = valid_failed
        .admission
        .clone()
        .ok_or("positive control admission")?;

    let valid_checkpoint = crate::run_checkpoint::RunCheckpointV1::capture(
        run_id,
        artifact_digest,
        core::num::NonZeroU16::new(2).ok_or("invalid valid checkpoint attempt number")?,
        vb_core::ids::StepIdx::ZERO,
        1,
        &valid_failed,
    )?;

    let valid_retry_frame =
        vb_core::frame::RunFrame::from_checkpoint_v1(valid_checkpoint.frame.clone())
            .map_err(|error| format!("valid retry frame fixture failed: {error:?}"))?;

    let valid_command = RecoverRunCommand::new(
        run_id,
        valid_retry_frame,
        artifact_digest,
        original.workflow.digest(),
        shard
            .journal_seq_get(run_id)
            .ok_or("recovered run must retain its next journal sequence")?,
        crate::primitives::collect::CollectStates::new(),
        crate::recovery::RecoveredRunBoundary::none(),
    )
    .with_validated_admission(valid_admission)
    .with_recovered_accounting(
        original.run_started_millis,
        Some(original.action_attempts.clone()),
        Some(original.journal_bytes_written),
    )
    .with_retry_checkpoint(valid_checkpoint);

    // Fresh shard with matching Relaxed policy. Must recover successfully.
    let positive_config = crate::shard::ShardConfig {
        command_queue_capacity: 64,
        trace_capacity: 64,
        step_budget_per_tick: 4,
        max_active_runs: 8,
        max_open_gates: 1024,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let positive_journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let mut positive_shard = crate::shard::Shard::new_with_journal_and_artifact_store(positive_config, positive_journal.clone(), shard.artifact_store.clone()).expect("valid test configuration");

    let valid_result = positive_shard.handle_recover(valid_command);
    assert!(
        valid_result.is_ok(),
        "valid checkpoint with matching provenance must recover successfully"
    );

    let restored = positive_shard
        .run_state_get(run_id)
        .ok_or("positive control run state not found after recovery")?;

    // Admission restored exactly from the valid checkpoint.
    assert_eq!(
        restored.admission, valid_failed.admission,
        "positive control: admission must match the valid budgeted checkpoint"
    );

    assert_eq!(
        restored.workflow.digest(),
        original.workflow.digest(),
        "positive control: workflow digest must match original"
    );

    assert_eq!(
        restored.registered_execution,
        crate::shard::types::RegisteredExecutionMode::Live,
        "positive control: failed-step retry recovery must restore Live execution"
    );

    // Verify the Do action was rescheduled after positive control recovery.
    let restored_ticket = positive_shard
        .pending_action_get(run_id)
        .ok_or("positive control: pending action not found after recovery")?;
    assert_eq!(restored_ticket.run, run_id);
    assert_eq!(restored_ticket.step, vb_core::ids::StepIdx::ZERO);
    assert_eq!(restored_ticket.action, vb_core::ids::ActionId::new(0));
    assert_eq!(restored_ticket.attempt, 1);

    // Original shard, its journal, and pending state remain unchanged.
    assert_eq!(
        shard.run_state_get(run_id),
        Some(&original),
        "original shard run state must be unchanged after positive control"
    );
    assert_eq!(
        shard.pending_boundary_snapshot(0, 8),
        original_pending,
        "original shard pending boundaries must be unchanged after positive control"
    );
    assert_eq!(
        journal.snapshot()?,
        original_journal,
        "original shard journal must be unchanged after positive control"
    );

    Ok(())
}

fn recovery_gate_digest(byte: u8) -> vb_core::manual_gate::GateDigest {
    vb_core::manual_gate::GateDigest::from_bytes([byte; 32])
}

fn recovery_gate_role() -> vb_core::manual_gate::ApproverRole {
    vb_core::manual_gate::ApproverRole::new("deploy-approver".to_string()).unwrap()
}

fn recovery_gate_person(value: &str) -> vb_core::manual_gate::ApproverId {
    vb_core::manual_gate::ApproverId::new(value.to_string()).unwrap()
}

fn recovery_test_gate(run: vb_core::ids::RunId, id: u64) -> vb_core::manual_gate::ManualGate {
    vb_core::manual_gate::ManualGate {
        id: vb_core::manual_gate::GateId::new(id),
        run,
        action: vb_core::ids::ActionId::new(9),
        risk: vb_core::manual_gate::RiskClass::High,
        compensatable: false,
        required_role: recovery_gate_role(),
        separation_of_duties: vb_core::manual_gate::SeparationOfDuties::DistinctFromRequester,
        evidence_bundle_digest: recovery_gate_digest(1),
        prompt_digest: recovery_gate_digest(2),
        timeout_ms: 1_000,
        escalation_policy_digest: recovery_gate_digest(3),
        break_glass_allowed: true,
        approval_reason_required: true,
    }
}

fn recovery_test_open_gate(run: vb_core::ids::RunId, id: u64) -> crate::shard::types::OpenGate {
    crate::shard::types::OpenGate {
        gate: recovery_test_gate(run, id),
        opened_at_ms: 0,
        requester: recovery_gate_person("bob"),
        delegated_to: None,
    }
}

fn recovery_approve_input(id: u64) -> vb_core::manual_gate::GateDecisionInput {
    vb_core::manual_gate::GateDecisionInput {
        gate: vb_core::manual_gate::GateId::new(id),
        decision: vb_core::manual_gate::GateDecision::Approve,
        approver: recovery_gate_person("alice"),
        approver_role: recovery_gate_role(),
        requester: recovery_gate_person("bob"),
        reason: Some("checked the rollout plan".to_string()),
        evidence_shown: true,
        evidence_bundle_digest: recovery_gate_digest(1),
        delegated_to: None,
        break_glass_justification: None,
        decided_at_ms: 500,
    }
}

#[derive(Debug, Default)]
struct RecoveryGateJournal {
    decisions: std::sync::Mutex<Vec<vb_core::manual_gate::GateOutcomeRecord>>,
}

impl RecoveryGateJournal {
    fn shared() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self::default())
    }

    fn decisions(
        &self,
    ) -> Result<Vec<vb_core::manual_gate::GateOutcomeRecord>, crate::RuntimeError> {
        self.decisions
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| crate::RuntimeError::JournalPoisoned)
    }
}

impl crate::journal::RuntimeJournal for RecoveryGateJournal {
    fn durability_profile(&self) -> Option<vb_storage::DurabilityProfile> {
        Some(vb_storage::DurabilityProfile::Volatile)
    }

    fn append(
        &self,
        _event: crate::journal::RuntimeJournalEvent,
    ) -> crate::error::RuntimeResult<()> {
        Ok(())
    }

    fn append_sequenced_batch(
        &self,
        events: &[crate::journal::RuntimeJournalEvent],
        _start_seq: vb_storage::EventSeq,
    ) -> crate::error::RuntimeResult<()> {
        for event in events {
            self.append(event.clone())?;
        }
        Ok(())
    }

    fn append_gate_decision_strict(
        &self,
        record: vb_core::manual_gate::GateOutcomeRecord,
        _seq: vb_storage::EventSeq,
    ) -> crate::error::RuntimeResult<()> {
        self.decisions
            .lock()
            .map_err(|_| crate::RuntimeError::JournalPoisoned)?
            .push(record);
        Ok(())
    }

    fn append_gate_opened_strict(
        &self,
        _gate: vb_core::manual_gate::ManualGate,
        _opened_at_ms: u64,
        _requester: vb_core::manual_gate::ApproverId,
        _seq: vb_storage::EventSeq,
    ) -> crate::error::RuntimeResult<()> {
        Ok(())
    }

    fn probe(&self) -> crate::error::RuntimeResult<()> {
        Ok(())
    }
}

fn recovery_gate_shard() -> Result<
    (
        crate::shard::Shard,
        std::sync::Arc<RecoveryGateJournal>,
        vb_core::ids::RunId,
        vb_core::ids::WorkflowDigest,
        vb_core::ids::WorkflowDigest,
        crate::admission::RunAdmission,
        vb_storage::EventSeq,
        u64,
        Box<[u16]>,
        u64,
    ),
    Box<dyn std::error::Error>,
> {
    let run_id = vb_core::ids::RunId::new(1);
    let do_node = vb_core::workflow::node(
        vb_core::ids::StepIdx::ZERO,
        vb_core::workflow::CompiledNodeKind::Do {
            action: vb_core::ids::ActionId::new(0),
            input: vb_core::ids::SlotIdx::new(0),
        },
        Some(vb_core::ids::SlotIdx::new(1)),
        Some(vb_core::ids::StepIdx::new(1)),
    );
    let finish_node = vb_core::workflow::node(
        vb_core::ids::StepIdx::new(1),
        vb_core::workflow::CompiledNodeKind::Finish {
            result: vb_core::ids::SlotIdx::new(1),
        },
        None,
        None,
    );
    let parts = vb_core::workflow::WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("recovery-test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([do_node, finish_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        input_slots: Box::from([vb_core::CompiledInputSlot {
            slot: vb_core::SlotIdx::ZERO,
            kind: vb_core::InputSlotKind::Any,
            constraint: vb_core::CompiledInputConstraint::required(),
        }]),
    };
    let contract = action_recovery_test_contract()?;
    let workflow =
        honest_digest_workflow(vb_core::workflow::CompiledWorkflow::try_from_parts(parts)?)?;
    let workflow_digest = workflow.digest();
    let (artifact_digest, store) = action_recovery_contract_store(&workflow, &contract)?;
    let artifact = store.load_accepted_artifact(artifact_digest)?;
    let admission = action_recovery_current_admission(
        run_id,
        &workflow,
        &artifact,
        vb_core::policy::RuntimePolicy::Relaxed,
    )?;
    let started_millis =
        crate::shard::helpers::wall_clock_millis_since_epoch().ok_or("clock unavailable")?;
    let (next_seq, action_attempts, journal_bytes) =
        current_recovery_header_accounting(run_id, &workflow, &admission, started_millis)?;
    let config = crate::shard::ShardConfig {
        command_queue_capacity: 64,
        trace_capacity: 64,
        step_budget_per_tick: 4,
        max_active_runs: 8,
        max_open_gates: 1024,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let journal = RecoveryGateJournal::shared();
    let shard =
        crate::shard::Shard::new_with_journal_and_artifact_store(config, journal.clone(), store).expect("valid test configuration");
    Ok((
        shard,
        journal,
        run_id,
        artifact_digest,
        workflow_digest,
        admission,
        next_seq,
        started_millis,
        action_attempts,
        journal_bytes,
    ))
}

fn recovery_gate_frame(
    run_id: vb_core::ids::RunId,
) -> Result<vb_core::frame::RunFrame, Box<dyn std::error::Error>> {
    let mut frame = vb_core::frame::RunFrame::new(run_id, vb_core::ids::StepIdx::ZERO, 2, 2)?;
    frame.write_slot(
        vb_core::ids::SlotIdx::new(0),
        vb_core::value::SlotValue::I64(0),
    )?;
    Ok(frame)
}

fn recovery_gate_command(
    run_id: vb_core::ids::RunId,
    artifact_digest: vb_core::ids::WorkflowDigest,
    workflow_digest: vb_core::ids::WorkflowDigest,
    admission: crate::admission::RunAdmission,
    next_seq: vb_storage::EventSeq,
    started_millis: u64,
    action_attempts: Box<[u16]>,
    journal_bytes: u64,
    boundary: RecoveredRunBoundary,
    open_gates: Vec<crate::shard::types::OpenGate>,
) -> Result<RecoverRunCommand, Box<dyn std::error::Error>> {
    Ok(RecoverRunCommand::new(
        run_id,
        recovery_gate_frame(run_id)?,
        artifact_digest,
        workflow_digest,
        next_seq,
        crate::primitives::collect::CollectStates::new(),
        boundary,
    )
    .with_validated_admission(admission)
    .with_recovered_accounting(
        Some(started_millis),
        Some(action_attempts),
        Some(journal_bytes),
    )
    .with_open_gates(open_gates))
}

#[test]
fn recover_restores_open_gate_suspends_run_and_approves_to_drive()
-> Result<(), Box<dyn std::error::Error>> {
    let (
        mut shard,
        journal,
        run_id,
        artifact_digest,
        workflow_digest,
        admission,
        next_seq,
        started_millis,
        action_attempts,
        journal_bytes,
    ) = recovery_gate_shard()?;
    let gate = recovery_test_gate(run_id, 7);
    let command = recovery_gate_command(
        run_id,
        artifact_digest,
        workflow_digest,
        admission,
        next_seq,
        started_millis,
        action_attempts,
        journal_bytes,
        RecoveredRunBoundary::none(),
        vec![recovery_test_open_gate(run_id, 7)],
    )?;
    shard.handle_recover(command)?;
    assert_eq!(
        shard.open_gate_for(run_id),
        Some(gate.id),
        "recovered gate must be registered in the shard registry"
    );
    assert_eq!(
        shard.runtime_state_get(run_id),
        Some(crate::shard::types::RuntimeState::Resumable),
        "recovered gated run must stay suspended, not running"
    );
    assert!(
        shard.pending_action_get(run_id).is_none(),
        "gated run must not be driven: no action may be scheduled"
    );
    assert_eq!(
        shard.handle_resume(run_id),
        Err(crate::shard::types::ResumeError::GateOpen {
            run_id,
            gate: gate.id,
        }),
        "resume must stay blocked while the recovered gate is open"
    );
    let record = shard.submit_gate_decision(run_id, recovery_approve_input(7))?;
    assert_eq!(
        record.outcome,
        vb_core::manual_gate::GateOutcome::Approved,
        "approval of the recovered gate must succeed"
    );
    let journaled = journal.decisions()?;
    assert_eq!(
        journaled.len(),
        1,
        "exactly one gate decision must reach the strict journal port"
    );
    assert_eq!(
        journaled[0], record,
        "journaled decision must match the returned record"
    );
    assert_eq!(
        shard.open_gate_for(run_id),
        None,
        "approved gate must be removed from the registry"
    );
    let ticket = shard
        .pending_action_get(run_id)
        .ok_or("approved run must drive and schedule its action")?;
    assert_eq!(ticket.run, run_id);
    assert_eq!(ticket.step, vb_core::ids::StepIdx::ZERO);
    assert_eq!(ticket.action, vb_core::ids::ActionId::new(0));
    assert_eq!(ticket.attempt, 1);
    Ok(())
}

#[test]
fn recover_rejects_open_gate_with_suspended_boundary() -> Result<(), Box<dyn std::error::Error>> {
    let (
        mut shard,
        _journal,
        run_id,
        artifact_digest,
        workflow_digest,
        admission,
        next_seq,
        started_millis,
        action_attempts,
        journal_bytes,
    ) = recovery_gate_shard()?;
    let ask = crate::recovery::RecoveredOpenAsk::new(
        vb_core::ids::StepIdx::ZERO,
        vb_storage::EventSeq::new(1),
    );
    let command = recovery_gate_command(
        run_id,
        artifact_digest,
        workflow_digest,
        admission,
        next_seq,
        started_millis,
        action_attempts,
        journal_bytes,
        RecoveredRunBoundary::from_open_ask(ask),
        vec![recovery_test_open_gate(run_id, 7)],
    )?;
    assert_eq!(
        shard.handle_recover(command),
        Err(crate::RuntimeError::Recovery {
            error: "recovered run has both an open gate and a suspended boundary".to_string(),
        }),
        "gate plus boundary is inconsistent evidence and must fail recovery"
    );
    assert_eq!(
        shard.open_gate_for(run_id),
        None,
        "rejected recovery must not register the gate"
    );
    Ok(())
}

#[test]
fn restore_open_gates_rejects_foreign_run_gate() -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::shard::ShardConfig {
        command_queue_capacity: 64,
        trace_capacity: 64,
        step_budget_per_tick: 4,
        max_active_runs: 8,
        max_open_gates: 1024,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = crate::shard::Shard::new(config).expect("valid test configuration");
    let run_id = vb_core::ids::RunId::new(1);
    let foreign = recovery_test_open_gate(vb_core::ids::RunId::new(99), 7);
    assert_eq!(
        shard.restore_open_gates(run_id, std::slice::from_ref(&foreign)),
        Err(crate::RuntimeError::Recovery {
            error: "recovered open gate 7 belongs to a different run".to_string(),
        }),
        "a gate for another run must fail restore"
    );
    assert_eq!(
        shard.open_gate_for(run_id),
        None,
        "rejected restore must not register the gate"
    );
    Ok(())
}

#[test]
fn restore_open_gates_rejects_duplicate_gate_id() -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::shard::ShardConfig {
        command_queue_capacity: 64,
        trace_capacity: 64,
        step_budget_per_tick: 4,
        max_active_runs: 8,
        max_open_gates: 1024,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = crate::shard::Shard::new(config).expect("valid test configuration");
    let run_id = vb_core::ids::RunId::new(1);
    let open = recovery_test_open_gate(run_id, 7);
    shard.restore_open_gates(run_id, std::slice::from_ref(&open))?;
    assert_eq!(
        shard.restore_open_gates(run_id, std::slice::from_ref(&open)),
        Err(crate::RuntimeError::GateAlreadyOpen {
            gate: vb_core::manual_gate::GateId::new(7),
        }),
        "restoring the same gate id twice must fail"
    );
    assert_eq!(
        shard.open_gate_for(run_id),
        Some(vb_core::manual_gate::GateId::new(7)),
        "the first restored gate must remain registered"
    );
    Ok(())
}

#[test]
fn restore_open_gates_enforces_registry_cap() -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::shard::ShardConfig {
        command_queue_capacity: 64,
        trace_capacity: 64,
        step_budget_per_tick: 4,
        max_active_runs: 1,
        max_open_gates: 2,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = crate::shard::Shard::new(config).expect("valid test configuration");
    let run_id = vb_core::ids::RunId::new(1);
    let first = recovery_test_open_gate(run_id, 7);
    let second = recovery_test_open_gate(run_id, 8);
    let third = recovery_test_open_gate(run_id, 9);
    shard.restore_open_gates(run_id, &[first, second])?;
    assert_eq!(
        shard.restore_open_gates(run_id, std::slice::from_ref(&third)),
        Err(crate::RuntimeError::OpenGateCapacityExceeded { capacity: 2 }),
        "restoring past the registry cap must fail"
    );
    assert_eq!(
        shard.open_gate_for(run_id),
        Some(vb_core::manual_gate::GateId::new(7)),
        "cap failure must not drop the gates already restored"
    );
    Ok(())
}
