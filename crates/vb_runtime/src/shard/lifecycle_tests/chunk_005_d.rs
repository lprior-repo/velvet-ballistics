#[test]
fn retry_await_charges_second_action_scheduled_ticket_bytes() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let run = RunId::new(778);
    submit_run(
        &mut shard,
        run,
        retry_workflow()?,
        &[(SlotIdx::ZERO, SlotValue::I64(0))],
    );
    let first_counter = shard
        .run_state_get(run)
        .ok_or_else(|| "run should still be active awaiting action".to_string())?
        .journal_bytes_written;
    let first_events = require_snapshot(&journal)?;
    let first_measured = measured_journal_bytes(&first_events)?;
    assert_eq!(
        first_events
            .iter()
            .filter(|event| {
                matches!(event, RuntimeJournalEvent::ActionScheduledTicket { ticket, .. } if ticket.run == run)
            })
            .count(),
        1
    );
    enqueue_action_failure(&mut shard, run, StepIdx::new(1), 1);
    let second_counter = shard
        .run_state_get(run)
        .ok_or_else(|| "run should still be active awaiting the retried action".to_string())?
        .journal_bytes_written;
    let second_events = require_snapshot(&journal)?;
    let second_measured = measured_journal_bytes(&second_events)?;
    let appended = second_measured
        .checked_sub(first_measured)
        .ok_or_else(|| "journal must not shrink across the retry".to_string())?;
    let delta = second_events
        .get(first_events.len()..)
        .ok_or_else(|| "retry snapshot must extend the submit snapshot".to_string())?;
    assert_eq!(
        delta
            .iter()
            .filter(|event| {
                matches!(event, RuntimeJournalEvent::ActionFailed { run: failed_run, .. } if *failed_run == run)
            })
            .count(),
        1
    );
    let second_tickets: Vec<&RuntimeJournalEvent> = second_events
        .iter()
        .filter(|event| {
            matches!(event, RuntimeJournalEvent::ActionScheduledTicket { ticket, .. } if ticket.run == run)
        })
        .collect();
    assert_eq!(second_tickets.len(), 2);
    let second_ticket_bytes = u64::try_from(
        postcard::to_allocvec(
            second_tickets
                .get(1)
                .ok_or_else(|| "second scheduled ticket must be journaled".to_string())?,
        )
        .map_err(|error| format!("event encode failed: {error:?}"))?
        .len(),
    )
    .map_err(|_| "length overflow".to_string())?;
    let charged = second_counter
        .checked_sub(first_counter)
        .ok_or_else(|| "counter must not decrease across the retry".to_string())?;
    // Command-path retries charge every appended event, including ActionFailed.
    assert_eq!(charged, appended);
    assert!(
        charged > second_ticket_bytes,
        "retry drive evidence must be charged alongside the ticket"
    );
    Ok(())
}

#[test]
fn empty_evidence_fail_charges_run_failed_bytes() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("tiny_journal_finished", tiny_journal_finished_workflow())?;
    let run = RunId::new(779);
    let digest = wf.digest();
    let mut state = crate::shard::helpers::make_run_state(wf, run)
        .ok_or_else(|| "run state fixture must build".to_string())?;
    state.store = vb_core::value_store::ValueStore::with_max_slots(
        state.workflow.resource_contract().max_slots,
    );
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        journal_bytes_budget(100_000),
    ));
    shard
        .admit_run_state(run, state, super::super::RuntimeState::Initial)
        .map_err(|error| error.to_string())?;
    let state = shard
        .take_run_state(run)
        .map_err(|error| error.to_string())?;
    assert!(matches!(
        shard
            .fail_run_state(run, state)
            .map_err(|error| error.to_string())?,
        crate::shard::transitions::TransitionOutcome::Applied
    ));
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    assert!(shard.terminal_runs_contains(run));
    let events = require_snapshot(&journal)?;
    let failed = events
        .iter()
        .find(|event| {
            matches!(event, RuntimeJournalEvent::RunFailed { run: failed_run, .. } if *failed_run == run)
        })
        .ok_or_else(|| "RunFailed must be journaled".to_string())?;
    let failed_len = u64::try_from(
        postcard::to_allocvec(failed)
            .map_err(|error| format!("event encode failed: {error:?}"))?
            .len(),
    )
    .map_err(|_| "length overflow".to_string())?;
    let checkpoint_event = events
        .iter()
        .find(|event| {
            matches!(event, RuntimeJournalEvent::RetryCheckpoint { run: checkpoint_run, .. } if *checkpoint_run == run)
        })
        .ok_or_else(|| "RetryCheckpoint must be journaled".to_string())?;
    let checkpoint_event_len = u64::try_from(
        postcard::to_allocvec(checkpoint_event)
            .map_err(|error| format!("event encode failed: {error:?}"))?
            .len(),
    )
    .map_err(|_| "length overflow".to_string())?;
    let RuntimeJournalEvent::RetryCheckpoint { payload, .. } = checkpoint_event else {
        return Err("event shape changed".to_string());
    };
    let checkpoint = postcard::from_bytes::<crate::run_checkpoint::RunCheckpointV1>(payload)
        .map_err(|error| format!("checkpoint decode failed: {error:?}"))?;
    // The stored counter includes the RunFailed event charged before capture
    // and the RetryCheckpoint event's own postcard bytes.
    assert_eq!(
        checkpoint.journal_bytes_written,
        failed_len
            .checked_add(checkpoint_event_len)
            .ok_or_else(|| "counter total overflow".to_string())?
    );
    Ok(())
}

#[test]
fn journal_bytes_after_append_projects_measured_total() -> Result<(), String> {
    let workflow = two_slot_workflow().ok_or_else(|| "two slot workflow builds".to_string())?;
    let run = RunId::new(780);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .ok_or_else(|| "run state builds".to_string())?;
    let event = RuntimeJournalEvent::RunFailed { run, attempt: 1 };
    let len = u64::try_from(
        postcard::to_allocvec(&event)
            .map_err(|error| format!("event encode failed: {error:?}"))?
            .len(),
    )
    .map_err(|_| "length overflow".to_string())?;
    assert_eq!(
        Shard::journal_bytes_after_append(&state, &[event])
            .map_err(|error| format!("projection failed: {error:?}"))?,
        len
    );
    assert_eq!(state.journal_bytes_written, 0);
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        journal_bytes_budget(len),
    ));
    assert!(!Shard::journal_bytes_exceeds_budget(&state, len));
    assert!(Shard::journal_bytes_exceeds_budget(
        &state,
        len.checked_add(1)
            .ok_or_else(|| "projection overflow".to_string())?
    ));
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        journal_bytes_budget(
            len.checked_sub(1)
                .ok_or_else(|| "budget underflow".to_string())?,
        ),
    ));
    assert!(Shard::journal_bytes_exceeds_budget(&state, len));
    Ok(())
}

#[test]
fn empty_evidence_finish_within_budget_applies() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("tiny_journal_finished", tiny_journal_finished_workflow())?;
    let run = RunId::new(781);
    let digest = wf.digest();
    let mut state = crate::shard::helpers::make_run_state(wf, run)
        .ok_or_else(|| "run state fixture must build".to_string())?;
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        journal_bytes_budget(100_000),
    ));
    shard
        .admit_run_state(run, state, super::super::RuntimeState::Initial)
        .map_err(|error| error.to_string())?;
    let state = shard
        .take_run_state(run)
        .map_err(|error| error.to_string())?;
    assert!(matches!(
        shard
            .finish_run(run, state)
            .map_err(|error| error.to_string())?,
        crate::shard::transitions::TransitionOutcome::Applied
    ));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert!(shard.terminal_runs_contains(run));
    let events = require_snapshot(&journal)?;
    assert_eq!(
        events
            .iter()
            .filter(|event| {
                matches!(event, RuntimeJournalEvent::RunFinished { run: finished_run, .. } if *finished_run == run)
            })
            .count(),
        1
    );
    Ok(())
}

#[test]
fn empty_evidence_finish_over_budget_fails_run_instead() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("tiny_journal_finished", tiny_journal_finished_workflow())?;
    let run = RunId::new(782);
    let digest = wf.digest();
    let mut state = crate::shard::helpers::make_run_state(wf, run)
        .ok_or_else(|| "run state fixture must build".to_string())?;
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        journal_bytes_budget(0),
    ));
    shard
        .admit_run_state(run, state, super::super::RuntimeState::Initial)
        .map_err(|error| error.to_string())?;
    let state = shard
        .take_run_state(run)
        .map_err(|error| error.to_string())?;
    let state = match shard
        .finish_run(run, state)
        .map_err(|error| error.to_string())?
    {
        crate::shard::transitions::TransitionOutcome::JournalBudgetExceeded(state) => *state,
        crate::shard::transitions::TransitionOutcome::Applied => {
            return Err("finish must not apply over budget".to_string());
        }
    };
    assert_eq!(state.journal_bytes_written, 0);
    assert_eq!(shard.counters().snapshot().runs_completed, 0);
    assert!(!shard.terminal_runs_contains(run));
    let events = require_snapshot(&journal)?;
    assert!(
        !events.iter().any(|event| {
            matches!(event, RuntimeJournalEvent::RunFinished { run: finished_run, .. } if *finished_run == run)
        }),
        "finished event must not be journaled over budget: {events:?}"
    );
    Ok(())
}

#[test]
fn empty_evidence_fail_over_budget_returns_state_unjournaled() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("tiny_journal_finished", tiny_journal_finished_workflow())?;
    let run = RunId::new(783);
    let digest = wf.digest();
    let mut state = crate::shard::helpers::make_run_state(wf, run)
        .ok_or_else(|| "run state fixture must build".to_string())?;
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        journal_bytes_budget(0),
    ));
    shard
        .admit_run_state(run, state, super::super::RuntimeState::Initial)
        .map_err(|error| error.to_string())?;
    let state = shard
        .take_run_state(run)
        .map_err(|error| error.to_string())?;
    let state = match shard
        .fail_run_state(run, state)
        .map_err(|error| error.to_string())?
    {
        crate::shard::transitions::TransitionOutcome::JournalBudgetExceeded(state) => *state,
        crate::shard::transitions::TransitionOutcome::Applied => {
            return Err("fail must not apply over budget".to_string());
        }
    };
    assert_eq!(state.journal_bytes_written, 0);
    assert!(!shard.terminal_runs_contains(run));
    let events = require_snapshot(&journal)?;
    assert!(
        events.is_empty(),
        "nothing must be journaled over budget: {events:?}"
    );
    Ok(())
}

#[test]
fn empty_evidence_await_timer_over_budget_returns_state_unjournaled() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("tiny_journal_finished", tiny_journal_finished_workflow())?;
    let run = RunId::new(784);
    let digest = wf.digest();
    let mut state = crate::shard::helpers::make_run_state(wf, run)
        .ok_or_else(|| "run state fixture must build".to_string())?;
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        journal_bytes_budget(0),
    ));
    shard
        .admit_run_state(run, state, super::super::RuntimeState::Initial)
        .map_err(|error| error.to_string())?;
    let state = shard
        .take_run_state(run)
        .map_err(|error| error.to_string())?;
    let state = match shard
        .await_timer(run, state, PendingTimerKind::Ask)
        .map_err(|error| error.to_string())?
    {
        crate::shard::transitions::TransitionOutcome::JournalBudgetExceeded(state) => *state,
        crate::shard::transitions::TransitionOutcome::Applied => {
            return Err("ask schedule must not apply over budget".to_string());
        }
    };
    assert_eq!(state.journal_bytes_written, 0);
    let events = require_snapshot(&journal)?;
    assert!(
        events.is_empty(),
        "nothing must be journaled over budget: {events:?}"
    );
    Ok(())
}

fn two_ask_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_prompt = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(1)),
    );
    let set_timeout = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(1),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(2)),
    );
    let ask_first = vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::Ask {
            prompt: SlotIdx::ZERO,
            timeout_slot: Some(SlotIdx::new(1)).into(),
        },
        Some(vb_core::ids::SlotIdx::new(2)),
        Some(StepIdx::new(3)),
    );
    let resume_first = vb_core::workflow::node(
        StepIdx::new(3),
        CompiledNodeKind::AskResume {
            answer: SlotIdx::new(2),
        },
        None,
        Some(StepIdx::new(4)),
    );
    let ask_second = vb_core::workflow::node(
        StepIdx::new(4),
        CompiledNodeKind::Ask {
            prompt: SlotIdx::ZERO,
            timeout_slot: Some(SlotIdx::new(1)).into(),
        },
        Some(vb_core::ids::SlotIdx::new(2)),
        Some(StepIdx::new(5)),
    );
    let resume_second = vb_core::workflow::node(
        StepIdx::new(5),
        CompiledNodeKind::AskResume {
            answer: SlotIdx::new(2),
        },
        None,
        Some(StepIdx::new(6)),
    );
    let finish_node = vb_core::workflow::node(
        StepIdx::new(6),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(2),
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("two_ask_then_finish"),
        digest: WorkflowDigest::from_bytes([41; 32]),
        nodes: Box::from([
            set_prompt,
            set_timeout,
            ask_first,
            resume_first,
            ask_second,
            resume_second,
            finish_node,
        ]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([
            ConstValue::Symbol(vb_core::ids::SymbolId::new(1)),
            ConstValue::I64(10),
        ]),
        slot_count: 3,
        symbols_count: 2,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn two_wait_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_deadline = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(1)),
    );
    let wait_first = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::ZERO,
        },
        None,
        Some(StepIdx::new(2)),
    );
    let wait_second = vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::ZERO,
        },
        None,
        Some(StepIdx::new(3)),
    );
    let finish_node = vb_core::workflow::node(
        StepIdx::new(3),
        CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("two_wait_then_finish"),
        digest: WorkflowDigest::from_bytes([42; 32]),
        nodes: Box::from([set_deadline, wait_first, wait_second, finish_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(10)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn two_action_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let first = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::ZERO,
        },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(1)),
    );
    let second = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::ZERO,
        },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(2)),
    );
    let finish_node = vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("two_action_then_finish"),
        digest: WorkflowDigest::from_bytes([43; 32]),
        nodes: Box::from([first, second, finish_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([vb_core::CompiledInputSlot {
            slot: vb_core::SlotIdx::new(0),
            kind: vb_core::InputSlotKind::Any,
            constraint: vb_core::CompiledInputConstraint::required(),
        }]),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn assert_counter_matches_journal(
    shard: &Shard,
    journal: &crate::journal::VolatileRuntimeJournal,
    run: RunId,
) -> Result<(), String> {
    let state = shard
        .run_state_get(run)
        .ok_or_else(|| "run should still be active".to_string())?;
    let events = require_snapshot(journal)?;
    let measured = measured_journal_bytes(&events)?;
    assert!(measured > 0, "journal must contain events");
    assert_eq!(state.journal_bytes_written, measured);
    Ok(())
}

fn cap_journal_budget(
    shard: &mut Shard,
    run: RunId,
    digest: WorkflowDigest,
    total: u64,
) -> Result<(), String> {
    let state = shard
        .run_state_get_mut(run)
        .ok_or_else(|| "run should still be active to cap its budget".to_string())?;
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        journal_bytes_budget(total),
    ));
    Ok(())
}

fn counter_before(shard: &Shard, run: RunId) -> Result<u64, String> {
    shard
        .run_state_get(run)
        .map(|state| state.journal_bytes_written)
        .ok_or_else(|| "run should still be active".to_string())
}

fn completion_output() -> ActionOutputReady {
    ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    }
}

fn count_events(
    events: &[RuntimeJournalEvent],
    run: RunId,
    matches: fn(&RuntimeJournalEvent, RunId) -> bool,
) -> usize {
    events
        .iter()
        .filter(|event| matches(event, run))
        .count()
}

fn is_action_completed(event: &RuntimeJournalEvent, run: RunId) -> bool {
    matches!(event, RuntimeJournalEvent::ActionCompletedEnvelope { ticket, .. } if ticket.run == run)
}

fn is_ask_answered(event: &RuntimeJournalEvent, run: RunId) -> bool {
    matches!(event, RuntimeJournalEvent::AskAnswered { run: r, .. } if *r == run)
}

fn is_wait_resolved(event: &RuntimeJournalEvent, run: RunId) -> bool {
    matches!(event, RuntimeJournalEvent::WaitResolved { run: r, .. } if *r == run)
}

fn is_run_cancelled(event: &RuntimeJournalEvent, run: RunId) -> bool {
    matches!(event, RuntimeJournalEvent::RunCancelled { run: r, .. } if *r == run)
}

fn is_run_killed(event: &RuntimeJournalEvent, run: RunId) -> bool {
    matches!(event, RuntimeJournalEvent::RunKilled { run: r, .. } if *r == run)
}

fn is_action_abandoned(event: &RuntimeJournalEvent, run: RunId) -> bool {
    matches!(event, RuntimeJournalEvent::ActionAbandoned { ticket } if ticket.run == run)
}

fn is_resumed(event: &RuntimeJournalEvent, run: RunId) -> bool {
    matches!(event, RuntimeJournalEvent::Resumed { run: r, .. } if *r == run)
}

#[test]
fn command_journal_bytes_exceeds_boundary() -> Result<(), String> {
    let workflow = two_slot_workflow().ok_or_else(|| "two slot workflow builds".to_string())?;
    let run = RunId::new(780);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .ok_or_else(|| "run state builds".to_string())?;
    let event = RuntimeJournalEvent::RunFailed { run, attempt: 1 };
    let bytes = postcard::to_allocvec(&event)
        .map_err(|error| format!("event encode failed: {error:?}"))?;
    let len = u64::try_from(bytes.len()).map_err(|_| "event length overflow".to_string())?;
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        journal_bytes_budget(len),
    ));
    assert!(
        !Shard::command_journal_bytes_exceeds(&state, &[event.clone()])
            .map_err(|error| format!("probe failed: {error:?}"))?,
        "appending exactly up to the budget must not exceed it"
    );
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        journal_bytes_budget(len.checked_sub(1).ok_or_else(|| "budget underflow".to_string())?),
    ));
    assert!(
        Shard::command_journal_bytes_exceeds(&state, &[event.clone()])
            .map_err(|error| format!("probe failed: {error:?}"))?,
        "one byte over the budget must exceed it"
    );
    state.admission = None;
    assert!(
        !Shard::command_journal_bytes_exceeds(&state, &[event])
            .map_err(|error| format!("probe failed: {error:?}"))?,
        "no admitted budget must never exceed"
    );
    Ok(())
}

#[test]
fn command_completion_charges_journal_bytes() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("two_action", two_action_workflow())?;
    let run = RunId::new(781);
    submit_run(&mut shard, run, wf, &[(SlotIdx::ZERO, SlotValue::I64(0))]);
    let ticket = pending_ticket(&shard, run, StepIdx::ZERO, 1);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket,
            output: completion_output(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    assert_eq!(count_events(&require_snapshot(&journal)?, run, is_action_completed), 1);
    assert_counter_matches_journal(&shard, &journal, run)
}

#[test]
fn command_completion_falls_back_to_single_run_failed_when_journal_budget_exceeded(
) -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = retry_workflow()?;
    let digest = wf.digest();
    let run = RunId::new(782);
    submit_run(&mut shard, run, wf, &[(SlotIdx::ZERO, SlotValue::I64(0))]);
    let before = counter_before(&shard, run)?;
    cap_journal_budget(&mut shard, run, digest, before)?;
    let ticket = pending_ticket(&shard, run, StepIdx::new(1), 1);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket,
            output: completion_output(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    assert!(shard.terminal_runs_contains(run));
    let events = require_snapshot(&journal)?;
    assert_eq!(run_failed_count(&events, run), 1);
    assert_eq!(
        count_events(&events, run, is_action_completed),
        0,
        "completion must not be journaled after fallback: {events:?}"
    );
    Ok(())
}

#[test]
fn command_legacy_completion_charges_journal_bytes() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("two_action", two_action_workflow())?;
    let run = RunId::new(783);
    submit_run(&mut shard, run, wf, &[(SlotIdx::ZERO, SlotValue::I64(0))]);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run,
            step: StepIdx::ZERO,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    let events = require_snapshot(&journal)?;
    assert!(
        events.iter().any(|event| matches!(
            event,
            RuntimeJournalEvent::StepSucceeded { run: r, step, .. }
                if *r == run && *step == StepIdx::ZERO
        )),
        "legacy completion must journal StepSucceeded for the completed step: {events:?}"
    );
    assert_counter_matches_journal(&shard, &journal, run)
}

#[test]
fn command_retryable_failure_charges_journal_bytes() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = retry_workflow()?;
    let run = RunId::new(784);
    submit_run(&mut shard, run, wf, &[(SlotIdx::ZERO, SlotValue::I64(0))]);
    enqueue_action_failure(&mut shard, run, StepIdx::new(1), 1);
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    assert_eq!(
        action_failed_count(&require_snapshot(&journal)?, run, StepIdx::new(1)),
        1
    );
    assert_counter_matches_journal(&shard, &journal, run)
}

#[test]
fn command_retryable_failure_falls_back_when_journal_budget_exceeded() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = retry_workflow()?;
    let digest = wf.digest();
    let run = RunId::new(785);
    submit_run(&mut shard, run, wf, &[(SlotIdx::ZERO, SlotValue::I64(0))]);
    let before = counter_before(&shard, run)?;
    cap_journal_budget(&mut shard, run, digest, before)?;
    enqueue_action_failure(&mut shard, run, StepIdx::new(1), 1);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    assert!(shard.terminal_runs_contains(run));
    let events = require_snapshot(&journal)?;
    assert_eq!(run_failed_count(&events, run), 1);
    assert_eq!(
        action_failed_count(&events, run, StepIdx::new(1)),
        0,
        "failure must not be journaled after fallback: {events:?}"
    );
    Ok(())
}

#[test]
fn command_fail_run_failure_records_fallback_only_when_journal_budget_exceeded(
) -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = retry_workflow()?;
    let digest = wf.digest();
    let run = RunId::new(786);
    submit_run(&mut shard, run, wf, &[(SlotIdx::ZERO, SlotValue::I64(0))]);
    let before = counter_before(&shard, run)?;
    cap_journal_budget(&mut shard, run, digest, before)?;
    let ticket = pending_ticket(&shard, run, StepIdx::new(1), 1);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    assert!(shard.terminal_runs_contains(run));
    let events = require_snapshot(&journal)?;
    assert_eq!(run_failed_count(&events, run), 1);
    assert_eq!(
        action_failed_count(&events, run, StepIdx::new(1)),
        0,
        "failure must not be journaled when only the fallback fits: {events:?}"
    );
    Ok(())
}

#[test]
fn command_fail_run_failure_records_both_events_when_budget_allows() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = retry_workflow()?;
    let run = RunId::new(787);
    submit_run(&mut shard, run, wf, &[(SlotIdx::ZERO, SlotValue::I64(0))]);
    let ticket = pending_ticket(&shard, run, StepIdx::new(1), 1);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    assert!(shard.terminal_runs_contains(run));
    let events = require_snapshot(&journal)?;
    assert_event_order(
        &events,
        RuntimeJournalEvent::ActionFailed {
            run,
            step: StepIdx::new(1),
            action: ActionId::new(0),
            attempt: 1,
        },
        RuntimeJournalEvent::RunFailed { run, attempt: 1 },
    );
    Ok(())
}

#[test]
fn command_ask_answer_charges_journal_bytes() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("two_ask", two_ask_workflow())?;
    let run = RunId::new(788);
    submit_with_journal(&mut shard, run, wf);
    let ticket = shard
        .lookup_pending_ask_ticket(run, StepIdx::new(2))
        .ok_or_else(|| "suspended ask must issue a ticket".to_string())?;
    assert_eq!(
        shard.enqueue(ShardCommand::AskAnswered {
            answer: AskAnswer {
                ticket,
                answer_slot: SlotIdx::new(2),
                value: SlotValue::I64(77),
                taint: Taint::Clean,
                encoded_len: 0,
            },
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    assert_eq!(count_events(&require_snapshot(&journal)?, run, is_ask_answered), 1);
    assert_counter_matches_journal(&shard, &journal, run)
}

#[test]
fn command_ask_answer_falls_back_when_journal_budget_exceeded() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("two_ask", two_ask_workflow())?;
    let digest = wf.digest();
    let run = RunId::new(789);
    submit_with_journal(&mut shard, run, wf);
    let before = counter_before(&shard, run)?;
    cap_journal_budget(&mut shard, run, digest, before)?;
    let ticket = shard
        .lookup_pending_ask_ticket(run, StepIdx::new(2))
        .ok_or_else(|| "suspended ask must issue a ticket".to_string())?;
    assert_eq!(
        shard.enqueue(ShardCommand::AskAnswered {
            answer: AskAnswer {
                ticket,
                answer_slot: SlotIdx::new(2),
                value: SlotValue::I64(77),
                taint: Taint::Clean,
                encoded_len: 0,
            },
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    assert!(shard.terminal_runs_contains(run));
    let events = require_snapshot(&journal)?;
    assert_eq!(run_failed_count(&events, run), 1);
    assert_eq!(
        count_events(&events, run, is_ask_answered),
        0,
        "answer must not be journaled after fallback: {events:?}"
    );
    Ok(())
}

#[test]
fn command_timer_resolution_charges_journal_bytes() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("two_wait", two_wait_workflow())?;
    let run = RunId::new(790);
    submit_with_journal(&mut shard, run, wf);
    assert_eq!(shard.enqueue(timer_command(&shard, run)), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    assert_eq!(count_events(&require_snapshot(&journal)?, run, is_wait_resolved), 1);
    assert_counter_matches_journal(&shard, &journal, run)
}

#[test]
fn command_timer_resolution_falls_back_when_journal_budget_exceeded() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("two_wait", two_wait_workflow())?;
    let digest = wf.digest();
    let run = RunId::new(791);
    submit_with_journal(&mut shard, run, wf);
    let before = counter_before(&shard, run)?;
    cap_journal_budget(&mut shard, run, digest, before)?;
    assert_eq!(shard.enqueue(timer_command(&shard, run)), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    assert!(shard.terminal_runs_contains(run));
    let events = require_snapshot(&journal)?;
    assert_eq!(run_failed_count(&events, run), 1);
    assert_eq!(
        count_events(&events, run, is_wait_resolved),
        0,
        "resolution must not be journaled after fallback: {events:?}"
    );
    Ok(())
}

#[test]
fn command_cancel_records_journal_bytes() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("suspended", suspended_workflow())?;
    let run = RunId::new(792);
    submit_run(&mut shard, run, wf, &[(SlotIdx::ZERO, SlotValue::I64(0))]);
    let attempt = shard
        .run_state_get(run)
        .map(|state| state.run_attempt.get())
        .ok_or_else(|| "run should still be active".to_string())?;
    shard.append_cancel_terminal_events(run, None, attempt)
        .map_err(|error| format!("cancel append failed: {error:?}"))?;
    assert_eq!(count_events(&require_snapshot(&journal)?, run, is_run_cancelled), 1);
    assert_eq!(count_events(&require_snapshot(&journal)?, run, is_action_abandoned), 1);
    assert_counter_matches_journal(&shard, &journal, run)
}

#[test]
fn command_kill_records_journal_bytes() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("suspended", suspended_workflow())?;
    let run = RunId::new(793);
    submit_run(&mut shard, run, wf, &[(SlotIdx::ZERO, SlotValue::I64(0))]);
    let attempt = shard
        .run_state_get(run)
        .map(|state| state.run_attempt.get())
        .ok_or_else(|| "run should still be active".to_string())?;
    shard.append_kill_terminal_events(run, attempt)
        .map_err(|error| format!("kill append failed: {error:?}"))?;
    assert_eq!(count_events(&require_snapshot(&journal)?, run, is_run_killed), 1);
    assert_eq!(count_events(&require_snapshot(&journal)?, run, is_action_abandoned), 1);
    assert_counter_matches_journal(&shard, &journal, run)
}

#[test]
fn command_resumed_records_journal_bytes() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("ask_then_finish", ask_workflow())?;
    let run = RunId::new(794);
    submit_with_journal(&mut shard, run, wf);
    shard
        .append_resumed_event(run)
        .map_err(|error| format!("resumed event append failed: {error:?}"))?;
    assert_eq!(count_events(&require_snapshot(&journal)?, run, is_resumed), 1);
    assert_counter_matches_journal(&shard, &journal, run)
}

fn retry_checkpoint_counter_case(base: u64, run: RunId) -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let workflow = two_slot_workflow().ok_or_else(|| "two slot workflow builds".to_string())?;
    let digest = workflow.digest();
    shard
        .append_journal_event(RuntimeJournalEvent::RunSubmitted {
            run,
            workflow: digest,
        })
        .map_err(|error| format!("seed journal failed: {error:?}"))?;
    let mut state = crate::shard::helpers::make_run_state(workflow.clone(), run)
        .ok_or_else(|| "run state builds".to_string())?;
    state.store = vb_core::value_store::ValueStore::with_max_slots(
        state.workflow.resource_contract().max_slots,
    );
    state.admission = Some(RunAdmission::new(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
    ));
    state.journal_bytes_written = base;
    state
        .frame
        .mark_running(StepIdx::ZERO)
        .map_err(|error| format!("mark running failed: {error:?}"))?;
    shard
        .capture_retry_checkpoint(run, StepIdx::ZERO, &mut state)
        .map_err(|error| format!("capture retry checkpoint failed: {error:?}"))?;
    let events = require_snapshot(&journal)?;
    let checkpoint_event = events
        .iter()
        .find(|event| {
            matches!(
                event,
                RuntimeJournalEvent::RetryCheckpoint { run: event_run, .. } if *event_run == run
            )
        })
        .ok_or_else(|| "retry checkpoint must be journaled".to_string())?;
    let event_bytes = postcard::to_allocvec(checkpoint_event)
        .map_err(|error| format!("event encode failed: {error:?}"))?;
    let event_len =
        u64::try_from(event_bytes.len()).map_err(|_| "event length overflow".to_string())?;
    let expected = base
        .checked_add(event_len)
        .ok_or_else(|| "expected counter overflow".to_string())?;
    let RuntimeJournalEvent::RetryCheckpoint {
        payload,
        payload_digest,
        attempt,
        step,
        ..
    } = checkpoint_event
    else {
        return Err("checkpoint event shape changed".to_string());
    };
    assert_eq!(
        *payload_digest,
        *blake3::hash(payload).as_bytes(),
        "checkpoint digest must match its payload"
    );
    let checkpoint = crate::run_checkpoint::decode_checkpoint_payload(payload)
        .map_err(|error| format!("checkpoint decode failed: {error}"))?;
    assert_eq!(checkpoint.journal_bytes_written, expected);
    let rebuilt = RuntimeJournalEvent::RetryCheckpoint {
        run,
        attempt: *attempt,
        step: *step,
        payload_digest: *payload_digest,
        payload: postcard::to_allocvec(&checkpoint)
            .map_err(|error| format!("checkpoint re-encode failed: {error:?}"))?,
    };
    let rebuilt_bytes = postcard::to_allocvec(&rebuilt)
        .map_err(|error| format!("rebuilt event encode failed: {error:?}"))?;
    assert_eq!(
        u64::try_from(rebuilt_bytes.len()).map_err(|_| "rebuilt length overflow")?,
        event_len
    );
    let (restored, restored_attempt, _, _) = checkpoint
        .restore(workflow, digest, &state.action_contracts)
        .map_err(|error| format!("checkpoint restore failed: {error}"))?;
    assert_eq!(restored.journal_bytes_written, expected);
    assert_eq!(restored_attempt.get(), 2);
    Ok(())
}

#[test]
fn retry_checkpoint_stores_counter_including_its_own_event_bytes() -> Result<(), String> {
    retry_checkpoint_counter_case(0, RunId::new(777))?;
    retry_checkpoint_counter_case(127, RunId::new(778))?;
    retry_checkpoint_counter_case(16_383, RunId::new(779))?;
    Ok(())
}
