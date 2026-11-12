fn crash_lab_minimal_parts(digest: WorkflowDigest) -> WorkflowParts {
    let do_node = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(1)),
    );
    let finish_node = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
        None,
        None,
    );
    WorkflowParts {
        source_metadata: None,
        semantic_source_identity: None,
        name: Box::from("crash-lab-minimal"),
        digest,
        nodes: Box::from([do_node, finish_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([vb_core::CompiledInputSlot {
            slot: SlotIdx::ZERO,
            kind: vb_core::InputSlotKind::Any,
            constraint: vb_core::CompiledInputConstraint::required(),
        }]),
    }
}

fn crash_lab_ticket_count(events: &[RuntimeJournalEvent], run: RunId) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(
                event,
                RuntimeJournalEvent::ActionScheduledTicket { ticket, .. } if ticket.run == run
            )
        })
        .count()
}

fn crash_lab_completion_count(events: &[RuntimeJournalEvent]) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(
                event,
                RuntimeJournalEvent::ActionCompleted { .. }
                    | RuntimeJournalEvent::ActionCompletedEnvelope { .. }
            )
        })
        .count()
}

fn crash_lab_run_finished_count(events: &[RuntimeJournalEvent], run: RunId) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(
                event,
                RuntimeJournalEvent::RunFinished { run: finished, .. } if *finished == run
            )
        })
        .count()
}

fn crash_lab_ticket_seq(
    events: &[RuntimeJournalEvent],
    run: RunId,
    ticket: ActionTicket,
) -> Result<vb_storage::types::EventSeq, Box<dyn std::error::Error>> {
    let ticket_index = events
        .iter()
        .position(|event| {
            matches!(
                event,
                RuntimeJournalEvent::ActionScheduledTicket { ticket: scheduled, .. }
                    if *scheduled == ticket
            )
        })
        .ok_or("journal must contain the pre-crash ActionScheduledTicket")?;
    let run_events_before = events
        .iter()
        .take(ticket_index)
        .filter(|event| event.run_id() == run)
        .count();
    Ok(vb_storage::types::EventSeq::new(u64::try_from(
        run_events_before,
    )?))
}

fn crash_lab_boundary_ticket(
    events: &[RuntimeJournalEvent],
    ticket_seq: vb_storage::types::EventSeq,
) -> Result<crate::recovery::RecoveredPendingActionTicket, Box<dyn std::error::Error>> {
    let Some((ticket, input, output, action_abi_digest)) =
        events.iter().find_map(|event| match event {
            RuntimeJournalEvent::ActionScheduledTicket {
                ticket,
                input,
                output,
                action_abi_digest,
            } => Some((*ticket, *input, *output, *action_abi_digest)),
            _ => None,
        })
    else {
        return Err("journal must contain an ActionScheduledTicket".into());
    };
    Ok(crate::recovery::RecoveredPendingActionTicket::new(
        ticket_seq,
        ticket,
        input,
        output,
        action_abi_digest,
    ))
}

#[test]
fn action_dispatch_crash_points() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(60_014);
    let workflow_digest = WorkflowDigest::from_bytes([42u8; 32]);
    let artifact_digest = WorkflowDigest::from_bytes([43u8; 32]);
    let parts = crash_lab_minimal_parts(workflow_digest);
    let workflow_ir = postcard::to_allocvec(&parts)?;
    let workflow = require_workflow(
        "crash-lab-minimal",
        CompiledWorkflow::try_from_parts(parts).ok(),
    )?;
    let policy_digest = vb_storage::admission::compute_policy_digest(&workflow)?;
    let contract = action_recovery_test_contract()?;
    let store = action_recovery_contract_store(
        artifact_digest,
        workflow_digest,
        policy_digest,
        workflow_ir,
        contract,
    );
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard =
        Shard::new_with_journal_and_artifact_store(small_config(), shared, store.clone());
    submit_run(
        &mut shard,
        run,
        workflow,
        &[(SlotIdx::ZERO, SlotValue::I64(0))],
    );
    assert_eq!(shard.tick(), Ok(true));
    let ticket = shard
        .pending_action_get(run)
        .ok_or("run must have a pending action after tick")?;
    assert_eq!(ticket.run, run);
    assert_eq!(ticket.step, StepIdx::ZERO);
    assert_eq!(ticket.action, ActionId::new(0));
    assert_eq!(ticket.attempt, 1);
    let pre_crash = require_snapshot(&journal)?;
    let ticket_seq = crash_lab_ticket_seq(&pre_crash, run, ticket)?;
    assert_eq!(
        shard.journal_seq_get(run),
        Some(vb_storage::types::EventSeq::new(u64::try_from(
            pre_crash.len()
        )?))
    );
    assert_eq!(crash_lab_ticket_count(&pre_crash, run), 1);
    assert!(matches!(
        pre_crash.last(),
        Some(RuntimeJournalEvent::ActionScheduledTicket { ticket: scheduled, .. })
            if scheduled.run == run
                && scheduled.step == StepIdx::ZERO
                && scheduled.action == ActionId::new(0)
    ));
    assert_eq!(crash_lab_completion_count(&pre_crash), 0);
    assert_eq!(crash_lab_run_finished_count(&pre_crash, run), 0);
    let frame = shard
        .run_state_get(run)
        .ok_or("run must have state before the crash")?
        .frame
        .clone();
    assert_eq!(
        frame.step_state(StepIdx::ZERO),
        Ok(vb_core::frame::StepState::Running)
    );
    assert_eq!(frame.read_slot(SlotIdx::ZERO), Ok(&SlotValue::I64(0)));
    drop(shard);
    let post_crash = require_snapshot(&journal)?;
    assert_eq!(post_crash, pre_crash);
    let mut recovered =
        Shard::new_with_journal_and_artifact_store(small_config(), journal.clone(), store);
    let boundary_ticket = crash_lab_boundary_ticket(&pre_crash, ticket_seq)?;
    let boundary = crate::recovery::RecoveredRunBoundary::from_pending_action(boundary_ticket);
    let command = crate::shard::types::RecoverRunCommand::new(
        run,
        frame,
        artifact_digest,
        workflow_digest,
        vb_storage::types::EventSeq::new(u64::try_from(pre_crash.len())?),
        crate::primitives::collect::CollectStates::new(),
        boundary,
    );
    recovered.handle_recover(command)?;
    assert_eq!(recovered.pending_action_get(run), Some(ticket));
    let post_recovery = require_snapshot(&journal)?;
    assert_eq!(crash_lab_ticket_count(&post_recovery, run), 1);
    assert_eq!(crash_lab_completion_count(&post_recovery), 0);
    assert!(post_recovery.starts_with(&pre_crash));
    let output = ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value: SlotValue::I64(42),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    recovered.enqueue(ShardCommand::ActionCompleted { ticket, output })?;
    assert_eq!(recovered.tick(), Ok(true));
    assert!(recovered.terminal_runs_contains(run));
    assert!(!recovered.run_state_contains(run));
    let finished = require_snapshot(&journal)?;
    assert_eq!(crash_lab_ticket_count(&finished, run), 1);
    assert_eq!(crash_lab_completion_count(&finished), 1);
    assert_eq!(crash_lab_run_finished_count(&finished, run), 1);
    assert!(matches!(
        finished.iter().find(|event| matches!(
            event,
            RuntimeJournalEvent::RunFinished { .. }
        )),
        Some(RuntimeJournalEvent::RunFinished {
            run: finished_run,
            result,
            attempt: 1
        }) if *finished_run == run && *result == SlotIdx::new(1)
    ));
    assert!(finished.starts_with(&pre_crash));
    recovered.enqueue(ShardCommand::ActionCompleted {
        ticket,
        output: ActionOutputReady {
            output_slot: SlotIdx::new(1),
            value: SlotValue::I64(99),
            taint: Taint::Clean,
            encoded_len: 2,
        },
    })?;
    assert_eq!(recovered.tick(), Err(RuntimeError::RunNotFound));
    let after_dup = require_snapshot(&journal)?;
    assert_eq!(after_dup, finished);
    assert_eq!(crash_lab_run_finished_count(&after_dup, run), 1);
    Ok(())
}
