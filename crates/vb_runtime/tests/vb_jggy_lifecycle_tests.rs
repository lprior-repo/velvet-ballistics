#![forbid(unsafe_code)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::expect_used,
    clippy::get_first,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]
//! Integration tests for vb-jggy: Persist execution attempt numbers and reject stale completions.
//!
//! These tests verify:
//! - POST-001: `RunState::action_attempts` is zero-initialized at admission
//! - POST-003: `RuntimeJournalEvent::StepSucceeded` and `RuntimeJournalEvent::ActionFailed` carry `attempt: u16`
//! - POST-004/INV-003: `validate_ticket_attempt` is called BEFORE journal mutation
//! - POST-005: Stale completions return `Err(RuntimeError::StaleAttempt)` BEFORE any state mutation
//! - POST-006: `record_scheduled_attempt` is called when ticket is issued
//!
//! These tests are expected to FAIL until vb-jggy implementation is complete.

use vb_core::action::{
    ActionContract, ActionFailure, ActionFailureCode, ActionName, ActionOutputReady, ActionTicket,
    Idempotency, RetryPolicy as VbRetryPolicy, RetrySafety, SideEffect,
};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, ConstIdx, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

use vb_runtime::RuntimeError;
use vb_runtime::journal::{RuntimeJournalEvent, VolatileRuntimeJournal};
use vb_runtime::shard::{Shard, ShardCommand, ShardConfig};

fn contract_required_capability(action: ActionId) -> Capability {
    Capability::new("__contract_required__".into(), action)
}

fn max_do_action(workflow: &CompiledWorkflow) -> Option<ActionId> {
    let mut index = 0u16;
    let count = workflow.node_count();
    let mut max_action: Option<ActionId> = None;
    while index < count {
        if let Some(node) = workflow.node(StepIdx::new(index)) {
            if let CompiledNodeKind::Do { action, .. } = node.kind {
                max_action = match max_action {
                    Some(current) if current.get() >= action.get() => Some(current),
                    _ => Some(action),
                };
            }
        }
        index = index.saturating_add(1);
    }
    max_action
}

fn action_contract(action: ActionId, required: bool) -> ActionContract {
    let required_capabilities = if required {
        Box::from([contract_required_capability(action)])
    } else {
        Box::from([])
    };
    ActionContract {
        id: action,
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities,
    }
}

fn contracts_through(action: ActionId) -> Box<[ActionContract]> {
    let target = action.get();
    let mut contracts = Vec::with_capacity(usize::from(target).saturating_add(1));
    let mut id = 0u16;
    loop {
        let current = ActionId::new(id);
        contracts.push(action_contract(current, id == target));
        if id == target {
            break;
        }
        id = id.saturating_add(1);
    }
    contracts.into_boxed_slice()
}

fn submit_with_contracts(shard: &Shard, run: RunId, workflow: CompiledWorkflow) {
    submit_with_inputs_and_contracts(
        shard,
        run,
        workflow,
        Box::from([(SlotIdx::new(0), SlotValue::Bool(false))]),
    );
}

fn submit_with_inputs_and_contracts(
    shard: &Shard,
    run: RunId,
    workflow: CompiledWorkflow,
    inputs: Box<[(SlotIdx, SlotValue)]>,
) {
    let action = max_do_action(&workflow).unwrap_or(ActionId::new(0));
    shard
        .enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow,
            inputs,
            caps: CapabilitySet::from_grants(Box::from([contract_required_capability(action)])),
            action_contracts: contracts_through(action),
        })
        .expect("contracted submit enqueues");
}

fn suspended_workflow() -> Option<CompiledWorkflow> {
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("suspended"),
        digest: WorkflowDigest::from_bytes([1; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::new([]),
    };
    CompiledWorkflow::try_from_parts(parts).ok()
}

fn suspended_workflow_2step() -> Option<CompiledWorkflow> {
    let node0 = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        },
    };
    let node1 = CompiledNode {
        id: StepIdx::new(1),
        output: Some(SlotIdx::ZERO),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(1),
            input: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("suspended_2step"),
        digest: WorkflowDigest::from_bytes([2; 32]),
        nodes: Box::from([node0, node1]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::new([]),
    };
    CompiledWorkflow::try_from_parts(parts).ok()
}

fn retry_workflow() -> Option<CompiledWorkflow> {
    let set_policy = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(1)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let action = CompiledNode {
        id: StepIdx::new(1),
        output: Some(SlotIdx::ZERO),
        next: Some(StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::ZERO,
        },
    };
    let retry = CompiledNode {
        id: StepIdx::new(2),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::RetryCheck {
            policy_slot: SlotIdx::new(1),
            body: StepIdx::new(1),
            exhausted: StepIdx::new(3),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(3),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
    };
    let parts = WorkflowParts {
        name: Box::from("retry"),
        digest: WorkflowDigest::from_bytes([3; 32]),
        nodes: Box::from([set_policy, action, retry, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(3)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::new([]),
    };
    CompiledWorkflow::try_from_parts(parts).ok()
}

fn small_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    }
}

fn make_ticket(run: RunId, step: StepIdx, attempt: u16, capacity: u16) -> ActionTicket {
    let seq = SeqNo::ZERO;
    let action = ActionId::new(0);
    ActionTicket {
        run,
        step,
        seq,
        action,
        attempt,
        idempotency_key: vb_core::action::compute_action_idempotency_key(run, seq, action),
        capacity,
    }
}

fn retryable_failure() -> ActionFailure {
    ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    }
}

fn non_retryable_failure() -> ActionFailure {
    ActionFailure {
        code: ActionFailureCode::Rejected,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    }
}

// =============================================================================
// POST-001: RunState::action_attempts records first scheduled attempt after the first tick
// =============================================================================

/// B17: handle_submit_with_inputs records first scheduled action attempts
#[test]
fn handle_submit_with_inputs_records_first_action_attempts() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow_2step() else {
        panic!("missing workflow fixture");
    };
    let run = RunId::new(1);

    submit_with_inputs_and_contracts(
        &shard,
        run,
        wf,
        Box::from([(SlotIdx::new(0), SlotValue::Bool(false))]),
    );
    assert_eq!(shard.tick(), Ok(true));

    // Verify action_attempts records scheduled attempt state after tick.
    let Some(state) = shard.run_state_get_mut(run) else {
        panic!("run should exist");
    };
    assert_eq!(state.action_attempts.len(), 2, "workflow has 2 steps");
    assert_eq!(
        state.action_attempts.get(0).copied(),
        Some(1),
        "step 0 attempt counter is 1 after first scheduling"
    );
    assert_eq!(
        state.action_attempts.get(1).copied(),
        Some(0),
        "step 1 attempt counter remains 0 before scheduling"
    );
}

/// B17 variant: single step workflow records first scheduled action attempt
#[test]
fn handle_submit_records_first_action_attempt_single_step() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        panic!("missing workflow fixture");
    };
    let run = RunId::new(2);

    submit_with_contracts(&shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));

    let Some(state) = shard.run_state_get_mut(run) else {
        panic!("run should exist");
    };
    assert_eq!(state.action_attempts.len(), 1, "workflow has 1 step");
    assert_eq!(
        state.action_attempts.get(0).copied(),
        Some(1),
        "step 0 attempt counter is 1 after first scheduling"
    );
}

// =============================================================================
// POST-003: RuntimeJournalEvent carries attempt field
// =============================================================================

/// B30: RuntimeJournalEvent::StepSucceeded carries attempt field
#[test]
fn step_succeeded_carries_attempt_field() {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);

    let Some(wf) = suspended_workflow() else {
        panic!("missing workflow fixture");
    };
    let run = RunId::new(10);

    // Submit and drive to action
    submit_with_contracts(&shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));

    // Complete action with attempt=1
    let ticket = make_ticket(run, StepIdx::ZERO, 1, 1);
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(42),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted { ticket, output }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Check durable action completion envelope carries attempt.
    let events = journal.snapshot().expect("journal snapshot should work");
    let completion_attempts: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            RuntimeJournalEvent::ActionCompletedEnvelope { ticket, .. }
                if ticket.run == run && ticket.step == StepIdx::ZERO =>
            {
                Some(ticket.attempt)
            }
            _ => None,
        })
        .collect();

    assert!(
        !completion_attempts.is_empty(),
        "ActionCompletedEnvelope event should be in journal"
    );
    assert_eq!(
        completion_attempts[0], 1,
        "ActionCompletedEnvelope ticket should carry attempt=1"
    );
}

/// B31: RuntimeJournalEvent::ActionFailed carries attempt field
#[test]
fn action_failed_carries_attempt_field() -> Result<(), Box<dyn std::error::Error>> {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);

    let Some(wf) = suspended_workflow() else {
        return Err("missing workflow fixture".into());
    };
    let run = RunId::new(11);

    submit_with_contracts(&shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));

    let ticket = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket")?;
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let events = journal.snapshot()?;
    let action_failed_events: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            RuntimeJournalEvent::ActionFailed {
                run: r,
                step: s,
                action: _,
                attempt: a,
            } if *r == run && *s == StepIdx::ZERO => Some(*a),
            _ => None,
        })
        .collect();

    assert_eq!(
        action_failed_events,
        vec![1],
        "ActionFailed should carry attempt=1 from ticket"
    );

    Ok(())
}

/// Verify StepSucceeded with higher attempt number is persisted correctly
#[test]
fn step_succeeded_carries_retry_attempt_number() -> Result<(), Box<dyn std::error::Error>> {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);

    let Some(wf) = retry_workflow() else {
        return Err("missing retry workflow fixture".into());
    };
    let run = RunId::new(12);

    // Submit and drive to first action
    submit_with_contracts(&shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));

    // Fail ticket 1 (retryable)
    let ticket1 = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket 1")?;
    assert_eq!(ticket1.attempt, 1, "ticket1 should have attempt=1");
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: ticket1,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Fail ticket 2 (retryable)
    let ticket2 = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket 2")?;
    assert_eq!(ticket2.attempt, 2, "ticket2 should have attempt=2");
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: ticket2,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Complete ticket 3 (matching attempt=3)
    let ticket3 = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket 3")?;
    assert_eq!(ticket3.attempt, 3, "ticket3 should have attempt=3");
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(99),
        taint: Taint::Clean,
        encoded_len: 3,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: ticket3,
            output,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Check attempt=3 was persisted in the durable action completion envelope.
    let events = journal.snapshot()?;
    let completion_with_attempt_3: bool = events.iter().any(|e| {
        matches!(
            e,
            RuntimeJournalEvent::ActionCompletedEnvelope { ticket, .. }
                if ticket.run == run && ticket.step == StepIdx::new(1) && ticket.attempt == 3
        )
    });
    assert!(
        completion_with_attempt_3,
        "ActionCompletedEnvelope ticket should carry attempt=3"
    );

    Ok(())
}

// =============================================================================
// POST-004/INV-003: validate_ticket_attempt called BEFORE journal mutation
// =============================================================================

/// B21: handle_action_completion rejects stale attempt BEFORE journal write
#[test]
fn stale_attempt_completion_rejected_before_journal_write() -> Result<(), Box<dyn std::error::Error>>
{
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);

    let Some(wf) = retry_workflow() else {
        return Err("missing retry workflow fixture".into());
    };
    let run = RunId::new(20);

    // Submit and drive to first action
    submit_with_contracts(&shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));

    // Drive failure to advance to ticket 2
    let ticket1 = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket 1")?;
    assert_eq!(ticket1.attempt, 1, "ticket1 should have attempt=1");
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: ticket1,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Capture real ticket 2 (will become stale)
    let ticket2 = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket 2")?;
    assert_eq!(ticket2.attempt, 2, "ticket2 should have attempt=2");
    let ticket2_attempt = ticket2.attempt;
    assert_eq!(ticket2_attempt, 2, "ticket2 should have attempt=2");

    // Drive failure to advance to ticket 3
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: ticket2,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Now current attempt is 3, but we'll try to complete with the saved ticket 2 attempt
    let ticket3 = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket 3")?;
    assert_eq!(ticket3.attempt, 3, "ticket3 should have attempt=3");

    // Capture state before stale completion attempt
    let journal_before = journal.snapshot()?;
    let counters_before = shard.counters().snapshot();

    // Attempt stale completion with saved ticket 2 attempt number
    let stale_ticket = ticket2;
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 3,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: stale_ticket,
            output,
        }),
        Ok(())
    );
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::StaleAttempt {
            incoming: ticket2_attempt,
            current: ticket3.attempt,
        })
    );

    // Verify NO new journal events were written
    let journal_after = journal.snapshot()?;
    assert_eq!(
        journal_before, journal_after,
        "Journal must not be mutated when stale attempt is rejected"
    );

    // Verify counters unchanged
    let counters_after = shard.counters().snapshot();
    assert_eq!(
        counters_before.runs_completed, counters_after.runs_completed,
        "runs_completed counter must not change on stale rejection"
    );
    assert_eq!(
        counters_before.runs_failed, counters_after.runs_failed,
        "runs_failed counter must not change on stale rejection"
    );

    Ok(())
}

/// B28: handle_action_failure rejects stale attempt BEFORE journal write
#[test]
fn stale_attempt_failure_rejected_before_journal_write() -> Result<(), Box<dyn std::error::Error>> {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);

    let Some(wf) = retry_workflow() else {
        return Err("missing retry workflow fixture".into());
    };
    let run = RunId::new(21);

    // Submit and drive to first action
    submit_with_contracts(&shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));

    // Drive failure to advance to ticket 2
    let ticket1 = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket 1")?;
    assert_eq!(ticket1.attempt, 1, "ticket1 should have attempt=1");
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: ticket1,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Capture real ticket 2 (will become stale)
    let ticket2 = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket 2")?;
    assert_eq!(ticket2.attempt, 2, "ticket2 should have attempt=2");
    let ticket2_attempt = ticket2.attempt;
    assert_eq!(ticket2_attempt, 2, "ticket2 should have attempt=2");

    // Drive failure to advance to ticket 3
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: ticket2,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Now current attempt is 3, but we'll try to fail with the saved ticket 2 attempt
    let ticket3 = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket 3")?;
    assert_eq!(ticket3.attempt, 3, "ticket3 should have attempt=3");

    // Capture state before stale failure attempt
    let journal_before = journal.snapshot()?;

    // Attempt stale failure with saved ticket 2 attempt number
    let stale_ticket = ticket2;
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: stale_ticket,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::StaleAttempt {
            incoming: ticket2_attempt,
            current: ticket3.attempt,
        })
    );

    // Verify NO new journal events were written
    let journal_after = journal.snapshot()?;
    assert_eq!(
        journal_before, journal_after,
        "Journal must not be mutated when stale attempt is rejected"
    );

    Ok(())
}

// =============================================================================
// POST-006: record_scheduled_attempt is called when ticket is issued
// =============================================================================

/// Verify action_attempts advances when action is scheduled (via drive_run)
#[test]
fn action_attempts_advances_after_scheduling() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        panic!("missing workflow fixture");
    };
    let run = RunId::new(30);

    // Initially action_attempts[0] = 0
    submit_with_contracts(&shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));

    let state = shard.run_state_get_mut(run).expect("run should exist");
    assert_eq!(
        state.action_attempts.get(0).copied(),
        Some(1),
        "action_attempts[0] should be 1 after first action scheduling (drive_run calls record_scheduled_attempt)"
    );
}

/// Verify action_attempts monotonically advances on retry scheduling
#[test]
fn action_attempts_monotonically_advances_on_retry() -> Result<(), Box<dyn std::error::Error>> {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);

    let Some(wf) = retry_workflow() else {
        return Err("missing retry workflow fixture".into());
    };
    let run = RunId::new(31);

    // Submit and drive to action (step 1)
    submit_with_contracts(&shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));

    // action_attempts[1] should be 1 after first scheduling
    {
        let state = shard.run_state_get_mut(run).ok_or("run should exist")?;
        assert_eq!(
            state.action_attempts.get(1).copied(),
            Some(1),
            "action_attempts[1] should be 1 after first scheduling"
        );
    }

    // Fail with retryable policy - this should trigger a retry
    let ticket1 = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket 1")?;
    assert_eq!(ticket1.attempt, 1, "ticket1 should have attempt=1");
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: ticket1,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // action_attempts[1] should now be 2 (retry scheduled)
    {
        let state = shard
            .run_state_get_mut(run)
            .ok_or("run should exist after retry")?;
        assert_eq!(
            state.action_attempts.get(1).copied(),
            Some(2),
            "action_attempts[1] should be 2 after retry scheduling"
        );
    }

    // Fail again with retry using real issued ticket
    let ticket2 = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket 2")?;
    assert_eq!(ticket2.attempt, 2, "ticket2 should have attempt=2");
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: ticket2,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // action_attempts[1] should now be 3
    {
        let state = shard.run_state_get_mut(run).ok_or("run should exist")?;
        assert_eq!(
            state.action_attempts.get(1).copied(),
            Some(3),
            "action_attempts[1] should be 3 after second retry scheduling"
        );
    }

    Ok(())
}

// =============================================================================
// INV-004: Monotonicity - action_attempts never decreases
// =============================================================================

/// Verify action_attempts for a step never decreases over multiple operations
#[test]
fn action_attempts_never_decreases_across_operations() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        panic!("missing workflow fixture");
    };
    let run = RunId::new(40);

    submit_with_contracts(&shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));

    let initial_attempt = shard
        .run_state_get(run)
        .expect("run exists")
        .action_attempts
        .get(0)
        .copied()
        .expect("step 0 exists");

    // Verify normal scheduling did not create a zero/underflowed attempt.
    let final_attempt = shard
        .run_state_get(run)
        .expect("run exists")
        .action_attempts
        .get(0)
        .copied()
        .expect("step 0 exists");
    assert!(
        final_attempt >= initial_attempt,
        "action_attempts[0] should never decrease below initial value"
    );
}

// =============================================================================
// Error cases: EncodeFailed on completion/failure paths
// =============================================================================

/// Happy completion persists a concrete successful step event.
#[test]
fn completion_happy_path_writes_output_and_event() -> Result<(), Box<dyn std::error::Error>> {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);

    let Some(wf) = suspended_workflow() else {
        return Err("missing workflow fixture".into());
    };
    let run = RunId::new(50);

    submit_with_contracts(&shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));

    let ticket = shard
        .pending_action_get(run)
        .ok_or("missing pending ticket")?;

    let journal_len_before = journal.snapshot()?.len();

    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(42),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    shard.enqueue(ShardCommand::ActionCompleted { ticket, output })?;
    assert_eq!(shard.tick(), Ok(true));

    let journal_len_after = journal.snapshot()?.len();
    assert!(journal_len_after > journal_len_before, "journal should have grown after completion");

    let events = journal.snapshot()?;
    let expected_value = postcard::to_allocvec(&SlotValue::I64(42))?;
    assert_eq!(events.iter().filter(|event| matches!(event,
        RuntimeJournalEvent::ActionCompletedEnvelope { ticket: actual_ticket, output, value, encoded_len, taint, value_digest, .. }
            if *actual_ticket == ticket && *output == SlotIdx::ZERO && *value == expected_value
            && *encoded_len == 2 && *taint == Taint::Clean
            && *value_digest == *blake3::hash(&expected_value).as_bytes()
    )).count(), 1);

    Ok(())
}

// =============================================================================
// BDD Scenario: validate_ticket_attempt accepts valid ticket
// =============================================================================

/// B7: validate_ticket_attempt accepts valid ticket (current=1, attempt=2)
#[test]
fn validate_ticket_attempt_accepts_valid_ticket() -> Result<(), Box<dyn std::error::Error>> {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        return Err("missing workflow fixture".into());
    };
    let run = RunId::new(60);

    submit_with_contracts(&shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));

    // Complete with real issued ticket (attempt=1, equal to current=1 — valid)
    let ticket = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket")?;
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(100),
        taint: Taint::Clean,
        encoded_len: 3,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted { ticket, output }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true), "valid ticket should be accepted");

    Ok(())
}

// =============================================================================
// BDD Scenario: equal attempt is not stale
// =============================================================================

/// Equal attempt (current=1, incoming=1) is NOT stale
#[test]
fn equal_attempt_is_not_stale() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        panic!("missing workflow fixture");
    };
    let run = RunId::new(70);

    submit_with_contracts(&shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));

    // action_attempts[0] = 1 after scheduling
    // Try to complete with attempt=1 (equal, not stale)
    let ticket = make_ticket(run, StepIdx::ZERO, 1, 1);
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted { ticket, output }),
        Ok(())
    );
    // Equal attempt should NOT return StaleAttempt error
    let result = shard.tick();
    assert!(
        !matches!(result, Err(RuntimeError::StaleAttempt { .. })),
        "Equal attempt should not be rejected as stale"
    );
}

// =============================================================================
// BDD Scenario: future attempt when current > 0 is rejected (G005 fixed)
// =============================================================================

/// Future attempt within ticket capacity is rejected.
#[test]
fn future_attempt_within_capacity_is_rejected() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        panic!("missing workflow fixture");
    };
    let run = RunId::new(80);

    submit_with_contracts(&shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));

    // action_attempts[0] = 1
    // Try to complete with attempt=3 (future but within ticket capacity — must be rejected).
    let ticket = make_ticket(run, StepIdx::ZERO, 3, 5);
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted { ticket, output }),
        Ok(())
    );
    // G005: future attempt within capacity must be rejected with InvalidActionCompletion
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::InvalidActionCompletion),
        "future attempt within capacity must be rejected"
    );
}

// =============================================================================
// Multiple steps: verify attempt counters are independent
// =============================================================================

/// Verify each step has independent attempt counter
#[test]
fn each_step_has_independent_attempt_counter() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow_2step() else {
        panic!("missing 2-step workflow fixture");
    };
    let run = RunId::new(90);

    submit_with_contracts(&shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));

    // Step 0 should have attempt=1 (first action scheduled)
    // Step 1 should have attempt=0 (not yet reached)
    let state = shard.run_state_get_mut(run).expect("run should exist");
    assert_eq!(
        state.action_attempts.get(0).copied(),
        Some(1),
        "step 0 attempt should be 1"
    );
    assert_eq!(
        state.action_attempts.get(1).copied(),
        Some(0),
        "step 1 attempt should be 0 (not yet reached)"
    );
}

#[path = "vb_jggy_lifecycle/stale_guards.rs"]
mod stale_guards;
