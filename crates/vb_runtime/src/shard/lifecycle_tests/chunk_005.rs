#[test]
fn action_failure_without_handler_emits_action_failed_before_run_failed() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("suspended", suspended_workflow())?;
    let run = RunId::new(600);
    submit_run(&mut shard, run, wf);
    let ticket = make_ticket(run, StepIdx::ZERO, 1);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let events = require_snapshot(&journal)?;
    assert_event_order(
        &events,
        RuntimeJournalEvent::ActionFailed {
            run,
            step: StepIdx::ZERO,
            action: ActionId::new(0),
            attempt: 1,
        },
        RuntimeJournalEvent::RunFailed { run, attempt: 1},
    );
    Ok(())
}

#[test]
fn action_failure_routes_to_error_handler() -> Result<(), String> {
    let mut shard = Shard::new(small_config());
    let wf = require_workflow("error_handler", error_handler_workflow())?;
    let run = RunId::new(61);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow: wf,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0
            )),
            inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))])
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let ticket = pending_ticket(&shard, run, StepIdx::new(1), 1);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    Ok(())
}

#[test]
fn action_failure_routed_to_handler_emits_action_failed_before_handler_step() -> Result<(), String>
{
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("error_handler", error_handler_workflow())?;
    let run = RunId::new(610);
    submit_run(&mut shard, run, wf);
    let ticket = pending_ticket(&shard, run, StepIdx::new(1), 1);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let events = require_snapshot(&journal)?;
    assert_event_order(
        &events,
        RuntimeJournalEvent::ActionFailed {
            run,
            step: StepIdx::new(1),
            action: ActionId::new(0),
            attempt: 1,
        },
        RuntimeJournalEvent::StepStarted {
            run,
            step: StepIdx::new(2), attempt: 1},
    );
    Ok(())
}

#[test]
fn action_failure_unknown_run_returns_run_not_found() {
    let mut shard = Shard::new(small_config());
    let ticket = make_ticket(RunId::new(9999), StepIdx::ZERO, 1);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn retry_exhaustion_emits_single_action_failed() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let run = RunId::new(620);
    submit_run(&mut shard, run, retry_workflow()?);
    enqueue_action_failure(&mut shard, run, StepIdx::new(1), 1);
    enqueue_action_failure(&mut shard, run, StepIdx::new(1), 2);
    let events = require_snapshot(&journal)?;
    assert_retry_exhaustion_journal(&events, run);
    Ok(())
}

#[test]
fn ask_answer_completes_ask_workflow() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = ask_workflow() else {
        return;
    };
    let run = RunId::new(70);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithContracts {
            run,
            workflow: wf,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0
            )),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 1);
    let ticket = shard
        .lookup_pending_ask_ticket(run, StepIdx::new(2))
        .expect("active ask boundary must issue a ticket");
    let answer = AskAnswer {
        ticket,
        answer_slot: SlotIdx::new(2),
        value: SlotValue::I64(77),
        taint: Taint::Clean,
        encoded_len: 0,
    };
    assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
}

#[test]
fn stale_ask_ticket_is_rejected_after_same_step_is_scheduled_again() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("ask", ask_workflow())?;
    let run = RunId::new(71);
    submit_run(&mut shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));

    let first = shard
        .lookup_pending_ask_ticket(run, StepIdx::new(2))
        .ok_or("first ask boundary must issue a ticket")?;
    let state = shard
        .take_run_state(run)
        .map_err(|error| format!("suspended ask state must be checked out: {error:?}"))?;
    let _previous_timer = shard.pending_timer_remove(run);
    assert!(matches!(
        shard
            .await_timer(run, state, PendingTimerKind::Ask)
            .map_err(|error| format!("ask reschedule failed: {error:?}"))?,
        crate::shard::transitions::TransitionOutcome::Applied
    ));
    let second = shard
        .lookup_pending_ask_ticket(run, StepIdx::new(2))
        .ok_or("reentered ask boundary must issue a ticket")?;
    if first == second {
        return Err(String::from(
            "reentered ask ticket reused its durable sequence",
        ));
    }

    let before = journal
        .snapshot()
        .map_err(|error| format!("journal snapshot failed: {error:?}"))?;
    let stale_answer = AskAnswer::new(first, SlotIdx::new(2), SlotValue::I64(88), Taint::Clean);
    assert_eq!(
        shard.enqueue(ShardCommand::AskAnswered {
            answer: stale_answer
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidActionCompletion));
    assert_eq!(
        journal
            .snapshot()
            .map_err(|error| format!("journal snapshot failed: {error:?}"))?,
        before
    );
    Ok(())
}

#[test]
fn ask_answer_unknown_run_returns_run_not_found() {
    let mut shard = Shard::new(small_config());
    let answer = AskAnswer {
        ticket: AskTicket {
            run: RunId::new(9999),
            ask_step: StepIdx::ZERO,
            resume_step: StepIdx::new(1),

            answer_slot: vb_core::ids::SlotIdx::ZERO,
            boundary_seq: vb_storage::EventSeq::ZERO,
        },
        answer_slot: SlotIdx::ZERO,
        value: SlotValue::I64(0),
        taint: Taint::Clean,
        encoded_len: 0,
    };
    assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn timer_fire_advances_wait_to_completion() -> Result<(), String> {
    let mut shard = Shard::new(small_config());
    let wf = require_workflow("wait_workflow", wait_workflow())?;
    let run = RunId::new(80);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithContracts {
            run,
            workflow: wf,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0
            )),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 1);
    assert_eq!(shard.enqueue(timer_command(&shard, run)), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.pending_timer_count(), 0);
    Ok(())
}

#[test]
fn timer_fire_for_non_timer_run_returns_invalid_timer_fire() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(81);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithContracts {
            run,
            workflow: wf,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0
            )),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(invalid_timer_command(run)), Ok(()));
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
}

#[test]
fn timer_fire_unknown_run_returns_run_not_found() {
    let mut shard = Shard::new(small_config());
    assert_eq!(
        shard.enqueue(invalid_timer_command(RunId::new(9999))),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
}

#[test]
fn cancel_removes_active_run_and_increments_failed() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(90);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow: wf,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0
            )),
            inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))])
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.active_run_count(), 1);
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
}

#[test]
fn cancel_nonexistent_run_succeeds_without_counter_change() {
    let mut shard = Shard::new(small_config());
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel {
            run: RunId::new(9999),
            reason: None
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
}

// =======================================================================
// finish_run terminal fence
// =======================================================================

#[test]
fn finish_run_appends_run_finished_event_and_inserts_terminal_run() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let Some(wf) = finished_workflow() else {
        return Ok(());
    };
    let run = RunId::new(500);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithContracts {
            run,
            workflow: wf,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0
            )),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Finished workflow should complete immediately
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    // terminal_runs should contain the finished run
    assert!(shard.terminal_runs_contains(run));
    // Journal should contain RunFinished event
    let events = require_snapshot(&journal)?;
    let has_run_finished = events
        .iter()
        .any(|e| matches!(e, RuntimeJournalEvent::RunFinished { run: r, .. } if *r == run));
    assert!(
        has_run_finished,
        "RunFinished event not found in journal: {events:?}"
    );
    Ok(())
}

// =======================================================================
// retry_is_available and apply_action_failure_to_state
// =======================================================================

#[test]
fn retry_remaining_advances_attempt_and_resumes_drive() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let run = RunId::new(501);
    submit_run(&mut shard, run, retry_workflow()?);
    // After submission, the run should be suspended on action step 1
    // with attempt 1 and capacity 2 from the retry policy
    let state = shard.run_state_get(run).ok_or("run should exist")?;
    assert_eq!(state.action_attempts.get(1).copied(), Some(1));
    // Send action failure with Retryable — should retry
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: pending_ticket(&shard, run, StepIdx::new(1), 1),
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Check that attempt was incremented
    let state = shard
        .run_state_get(run)
        .ok_or("run should exist after retry")?;
    assert_eq!(state.action_attempts.get(1).copied(), Some(2));
    // Run is still active
    assert_eq!(shard.active_run_count(), 1);
    Ok(())
}

#[test]
fn retry_exhausted_fails_run_when_no_more_attempts() -> Result<(), String> {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(502);
    submit_run(&mut shard, run, retry_workflow()?);
    // retry_workflow has max_attempts=2 (from ConstValue::I64(2))
    // First failure at attempt 1 → retry to attempt 2
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: pending_ticket(&shard, run, StepIdx::new(1), 1),
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Second failure at attempt 2 → retry exhausted, run fails
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: pending_ticket(&shard, run, StepIdx::new(1), 2),
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Run should be failed
    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    Ok(())
}

#[test]
fn non_retryable_failure_fails_run_immediately() -> Result<(), String> {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(503);
    submit_run(&mut shard, run, retry_workflow()?);
    // Send non-retryable failure
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: pending_ticket(&shard, run, StepIdx::new(1), 1),
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Run should be failed
    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    Ok(())
}

#[test]
fn action_failure_drives_error_handler_when_no_retry_metadata() -> Result<(), String> {
    let mut shard = Shard::new(small_config());
    let wf = require_workflow("error_handler", error_handler_workflow())?;
    let run = RunId::new(504);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow: wf,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0
            )),
            inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))])
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // error_handler_workflow has no retry policy, so non-retryable failure
    // should route to error handler, which drives the run to completion
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: pending_ticket(&shard, run, StepIdx::new(1), 1),
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Run completes via error handler path (handler -> finish)
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.active_run_count(), 0);
    Ok(())
}

#[test]
fn handle_action_failure_rejects_when_step_out_of_bounds() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(505);
    submit_run(&mut shard, run, wf);
    // Step 99 is out of bounds
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: make_ticket(run, StepIdx::new(99), 1),
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    // validate_action_completion catches the out-of-bounds step
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidActionCompletion));
}

use vb_core::budget::AggregateResourceBudget;
use vb_core::engine::EngineError;
use crate::admission::RunAdmission;
use crate::engine::types::{EvidenceCollector, RuntimeEngineError, RuntimeSignal};

fn total_slots_written_budget(max_total_slots_written: u32) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 100,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_retries_per_action: 10,
        max_gather_pages: 10,
        max_gather_items: 10,
        max_for_each_iterations: 10,
        max_together_branches: 10,
        max_repeat_attempts: 10,
        max_run_time_seconds: 100,
        max_result_bytes: 100,
        max_total_slots_written,
        max_timer_entries: 100,
        max_trace_events: 100,
        max_queue_depth: 100,
        max_journal_batch_bytes: 100,
        max_ipc_payload_bytes: 100,
        max_blob_bytes: 100,
        max_input_bytes: 100,
        max_step_budget_per_tick: 100,
        max_transitions_per_tick: 100,
        max_open_gates: 10,
        max_signal_subscriptions: 10,
        max_incident_emissions: 100,
        max_transitions: 100,
        max_journal_bytes_total: 100,
        max_value_store_entries: 100,
        max_cost_units: 100,
    }
}

fn two_slot_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let first = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst { value: ConstIdx::new(0) },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(1)),
    );
    let second = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::SetConst { value: ConstIdx::new(0) },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(2)),
    );
    let finish = vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::Finish { result: SlotIdx::new(1) },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("two_slot_budget"),
        digest: WorkflowDigest::from_bytes([11; 32]),
        nodes: Box::from([first, second, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(10)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn rewrite_slot_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let first = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst { value: ConstIdx::new(0) },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(1)),
    );
    let second = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::SetConst { value: ConstIdx::new(0) },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(2)),
    );
    let finish = vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::Finish { result: SlotIdx::ZERO },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("rewrite_slot_budget"),
        digest: WorkflowDigest::from_bytes([12; 32]),
        nodes: Box::from([first, second, finish]),
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

#[test]
fn drive_state_fails_when_written_slots_exceed_admitted_limit() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(740);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .expect("run state builds");
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        total_slots_written_budget(1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "total_slots_written",
            limit: 1,
        }))
    ));
}

#[test]
fn drive_state_at_written_slots_limit_finishes() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(741);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .expect("run state builds");
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        total_slots_written_budget(2),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}

#[test]
fn drive_state_rewriting_one_slot_stays_within_limit() {
    let workflow = rewrite_slot_workflow().expect("rewrite slot workflow builds");
    let run = RunId::new(742);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .expect("run state builds");
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        total_slots_written_budget(1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}

#[test]
fn drive_state_without_admitted_budget_ignores_written_slots() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(743);
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .expect("run state builds");
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}

fn steps_executable_budget(max_steps_executable: u32) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_retries_per_action: 10,
        max_gather_pages: 10,
        max_gather_items: 10,
        max_for_each_iterations: 10,
        max_together_branches: 10,
        max_repeat_attempts: 10,
        max_run_time_seconds: 100,
        max_result_bytes: 100,
        max_total_slots_written: 100,
        max_timer_entries: 100,
        max_trace_events: 100,
        max_queue_depth: 100,
        max_journal_batch_bytes: 100,
        max_ipc_payload_bytes: 100,
        max_blob_bytes: 100,
        max_input_bytes: 100,
        max_step_budget_per_tick: 100,
        max_transitions_per_tick: 100,
        max_open_gates: 10,
        max_signal_subscriptions: 10,
        max_incident_emissions: 100,
        max_transitions: 100,
        max_journal_bytes_total: 100,
        max_value_store_entries: 100,
        max_cost_units: 100,
    }
}

fn drive_state_with_steps_budget(
    run: RunId,
    max_steps_executable: u32,
    prior_executions: u64,
) -> Result<RuntimeSignal, RuntimeEngineError> {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run).expect("run state builds");
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        steps_executable_budget(max_steps_executable),
    ));
    for _ in 0..prior_executions {
        state
            .frame
            .increment_executed()
            .expect("prior execution count increments");
    }
    let mut evidence = EvidenceCollector::new();
    Shard::drive_state(&mut state, 100, 0, &mut evidence)
}

#[test]
fn drive_state_fails_when_executed_steps_exceed_admitted_limit() {
    let result = drive_state_with_steps_budget(RunId::new(754), 2, 0);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "steps_executable",
            limit: 2,
        }))
    ));
}

#[test]
fn drive_state_at_steps_executable_limit_finishes() {
    let result = drive_state_with_steps_budget(RunId::new(755), 3, 0);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}

#[test]
fn drive_state_counts_prior_executions_against_steps_executable_limit() {
    let result = drive_state_with_steps_budget(RunId::new(756), 5, 3);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "steps_executable",
            limit: 5,
        }))
    ));
}

#[test]
fn drive_state_retried_run_within_steps_executable_limit_finishes() {
    let result = drive_state_with_steps_budget(RunId::new(757), 5, 2);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}

#[test]
fn drive_state_without_admitted_budget_ignores_steps_executable() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(758);
    let mut state = crate::shard::helpers::make_run_state(workflow, run).expect("run state builds");
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}
fn parallel_in_flight_budget(max_parallel_in_flight: u16) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 100,
        max_action_tickets: 10,
        max_parallel_in_flight,
        max_retries_per_action: 10,
        max_gather_pages: 10,
        max_gather_items: 10,
        max_for_each_iterations: 10,
        max_together_branches: 10,
        max_repeat_attempts: 10,
        max_run_time_seconds: 100,
        max_result_bytes: 100,
        max_total_slots_written: 100,
        max_timer_entries: 100,
        max_trace_events: 100,
        max_queue_depth: 100,
        max_journal_batch_bytes: 100,
        max_ipc_payload_bytes: 100,
        max_blob_bytes: 100,
        max_input_bytes: 100,
        max_step_budget_per_tick: 100,
        max_transitions_per_tick: 100,
        max_open_gates: 10,
        max_signal_subscriptions: 10,
        max_incident_emissions: 100,
        max_transitions: 100,
        max_journal_bytes_total: 100,
        max_value_store_entries: 100,
        max_cost_units: 100,
    }
}

fn parallel_together_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_deadline = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst { value: ConstIdx::new(0) },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(1)),
    );
    let together = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::TogetherStart {
            branches: Box::from([StepIdx::new(2), StepIdx::new(4)]),
            join: StepIdx::new(6),
        },
        Some(SlotIdx::new(1)),
        None,
    );
    let branch_one = vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::TogetherBranch {
            branch: 0,
            entry: StepIdx::new(3),
            join: StepIdx::new(6),
            accumulator: SlotIdx::new(1),
        },
        Some(SlotIdx::new(2)),
        None,
    );
    let wait = vb_core::workflow::node(
        StepIdx::new(3),
        CompiledNodeKind::WaitUntil { deadline_slot: SlotIdx::ZERO },
        None,
        Some(StepIdx::new(4)),
    );
    let branch_two = vb_core::workflow::node(
        StepIdx::new(4),
        CompiledNodeKind::TogetherBranch {
            branch: 1,
            entry: StepIdx::new(5),
            join: StepIdx::new(6),
            accumulator: SlotIdx::new(1),
        },
        Some(SlotIdx::new(2)),
        None,
    );
    let set_branch_result = vb_core::workflow::node(
        StepIdx::new(5),
        CompiledNodeKind::SetConst { value: ConstIdx::new(0) },
        Some(SlotIdx::new(2)),
        Some(StepIdx::new(6)),
    );
    let join = vb_core::workflow::node(
        StepIdx::new(6),
        CompiledNodeKind::TogetherJoin { branch_count: 2, accumulator: SlotIdx::new(1) },
        Some(SlotIdx::new(2)),
        Some(StepIdx::new(7)),
    );
    let finish = vb_core::workflow::node(
        StepIdx::new(7),
        CompiledNodeKind::Finish { result: SlotIdx::new(2) },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("parallel_in_flight_budget"),
        digest: WorkflowDigest::from_bytes([7; 32]),
        nodes: Box::from([
            set_deadline,
            together,
            branch_one,
            wait,
            branch_two,
            set_branch_result,
            join,
            finish,
        ]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(10)]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

#[test]
fn drive_state_fails_when_parallel_in_flight_exceeds_admitted_limit() {
    let workflow = parallel_together_workflow().expect("parallel together workflow builds");
    let run = RunId::new(730);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .expect("run state builds");
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        parallel_in_flight_budget(1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "parallel_in_flight",
            limit: 1,
        }))
    ));
}

#[test]
fn drive_state_at_parallel_in_flight_limit_suspends_on_wait() {
    let workflow = parallel_together_workflow().expect("parallel together workflow builds");
    let run = RunId::new(731);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .expect("run state builds");
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        parallel_in_flight_budget(2),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::AwaitingWait)));
}

#[test]
fn drive_state_without_admitted_budget_ignores_parallel_count() {
    let workflow = parallel_together_workflow().expect("parallel together workflow builds");
    let run = RunId::new(732);
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .expect("run state builds");
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::AwaitingWait)));
}
fn timer_entry_budget(max_timer_entries: u32) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 100,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_retries_per_action: 10,
        max_gather_pages: 10,
        max_gather_items: 10,
        max_for_each_iterations: 10,
        max_together_branches: 10,
        max_repeat_attempts: 10,
        max_run_time_seconds: 100,
        max_result_bytes: 100,
        max_total_slots_written: 100,
        max_timer_entries,
        max_trace_events: 100,
        max_queue_depth: 100,
        max_journal_batch_bytes: 100,
        max_ipc_payload_bytes: 100,
        max_blob_bytes: 100,
        max_input_bytes: 100,
        max_step_budget_per_tick: 100,
        max_transitions_per_tick: 100,
        max_open_gates: 10,
        max_signal_subscriptions: 10,
        max_incident_emissions: 100,
        max_transitions: 100,
        max_journal_bytes_total: 100,
        max_value_store_entries: 100,
        max_cost_units: 100,
    }
}

fn timer_wait_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_deadline = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst { value: ConstIdx::new(0) },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(1)),
    );
    let wait = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::WaitUntil { deadline_slot: SlotIdx::ZERO },
        None,
        Some(StepIdx::new(2)),
    );
    let finish = vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::Finish { result: SlotIdx::ZERO },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("timer_wait_budget"),
        digest: WorkflowDigest::from_bytes([9; 32]),
        nodes: Box::from([set_deadline, wait, finish]),
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

fn timer_ask_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_prompt = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst { value: ConstIdx::new(0) },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(1)),
    );
    let set_timeout = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::SetConst { value: ConstIdx::new(1) },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(2)),
    );
    let ask = vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::Ask { prompt: SlotIdx::ZERO, timeout_slot: Some(SlotIdx::new(1)) },
        None,
        Some(StepIdx::new(3)),
    );
    let resume = vb_core::workflow::node(
        StepIdx::new(3),
        CompiledNodeKind::AskResume { answer: SlotIdx::new(2) },
        None,
        Some(StepIdx::new(4)),
    );
    let finish = vb_core::workflow::node(
        StepIdx::new(4),
        CompiledNodeKind::Finish { result: SlotIdx::new(2) },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("timer_ask_budget"),
        digest: WorkflowDigest::from_bytes([8; 32]),
        nodes: Box::from([set_prompt, set_timeout, ask, resume, finish]),
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

#[test]
fn drive_state_fails_when_timer_entries_exceed_admitted_limit() {
    let Some(workflow) = timer_wait_workflow() else {
        return;
    };
    let run = RunId::new(720);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        timer_entry_budget(0),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "timer_entries",
            limit: 0,
        }))
    ));
}

#[test]
fn drive_state_at_timer_entry_limit_suspends_on_wait_until() {
    let Some(workflow) = timer_wait_workflow() else {
        return;
    };
    let run = RunId::new(721);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        timer_entry_budget(1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::AwaitingWait)));
}

#[test]
fn drive_state_fails_when_ask_timer_exceeds_admitted_limit() {
    let Some(workflow) = timer_ask_workflow() else {
        return;
    };
    let run = RunId::new(722);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        timer_entry_budget(0),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "timer_entries",
            limit: 0,
        }))
    ));
}

#[test]
fn drive_state_without_admitted_budget_ignores_timer_counts() {
    let Some(workflow) = timer_wait_workflow() else {
        return;
    };
    let run = RunId::new(723);
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::AwaitingWait)));
}
fn incident_budget(max_incident_emissions: u64) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_retries_per_action: 10,
        max_gather_pages: 10,
        max_gather_items: 10,
        max_for_each_iterations: 10,
        max_together_branches: 10,
        max_repeat_attempts: 10,
        max_run_time_seconds: 1000,
        max_result_bytes: 1024,
        max_total_slots_written: 10,
        max_timer_entries: 10,
        max_trace_events: 1000,
        max_queue_depth: 10,
        max_journal_batch_bytes: 1024,
        max_ipc_payload_bytes: 1024,
        max_blob_bytes: 1024,
        max_input_bytes: 1024,
        max_step_budget_per_tick: 100,
        max_transitions_per_tick: 100,
        max_open_gates: 10,
        max_signal_subscriptions: 10,
        max_incident_emissions,
        max_transitions: 1000,
        max_journal_bytes_total: 100_000,
        max_value_store_entries: 1000,
        max_cost_units: 1000,
    }
}

fn incident_state(
    run_id: u64,
    max_incident_emissions: u64,
    executed: u64,
) -> Option<crate::shard::types::RunState> {
    let Some(workflow) = suspended_workflow() else {
        return None;
    };
    let run = RunId::new(run_id);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)?;
    let _ = state.frame.write_slot(SlotIdx::ZERO, SlotValue::I64(0));
    state.action_contracts =
        crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0));
    for _ in 0..executed {
        let _ = state.frame.increment_executed();
    }
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        vb_core::policy::RuntimePolicy::Relaxed,
        incident_budget(max_incident_emissions),
    ));
    Some(state)
}

#[test]
fn drive_state_within_incident_limit_issues_action() {
    let Some(mut state) = incident_state(750, 10, 0) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket)) if ticket.step == StepIdx::ZERO
    ));
}

#[test]
fn drive_state_fails_when_executed_steps_exceed_incident_limit() {
    let Some(mut state) = incident_state(751, 2, 3) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "incident_emissions",
            limit: 2,
        }))
    ));
}

#[test]
fn drive_state_fails_on_exceeded_incidents_without_new_steps() {
    let Some(mut state) = incident_state(752, 2, 3) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 0, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "incident_emissions",
            limit: 2,
        }))
    ));
}

#[test]
fn drive_state_ignores_incident_check_without_budget() {
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(753);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    let _ = state.frame.write_slot(SlotIdx::ZERO, SlotValue::I64(0));
    state.action_contracts =
        crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0));
    for _ in 0..50 {
        let _ = state.frame.increment_executed();
    }
    state.admission = Some(RunAdmission::new(
        digest,
        run,
        crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        vb_core::policy::RuntimePolicy::Relaxed,
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket)) if ticket.step == StepIdx::ZERO
    ));
}
fn retry_budget(max_retries_per_action: u16) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_retries_per_action,
        max_gather_pages: 10,
        max_gather_items: 10,
        max_for_each_iterations: 10,
        max_together_branches: 10,
        max_repeat_attempts: 10,
        max_run_time_seconds: 1000,
        max_result_bytes: 1024,
        max_total_slots_written: 10,
        max_timer_entries: 10,
        max_trace_events: 1000,
        max_queue_depth: 10,
        max_journal_batch_bytes: 1024,
        max_ipc_payload_bytes: 1024,
        max_blob_bytes: 1024,
        max_input_bytes: 1024,
        max_step_budget_per_tick: 100,
        max_transitions_per_tick: 100,
        max_open_gates: 10,
        max_signal_subscriptions: 10,
        max_incident_emissions: 100,
        max_transitions: 1000,
        max_journal_bytes_total: 100_000,
        max_value_store_entries: 1000,
        max_cost_units: 1000,
    }
}

fn retry_state(
    run_id: u64,
    max_retries_per_action: u16,
    recorded_attempt: u16,
) -> Option<crate::shard::types::RunState> {
    let Some(workflow) = suspended_workflow() else {
        return None;
    };
    let run = RunId::new(run_id);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)?;
    let _ = state.frame.write_slot(SlotIdx::ZERO, SlotValue::I64(0));
    state.action_contracts =
        crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = recorded_attempt;
    }
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        vb_core::policy::RuntimePolicy::Relaxed,
        retry_budget(max_retries_per_action),
    ));
    Some(state)
}

#[test]
fn drive_state_first_attempt_within_zero_retry_limit_issues_action() {
    let Some(mut state) = retry_state(740, 0, 0) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket)) if ticket.step == StepIdx::ZERO
    ));
}

#[test]
fn drive_state_at_retry_limit_issues_action() {
    let Some(mut state) = retry_state(741, 1, 2) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket)) if ticket.step == StepIdx::ZERO
    ));
}

#[test]
fn drive_state_fails_when_recorded_retries_exceed_admitted_limit() {
    let Some(mut state) = retry_state(742, 1, 3) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "retries_per_action",
            limit: 1,
        }))
    ));
}

#[test]
fn drive_state_fails_on_recorded_retries_without_new_issue() {
    let Some(mut state) = retry_state(743, 0, 2) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 0, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "retries_per_action",
            limit: 0,
        }))
    ));
}

#[test]
fn drive_state_ignores_retry_check_without_budget() {
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(744);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    let _ = state.frame.write_slot(SlotIdx::ZERO, SlotValue::I64(0));
    state.action_contracts =
        crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 5;
    }
    state.admission = Some(RunAdmission::new(
        digest,
        run,
        crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        vb_core::policy::RuntimePolicy::Relaxed,
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket)) if ticket.step == StepIdx::ZERO
    ));
}
fn ticket_budget(max_action_tickets: u32) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 10,
        max_gather_items: 10,
        max_for_each_iterations: 10,
        max_together_branches: 10,
        max_repeat_attempts: 10,
        max_run_time_seconds: 1000,
        max_result_bytes: 1024,
        max_total_slots_written: 10,
        max_timer_entries: 10,
        max_trace_events: 1000,
        max_queue_depth: 10,
        max_journal_batch_bytes: 1024,
        max_ipc_payload_bytes: 1024,
        max_blob_bytes: 1024,
        max_input_bytes: 1024,
        max_step_budget_per_tick: 100,
        max_transitions_per_tick: 100,
        max_open_gates: 10,
        max_signal_subscriptions: 10,
        max_incident_emissions: 100,
        max_transitions: 1000,
        max_journal_bytes_total: 100_000,
        max_value_store_entries: 1000,
        max_cost_units: 1000,
    }
}

fn ticket_state(
    run_id: u64,
    max_action_tickets: u32,
    recorded_attempt: u16,
) -> Option<crate::shard::types::RunState> {
    let Some(workflow) = suspended_workflow() else {
        return None;
    };
    let run = RunId::new(run_id);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)?;
    let _ = state.frame.write_slot(SlotIdx::ZERO, SlotValue::I64(0));
    state.action_contracts =
        crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = recorded_attempt;
    }
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        vb_core::policy::RuntimePolicy::Relaxed,
        ticket_budget(max_action_tickets),
    ));
    Some(state)
}

#[test]
fn drive_state_fails_when_action_tickets_exceed_admitted_limit() {
    let Some(mut state) = ticket_state(730, 0, 0) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "action_tickets",
            limit: 0,
        }))
    ));
}

#[test]
fn drive_state_at_action_ticket_limit_issues_action() {
    let Some(mut state) = ticket_state(731, 1, 0) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket)) if ticket.step == StepIdx::ZERO
    ));
}

#[test]
fn drive_state_retry_of_same_action_does_not_count_again() {
    let Some(mut state) = ticket_state(732, 1, 3) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket)) if ticket.step == StepIdx::ZERO
    ));
}

#[test]
fn drive_state_fails_on_recorded_tickets_without_new_issue() {
    let Some(mut state) = ticket_state(733, 0, 2) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 0, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "action_tickets",
            limit: 0,
        }))
    ));
}

#[test]
fn drive_state_ignores_ticket_check_without_budget() {
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(734);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    let _ = state.frame.write_slot(SlotIdx::ZERO, SlotValue::I64(0));
    state.action_contracts =
        crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0));
    state.admission = Some(RunAdmission::new(
        digest,
        run,
        crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        vb_core::policy::RuntimePolicy::Relaxed,
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket)) if ticket.step == StepIdx::ZERO
    ));
}
fn cost_budget(max_cost_units: u64) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 10,
        max_gather_items: 10,
        max_for_each_iterations: 10,
        max_together_branches: 10,
        max_repeat_attempts: 10,
        max_run_time_seconds: 1000,
        max_result_bytes: 1024,
        max_total_slots_written: 10,
        max_timer_entries: 10,
        max_trace_events: 1000,
        max_queue_depth: 10,
        max_journal_batch_bytes: 1024,
        max_ipc_payload_bytes: 1024,
        max_blob_bytes: 1024,
        max_input_bytes: 1024,
        max_step_budget_per_tick: 100,
        max_transitions_per_tick: 100,
        max_open_gates: 10,
        max_signal_subscriptions: 10,
        max_incident_emissions: 100,
        max_transitions: 1000,
        max_journal_bytes_total: 100_000,
        max_value_store_entries: 1000,
        max_cost_units,
    }
}

fn cost_state(
    run_id: u64,
    max_cost_units: u64,
    recorded_attempt: u16,
) -> Option<crate::shard::types::RunState> {
    let Some(workflow) = suspended_workflow() else {
        return None;
    };
    let run = RunId::new(run_id);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)?;
    let _ = state.frame.write_slot(SlotIdx::ZERO, SlotValue::I64(0));
    state.action_contracts =
        crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = recorded_attempt;
    }
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        vb_core::policy::RuntimePolicy::Relaxed,
        cost_budget(max_cost_units),
    ));
    Some(state)
}

#[test]
fn drive_state_fails_when_cost_units_exceed_admitted_limit() {
    let Some(mut state) = cost_state(720, 0, 0) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "cost_units",
            limit: 0,
        }))
    ));
}

#[test]
fn drive_state_at_cost_unit_limit_issues_action() {
    let Some(mut state) = cost_state(721, 1, 0) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket))
            if ticket.step == StepIdx::ZERO && ticket.attempt == 1
    ));
}

#[test]
fn drive_state_fails_when_recorded_tickets_exceed_limit() {
    let Some(mut state) = cost_state(722, 1, 2) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "cost_units",
            limit: 1,
        }))
    ));
}

#[test]
fn drive_state_at_cost_unit_limit_with_recorded_ticket_issues_action() {
    let Some(mut state) = cost_state(723, 1, 1) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket))
            if ticket.step == StepIdx::ZERO && ticket.attempt == 1
    ));
}

#[test]
fn drive_state_ignores_cost_check_without_budget() {
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(724);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    let _ = state.frame.write_slot(SlotIdx::ZERO, SlotValue::I64(0));
    state.action_contracts =
        crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0));
    state.admission = Some(RunAdmission::new(
        digest,
        run,
        crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        vb_core::policy::RuntimePolicy::Relaxed,
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket)) if ticket.step == StepIdx::ZERO
    ));
}
fn suspension_budget(max_open_gates: u32, max_signal_subscriptions: u32) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 1,
        max_parallel_in_flight: 1,
        max_retries_per_action: 1,
        max_gather_pages: 1,
        max_gather_items: 1,
        max_for_each_iterations: 1,
        max_together_branches: 1,
        max_repeat_attempts: 1,
        max_run_time_seconds: 10,
        max_result_bytes: 1,
        max_total_slots_written: 100,
        max_timer_entries: 1,
        max_trace_events: 1,
        max_queue_depth: 1,
        max_journal_batch_bytes: 1,
        max_ipc_payload_bytes: 1,
        max_blob_bytes: 1,
        max_input_bytes: 1,
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1,
        max_open_gates,
        max_signal_subscriptions,
        max_incident_emissions: 100,
        max_transitions: 10,
        max_journal_bytes_total: 1,
        max_value_store_entries: 1,
        max_cost_units: 1,
    }
}

fn wait_event_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_event = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst { value: ConstIdx::new(0) },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(1)),
    );
    let set_timeout = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::SetConst { value: ConstIdx::new(1) },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(2)),
    );
    let wait = vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::WaitEvent { event: SlotIdx::ZERO, timeout_slot: Some(SlotIdx::new(1)) },
        None,
        Some(StepIdx::new(3)),
    );
    let finish_node = vb_core::workflow::node(
        StepIdx::new(3),
        CompiledNodeKind::Finish { result: SlotIdx::ZERO },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("wait_event_budget"),
        digest: WorkflowDigest::from_bytes([7; 32]),
        nodes: Box::from([set_event, set_timeout, wait, finish_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(7), ConstValue::I64(10)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

#[test]
fn drive_state_fails_when_open_gates_exceed_admitted_limit() {
    let Some(workflow) = ask_workflow() else {
        return;
    };
    let run = RunId::new(710);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        suspension_budget(0, 1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "open_gates",
            limit: 0,
        }))
    ));
}

#[test]
fn drive_state_at_open_gate_limit_suspends_on_ask() {
    let Some(workflow) = ask_workflow() else {
        return;
    };
    let run = RunId::new(711);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        suspension_budget(1, 1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::AwaitingAsk)));
}

#[test]
fn drive_state_fails_when_signal_subscriptions_exceed_admitted_limit() {
    let Some(workflow) = wait_event_workflow() else {
        return;
    };
    let run = RunId::new(712);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        suspension_budget(1, 0),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "signal_subscriptions",
            limit: 0,
        }))
    ));
}

#[test]
fn drive_state_at_signal_subscription_limit_suspends_on_wait_event() {
    let Some(workflow) = wait_event_workflow() else {
        return;
    };
    let run = RunId::new(713);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        suspension_budget(1, 1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::AwaitingWait)));
}

#[test]
fn drive_state_timer_wait_is_not_a_signal_subscription() {
    let Some(workflow) = wait_workflow() else {
        return;
    };
    let run = RunId::new(714);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        suspension_budget(1, 0),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::AwaitingWait)));
}

#[test]
fn drive_state_without_admitted_budget_ignores_suspension_counts() {
    let Some(workflow) = ask_workflow() else {
        return;
    };
    let run = RunId::new(715);
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::AwaitingAsk)));
}
#[test]
fn drive_state_fails_when_ask_steps_at_cap_plus_manual_gate() {
    let Some(workflow) = ask_workflow() else {
        return;
    };
    let run = RunId::new(716);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        suspension_budget(1, 1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 1, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "open_gates",
            limit: 1,
        }))
    ));
}

#[test]
fn drive_state_fails_when_manual_gates_alone_exceed_cap() {
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(717);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    let _ = state.frame.write_slot(SlotIdx::ZERO, SlotValue::I64(0));
    state.action_contracts =
        crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0));
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        vb_core::policy::RuntimePolicy::Relaxed,
        suspension_budget(0, 1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 1, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "open_gates",
            limit: 0,
        }))
    ));
}

#[test]
fn drive_state_at_combined_open_gate_limit_suspends_on_ask() {
    let Some(workflow) = ask_workflow() else {
        return;
    };
    let run = RunId::new(718);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        suspension_budget(2, 1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 1, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::AwaitingAsk)));
}

#[test]
fn drive_state_without_admitted_budget_ignores_manual_gate_count() {
    let Some(workflow) = ask_workflow() else {
        return;
    };
    let run = RunId::new(719);
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 5, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::AwaitingAsk)));
}
fn value_store_budget(max_value_store_entries: u64) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 1,
        max_parallel_in_flight: 1,
        max_retries_per_action: 1,
        max_gather_pages: 1,
        max_gather_items: 1,
        max_for_each_iterations: 1,
        max_together_branches: 1,
        max_repeat_attempts: 1,
        max_run_time_seconds: 10,
        max_result_bytes: 1,
        max_total_slots_written: 100,
        max_timer_entries: 1,
        max_trace_events: 1,
        max_queue_depth: 1,
        max_journal_batch_bytes: 1,
        max_ipc_payload_bytes: 1,
        max_blob_bytes: 1,
        max_input_bytes: 1,
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1,
        max_open_gates: 1,
        max_signal_subscriptions: 1,
        max_incident_emissions: 100,
        max_transitions: 10,
        max_journal_bytes_total: 1,
        max_value_store_entries,
        max_cost_units: 1,
    }
}

fn value_store_budget_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let mut nodes = Vec::new();
    nodes.push(vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(1)),
    ));
    nodes.push(vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::BuildList {
            items: Box::from([SlotIdx::ZERO]),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(2)),
    ));
    nodes.push(vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
        None,
        None,
    ));
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("value_store_budget"),
        digest: WorkflowDigest::from_bytes([8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::Bool(true)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

#[test]
fn drive_state_fails_when_value_store_entries_exceed_admitted_limit() {
    let Some(workflow) = value_store_budget_workflow() else {
        return;
    };
    let run = RunId::new(702);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        value_store_budget(0),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "value_store_entries",
            limit: 0,
        }))
    ));
    assert_eq!(state.store.total_arena_count(), 1);
}

#[test]
fn drive_state_at_value_store_limit_drives_to_finish() {
    let Some(workflow) = value_store_budget_workflow() else {
        return;
    };
    let run = RunId::new(703);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        value_store_budget(1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
    assert_eq!(state.store.total_arena_count(), 1);
}

#[test]
fn drive_state_without_admitted_budget_ignores_value_store_count() {
    let Some(workflow) = value_store_budget_workflow() else {
        return;
    };
    let run = RunId::new(704);
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
    assert_eq!(state.store.total_arena_count(), 1);
}
fn gather_items_budget(max_gather_items: u32) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 100,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_retries_per_action: 10,
        max_gather_pages: 10,
        max_gather_items,
        max_for_each_iterations: 10,
        max_together_branches: 10,
        max_repeat_attempts: 10,
        max_run_time_seconds: 100,
        max_result_bytes: 100,
        max_total_slots_written: 100,
        max_timer_entries: 100,
        max_trace_events: 100,
        max_queue_depth: 100,
        max_journal_batch_bytes: 100,
        max_ipc_payload_bytes: 100,
        max_blob_bytes: 100,
        max_input_bytes: 100,
        max_step_budget_per_tick: 100,
        max_transitions_per_tick: 100,
        max_open_gates: 10,
        max_signal_subscriptions: 10,
        max_incident_emissions: 100,
        max_transitions: 100,
        max_journal_bytes_total: 100,
        max_value_store_entries: 100,
        max_cost_units: 100,
    }
}

fn seed_collect_items_consumed(
    state: &mut crate::shard::types::RunState,
    run: vb_core::ids::RunId,
    cursor: usize,
) {
    state
        .collect_states
        .upsert(crate::primitives::collect::CollectPaginationState {
            run_id: run,
            collector_slot: SlotIdx::ZERO,
            source: vb_core::ids::ListId::new(1),
            current_page: vb_core::ids::ListId::new(1),
            cursor,
            page_size: cursor,
            item_count: cursor,
            limit: 100,
            time_limit_ms: None,
            start_millis: 0,
        })
        .expect("seed collect items");
}

#[test]
fn drive_state_fails_when_gather_items_exceed_admitted_limit() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(750);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .expect("run state builds");
    seed_collect_items_consumed(&mut state, run, 3);
    assert_eq!(state.collect_states.total_items_consumed(), 3);
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        gather_items_budget(2),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "gather_items",
            limit: 2,
        }))
    ));
}

#[test]
fn drive_state_at_gather_items_limit_finishes() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(751);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .expect("run state builds");
    seed_collect_items_consumed(&mut state, run, 3);
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        gather_items_budget(3),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}

#[test]
fn drive_state_without_admitted_budget_ignores_gather_items() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(752);
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .expect("run state builds");
    seed_collect_items_consumed(&mut state, run, 3);
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}

fn transition_budget(max_transitions: u64) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 1,
        max_parallel_in_flight: 1,
        max_retries_per_action: 1,
        max_gather_pages: 1,
        max_gather_items: 1,
        max_for_each_iterations: 1,
        max_together_branches: 1,
        max_repeat_attempts: 1,
        max_run_time_seconds: 10,
        max_result_bytes: 1,
        max_total_slots_written: 1,
        max_timer_entries: 1,
        max_trace_events: 1,
        max_queue_depth: 1,
        max_journal_batch_bytes: 1,
        max_ipc_payload_bytes: 1,
        max_blob_bytes: 1,
        max_input_bytes: 1,
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1,
        max_open_gates: 1,
        max_signal_subscriptions: 1,
        max_incident_emissions: 100,
        max_transitions,
        max_journal_bytes_total: 1,
        max_value_store_entries: 1,
        max_cost_units: 1,
    }
}

fn transition_budget_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let mut nodes = Vec::new();
    nodes.push(vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(1)),
    ));
    let mut index = 1;
    while index < 10 {
        nodes.push(vb_core::workflow::node(
            StepIdx::new(index),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(index + 1)),
        ));
        index += 1;
    }
    nodes.push(vb_core::workflow::node(
        StepIdx::new(10),
        CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
        None,
        None,
    ));
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("transition_budget"),
        digest: WorkflowDigest::from_bytes([9; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

#[test]
fn drive_state_caps_tick_at_admitted_total_transitions_and_fails_on_exhaustion() {
    let Some(workflow) = transition_budget_workflow() else {
        return;
    };
    let run = RunId::new(700);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        transition_budget(3),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::StepBudgetExhausted)));
    assert_eq!(state.frame.executed(), 3);
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "transitions",
            limit: 3,
        }))
    ));
    assert_eq!(state.frame.executed(), 3);
}

#[test]
fn drive_state_without_admitted_budget_drives_to_finish() {
    let Some(workflow) = transition_budget_workflow() else {
        return;
    };
    let run = RunId::new(701);
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
    assert_eq!(state.frame.executed(), 11);
}

#[test]
fn queued_retry_flushes_emitted_prefix_and_resumes_drive() -> Result<(), String> {
    let (journal, _queue, _adapter, shared) =
        queued_journal_fixture("vb-runtime-retry-watermark-", 16, 4)?;
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let run = RunId::new(710);
    submit_run(&mut shard, run, retry_workflow()?);
    // Submission events sit in the queue: nothing durable yet.
    assert!(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?
            .is_empty()
    );
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: pending_ticket(&shard, run, StepIdx::new(1), 1),
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // The retry flushed the emitted prefix (durable) and resumed the drive.
    assert!(
        !journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?
            .is_empty()
    );
    let state = shard
        .run_state_get(run)
        .ok_or("run should exist after queued retry")?;
    assert_eq!(state.action_attempts.get(1).copied(), Some(2));
    assert_eq!(shard.active_run_count(), 1);
    Ok(())
}

/// Journal stub that always reports a pending high-water, even after a
/// flush. Exercises the fail-closed arm of the retry watermark gate.
struct NeverDurableJournal;

impl crate::journal::RuntimeJournal for NeverDurableJournal {
    fn append(&self, _event: RuntimeJournalEvent) -> Result<(), crate::RuntimeError> {
        Ok(())
    }

    fn probe(&self) -> Result<(), crate::RuntimeError> {
        Ok(())
    }

    fn append_sequenced(
        &self,
        _event: RuntimeJournalEvent,
        _seq: vb_storage::EventSeq,
    ) -> Result<(), crate::RuntimeError> {
        Ok(())
    }

    fn append_sequenced_batch(
        &self,
        _events: &[RuntimeJournalEvent],
        _start_seq: vb_storage::EventSeq,
    ) -> Result<(), crate::RuntimeError> {
        Ok(())
    }

    fn pending_high_water_for_run(
        &self,
        _run: RunId,
    ) -> Result<Option<vb_storage::EventSeq>, crate::RuntimeError> {
        Ok(Some(vb_storage::EventSeq::new(0)))
    }
}

#[test]
fn blocked_retry_reports_typed_error_and_keeps_run() -> Result<(), String> {
    let shared: SharedRuntimeJournal = std::sync::Arc::new(NeverDurableJournal);
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let run = RunId::new(711);
    submit_run(&mut shard, run, retry_workflow()?);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: pending_ticket(&shard, run, StepIdx::new(1), 1),
            failure: retryable_failure(),
        }),
        Ok(())
    );
    match shard.tick() {
        Err(crate::RuntimeError::RetryBlockedOnUndrainedJournal { run: blocked }) => {
            assert_eq!(blocked, run);
        }
        other => {
            return Err(format!(
                "expected RetryBlockedOnUndrainedJournal, got {other:?}"
            ));
        }
    }
    // Nothing mutated: run, pending action, and attempt counter are intact.
    let state = shard
        .run_state_get(run)
        .ok_or("blocked retry must keep the run")?;
    assert_eq!(state.action_attempts.get(1).copied(), Some(1));
    pending_ticket(&shard, run, StepIdx::new(1), 1);
    Ok(())
}

fn result_bytes_budget(max_result_bytes: u32) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 100,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_retries_per_action: 10,
        max_gather_pages: 10,
        max_gather_items: 10,
        max_for_each_iterations: 10,
        max_together_branches: 10,
        max_repeat_attempts: 10,
        max_run_time_seconds: 100,
        max_result_bytes,
        max_total_slots_written: 100,
        max_timer_entries: 100,
        max_trace_events: 100,
        max_queue_depth: 100,
        max_journal_batch_bytes: 100,
        max_ipc_payload_bytes: 100,
        max_blob_bytes: 100,
        max_input_bytes: 100,
        max_step_budget_per_tick: 100,
        max_transitions_per_tick: 100,
        max_open_gates: 10,
        max_signal_subscriptions: 10,
        max_incident_emissions: 100,
        max_transitions: 100,
        max_journal_bytes_total: 100,
        max_value_store_entries: 100,
        max_cost_units: 100,
    }
}

fn list_result_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let first = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(1)),
    );
    let second = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(1),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(2)),
    );
    let build = vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::BuildList {
            items: Box::from([SlotIdx::ZERO, SlotIdx::new(1)]),
        },
        Some(SlotIdx::new(2)),
        Some(StepIdx::new(3)),
    );
    let finish = vb_core::workflow::node(
        StepIdx::new(3),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(2),
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("list_result_budget"),
        digest: WorkflowDigest::from_bytes([13; 32]),
        nodes: Box::from([first, second, build, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(10), ConstValue::I64(20)]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

#[test]
fn drive_state_fails_when_finished_list_deep_size_exceeds_admitted_limit() {
    let workflow = list_result_workflow().expect("list result workflow builds");
    let run = RunId::new(750);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .expect("run state builds");
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        result_bytes_budget(15),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "result_bytes",
            limit: 15,
        }))
    ));
}

#[test]
fn drive_state_at_finished_list_deep_size_limit_finishes() {
    let workflow = list_result_workflow().expect("list result workflow builds");
    let run = RunId::new(751);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .expect("run state builds");
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        result_bytes_budget(16),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}

#[test]
fn drive_state_without_admitted_budget_ignores_finished_list_size() {
    let workflow = list_result_workflow().expect("list result workflow builds");
    let run = RunId::new(752);
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .expect("run state builds");
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}

fn gather_pages_budget(max_gather_pages: u32) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 100,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_retries_per_action: 10,
        max_gather_pages,
        max_gather_items: 10,
        max_for_each_iterations: 10,
        max_together_branches: 10,
        max_repeat_attempts: 10,
        max_run_time_seconds: 100,
        max_result_bytes: 100,
        max_total_slots_written: 100,
        max_timer_entries: 100,
        max_trace_events: 100,
        max_queue_depth: 100,
        max_journal_batch_bytes: 100,
        max_ipc_payload_bytes: 100,
        max_blob_bytes: 100,
        max_input_bytes: 100,
        max_step_budget_per_tick: 100,
        max_transitions_per_tick: 100,
        max_open_gates: 10,
        max_signal_subscriptions: 10,
        max_incident_emissions: 100,
        max_transitions: 100,
        max_journal_bytes_total: 100,
        max_value_store_entries: 100,
        max_cost_units: 100,
    }
}

fn deliver_collect_page(
    states: &mut crate::primitives::collect::CollectStates,
    run: RunId,
    page: u32,
) {
    states
        .upsert(crate::primitives::collect::CollectPaginationState {
            run_id: run,
            collector_slot: SlotIdx::new(1),
            source: vb_core::ids::ListId::new(1),
            current_page: vb_core::ids::ListId::new(page),
            cursor: 1,
            page_size: 1,
            item_count: 1,
            limit: 1,
            time_limit_ms: None,
            start_millis: 0,
        })
        .expect("page upsert must succeed");
}

#[test]
fn drive_state_fails_when_delivered_pages_exceed_admitted_limit() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(764);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .expect("run state builds");
    deliver_collect_page(&mut state.collect_states, run, 1);
    deliver_collect_page(&mut state.collect_states, run, 2);
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        gather_pages_budget(1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "gather_pages",
            limit: 1,
        }))
    ));
}

#[test]
fn drive_state_at_delivered_pages_limit_finishes() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(765);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .expect("run state builds");
    deliver_collect_page(&mut state.collect_states, run, 1);
    deliver_collect_page(&mut state.collect_states, run, 2);
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        gather_pages_budget(2),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}

#[test]
fn drive_state_without_admitted_budget_ignores_delivered_pages() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(766);
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .expect("run state builds");
    deliver_collect_page(&mut state.collect_states, run, 1);
    deliver_collect_page(&mut state.collect_states, run, 2);
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}

#[test]
fn drive_state_fails_when_delivered_pages_exceed_limit_after_removal() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(767);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .expect("run state builds");
    deliver_collect_page(&mut state.collect_states, run, 1);
    deliver_collect_page(&mut state.collect_states, run, 2);
    state.collect_states.remove(run, SlotIdx::new(1));
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        gather_pages_budget(1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "gather_pages",
            limit: 1,
        }))
    ));
}

fn run_time_budget(max_run_time_seconds: u64) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 100,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_retries_per_action: 10,
        max_gather_pages: 10,
        max_gather_items: 10,
        max_for_each_iterations: 10,
        max_together_branches: 10,
        max_repeat_attempts: 10,
        max_run_time_seconds,
        max_result_bytes: 100,
        max_total_slots_written: 100,
        max_timer_entries: 100,
        max_trace_events: 100,
        max_queue_depth: 100,
        max_journal_batch_bytes: 100,
        max_ipc_payload_bytes: 100,
        max_blob_bytes: 100,
        max_input_bytes: 100,
        max_step_budget_per_tick: 100,
        max_transitions_per_tick: 100,
        max_open_gates: 10,
        max_signal_subscriptions: 10,
        max_incident_emissions: 100,
        max_transitions: 100,
        max_journal_bytes_total: 100,
        max_value_store_entries: 100,
        max_cost_units: 100,
    }
}

#[test]
fn drive_state_fails_when_run_time_exceeds_admitted_limit() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(744);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .expect("run state builds");
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        run_time_budget(10),
    ));
    state.run_started_millis = Some(0);
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "run_time",
            limit: 10,
        }))
    ));
}

#[test]
fn drive_state_within_run_time_limit_finishes() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(745);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .expect("run state builds");
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        run_time_budget(1000),
    ));
    state.run_started_millis =
        crate::shard::helpers::wall_clock_millis_since_epoch();
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}

#[test]
fn drive_state_without_recorded_start_records_wall_clock_on_first_drive() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(746);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .expect("run state builds");
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        run_time_budget(1000),
    ));
    assert_eq!(state.run_started_millis, None);
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
    assert_ne!(state.run_started_millis, None);
}

#[test]
fn drive_state_without_admitted_budget_ignores_run_time() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(747);
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .expect("run state builds");
    state.run_started_millis = Some(0);
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}

fn tiny_journal_contract() -> ResourceContract {
    ResourceContract {
        max_journal_batch_bytes: 1,
        ..ResourceContract::DEFAULT
    }
}

fn tiny_journal_finished_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_const = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::new(0)),
        Some(StepIdx::new(1)),
    );
    let finish = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("tiny_journal_finished"),
        digest: WorkflowDigest::from_bytes([21; 32]),
        nodes: Box::from([set_const, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: tiny_journal_contract(),
        input_slots: Box::from([]),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn tiny_journal_chain_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let mut nodes = Vec::new();
    for step in 0..6u16 {
        nodes.push(vb_core::workflow::node(
            StepIdx::new(step),
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::new(0)),
            Some(StepIdx::new(step + 1)),
        ));
    }
    nodes.push(vb_core::workflow::node(
        StepIdx::new(6),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    ));
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("tiny_journal_chain"),
        digest: WorkflowDigest::from_bytes([22; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(1)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: tiny_journal_contract(),
        input_slots: Box::from([]),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn tiny_journal_ask_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
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
    let ask = vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::Ask {
            prompt: SlotIdx::ZERO,
            timeout_slot: Some(SlotIdx::new(1)),
        },
        None,
        Some(StepIdx::new(3)),
    );
    let resume = vb_core::workflow::node(
        StepIdx::new(3),
        CompiledNodeKind::AskResume {
            answer: SlotIdx::new(2),
        },
        None,
        Some(StepIdx::new(4)),
    );
    let finish_node = vb_core::workflow::node(
        StepIdx::new(4),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(2),
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("tiny_journal_ask"),
        digest: WorkflowDigest::from_bytes([23; 32]),
        nodes: Box::from([set_prompt, set_timeout, ask, resume, finish_node]),
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
        resource_contract: tiny_journal_contract(),
        input_slots: Box::from([]),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn journal_bytes_budget(total: u64) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 100,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_retries_per_action: 10,
        max_gather_pages: 10,
        max_gather_items: 10,
        max_for_each_iterations: 10,
        max_together_branches: 10,
        max_repeat_attempts: 10,
        max_run_time_seconds: 100,
        max_result_bytes: 100,
        max_total_slots_written: 100,
        max_timer_entries: 100,
        max_trace_events: 100,
        max_queue_depth: 100,
        max_journal_batch_bytes: 100,
        max_ipc_payload_bytes: 100,
        max_blob_bytes: 100,
        max_input_bytes: 100,
        max_step_budget_per_tick: 100,
        max_transitions_per_tick: 100,
        max_open_gates: 10,
        max_signal_subscriptions: 10,
        max_incident_emissions: 100,
        max_transitions: 100,
        max_journal_bytes_total: total,
        max_value_store_entries: 100,
        max_cost_units: 100,
    }
}

fn submit_with_journal(
    shard: &mut Shard,
    run: RunId,
    workflow: vb_core::workflow::CompiledWorkflow,
) {
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithContracts {
            run,
            workflow,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0
            )),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
}

fn run_failed_count(events: &[RuntimeJournalEvent], run: RunId) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(
                event,
                RuntimeJournalEvent::RunFailed { run: r, .. } if *r == run
            )
        })
        .count()
}

#[test]
fn charge_drive_journal_bytes_enforces_cumulative_total() -> Result<(), String> {
    let workflow = two_slot_workflow().ok_or_else(|| "two slot workflow builds".to_string())?;
    let run = RunId::new(770);
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
        journal_bytes_budget(len.checked_mul(2).ok_or_else(|| "budget overflow".to_string())?),
    ));
    assert!(Shard::charge_drive_journal_bytes(&mut state, &[event.clone()])
        .map_err(|error| format!("first charge failed: {error:?}"))?);
    assert_eq!(state.journal_bytes_written, len);
    assert!(Shard::charge_drive_journal_bytes(&mut state, &[event.clone()])
        .map_err(|error| format!("second charge failed: {error:?}"))?);
    assert_eq!(
        state.journal_bytes_written,
        len.checked_mul(2).ok_or_else(|| "counter overflow".to_string())?
    );
    assert!(!Shard::charge_drive_journal_bytes(&mut state, &[event])
        .map_err(|error| format!("third charge failed: {error:?}"))?);
    assert_eq!(
        state.journal_bytes_written,
        len.checked_mul(2).ok_or_else(|| "counter overflow".to_string())?
    );
    Ok(())
}

#[test]
fn charge_drive_journal_bytes_without_budget_charges_without_enforcing() -> Result<(), String> {
    let workflow = two_slot_workflow().ok_or_else(|| "two slot workflow builds".to_string())?;
    let run = RunId::new(771);
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .ok_or_else(|| "run state builds".to_string())?;
    let event = RuntimeJournalEvent::RunFailed { run, attempt: 1 };
    let bytes = postcard::to_allocvec(&event)
        .map_err(|error| format!("event encode failed: {error:?}"))?;
    let len = u64::try_from(bytes.len()).map_err(|_| "event length overflow".to_string())?;
    assert!(Shard::charge_drive_journal_bytes(&mut state, &[event])
        .map_err(|error| format!("charge failed: {error:?}"))?);
    assert_eq!(state.journal_bytes_written, len);
    Ok(())
}

#[test]
fn drive_finished_run_falls_back_to_single_run_failed_when_journal_budget_exceeded(
) -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("tiny_journal_finished", tiny_journal_finished_workflow())?;
    let run = RunId::new(772);
    submit_with_journal(&mut shard, run, wf);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    assert_eq!(shard.counters().snapshot().runs_completed, 0);
    assert!(shard.terminal_runs_contains(run));
    let events = require_snapshot(&journal)?;
    assert_eq!(run_failed_count(&events, run), 1);
    assert!(
        !events.iter().any(|event| matches!(
            event,
            RuntimeJournalEvent::RunFinished { run: r, .. } if *r == run
        )),
        "finished event must not be journaled after fallback: {events:?}"
    );
    Ok(())
}

#[test]
fn drive_continue_run_falls_back_to_single_run_failed_when_journal_budget_exceeded(
) -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("tiny_journal_chain", tiny_journal_chain_workflow())?;
    let run = RunId::new(773);
    submit_with_journal(&mut shard, run, wf);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    assert!(shard.terminal_runs_contains(run));
    let events = require_snapshot(&journal)?;
    assert_eq!(run_failed_count(&events, run), 1);
    assert!(
        !events.iter().any(|event| matches!(
            event,
            RuntimeJournalEvent::StepSucceeded { run: r, .. } if *r == run
        )),
        "drive evidence must not be journaled after fallback: {events:?}"
    );
    Ok(())
}

#[test]
fn drive_awaiting_run_falls_back_to_single_run_failed_when_journal_budget_exceeded(
) -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("tiny_journal_ask", tiny_journal_ask_workflow())?;
    let run = RunId::new(774);
    submit_with_journal(&mut shard, run, wf);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    assert!(shard.terminal_runs_contains(run));
    let events = require_snapshot(&journal)?;
    assert_eq!(run_failed_count(&events, run), 1);
    assert!(
        !events.iter().any(|event| matches!(
            event,
            RuntimeJournalEvent::AskScheduled { run: r, .. } if *r == run
        )),
        "ask schedule must not be journaled after fallback: {events:?}"
    );
    Ok(())
}

#[test]
fn journal_bytes_counter_matches_appended_header_and_drive_events() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("ask_then_finish", ask_workflow())?;
    let run = RunId::new(775);
    submit_with_journal(&mut shard, run, wf);
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    let state = shard
        .run_state_get(run)
        .ok_or_else(|| "run should still be active awaiting ask".to_string())?;
    let events = require_snapshot(&journal)?;
    let mut measured: u64 = 0;
    for event in &events {
        let encoded = postcard::to_allocvec(event)
            .map_err(|error| format!("event encode failed: {error:?}"))?;
        measured = measured
            .checked_add(u64::try_from(encoded.len()).map_err(|_| "length overflow")?)
            .ok_or_else(|| "measured total overflow".to_string())?;
    }
    assert!(measured > 0, "journal must contain events");
    assert_eq!(state.journal_bytes_written, measured);
    Ok(())
}

#[test]
fn pre_persisted_submit_seeds_counter_with_admission_event_only() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("ask_then_finish", ask_workflow())?;
    let run = RunId::new(776);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitPrePersisted {
            run,
            workflow: wf,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    let state = shard
        .run_state_get(run)
        .ok_or_else(|| "run should still be active awaiting ask".to_string())?;
    let events = require_snapshot(&journal)?;
    assert!(
        !events.iter().any(|event| matches!(
            event,
            RuntimeJournalEvent::RunSubmitted { run: r, .. } if *r == run
        )),
        "pre-persisted submit must not journal a second submitted event: {events:?}"
    );
    assert!(
        events.iter().any(|event| matches!(
            event,
            RuntimeJournalEvent::RunAdmission { admission } if admission.run_id() == run
        )),
        "pre-persisted submit must journal the admission event: {events:?}"
    );
    let mut measured: u64 = 0;
    for event in &events {
        let encoded = postcard::to_allocvec(event)
            .map_err(|error| format!("event encode failed: {error:?}"))?;
        measured = measured
            .checked_add(u64::try_from(encoded.len()).map_err(|_| "length overflow")?)
            .ok_or_else(|| "measured total overflow".to_string())?;
    }
    assert!(measured > 0, "journal must contain events");
    assert_eq!(state.journal_bytes_written, measured);
    Ok(())
}

fn measured_journal_bytes(events: &[RuntimeJournalEvent]) -> Result<u64, String> {
    let mut measured: u64 = 0;
    for event in events {
        let encoded = postcard::to_allocvec(event)
            .map_err(|error| format!("event encode failed: {error:?}"))?;
        measured = measured
            .checked_add(u64::try_from(encoded.len()).map_err(|_| "length overflow")?)
            .ok_or_else(|| "measured total overflow".to_string())?;
    }
    Ok(measured)
}

#[test]
fn empty_evidence_await_timer_charges_ask_scheduled_bytes() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("ask", ask_workflow())?;
    let run = RunId::new(777);
    submit_run(&mut shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));
    let before_counter = shard
        .run_state_get(run)
        .ok_or_else(|| "run should still be active awaiting ask".to_string())?
        .journal_bytes_written;
    let before_events = require_snapshot(&journal)?;
    let before_measured = measured_journal_bytes(&before_events)?;
    let before_asks = before_events
        .iter()
        .filter(|event| {
            matches!(event, RuntimeJournalEvent::AskScheduled { run: ask_run, .. } if *ask_run == run)
        })
        .count();
    let state = shard
        .take_run_state(run)
        .map_err(|error| format!("suspended ask state must be checked out: {error:?}"))?;
    let _previous_timer = shard.pending_timer_remove(run);
    assert!(matches!(
        shard
            .await_timer(run, state, PendingTimerKind::Ask)
            .map_err(|error| format!("ask reschedule failed: {error:?}"))?,
        crate::shard::transitions::TransitionOutcome::Applied
    ));
    let after_counter = shard
        .run_state_get(run)
        .ok_or_else(|| "run should still be active after reschedule".to_string())?
        .journal_bytes_written;
    let after_events = require_snapshot(&journal)?;
    let after_measured = measured_journal_bytes(&after_events)?;
    let after_asks = after_events
        .iter()
        .filter(|event| {
            matches!(event, RuntimeJournalEvent::AskScheduled { run: ask_run, .. } if *ask_run == run)
        })
        .count();
    assert_eq!(after_asks, before_asks + 1);
    let charged = after_counter
        .checked_sub(before_counter)
        .ok_or_else(|| "counter must not decrease across the reschedule".to_string())?;
    let appended = after_measured
        .checked_sub(before_measured)
        .ok_or_else(|| "journal must not shrink across the reschedule".to_string())?;
    assert_eq!(charged, appended);
    assert!(
        charged > 0,
        "rescheduled ask must charge its journal event"
    );
    Ok(())
}

#[test]
fn retry_await_charges_second_action_scheduled_ticket_bytes() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let run = RunId::new(778);
    submit_run(&mut shard, run, retry_workflow()?);
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
            timeout_slot: Some(SlotIdx::new(1)),
        },
        None,
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
            timeout_slot: Some(SlotIdx::new(1)),
        },
        None,
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
        input_slots: Box::from([]),
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
    submit_run(&mut shard, run, wf);
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
    submit_run(&mut shard, run, wf);
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
    submit_run(&mut shard, run, wf);
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
    submit_run(&mut shard, run, wf);
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
    submit_run(&mut shard, run, wf);
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
    submit_run(&mut shard, run, wf);
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
    submit_run(&mut shard, run, wf);
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
    submit_run(&mut shard, run, wf);
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
    submit_run(&mut shard, run, wf);
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
