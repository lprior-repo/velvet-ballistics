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
    digest: vb_core::ids::WorkflowDigest,
    source_digest: vb_core::ids::WorkflowDigest,
    policy_digest: vb_core::ids::WorkflowDigest,
    workflow_ir: Vec<u8>,
    contract: vb_core::action::ActionContract,
) -> crate::admission::SharedAcceptedArtifactStore {
    #[derive(Debug)]
    struct ActionRecoveryContractStore {
        digest: vb_core::ids::WorkflowDigest,
        source_digest: vb_core::ids::WorkflowDigest,
        policy_digest: vb_core::ids::WorkflowDigest,
        workflow_ir: Vec<u8>,
        contract: vb_core::action::ActionContract,
    }

    impl crate::admission::AcceptedArtifactStore for ActionRecoveryContractStore {
        fn load_accepted_artifact(
            &self,
            artifact_digest: vb_core::ids::WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, crate::admission::ArtifactEnvelopeError>
        {
            if artifact_digest != self.digest {
                return Err(crate::admission::ArtifactEnvelopeError::ArtifactNotFound {
                    digest: artifact_digest,
                });
            }
            Ok(vb_storage::admission::AcceptedArtifact {
                digest: self.digest,
                source_digest: self.source_digest,
                policy_digest: self.policy_digest,
                ir: self.workflow_ir.clone(),
                verification: vb_storage::admission::VerificationProof {
                    evidence: vb_core::artifact::ArtifactEvidence::Unverified,
                    digest: self.digest,
                    gate_count: 8,
                    durable: true,
                    bounded_claimed: true,
                    taint_safe_claimed: true,
                    retry_safe_claimed: true,
                    idempotency_verified_claimed: true,
                    replayable_claimed: true,
                    idempotency_keyed: Box::new([]),
                    idempotency_attested: Box::new([]),
                    warnings: Vec::new(),
                },
                accepted_at_seq: vb_storage::types::EventSeq::new(0),
                required_capabilities: Box::new([]),
                action_contracts: Box::from([self.contract.clone()]),
            })
        }
    }

    std::sync::Arc::new(ActionRecoveryContractStore {
        digest,
        source_digest,
        policy_digest,
        workflow_ir,
        contract,
    })
}

#[test]
fn recover_restores_action_contracts_schedules_completes_and_finishes()
-> Result<(), Box<dyn std::error::Error>> {
    let run_id = vb_core::ids::RunId::new(1);
    let workflow_digest = vb_core::ids::WorkflowDigest::from_bytes([42u8; 32]);
    // vb-peb9u: artifact identity differs from workflow identity. The artifact
    // digest is the content hash of the compiled artifact; the workflow digest
    // is the semantic hash of the workflow. Mocked as distinct byte patterns.
    let artifact_digest = vb_core::ids::WorkflowDigest::from_bytes([43u8; 32]);
    assert_ne!(artifact_digest, workflow_digest);

    let do_node = vb_core::workflow::CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: Some(vb_core::ids::SlotIdx::new(1)),
        next: Some(vb_core::ids::StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: vb_core::workflow::CompiledNodeKind::Do {
            action: vb_core::ids::ActionId::new(0),
            input: vb_core::ids::SlotIdx::new(0),
        },
    };
    let finish_node = vb_core::workflow::CompiledNode {
        id: vb_core::ids::StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: vb_core::workflow::CompiledNodeKind::Finish {
            result: vb_core::ids::SlotIdx::new(1),
        },
    };
    let parts = vb_core::workflow::WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("recovery-test"),
        digest: workflow_digest,
        nodes: Box::from([do_node, finish_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };

    let workflow_ir = postcard::to_allocvec(&parts)?;
    let contract = action_recovery_test_contract()?;

    // Compute the actual policy digest from the workflow for fixture provenance.
    // The parts are consumed here after serialization, so the mock store can
    // carry the genuine computed digest rather than a made-up one.
    let policy_digest = vb_storage::admission::compute_policy_digest(
        &vb_core::workflow::CompiledWorkflow::try_from_parts(parts)?,
    )?;

    let config = crate::shard::ShardConfig {
        command_queue_capacity: 64,
        trace_capacity: 64,
        step_budget_per_tick: 4,
        max_active_runs: 8,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    // vb-peb9u: Mocked store with internally consistent provenance. The artifact
    // digest is content hash [43u8;32]; source digest matches workflow identity
    // [42u8;32]; policy digest is the real computed value. Full admission would
    // accept these together; the Unverified pin remains the test subject.
    let store = action_recovery_contract_store(
        artifact_digest,
        workflow_digest,
        policy_digest,
        workflow_ir,
        contract.clone(),
    );
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared_journal = journal.clone();
    let mut shard = crate::shard::Shard::new_with_journal_and_artifact_store(
        config,
        shared_journal,
        store.clone(),
    );

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
        vb_storage::types::EventSeq::new(1),
        crate::primitives::collect::CollectStates::new(),
        RecoveredRunBoundary::none(),
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

    // vb-peb9u: Raw 7-argument recovery cannot gain a verified policy pin.
    // The Unverified admission record is built from the accepted artifact's
    // required capabilities and current runtime policy, but the pin remains
    // Unverified. This must never promote to Pinned.
    let admission = state
        .admission
        .as_ref()
        .ok_or("raw recovery must build an Unverified admission from the accepted artifact")?;
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
        vb_storage::RunPolicyPin::Unverified,
        "raw 7-argument recovery must remain Unverified; cannot gain policy pin"
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

    // vb-peb9u: Raw 7-argument recovery produces Unverified admission (no policy pin).
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
        vb_storage::types::EventSeq::new(2),
        crate::primitives::collect::CollectStates::new(),
        crate::recovery::RecoveredRunBoundary::none(),
    )
    .with_retry_checkpoint(checkpoint);

    // The shard must reject this because the checkpoint's policy pin (forged to
    // [44u8;32]) does not match the artifact's actual policy digest (derived
    // from [43u8;32]).
    let result = shard.handle_recover(command);
    assert_eq!(
        result,
        Err(crate::RuntimeError::RecoveryCannotResume {
            reason: "admission_policy_pin_mismatch".to_string()
        }),
        "shard must reject retry checkpoint with mismatched policy pin"
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

    // Second probe: admission policy mismatch (Strict configured vs Relaxed
    // actual shard policy) with a valid Unverified historical pin. The pin check
    // passes, but the configured policy in the admission record does not match
    // the shard's actual runtime policy. This should be rejected with
    // admission_policy_mismatch.
    // Use a fresh shard so the run is not already tracked (avoids RunAlreadyExists
    // masking the policy admission check).
    let mut failed_policy = original.clone();
    failed_policy
        .frame
        .mark_failed(vb_core::ids::StepIdx::ZERO)?;

    let strict_admission = crate::admission::RunAdmission::new(
        artifact_digest,
        run_id,
        original
            .admission
            .as_ref()
            .ok_or("original must carry admission record")?
            .granted_capabilities()
            .clone(),
        vb_core::policy::RuntimePolicy::Strict,
    )
    .with_policy_pin(vb_storage::RunPolicyPin::Unverified);
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
        vb_storage::types::EventSeq::new(2),
        crate::primitives::collect::CollectStates::new(),
        crate::recovery::RecoveredRunBoundary::none(),
    )
    .with_retry_checkpoint(policy_checkpoint);

    // Fresh shard with matching Relaxed policy, same config as original.
    let fresh_config = crate::shard::ShardConfig {
        command_queue_capacity: 64,
        trace_capacity: 64,
        step_budget_per_tick: 4,
        max_active_runs: 8,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let fresh_journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let mut fresh_shard = crate::shard::Shard::new_with_journal_and_artifact_store(
        fresh_config,
        fresh_journal.clone(),
        shard.artifact_store.clone(),
    );

    // Capture fresh shard pre-state: empty pending and journal.
    let fresh_pending = fresh_shard.pending_boundary_snapshot(0, 8);
    let fresh_journal_snapshot = fresh_journal.snapshot()?;

    // The fresh shard must reject because the admission's configured policy (Strict)
    // differs from the actual shard policy (Relaxed), even though the pin is
    // Unverified (historical) and therefore passes the pin check.
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
        vb_storage::types::EventSeq::new(2),
        crate::primitives::collect::CollectStates::new(),
        crate::recovery::RecoveredRunBoundary::none(),
    )
    .with_retry_checkpoint(valid_checkpoint);

    // Fresh shard with matching Relaxed policy. Must recover successfully.
    let positive_config = crate::shard::ShardConfig {
        command_queue_capacity: 64,
        trace_capacity: 64,
        step_budget_per_tick: 4,
        max_active_runs: 8,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let positive_journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let mut positive_shard = crate::shard::Shard::new_with_journal_and_artifact_store(
        positive_config,
        positive_journal.clone(),
        shard.artifact_store.clone(),
    );

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
        restored.admission, original.admission,
        "positive control: admission must match original (Unverified, correct run/artifact/policy)"
    );

    assert_eq!(
        restored.workflow.digest(),
        original.workflow.digest(),
        "positive control: workflow digest must match original"
    );

    assert_eq!(
        restored.registered_execution,
        crate::shard::types::RegisteredExecutionMode::ReconcileOnly,
        "positive control: execution must be ReconcileOnly for recovered run"
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
