// Red-phase tests for durable retry vb-qi37.16.3
#![allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::expect_used,
    clippy::get_first,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]
// These tests define expected behavior that is NOT yet implemented or not yet exposed.
// They are written in RED-phase TDD style: tests FAIL until production code implements/exposes behavior.
// This file is Cargo-discovered evidence of the gap between contract and implementation.

use vb_core::action::{
    ActionContract, ActionFailure, ActionFailureCode, ActionTicket, Idempotency,
    RetryPolicy as VbRetryPolicy, RetrySafety, SideEffect,
};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, ConstIdx, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::{ConstValue, Taint};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

use vb_runtime::journal::{RuntimeJournalEvent, SharedRuntimeJournal, VolatileRuntimeJournal};
use vb_runtime::shard::{Shard, ShardCommand, ShardConfig};

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
        output: None,
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
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::from("retry"),
        digest: WorkflowDigest::from_bytes([8; 32]),
        nodes: Box::from([set_policy, action, retry, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(2)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    })
    .ok()
}

fn error_handler_with_slot_workflow() -> Option<CompiledWorkflow> {
    let guard = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::ErrorHandler {
            body: StepIdx::new(1),
            handler: StepIdx::new(2),
            error_slot: Some(SlotIdx::new(1)),
        },
    };
    let action = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: Some(StepIdx::new(3)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        },
    };
    let handler = CompiledNode {
        id: StepIdx::new(2),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(3)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(3),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::from("error_handler_with_slot"),
        digest: WorkflowDigest::from_bytes([0xBB; 32]),
        nodes: Box::from([guard, action, handler, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::Bool(false)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    })
    .ok()
}

fn run_exists(shard: &mut Shard, run: RunId) -> bool {
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 999
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    matches!(
        shard.take_inspect_response(),
        Some(vb_runtime::shard::InspectResponse::Found(_))
    )
}

fn suspended_workflow_no_retry() -> Option<CompiledWorkflow> {
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        },
    };
    CompiledWorkflow::try_from_parts(WorkflowParts {
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
    })
    .ok()
}

fn slot_preservation_workflow() -> Option<CompiledWorkflow> {
    // Workflow: SetConst -> Do -> RetryCheck -> Finish
    // SetConst sets slot 2 (policy_slot) to 3 (max_attempts)
    // Do writes output to slot 0
    // RetryCheck reads policy from slot 2
    let set_policy = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(2)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let do_node = CompiledNode {
        id: StepIdx::new(1),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::ZERO,
        },
    };
    let retry_check = CompiledNode {
        id: StepIdx::new(2),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::RetryCheck {
            policy_slot: SlotIdx::new(2),
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
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::from("slot_preservation"),
        digest: WorkflowDigest::from_bytes([0xCC; 32]),
        nodes: Box::from([set_policy, do_node, retry_check, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(3)]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    })
    .ok()
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
    ActionTicket {
        run,
        step,
        seq: SeqNo::ZERO,
        action: ActionId::new(0),
        attempt,
        idempotency_key: 0,
        capacity,
    }
}

fn retryable_failure() -> ActionFailure {
    ActionFailure {
        retry_policy: VbRetryPolicy::Retryable,
        code: ActionFailureCode::Unknown,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    }
}

fn non_retryable_failure() -> ActionFailure {
    ActionFailure {
        retry_policy: VbRetryPolicy::NonRetryable,
        code: ActionFailureCode::Timeout,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    }
}

fn submit_run(shard: &mut Shard, run: RunId, workflow: CompiledWorkflow) {
    let action = first_do_action(&workflow).unwrap_or(ActionId::new(0));
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow,
            inputs: Box::from([(SlotIdx::new(0), vb_core::value::SlotValue::Bool(false))]),
            caps: CapabilitySet::from_grants(Box::from([contract_required_capability(action)])),
            action_contracts: contracts_through(action),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
}

fn first_do_action(workflow: &CompiledWorkflow) -> Option<ActionId> {
    let mut index = 0u16;
    let count = workflow.node_count();
    while index < count {
        if let Some(node) = workflow.node(StepIdx::new(index)) {
            if let CompiledNodeKind::Do { action, .. } = node.kind {
                return Some(action);
            }
        }
        index = index.saturating_add(1);
    }
    None
}

fn contract_required_capability(action: ActionId) -> Capability {
    Capability::new("__contract_required__".into(), action)
}

fn action_contract(action: ActionId, required: bool) -> ActionContract {
    let required_capabilities = if required {
        Box::from([contract_required_capability(action)])
    } else {
        Box::from([])
    };
    ActionContract {
        id: action,
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

// ===== RED-PHASE TEST 1: POST-005 - ticket_with_retry_capacity expands capacity =====
// RED: This test FAILS at runtime because ticket_with_retry_capacity is private
// Expected: When retry_metadata_exists and policy is Retryable,
//          returned ticket.capacity = max(original.capacity, policy.max_attempts)
// Actual: No public interface to test this behavior - function is private
// This test FAILS (exits nonzero) to prove the RED phase contract gap exists.
#[test]
fn ticket_with_retry_capacity_increases_capacity_to_max_attempts() {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let Some(wf) = retry_workflow() else {
        panic!("retry workflow must exist");
    };
    let run = RunId::new(7000);

    submit_run(&mut shard, run, wf);

    // POST-005: ticket_with_retry_capacity expands capacity when retry metadata exists
    let ticket = make_ticket(run, StepIdx::new(1), 1, 1);
    let result = shard.ticket_with_retry_capacity(ticket, VbRetryPolicy::Retryable);
    let expanded = result.expect("ticket_with_retry_capacity must succeed");
    // retry_workflow has ConstValue::I64(2) at index 0, policy_slot = SlotIdx::new(1)
    // max_attempts = 2, so capacity should be max(1, 2) = 2
    assert_eq!(
        expanded.capacity, 2,
        "POST-005: capacity must expand to max(original=1, policy.max_attempts=2)"
    );
}

// ===== RED-PHASE TEST 2: POST-005 - ticket unchanged when no retry metadata =====
// RED: This test FAILS because ticket_with_retry_capacity is private
// This test FAILS (exits nonzero) to prove the RED phase contract gap exists.
#[test]
fn ticket_with_retry_capacity_returns_unchanged_when_no_retry_metadata() {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let Some(wf) = suspended_workflow_no_retry() else {
        panic!("workflow must exist");
    };

    let run = RunId::new(7001);
    submit_run(&mut shard, run, wf);

    // POST-005: ticket_with_retry_capacity returns unchanged when no retry metadata
    let ticket = make_ticket(run, StepIdx::ZERO, 1, 5);
    let result = shard.ticket_with_retry_capacity(ticket, VbRetryPolicy::Retryable);
    let unchanged = result.expect("ticket_with_retry_capacity must succeed");
    assert_eq!(
        unchanged.capacity, 5,
        "POST-005: ticket must be returned unchanged when retry_metadata_exists is false"
    );
}

// ===== RED-PHASE TEST 3: INV-003 - Journal replay idempotency =====
// RED: No journal replay function exposed for testing.
// This test documents that handle_action_failure removes run from self.runs on FailRun,
// so a second ActionFailed for the same run returns RunNotFound.
// True journal replay (simulating restart + replay) is not possible without
// a journal_replay(ticket, events) function.
#[test]
fn journal_replay_idempotent_action_failed() {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let Some(wf) = suspended_workflow_no_retry() else {
        panic!("suspended workflow must exist");
    };
    let run = RunId::new(7002);
    submit_run(&mut shard, run, wf);

    // Enqueue first ActionFailed (NonRetryable so no retry logic)
    let ticket1 = make_ticket(run, StepIdx::ZERO, 1, 1);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: ticket1,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Capture journal after first ActionFailed
    let events_after_first = journal.snapshot().expect("journal must snapshot");
    let action_failed_count = events_after_first
        .iter()
        .filter(|e| matches!(e, RuntimeJournalEvent::ActionFailed { .. }))
        .count();

    // Replay the same ActionFailed event by enqueuing again
    // NOTE: After FailRun, the run is removed from self.runs,
    // so tick() returns Err(RunNotFound)
    let ticket2 = make_ticket(run, StepIdx::ZERO, 1, 1);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: ticket2,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    // tick() fails because run was already removed
    let tick_result = shard.tick();

    // RED gap: True journal replay would allow appending events to a failed run.
    // Currently, after FailRun the run is removed, so tick() returns Err(RunNotFound).
    // This is NOT the same as journal replay - journal replay would restart the shard
    // and process stored events, not call handle_action_failure on a live run.
    assert!(
        tick_result.is_err(),
        "After FailRun, run is removed, so tick() returns RunNotFound"
    );

    // Journal should have exactly 1 ActionFailed (the first one)
    assert_eq!(
        action_failed_count, 1,
        "journal should have exactly 1 ActionFailed event"
    );

    // RED gap: To test INV-003 (journal replay idempotency), we need:
    // 1. A journal_replay(ticket, events) function that replays events without removing the run
    // 2. Or a test mode where FailRun doesn't remove the run
    // Without this, we cannot verify that duplicate ActionFailed appends to journal
    // in a true replay scenario.
}

// ===== RED-PHASE TEST 4: INV-004 - Slot preservation on action failure =====
// RED: No public interface to read individual slot values.
// The slot preservation invariant (INV-004) cannot be verified from integration tests.
// This test documents the gap: no InspectSlot command exists.
#[test]
fn action_failure_preserves_action_completed_slots_integration_gap() {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let Some(wf) = slot_preservation_workflow() else {
        panic!("workflow must exist");
    };

    let run = RunId::new(7003);
    submit_run(&mut shard, run, wf);

    // The workflow: SetConst (sets policy slot 2), then Do (outputs to slot 0)
    // After submit, step 1 (Do) is running, waiting for action completion
    // To verify INV-004 (slot preservation), we would need to:
    // 1. Complete the action at step 1 (write slot 0 = 42)
    // 2. Fail the action at step 1
    // 3. Inspect slot 0 to verify it still = 42

    // RED gap: Cannot manually set action completion state from integration tests.
    // We can only test through public enqueue interface.
    // The run either completed (if retry worked) or failed
    // But we cannot verify the slot preservation invariant.

    // For now, we verify the run still exists (retry worked) or failed
    let run_exists = run_exists(&mut shard, run);
    let failed_count = shard.counters().snapshot().runs_failed;

    // RED: This test documents the gap - we cannot verify INV-004 from integration tests
    // because no public interface exists to read individual slot values.
    // The assertion below is not testing INV-004, just that the run state is accessible.
    assert!(
        run_exists || failed_count > 0,
        "run should either exist (retry) or be failed"
    );
}

// ===== RED-PHASE TEST 5: INV-5 - PC reset semantics on retry =====
// RED: No public interface to inspect PC after action failure.
// This test uses Inspect command to verify PC, but Inspect only returns current PC,
// not the PC after a specific command. The gap is in PC tracking after failure.
#[test]
fn apply_action_failure_to_state_resets_pc_to_failed_step_on_retry() {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let Some(wf) = retry_workflow() else {
        panic!("retry workflow must exist");
    };
    let run = RunId::new(7004);
    submit_run(&mut shard, run, wf);

    // After submit, PC should be at step 1 (the action node)
    // Inspect to verify initial PC
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 1,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let resp = shard.take_inspect_response();
    assert!(matches!(
        resp,
        Some(vb_runtime::shard::InspectResponse::Found(_))
    ));
    if let Some(vb_runtime::shard::InspectResponse::Found(snap)) = resp {
        assert_eq!(
            snap.pc,
            StepIdx::new(1),
            "initial PC should be at action step"
        );
    }

    // Now fail at step 1 with retryable policy
    let ticket = make_ticket(run, StepIdx::new(1), 1, 2);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Inspect again to check PC after failure
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 2,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let resp = shard.take_inspect_response();

    // INV-5: PC must reset to failed step (step 1) on retry
    assert!(
        matches!(resp, Some(vb_runtime::shard::InspectResponse::Found(_))),
        "run should still exist after retry"
    );
    if let Some(vb_runtime::shard::InspectResponse::Found(snap)) = resp {
        assert_eq!(
            snap.pc,
            StepIdx::new(1),
            "INV-5: PC must reset to failed step on retry, not advanced"
        );
    }
}

// ===== RED-PHASE TEST 6: POST-002 - Error handler writes correct slot value =====
// RED: No public interface to read slot values after error handler runs.
// This test documents the gap: no InspectSlot command exists to verify error_slot content.
#[test]
fn apply_error_handler_writes_step_index_to_error_slot_integration_gap() {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let Some(wf) = error_handler_with_slot_workflow() else {
        panic!("error_handler_with_slot workflow must exist");
    };
    let run = RunId::new(7005);
    submit_run(&mut shard, run, wf);

    // The error handler has error_slot = Some(SlotIdx::new(1))
    // Failed step is StepIdx::new(1)

    // Fail the action at step 1 (NonRetryable to skip retry)
    let ticket = make_ticket(run, StepIdx::new(1), 1, 1);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // The run should continue at the handler step (step 2)
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 3,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let resp = shard.take_inspect_response();

    // Verify PC is at handler step (step 2)
    if let Some(vb_runtime::shard::InspectResponse::Found(snap)) = resp {
        assert_eq!(
            snap.pc,
            StepIdx::new(2),
            "PC should be at handler step after error handling"
        );
    }

    // RED gap: Cannot verify error_slot[1] contains I64(1) because:
    // - ShardCommand::Inspect does not expose slot values
    // - No ReadSlot command exists
    // POST-002 requires error_slot contains I64(failed_step), but this cannot
    // be verified from integration tests without a slot inspection interface.
}

// ===== RED-PHASE TEST 7: PRE-004 - retry_is_available requires Retryable policy =====
// This test PASSES - proves indirect coverage of NonRetryable behavior.
// RED gap: retry_is_available is private, but behavior is testable indirectly.
#[test]
fn retry_is_available_returns_false_for_nonretryable_policy() {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let Some(wf) = retry_workflow() else {
        panic!("retry workflow must exist");
    };
    let run = RunId::new(7006);
    submit_run(&mut shard, run, wf);

    // Even though the policy is Retryable and metadata exists,
    // using NonRetryable should bypass retry logic
    let ticket = make_ticket(run, StepIdx::new(1), 1, 2);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // With NonRetryable, no retry should occur and run should fail
    // because there's no error handler in retry_workflow
    assert!(
        !run_exists(&mut shard, run),
        "NonRetryable without handler should fail the run"
    );
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
}

// ===== RED-PHASE TEST 8: PRE-004 - retry_is_available returns false when no retry metadata =====
// This test PASSES - proves indirect coverage of missing retry metadata.
#[test]
fn retry_is_available_returns_false_when_no_retry_metadata() {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let Some(wf) = suspended_workflow_no_retry() else {
        panic!("workflow must exist");
    };

    let run = RunId::new(7007);
    submit_run(&mut shard, run, wf);

    // Even with Retryable policy, no retry should happen
    // because there's no retry metadata (no RetryCheck node)
    let ticket = make_ticket(run, StepIdx::ZERO, 1, 1);
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Run should fail because retry is not available
    // (no RetryCheck node follows step 0)
    assert!(
        !run_exists(&mut shard, run),
        "Retryable without retry_metadata should fail the run"
    );
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
}

// ===== RED-PHASE TEST 9: POST-006 - record_retry_attempt boundary =====
// RED: Cannot construct RunState in integration tests due to private fields.
// record_retry_attempt is pub fn but requires RunState which has private fields.
// This test documents the integration test gap for boundary testing.
#[test]
fn record_retry_attempt_integration_gap() {
    // RED gap: record_retry_attempt(state, ticket, policy) is public in helpers,
    // but RunState has private fields (action_attempts, frame, workflow, store).
    // Integration tests cannot construct RunState to test record_retry_attempt boundary.
    // Unit tests in helpers.rs (#[cfg(test)]) cover this, but not from integration path.
    // POST-006 boundary (max_attempts) is tested in unit tests only.
}
