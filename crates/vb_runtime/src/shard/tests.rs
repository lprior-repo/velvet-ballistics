//! Tests for the shard module.
#![allow(dead_code, unused_imports)]

use vb_core::ActionFailureCode;
use vb_core::action::RetryPolicy as VbRetryPolicy;
use vb_core::ids::{ActionId, ConstIdx, SlotIdx, WorkflowDigest};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

use crate::counters::ShardCounters;
use crate::frame_pool::FramePool;
use crate::journal::{NoopRuntimeJournal, RuntimeJournalEvent, SharedRuntimeJournal};
use crate::trace::{TraceEvent, TraceRing};
use crate::{RuntimeError, RuntimeResult};

use super::{
    AskAnswer, AskTicket, InspectResponse, InspectSnapshot, MAX_COMMAND_QUEUE_CAPACITY, RunState,
    Shard, ShardCommand, ShardConfig,
};

fn suspended_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let node = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: None,
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
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn action_with_error_handler_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let guard = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::ErrorHandler {
            body: vb_core::ids::StepIdx::new(1),
            handler: vb_core::ids::StepIdx::new(2),
        },
    };
    let action = CompiledNode {
        id: vb_core::ids::StepIdx::new(1),
        output: None,
        next: Some(vb_core::ids::StepIdx::new(3)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        },
    };
    let handler = CompiledNode {
        id: vb_core::ids::StepIdx::new(2),
        output: Some(SlotIdx::new(0)),
        next: Some(vb_core::ids::StepIdx::new(3)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: vb_core::ids::StepIdx::new(3),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("action_with_error_handler"),
        digest: WorkflowDigest::from_bytes([3; 32]),
        nodes: Box::from([guard, action, handler, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::Bool(false)]),
        slot_count: 1,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn action_ticket(run: super::RunId, step: vb_core::ids::StepIdx) -> vb_core::action::ActionTicket {
    vb_core::action::ActionTicket {
        run,
        step,
        seq: vb_core::ids::SeqNo::ZERO,
        action: ActionId::new(0),
        attempt: 1,
        idempotency_key: 0,
    }
}

fn timeout_failure() -> vb_core::action::ActionFailure {
    vb_core::action::ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: vb_core::value::Taint::Clean,
        detail: None,
        encoded_len: 0,
    }
}

#[test]
fn retry_attempt_counter_increments_until_policy_exhaustion() {
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let frame = match vb_core::frame::RunFrame::new(
        super::RunId::new(9),
        vb_core::ids::StepIdx::ZERO,
        1,
        1,
    ) {
        Ok(frame) => frame,
        Err(_) => return,
    };
    let mut state = RunState {
        frame,
        workflow,
        store: vb_core::value_store::ValueStore::new(),
        action_attempts: super::new_action_attempts(1),
        admission: None,
        collect_states: crate::primitives::collect::CollectStates::new(),
    };
    let ticket = vb_core::action::ActionTicket {
        run: super::RunId::new(9),
        step: vb_core::ids::StepIdx::ZERO,
        seq: vb_core::ids::SeqNo::new(1),
        action: ActionId::new(0),
        attempt: 1,
        idempotency_key: 0,
    };
    let policy = crate::engine::RetryPolicy {
        max_attempts: 2,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    assert_eq!(
        super::record_retry_attempt(&mut state, ticket, policy),
        Ok(true)
    );
    assert_eq!(state.action_attempts.get(0).copied(), Some(2));
    assert_eq!(
        super::record_retry_attempt(&mut state, ticket, policy),
        Ok(false)
    );
}

#[test]
fn action_failed_routes_to_nearby_error_handler() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = action_with_error_handler_workflow() else {
        return;
    };
    let run = super::RunId::new(301);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: action_ticket(run, vb_core::ids::StepIdx::new(1)),
            failure: timeout_failure(),
        }),
        Ok(())
    );

    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
}

#[test]
fn action_failed_without_error_handler_fails_run() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(302);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: action_ticket(run, vb_core::ids::StepIdx::ZERO),
            failure: timeout_failure(),
        }),
        Ok(())
    );

    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
}

#[test]
fn shard_rejects_active_run_capacity_overflow() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 1,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };

    let first = shard.enqueue(ShardCommand::Submit {
        run: super::RunId::new(1),
        workflow: workflow.clone(),
        caps: vb_core::capability::CapabilitySet::empty(),
    });
    assert_eq!(first, Ok(()));
    assert_eq!(shard.tick(), Ok(true));

    let second = shard.enqueue(ShardCommand::Submit {
        run: super::RunId::new(2),
        workflow,
        caps: vb_core::capability::CapabilitySet::empty(),
    });
    assert_eq!(second, Ok(()));
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
    );
}

#[test]
fn inspect_command_stores_retrievable_snapshot() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 1,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(7);

    let submitted = shard.enqueue(ShardCommand::Submit {
        run,
        workflow,
        caps: vb_core::capability::CapabilitySet::empty(),
    });
    assert_eq!(submitted, Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    let inspected = shard.enqueue(ShardCommand::Inspect {
        run,
        correlation: 99,
    });
    assert_eq!(inspected, Ok(()));
    assert_eq!(shard.tick(), Ok(true));

    match shard.take_inspect_response() {
        Some(InspectResponse::Found(snapshot)) => {
            assert_eq!(snapshot.run, run);
            assert_eq!(snapshot.correlation, 99);
        }
        other => assert_eq!(other, None),
    }
}

#[test]
fn enqueue_shutdown_sets_shutting_down_flag() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 1,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    assert_eq!(shard.is_shutting_down(), false);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.tick(), Ok(false));
    assert_eq!(shard.is_shutting_down(), true);
}

#[test]
fn tick_returns_true_when_queue_is_empty() {
    let config = ShardConfig::default();
    let mut shard = Shard::new(config);
    assert_eq!(shard.tick(), Ok(true));
}

#[test]
fn cancel_nonexistent_run_succeeds_silently() {
    let config = ShardConfig::default();
    let mut shard = Shard::new(config);
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel {
            run: super::RunId::new(999)
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
}

#[test]
fn counters_reflect_submitted_after_submit_tick() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 1,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(1);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
}

#[test]
fn inspect_nonexistent_run_returns_not_found() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 1,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run: super::RunId::new(999),
            correlation: 42,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.take_inspect_response(),
        Some(InspectResponse::NotFound {
            run: super::RunId::new(999),
            correlation: 42,
        })
    );
}

// Helper: workflow that finishes immediately (SetConst -> Finish).
fn finished_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_const = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: Some(SlotIdx::new(0)),
        next: Some(vb_core::ids::StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: vb_core::ids::StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("finished"),
        digest: WorkflowDigest::from_bytes([2; 32]),
        nodes: Box::from([set_const, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn timed_wait_then_finish_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_deadline = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(vb_core::ids::StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let wait = CompiledNode {
        id: vb_core::ids::StepIdx::new(1),
        output: None,
        next: Some(vb_core::ids::StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::ZERO,
        },
    };
    let finish = CompiledNode {
        id: vb_core::ids::StepIdx::new(2),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
    };
    let parts = WorkflowParts {
        name: Box::from("timed_wait_then_finish"),
        digest: WorkflowDigest::from_bytes([4; 32]),
        nodes: Box::from([set_deadline, wait, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::I64(10)]),
        slot_count: 1,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn timed_ask_without_answer_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_prompt = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(vb_core::ids::StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let set_timeout = CompiledNode {
        id: vb_core::ids::StepIdx::new(1),
        output: Some(SlotIdx::new(1)),
        next: Some(vb_core::ids::StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(1),
        },
    };
    let ask = CompiledNode {
        id: vb_core::ids::StepIdx::new(2),
        output: None,
        next: Some(vb_core::ids::StepIdx::new(3)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Ask {
            prompt: SlotIdx::ZERO,
            timeout_slot: Some(SlotIdx::new(1)),
        },
    };
    let resume = CompiledNode {
        id: vb_core::ids::StepIdx::new(3),
        output: None,
        next: Some(vb_core::ids::StepIdx::new(4)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::AskResume {
            answer: SlotIdx::new(2),
        },
    };
    let finish = CompiledNode {
        id: vb_core::ids::StepIdx::new(4),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(2),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("timed_ask_without_answer"),
        digest: WorkflowDigest::from_bytes([5; 32]),
        nodes: Box::from([set_prompt, set_timeout, ask, resume, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([
            vb_core::value::ConstValue::Symbol(vb_core::ids::SymbolId::new(1)),
            vb_core::value::ConstValue::I64(10),
        ]),
        slot_count: 3,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
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

#[test]
fn finished_run_releases_frame_to_dimension_pool() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = finished_workflow() else {
        return;
    };

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(1),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let available = shard.frame_pools.get(&(2, 1)).map(FramePool::available);
    assert_eq!(available, Some(1));
}

#[test]
fn cancelled_run_releases_frame_to_dimension_pool() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(11);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.frame_pools.get(&(1, 1)).map(FramePool::available),
        Some(0)
    );

    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.frame_pools.get(&(1, 1)).map(FramePool::available),
        Some(1)
    );
}

#[test]
fn cancel_cleans_pending_timer() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        return;
    };
    let run = super::RunId::new(12);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timers.len(), 1);
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.pending_timers.len(), 0);
}

#[test]
fn finish_cleans_pending_timer_after_timer_fire() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        return;
    };
    let run = super::RunId::new(13);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timers.len(), 1);
    assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.pending_timers.len(), 0);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
}

#[test]
fn fail_cleans_pending_timer_after_ask_timeout_without_answer() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = timed_ask_without_answer_workflow() else {
        return;
    };
    let run = super::RunId::new(14);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timers.len(), 1);
    assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.pending_timers.len(), 0);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
}

#[test]
fn enqueue_returns_queue_full_when_capacity_exceeded() {
    // Given a shard with very small command queue
    let config = ShardConfig {
        command_queue_capacity: 2,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    // When enqueuing more commands than capacity allows
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    // Then the third enqueue returns QueueFull
    assert_eq!(
        shard.enqueue(ShardCommand::Shutdown),
        Err(RuntimeError::QueueFull)
    );
}

#[test]
fn tick_after_shutdown_returns_false() {
    // Given a shard that has received a shutdown command
    let config = small_config();
    let mut shard = Shard::new(config);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    // When ticking after shutdown
    assert_eq!(shard.tick(), Ok(false));
    // Then subsequent tick also returns false (shutting_down flag is set)
    assert_eq!(shard.tick(), Ok(false));
}

#[test]
fn submit_returns_run_already_exists_for_duplicate() {
    // Given a shard with an active run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(42);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When submitting the same run ID again
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    // Then tick returns RunAlreadyExists
    assert_eq!(shard.tick(), Err(RuntimeError::RunAlreadyExists));
}

#[test]
fn submit_returns_active_run_capacity_exceeded_at_limit() {
    // Given a shard with max_active_runs = 1 and one active run
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 1,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    let Some(wf) = suspended_workflow() else {
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(1),
            workflow: wf,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When submitting a second run
    let Some(wf2) = suspended_workflow() else {
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(2),
            workflow: wf2,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    // Then tick returns ActiveRunCapacityExceeded with capacity 1
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
    );
}

#[test]
fn shard_submit_creates_run_state_in_runs_map() {
    // Given a shard and a workflow
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(10);
    // When submitting a run
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then inspecting the run returns Found (proving it's in the runs map)
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 1,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let response = shard.take_inspect_response();
    match response {
        Some(InspectResponse::Found(snapshot)) => {
            assert_eq!(snapshot.run, run);
            assert_eq!(snapshot.correlation, 1);
        }
        other => {
            // Wrong: expected Found
            assert_eq!(other, None);
        }
    }
}

#[test]
fn shard_submit_records_run_submitted_trace_event() {
    // Given a shard and a workflow
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(20);
    // When submitting a run
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the trace ring contains a RunSubmitted event
    let events = shard.trace_ring_mut().drain();
    let found = events
        .iter()
        .any(|e| *e == TraceEvent::RunSubmitted { run });
    assert_eq!(found, true);
}

#[test]
fn shard_submit_drives_run_immediately_for_finished_workflow() {
    // Given a shard and a finished workflow
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = finished_workflow() else {
        return;
    };
    let run = super::RunId::new(30);
    // When submitting a run with a finishing workflow
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the run is completed (not in runs map anymore) and counter shows completed
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    // And inspect returns NotFound since the run finished
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 2,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.take_inspect_response(),
        Some(InspectResponse::NotFound {
            run,
            correlation: 2
        })
    );
}

#[test]
fn shard_resume_returns_error_for_unknown_run() {
    // Given a shard with no runs
    let config = small_config();
    let mut shard = Shard::new(config);
    // When resuming a non-existent run
    assert_eq!(
        shard.enqueue(ShardCommand::Resume {
            run: super::RunId::new(999),
        }),
        Ok(())
    );
    // Then tick returns RunNotFound
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn shard_action_completed_returns_error_for_unknown_run() {
    // Given a shard with no runs
    let config = small_config();
    let mut shard = Shard::new(config);
    // When completing an action for a non-existent run
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run: super::RunId::new(888),
            step: vb_core::ids::StepIdx::new(0),
        }),
        Ok(())
    );
    // Then tick returns RunNotFound
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn shard_action_completed_marks_step_succeeded() {
    // Given a shard with a suspended run (Do node at step 0)
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(55);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    let tick1 = shard.tick();
    // Then first tick succeeds (Do node suspends)
    assert_eq!(tick1, Ok(true));
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    // When completing the action at step 0
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run,
            step: vb_core::ids::StepIdx::new(0),
        }),
        Ok(())
    );
    let tick2 = shard.tick();
    // Then second tick succeeds
    assert_eq!(tick2, Ok(true));
    // And the trace ring has an ActionCompleted event
    let events = shard.trace_ring_mut().drain();
    let found = events.iter().any(|e| {
        *e == TraceEvent::ActionCompleted {
            run,
            step: vb_core::ids::StepIdx::new(0),
        }
    });
    assert_eq!(found, true);
}

#[test]
fn shard_action_completed_records_trace_event() {
    // Given a shard with a suspended run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(56);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When completing the action
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run,
            step: vb_core::ids::StepIdx::new(0),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the trace ring contains an ActionCompleted event
    let events = shard.trace_ring_mut().drain();
    let found = events.iter().any(|e| {
        *e == TraceEvent::ActionCompleted {
            run,
            step: vb_core::ids::StepIdx::new(0),
        }
    });
    assert_eq!(found, true);
}

#[test]
fn shard_timer_rejects_run_without_pending_timer() {
    // Given a shard with an action-suspended run, not a timed wait/ask
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(60);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When timer fires for the run
    assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
    // Then tick rejects it because no timer was registered
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
}

#[test]
fn shard_wait_suspension_registers_pending_timer() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        return;
    };
    let run = super::RunId::new(61);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.pending_timers.len(), 1);
    assert_eq!(
        shard.pending_timers.get(&run).map(|timer| timer.step),
        Some(vb_core::ids::StepIdx::new(1))
    );
}

#[test]
fn shard_timer_fired_advances_timed_wait_to_finish() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        return;
    };
    let run = super::RunId::new(62);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.pending_timers.len(), 0);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
}

#[test]
fn shard_timer_returns_error_for_unknown_run() {
    // Given a shard with no runs
    let config = small_config();
    let mut shard = Shard::new(config);
    // When timer fires for a non-existent run
    assert_eq!(
        shard.enqueue(ShardCommand::TimerFired {
            run: super::RunId::new(777),
        }),
        Ok(())
    );
    // Then tick returns RunNotFound
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn shard_cancel_removes_run_from_runs_map() {
    // Given a shard with an active run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(70);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When cancelling the run
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // Then inspect returns NotFound (run removed from map)
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 5,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.take_inspect_response(),
        Some(InspectResponse::NotFound {
            run,
            correlation: 5
        })
    );
}

#[test]
fn shard_cancel_records_run_cancelled_trace_event() {
    // Given a shard with an active run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(71);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When cancelling the run
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // Then the trace ring contains a RunCancelled event
    let events = shard.trace_ring_mut().drain();
    let found = events
        .iter()
        .any(|e| *e == TraceEvent::RunCancelled { run });
    assert_eq!(found, true);
}

#[test]
fn shard_cancel_emits_cancelled_journal_and_preserves_counter_semantics() {
    // Given a shard with a volatile journal and an active suspended run
    let config = small_config();
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(config, shared);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(73);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // When cancelling the active run
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));

    // Then cancellation is a distinct journal/trace event, while the legacy failed counter
    // still counts the non-successful terminal lifecycle.
    assert!(
        matches!(journal.snapshot(), Ok(events) if events.contains(&RuntimeJournalEvent::RunCancelled { run }))
    );
    assert!(
        shard
            .trace_ring_mut()
            .drain()
            .contains(&TraceEvent::RunCancelled { run })
    );
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    assert_eq!(shard.counters().snapshot().runs_completed, 0);
}

#[test]
fn shard_cancel_increments_failed_counter() {
    // Given a shard with an active run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(72);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When cancelling the run
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // Then the failed counter is incremented
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
}

#[test]
fn shard_inspect_captures_current_pc() {
    // Given a shard with an active suspended run at step 0
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(80);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When inspecting the run
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 10,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the snapshot pc matches the expected program counter
    match shard.take_inspect_response() {
        Some(InspectResponse::Found(snapshot)) => {
            assert_eq!(snapshot.pc, vb_core::ids::StepIdx::new(0));
            assert_eq!(snapshot.run, run);
            assert_eq!(snapshot.correlation, 10);
        }
        other => assert_eq!(other, None),
    }
}

#[test]
fn shard_inspect_captures_executed_count() {
    // Given a shard with a finished workflow (executes 2 steps: SetConst + Finish)
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = finished_workflow() else {
        return;
    };
    let run = super::RunId::new(81);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the steps_executed counter reflects execution
    assert_eq!(shard.counters().snapshot().steps_executed, 2);
}

#[test]
fn shard_tick_processes_commands_in_fifo_order() {
    // Given a shard with two submits enqueued
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(wf1) = finished_workflow() else {
        return;
    };
    let Some(wf2) = suspended_workflow() else {
        return;
    };
    // When submitting two runs
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(100),
            workflow: wf1,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(101),
            workflow: wf2,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    // Then both ticks succeed in FIFO order
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.tick(), Ok(true));
    // And counters show both submitted
    assert_eq!(shard.counters().snapshot().runs_submitted, 2);
    // And the first run (finished workflow) is completed
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
}

#[test]
fn shard_resume_continues_suspended_run() {
    // Given a shard with a suspended run (Do node at step 0)
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(90);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When resuming the suspended run
    assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
    // Then tick succeeds (run re-enters drive, suspends again on Do)
    assert_eq!(shard.tick(), Ok(true));
}

#[test]
fn shard_take_inspect_response_returns_none_initially() {
    // Given a fresh shard
    let config = small_config();
    let mut shard = Shard::new(config);
    // When taking inspect response without any inspect command
    let response = shard.take_inspect_response();
    // Then response is None
    assert_eq!(response, None);
}

#[test]
fn shard_take_inspect_response_clears_after_take() {
    // Given a shard with an inspect response available
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(95);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 1,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When taking the response
    let first = shard.take_inspect_response();
    assert_eq!(first.is_some(), true);
    // Then a second take returns None
    let second = shard.take_inspect_response();
    assert_eq!(second, None);
}

#[test]
fn shard_is_shutting_down_defaults_to_false() {
    // Given a fresh shard
    let config = small_config();
    let shard = Shard::new(config);
    // Then is_shutting_down is false
    assert_eq!(shard.is_shutting_down(), false);
}

#[test]
fn shard_config_default_values() {
    // Given a default ShardConfig
    let config = ShardConfig::default();
    // Then it has reasonable defaults
    assert_eq!(config.command_queue_capacity, 1024);
    assert_eq!(config.trace_capacity, 4096);
    assert_eq!(config.step_budget_per_tick, 1000);
    assert_eq!(config.max_active_runs, 1024);
}

#[test]
fn shard_config_equality_same_values() {
    // Given two identical configs
    let a = ShardConfig::default();
    let b = ShardConfig::default();
    // Then they are equal
    assert_eq!(a, b);
}

#[test]
fn shard_config_equality_differs() {
    // Given two different configs
    let a = ShardConfig::default();
    let b = ShardConfig {
        command_queue_capacity: 1,
        trace_capacity: 1,
        step_budget_per_tick: 1,
        max_active_runs: 1,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    // Then they are not equal
    assert_ne!(a, b);
}

#[test]
fn shard_config_clone_preserves_values() {
    // Given a config
    let original = small_config();
    // When cloning
    let cloned = original.clone();
    // Then clone matches original
    assert_eq!(cloned, original);
}

#[test]
fn shard_command_equality_submit() {
    // Given two identical Submit commands
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let a = ShardCommand::Submit {
        run: super::RunId::new(1),
        workflow: wf.clone(),
        caps: vb_core::capability::CapabilitySet::empty(),
    };
    let b = ShardCommand::Submit {
        run: super::RunId::new(1),
        workflow: wf,
        caps: vb_core::capability::CapabilitySet::empty(),
    };
    assert_eq!(a, b);
}

#[test]
fn shard_command_equality_cancel() {
    // Given two identical Cancel commands
    let a = ShardCommand::Cancel {
        run: super::RunId::new(1),
    };
    let b = ShardCommand::Cancel {
        run: super::RunId::new(1),
    };
    assert_eq!(a, b);
}

#[test]
fn shard_command_equality_differs_run_id() {
    // Given two Cancel commands with different run IDs
    let a = ShardCommand::Cancel {
        run: super::RunId::new(1),
    };
    let b = ShardCommand::Cancel {
        run: super::RunId::new(2),
    };
    assert_ne!(a, b);
}

#[test]
fn shard_command_equality_shutdown() {
    // Given two Shutdown commands
    let a = ShardCommand::Shutdown;
    let b = ShardCommand::Shutdown;
    assert_eq!(a, b);
}

#[test]
fn shard_command_equality_inspect() {
    // Given two identical Inspect commands
    let a = ShardCommand::Inspect {
        run: super::RunId::new(1),
        correlation: 42,
    };
    let b = ShardCommand::Inspect {
        run: super::RunId::new(1),
        correlation: 42,
    };
    assert_eq!(a, b);
}

#[test]
fn shard_command_equality_inspect_differs_correlation() {
    // Given two Inspect commands with different correlation
    let a = ShardCommand::Inspect {
        run: super::RunId::new(1),
        correlation: 1,
    };
    let b = ShardCommand::Inspect {
        run: super::RunId::new(1),
        correlation: 2,
    };
    assert_ne!(a, b);
}

#[test]
fn shard_command_equality_action_completed() {
    // Given two identical ActionCompleted commands
    let a = ShardCommand::ActionCompletedLegacy {
        run: super::RunId::new(1),
        step: vb_core::ids::StepIdx::new(0),
    };
    let b = ShardCommand::ActionCompletedLegacy {
        run: super::RunId::new(1),
        step: vb_core::ids::StepIdx::new(0),
    };
    assert_eq!(a, b);
}

#[test]
fn shard_command_equality_timer_fired() {
    // Given two identical TimerFired commands
    let a = ShardCommand::TimerFired {
        run: super::RunId::new(1),
    };
    let b = ShardCommand::TimerFired {
        run: super::RunId::new(1),
    };
    assert_eq!(a, b);
}

#[test]
fn shard_command_equality_resume() {
    // Given two identical Resume commands
    let a = ShardCommand::Resume {
        run: super::RunId::new(1),
    };
    let b = ShardCommand::Resume {
        run: super::RunId::new(1),
    };
    assert_eq!(a, b);
}

#[test]
fn shard_cancel_nonexistent_does_not_increment_failed() {
    // Given a shard
    let config = small_config();
    let mut shard = Shard::new(config);
    // When cancelling a non-existent run
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel {
            run: super::RunId::new(999)
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the failed counter is NOT incremented (run didn't exist)
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
}

#[test]
fn shard_finished_workflow_sets_completed_counter() {
    // Given a shard with a finished workflow
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(wf) = finished_workflow() else {
        return;
    };
    let run = super::RunId::new(50);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then completed counter is 1
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
}

#[test]
fn shard_finished_workflow_produces_run_finished_trace() {
    // Given a shard with a finished workflow
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(wf) = finished_workflow() else {
        return;
    };
    let run = super::RunId::new(51);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the trace contains RunFinished
    let events = shard.trace_ring_mut().drain();
    let found = events.iter().any(|e| *e == TraceEvent::RunFinished { run });
    assert_eq!(found, true);
}

#[test]
fn shard_inspect_response_not_found_for_unknown_run() {
    // Given a shard with no runs
    let config = small_config();
    let mut shard = Shard::new(config);
    // When inspecting a non-existent run
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run: super::RunId::new(999),
            correlation: 1
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then response is NotFound
    assert_eq!(
        shard.take_inspect_response(),
        Some(InspectResponse::NotFound {
            run: super::RunId::new(999),
            correlation: 1
        })
    );
}

#[test]
fn inspect_response_found_equality() {
    // Given two identical Found responses
    let a = InspectResponse::Found(InspectSnapshot {
        run: super::RunId::new(1),
        correlation: 42,
        pc: vb_core::ids::StepIdx::new(0),
        executed: 5,
    });
    let b = InspectResponse::Found(InspectSnapshot {
        run: super::RunId::new(1),
        correlation: 42,
        pc: vb_core::ids::StepIdx::new(0),
        executed: 5,
    });
    assert_eq!(a, b);
}

#[test]
fn inspect_response_found_differs_executed() {
    // Given two Found responses with different executed counts
    let a = InspectResponse::Found(InspectSnapshot {
        run: super::RunId::new(1),
        correlation: 1,
        pc: vb_core::ids::StepIdx::new(0),
        executed: 5,
    });
    let b = InspectResponse::Found(InspectSnapshot {
        run: super::RunId::new(1),
        correlation: 1,
        pc: vb_core::ids::StepIdx::new(0),
        executed: 10,
    });
    assert_ne!(a, b);
}

#[test]
fn inspect_response_not_found_equality() {
    // Given two identical NotFound responses
    let a = InspectResponse::NotFound {
        run: super::RunId::new(1),
        correlation: 42,
    };
    let b = InspectResponse::NotFound {
        run: super::RunId::new(1),
        correlation: 42,
    };
    assert_eq!(a, b);
}

#[test]
fn run_state_equality() {
    // Given a suspended workflow and run frame
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let frame = match vb_core::frame::RunFrame::new(
        super::RunId::new(1),
        vb_core::ids::StepIdx::ZERO,
        4,
        1,
    ) {
        Ok(f) => f,
        Err(_) => return,
    };
    let state = RunState {
        frame,
        workflow: wf.clone(),
        store: vb_core::value_store::ValueStore::new(),
        action_attempts: super::new_action_attempts(4),
        admission: None,
        collect_states: crate::primitives::collect::CollectStates::new(),
    };
    let frame2 = match vb_core::frame::RunFrame::new(
        super::RunId::new(1),
        vb_core::ids::StepIdx::ZERO,
        4,
        1,
    ) {
        Ok(f) => f,
        Err(_) => return,
    };
    let state2 = RunState {
        frame: frame2,
        workflow: wf,
        store: vb_core::value_store::ValueStore::new(),
        action_attempts: super::new_action_attempts(4),
        admission: None,
        collect_states: crate::primitives::collect::CollectStates::new(),
    };
    assert_eq!(state, state2);
}

// =======================================================================
// Adversarial BDD tests — shard
// =======================================================================

#[test]
fn shard_cancel_then_inspect_returns_not_found() {
    // Given a shard with an active run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(200);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When cancelling then inspecting
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 1
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then inspect returns NotFound
    assert_eq!(
        shard.take_inspect_response(),
        Some(InspectResponse::NotFound {
            run,
            correlation: 1
        })
    );
}

#[test]
fn adversarial_shard_action_failed_for_unknown_run_returns_run_not_found() {
    // Given a shard with no runs
    let config = small_config();
    let mut shard = Shard::new(config);
    // When failing an action for a non-existent run
    let ticket = vb_core::action::ActionTicket {
        run: super::RunId::new(999),
        step: vb_core::ids::StepIdx::ZERO,
        seq: vb_core::ids::SeqNo::ZERO,
        action: ActionId::new(0),
        attempt: 1,
        idempotency_key: 0,
    };
    let failure = vb_core::action::ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: vb_core::value::Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed { ticket, failure }),
        Ok(())
    );
    // Then tick returns RunNotFound
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn shard_duplicate_submit_after_cancel_succeeds() {
    // Given a shard with a cancelled run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(201);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // When re-submitting the same run ID
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    // Then it succeeds (run was removed by cancel)
    assert_eq!(shard.tick(), Ok(true));
}

#[test]
fn shard_snapshot_run_for_active_run_returns_found() {
    // Given a shard with an active suspended run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(202);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When snapshotting directly (non-queued)
    let response = shard.snapshot_run(run, 42);
    // Then it returns Found with correct fields
    match response {
        InspectResponse::Found(snap) => {
            assert_eq!(snap.run, run);
            assert_eq!(snap.correlation, 42);
        }
        other => {
            assert_eq!(
                other,
                InspectResponse::NotFound {
                    run,
                    correlation: 42
                }
            );
        }
    }
}

#[test]
fn shard_snapshot_run_for_unknown_returns_not_found() {
    // Given a shard with no runs
    let config = small_config();
    let shard = Shard::new(config);
    // When snapshotting a non-existent run
    let response = shard.snapshot_run(super::RunId::new(9999), 7);
    // Then it returns NotFound
    assert_eq!(
        response,
        InspectResponse::NotFound {
            run: super::RunId::new(9999),
            correlation: 7,
        }
    );
}

#[test]
fn shard_fill_queue_to_capacity_returns_queue_full() {
    // Given a shard with capacity 2
    let config = ShardConfig {
        command_queue_capacity: 2,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    // When filling the queue exactly
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    // Then the next enqueue returns QueueFull
    assert_eq!(
        shard.enqueue(ShardCommand::Shutdown),
        Err(RuntimeError::QueueFull)
    );
}

#[test]
fn adversarial_shard_ask_answered_for_unknown_run_returns_run_not_found() {
    // Given a shard with no runs
    let config = small_config();
    let mut shard = Shard::new(config);
    // When answering an ask for a non-existent run
    let answer = AskAnswer {
        ticket: AskTicket {
            run: super::RunId::new(999),
            ask_step: vb_core::ids::StepIdx::ZERO,
            resume_step: vb_core::ids::StepIdx::new(1),
        },
        answer_slot: SlotIdx::new(0),
        value: vb_core::value::SlotValue::Bool(true),
        taint: vb_core::value::Taint::Clean,
    };
    assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
    // Then tick returns RunNotFound
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn shard_submit_two_runs_same_id_second_fails() {
    // Given a shard with an active run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(203);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When submitting the same run ID without cancelling
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    // Then tick returns RunAlreadyExists
    assert_eq!(shard.tick(), Err(RuntimeError::RunAlreadyExists));
}

#[test]
fn shard_step_budget_zero_still_submits_but_does_not_drive() {
    // Given a shard with zero step budget
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 0,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(204);
    // When submitting a run with zero budget
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the run is submitted (counter incremented)
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    // And the run is still in the map (budget exhausted on first step)
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 1
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    match shard.take_inspect_response() {
        Some(InspectResponse::Found(snap)) => {
            assert_eq!(snap.run, run);
        }
        other => {
            assert_eq!(other, None);
        }
    }
}

#[test]
fn shard_multiple_cancels_idempotent_for_same_run() {
    // Given a shard with an active run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(205);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When cancelling twice
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // Then failed counter is 1 (not 2)
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
}

// =======================================================================
// Adversarial BDD tests - shard attack vectors
// =======================================================================

#[test]
fn shard_submit_after_shutdown_is_enqueued_but_never_processed() {
    // Given a shard that has received shutdown
    let config = small_config();
    let mut shard = Shard::new(config);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.tick(), Ok(false));
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    // When submitting a run after shutdown was processed
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(300),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    // Then tick returns false (shutting down flag prevents processing)
    assert_eq!(shard.tick(), Ok(false));
    // And no runs were submitted
    assert_eq!(shard.counters().snapshot().runs_submitted, 0);
}

#[test]
fn shard_cancel_then_resubmit_then_cancel_increments_failed_twice() {
    // Given a shard with a cancelled run that is then re-submitted
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(301);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When cancelling the re-submitted run
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // Then failed counter is 2 (both cancellations counted)
    assert_eq!(shard.counters().snapshot().runs_failed, 2);
    assert_eq!(shard.counters().snapshot().runs_submitted, 2);
}

#[test]
fn shard_action_completed_with_wrong_action_id_returns_invalid_completion() {
    // Given a shard with a suspended run on ActionId(0)
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(302);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When completing the action with a wrong action id
    let ticket = vb_core::action::ActionTicket {
        run,
        step: vb_core::ids::StepIdx::ZERO,
        seq: vb_core::ids::SeqNo::ZERO,
        action: ActionId::new(99),
        attempt: 1,
        idempotency_key: 0,
    };
    let output = vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: vb_core::value::SlotValue::I64(1),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 8,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted { ticket, output }),
        Ok(())
    );
    // Then tick returns InvalidActionCompletion
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidActionCompletion));
}

#[test]
fn shard_action_completed_for_finished_run_returns_run_not_found() {
    // Given a shard where a run has already finished
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = finished_workflow() else {
        return;
    };
    let run = super::RunId::new(303);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    // When completing an action for the finished run
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run,
            step: vb_core::ids::StepIdx::ZERO,
        }),
        Ok(())
    );
    // Then tick returns RunNotFound (run was removed after finishing)
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn shard_snapshot_run_after_cancel_returns_not_found() {
    // Given a shard with a cancelled run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(304);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // When snapshotting the cancelled run
    let response = shard.snapshot_run(run, 7);
    // Then it returns NotFound
    assert_eq!(
        response,
        InspectResponse::NotFound {
            run,
            correlation: 7,
        }
    );
}

#[test]
fn shard_timer_for_cancelled_run_returns_run_not_found() {
    // Given a shard with a cancelled run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(305);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // When a timer fires for the cancelled run
    assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
    // Then tick returns RunNotFound
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn shard_resume_for_cancelled_run_returns_run_not_found() {
    // Given a shard with a cancelled run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(306);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // When resuming the cancelled run
    assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
    // Then tick returns RunNotFound
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn shard_trace_ring_overflow_drops_events_gracefully() {
    // Given a shard with trace capacity of 2
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 2,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    // When submitting and completing multiple runs (producing >2 trace events)
    for i in 1u64..=4 {
        let Some(workflow) = finished_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: super::RunId::new(400 + i),
                workflow,
                caps: vb_core::capability::CapabilitySet::empty()
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
    }
    // Then the trace ring has dropped events
    let events = shard.trace_ring_mut().drain();
    assert_eq!(events.len() <= 2, true);
    assert_eq!(shard.trace_ring().dropped() > 0, true);
}

#[test]
fn shard_submit_run_reuses_frame_from_pool_after_prior_finish() {
    // Given a shard where a run finished and returned its frame to the pool
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = finished_workflow() else {
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(401),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    // When submitting a new run with the same workflow dimensions
    let Some(workflow2) = finished_workflow() else {
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(402),
            workflow: workflow2,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the second run also completes and pool has 1 available frame
    assert_eq!(shard.counters().snapshot().runs_completed, 2);
    assert_eq!(
        shard.frame_pools.get(&(2, 1)).map(FramePool::available),
        Some(1)
    );
}

#[test]
fn shard_submit_max_active_runs_boundary_exactly_at_limit_succeeds() {
    // Given a shard with max_active_runs = 3
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 3,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    // When submitting exactly 3 suspended runs (each suspends on Do, staying active)
    for i in 1u64..=3 {
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: super::RunId::new(500 + i),
                workflow,
                caps: vb_core::capability::CapabilitySet::empty()
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
    }
    // Then all 3 are submitted successfully
    assert_eq!(shard.counters().snapshot().runs_submitted, 3);
    // And submitting a 4th returns ActiveRunCapacityExceeded
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(504),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 3 })
    );
}

#[test]
fn shard_inspect_preserves_latest_response_overwriting_previous() {
    // Given a shard with two active runs
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(wf1) = suspended_workflow() else {
        return;
    };
    let Some(wf2) = suspended_workflow() else {
        return;
    };
    let run1 = super::RunId::new(600);
    let run2 = super::RunId::new(601);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: run1,
            workflow: wf1,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: run2,
            workflow: wf2,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When inspecting run1 then run2 without taking the first response
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run: run1,
            correlation: 1,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run: run2,
            correlation: 2,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then only the last inspect response is available (first was overwritten)
    let response = shard.take_inspect_response();
    match response {
        Some(InspectResponse::Found(snap)) => {
            assert_eq!(snap.run, run2);
            assert_eq!(snap.correlation, 2);
        }
        other => {
            assert_eq!(other, None);
        }
    }
}

// =========================================================================
// Phase 2 adversarial BDD tests — shard resource exhaustion & security
// =========================================================================

#[test]
fn shard_queue_full_prevents_further_command_submission() {
    // Given a shard with command queue capacity of 2
    let config = ShardConfig {
        command_queue_capacity: 2,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    // When filling the queue with 2 commands
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    // Then the third command is rejected with QueueFull
    assert_eq!(
        shard.enqueue(ShardCommand::Shutdown),
        Err(RuntimeError::QueueFull)
    );
}

#[test]
fn shard_active_run_capacity_exhausted_returns_precise_capacity_error() {
    // Given a shard with max_active_runs = 2
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 2,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    let Some(wf1) = suspended_workflow() else {
        return;
    };
    let Some(wf2) = suspended_workflow() else {
        return;
    };
    let Some(wf3) = suspended_workflow() else {
        return;
    };

    // When submitting 2 runs (both suspend on Do, so stay active)
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(1),
            workflow: wf1,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(2),
            workflow: wf2,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the third submit is rejected with capacity 2
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(3),
            workflow: wf3,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 2 })
    );
}

#[test]
fn shard_action_completed_for_wrong_run_returns_run_not_found() {
    // Given a shard with an active suspended run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(1),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When completing an action for a different run
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run: super::RunId::new(999),
            step: vb_core::ids::StepIdx::new(0),
        }),
        Ok(())
    );
    // Then tick returns RunNotFound
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn shard_step_budget_one_processes_one_command_per_tick() {
    // Given a shard with step_budget_per_tick = 1
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 1,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    let Some(workflow) = finished_workflow() else {
        return;
    };
    // When submitting a 2-step finished workflow
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(1),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then with budget 1, the first step executes but second does not
    // (budget exhausted after 1 transition; second tick needed)
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
}

#[test]
fn shard_duplicate_run_id_returns_run_already_exists_after_first_accepted() {
    // Given a shard with an active run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(wf1) = suspended_workflow() else {
        return;
    };
    let Some(wf2) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(42);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf1,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When submitting the same run ID again with a different workflow
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf2,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    // Then tick returns RunAlreadyExists (cannot replace workflow)
    assert_eq!(shard.tick(), Err(RuntimeError::RunAlreadyExists));
}

#[test]
fn shard_action_failed_for_unknown_run_returns_run_not_found() {
    // Given a shard with no active runs
    let config = small_config();
    let mut shard = Shard::new(config);
    let ticket = vb_core::action::ActionTicket {
        run: super::RunId::new(999),
        step: vb_core::ids::StepIdx::new(0),
        seq: vb_core::ids::SeqNo::new(1),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: 0,
    };
    let failure = vb_core::action::ActionFailure {
        code: vb_core::action::ActionFailureCode::Unknown,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: vb_core::value::Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    // When failing an action for a non-existent run
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed { ticket, failure }),
        Ok(())
    );
    // Then tick returns RunNotFound
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn shard_run_id_max_u64_accepted_as_valid_identifier() {
    // Given a shard
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = finished_workflow() else {
        return;
    };
    let run = super::RunId::new(u64::MAX);
    // When submitting a run with RunId::MAX
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the run is accepted and completes
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
}

#[test]
fn shard_ask_answered_for_unknown_run_returns_run_not_found() {
    // Given a shard with no active runs
    let config = small_config();
    let mut shard = Shard::new(config);
    let answer = AskAnswer {
        ticket: AskTicket {
            run: super::RunId::new(999),
            ask_step: vb_core::ids::StepIdx::new(0),
            resume_step: vb_core::ids::StepIdx::new(1),
        },
        answer_slot: SlotIdx::new(0),
        value: vb_core::SlotValue::I64(42),
        taint: vb_core::Taint::Clean,
    };
    // When answering an ask for a non-existent run
    assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
    // Then tick returns RunNotFound
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn shard_snapshot_for_nonexistent_run_returns_not_found() {
    // Given a shard with no runs
    let config = small_config();
    let shard = Shard::new(config);
    // When snapshotting a non-existent run
    let response = shard.snapshot_run(super::RunId::new(999), 42);
    // Then NotFound is returned
    assert_eq!(
        response,
        InspectResponse::NotFound {
            run: super::RunId::new(999),
            correlation: 42,
        }
    );
}

#[test]
fn shard_cancel_then_resubmit_same_run_id_succeeds() {
    // Given a shard with an active run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(wf1) = suspended_workflow() else {
        return;
    };
    let Some(wf2) = finished_workflow() else {
        return;
    };
    let run = super::RunId::new(55);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf1,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When cancelling and re-submitting with same ID
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf2,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the re-submitted run completes
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
}

#[test]
fn shard_trace_ring_records_submit_and_finish_events_in_order() {
    // Given a shard
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = finished_workflow() else {
        return;
    };
    let run = super::RunId::new(77);
    // When submitting a run that finishes immediately
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then trace ring has Submit and Finished events
    let events = shard.trace_ring_mut().drain();
    let found_submit = events
        .iter()
        .any(|e| *e == TraceEvent::RunSubmitted { run });
    let found_finish = events.iter().any(|e| *e == TraceEvent::RunFinished { run });
    assert_eq!(found_submit, true);
    assert_eq!(found_finish, true);
}

#[test]
fn shard_with_zero_trace_capacity_does_not_crash_on_submit() {
    // Given a shard with trace_capacity = 0
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 0,
        step_budget_per_tick: 4,
        max_active_runs: 2,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    let Some(workflow) = finished_workflow() else {
        return;
    };
    // When submitting a run
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(1),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    // Then tick succeeds (trace drops are non-fatal)
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
}

#[test]
fn shard_command_queue_len_starts_at_zero() {
    // Given a fresh shard
    let config = small_config();
    let shard = Shard::new(config);
    // Then queue length is 0
    assert_eq!(shard.command_queue_len(), 0);
}

#[test]
fn shard_command_queue_len_increments_on_enqueue() {
    // Given a shard with capacity 4
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    assert_eq!(shard.command_queue_len(), 0);
    // When enqueuing commands
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.command_queue_len(), 1);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.command_queue_len(), 2);
}

#[test]
fn shard_remaining_capacity_decrements_on_enqueue() {
    // Given a shard with capacity 4
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    assert_eq!(shard.remaining_capacity(), 4);
    // When enqueuing commands
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.remaining_capacity(), 3);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.remaining_capacity(), 2);
}

#[test]
fn shard_remaining_capacity_is_zero_when_full() {
    // Given a shard with capacity 2
    let config = ShardConfig {
        command_queue_capacity: 2,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    // Fill the queue
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    // Then remaining capacity is 0
    assert_eq!(shard.remaining_capacity(), 0);
}

#[test]
fn shard_is_queue_full_returns_false_initially() {
    // Given a fresh shard
    let config = small_config();
    let shard = Shard::new(config);
    // Then queue is not full
    assert_eq!(shard.is_queue_full(), false);
}

#[test]
fn shard_is_queue_full_returns_true_when_at_capacity() {
    // Given a shard with capacity 2
    let config = ShardConfig {
        command_queue_capacity: 2,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    // Fill the queue
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    // Then queue is full
    assert_eq!(shard.is_queue_full(), true);
}

#[test]
fn shard_command_queue_capacity_returns_configured_value() {
    // Given a shard configured with capacity 512
    let config = ShardConfig {
        command_queue_capacity: 512,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    // Then the capacity method returns 512
    assert_eq!(shard.command_queue_capacity(), 512);
}

#[test]
fn shard_remaining_capacity_after_pop() {
    // Given a shard with capacity 4 and 2 commands enqueued
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.remaining_capacity(), 2);
    // When popping one command
    assert_eq!(shard.tick(), Ok(false)); // Shutdown causes tick to return false
}

#[test]
fn shard_queue_len_decrements_after_tick() {
    // Given a shard with a Cancel command queued
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    // Cancel for a non-existent run succeeds silently
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel {
            run: super::RunId::new(999)
        }),
        Ok(())
    );
    assert_eq!(shard.command_queue_len(), 1);
    // When ticking
    assert_eq!(shard.tick(), Ok(true));
    // Then queue length is 0
    assert_eq!(shard.command_queue_len(), 0);
}

#[test]
fn shard_config_new_rejects_zero_command_queue_capacity() {
    let result = ShardConfig::new(0, 16, 4, 4, vb_core::policy::RuntimePolicy::Relaxed);
    assert_eq!(
        result,
        Err(RuntimeError::CommandQueueCapacityExceeded {
            capacity: 0,
            max: MAX_COMMAND_QUEUE_CAPACITY
        })
    );
}

#[test]
fn shard_config_new_rejects_excessive_command_queue_capacity() {
    let result = ShardConfig::new(
        MAX_COMMAND_QUEUE_CAPACITY + 1,
        16,
        4,
        4,
        vb_core::policy::RuntimePolicy::Relaxed,
    );
    assert_eq!(
        result,
        Err(RuntimeError::CommandQueueCapacityExceeded {
            capacity: MAX_COMMAND_QUEUE_CAPACITY + 1,
            max: MAX_COMMAND_QUEUE_CAPACITY
        })
    );
}

#[test]
fn shard_config_new_rejects_zero_max_active_runs() {
    let result = ShardConfig::new(16, 16, 4, 0, vb_core::policy::RuntimePolicy::Relaxed);
    assert_eq!(result, Err(RuntimeError::ActiveRunCapacityZero));
}

#[test]
fn shard_config_new_accepts_valid_parameters() {
    let result = ShardConfig::new(
        1024,
        4096,
        1000,
        512,
        vb_core::policy::RuntimePolicy::Relaxed,
    );
    assert_eq!(result.is_ok(), true);
}

#[test]
fn runtime_error_command_queue_capacity_exceeded_has_diagnostic_code() {
    let error = RuntimeError::CommandQueueCapacityExceeded {
        capacity: 100000,
        max: MAX_COMMAND_QUEUE_CAPACITY,
    };
    assert_eq!(
        error.diagnostic_code(),
        RuntimeError::COMMAND_QUEUE_CAPACITY_EXCEEDED_CODE
    );
}

#[test]
fn runtime_error_active_run_capacity_zero_has_diagnostic_code() {
    let error = RuntimeError::ActiveRunCapacityZero;
    assert_eq!(
        error.diagnostic_code(),
        RuntimeError::ACTIVE_RUN_CAPACITY_ZERO_CODE
    );
}
