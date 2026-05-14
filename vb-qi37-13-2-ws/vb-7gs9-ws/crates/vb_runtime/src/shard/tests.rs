//! RED PHASE Tests for vb-7gs9 — Shard scheduler bounded ownership evidence
//!
//! These tests validate the bounded ownership invariants described in the contract.
//! They MUST fail against the current implementation until the bead is completed.
#![allow(dead_code, unused_imports)]

use vb_core::ActionFailureCode;
use vb_core::action::RetryPolicy as VbRetryPolicy;
use vb_core::ids::{ActionId, ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

use vb_runtime::counters::ShardCounters;
use vb_runtime::frame_pool::FramePool;
use vb_runtime::journal::{NoopRuntimeJournal, RuntimeJournalEvent, SharedRuntimeJournal};
use vb_runtime::trace::{TraceEvent, TraceRing};
use vb_runtime::{RuntimeError, RuntimeResult};

use vb_runtime::shard::types::{
    InspectResponse, InspectSnapshot, MAX_COMMAND_QUEUE_CAPACITY, RunState,
    Shard, ShardCommand, ShardConfig, PendingTimer, PendingTimerKind,
};
use vb_runtime::engine::EvidenceCollector;
use vb_runtime::engine::EvidenceEvent;

fn small_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    }
}

fn finished_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_const = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
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
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn suspended_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
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
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn timed_wait_then_finish_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_deadline = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let wait = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: Some(StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::ZERO,
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(2),
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
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn action_with_error_handler_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let guard = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::ErrorHandler {
            body: StepIdx::new(1),
            handler: StepIdx::new(2),
            error_slot: None,
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
    let parts = WorkflowParts {
        name: Box::from("action_with_error_handler"),
        digest: WorkflowDigest::from_bytes([3; 32]),
        nodes: Box::from([guard, action, handler, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::Bool(false)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

// =======================================================================
// RED PHASE: ShardConfig::new validation tests
// These tests validate preconditions that should be enforced
// =======================================================================

#[test]
fn ut_config_new_accepts_min_valid_capacity() {
    let result = ShardConfig::new(1, 1, 1, 1, vb_core::policy::RuntimePolicy::Relaxed);
    let expected = ShardConfig {
        command_queue_capacity: 1,
        trace_capacity: 1,
        step_budget_per_tick: 1,
        max_active_runs: 1,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    assert_eq!(result, Ok(expected));
}

#[test]
fn ut_config_new_rejects_zero_capacity() {
    let result = ShardConfig::new(0, 1, 1, 1, vb_core::policy::RuntimePolicy::Relaxed);
    assert_eq!(
        result,
        Err(RuntimeError::CommandQueueCapacityExceeded {
            capacity: 0,
            max: MAX_COMMAND_QUEUE_CAPACITY,
        })
    );
}

#[test]
fn ut_config_new_rejects_capacity_exceeding_max() {
    let too_large = MAX_COMMAND_QUEUE_CAPACITY.saturating_add(1);
    let result = ShardConfig::new(too_large, 1, 1, 1, vb_core::policy::RuntimePolicy::Relaxed);
    assert_eq!(
        result,
        Err(RuntimeError::CommandQueueCapacityExceeded {
            capacity: too_large,
            max: MAX_COMMAND_QUEUE_CAPACITY,
        })
    );
}

#[test]
fn ut_config_new_rejects_zero_max_active_runs() {
    let result = ShardConfig::new(1, 1, 1, 0, vb_core::policy::RuntimePolicy::Relaxed);
    assert_eq!(result, Err(RuntimeError::ActiveRunCapacityZero));
}

#[test]
fn ut_config_new_accepts_max_boundary_capacity() {
    let result = ShardConfig::new(
        MAX_COMMAND_QUEUE_CAPACITY,
        1,
        1,
        1,
        vb_core::policy::RuntimePolicy::Relaxed,
    );
    assert!(result.is_ok());
}

#[test]
fn ut_config_new_rejects_arbitrary_capacity_over_max() {
    let result = ShardConfig::new(100_000, 1, 1, 1, vb_core::policy::RuntimePolicy::Relaxed);
    assert_eq!(
        result,
        Err(RuntimeError::CommandQueueCapacityExceeded {
            capacity: 100_000,
            max: MAX_COMMAND_QUEUE_CAPACITY,
        })
    );
}

// =======================================================================
// RED PHASE: Shard::new construction tests
// =======================================================================

#[test]
fn ut_shard_new_creates_empty_shard() {
    let shard = Shard::new(small_config());
    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.pending_timer_count(), 0);
    assert_eq!(shard.command_queue_len(), 0);
    assert_eq!(shard.is_shutting_down(), false);
}

#[test]
fn ut_shard_new_sets_step_budget_per_tick() {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 7,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    assert_eq!(shard.counters().snapshot()._step_budget_per_tick, 7);
}

#[test]
fn ut_shard_new_sets_max_active_runs() {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 5,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    assert_eq!(shard.counters().snapshot()._max_active_runs, 5);
}

#[test]
fn ut_shard_new_sets_policy() {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Strict,
    };
    let shard = Shard::new(config);
    assert_eq!(shard.status().runtime_policy, vb_core::policy::RuntimePolicy::Strict);
}

#[test]
fn ut_shard_new_initializes_trace_ring() {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 128,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    assert_eq!(shard.status().trace_capacity, 128);
}

// =======================================================================
// RED PHASE: Enqueue bounded admission tests
// =======================================================================

#[test]
fn ut_enqueue_increments_queue_len() {
    let shard = Shard::new(small_config());
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.command_queue_len(), 1);
}

#[test]
fn ut_enqueue_decrements_remaining_capacity() {
    let shard = Shard::new(small_config());
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.remaining_capacity(), 14);
}

#[test]
fn ut_enqueue_returns_ok_on_space_available() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
}

#[test]
fn ut_enqueue_returns_queue_full_at_capacity() {
    let config = ShardConfig {
        command_queue_capacity: 2,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Err(RuntimeError::QueueFull));
}

#[test]
fn ut_enqueue_returns_queue_full_when_totally_full() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    for _ in 0..4 {
        let _ = shard.enqueue(ShardCommand::Shutdown);
    }
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Err(RuntimeError::QueueFull));
}

#[test]
fn ut_enqueue_is_idempotent_on_full_queue() {
    let config = ShardConfig {
        command_queue_capacity: 2,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    let first_full = shard.enqueue(ShardCommand::Shutdown);
    let second_full = shard.enqueue(ShardCommand::Shutdown);
    assert_eq!(first_full, Err(RuntimeError::QueueFull));
    assert_eq!(second_full, Err(RuntimeError::QueueFull));
    assert_eq!(shard.command_queue_len(), 2);
}

// =======================================================================
// RED PHASE: Tick command processing FIFO tests
// =======================================================================

#[test]
fn ut_tick_returns_true_on_empty_queue() {
    let mut shard = Shard::new(small_config());
    assert_eq!(shard.tick(), Ok(true));
}

#[test]
fn ut_tick_processes_shutdown_returns_false() {
    let mut shard = Shard::new(small_config());
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.tick(), Ok(false));
    assert_eq!(shard.is_shutting_down(), true);
}

#[test]
fn ut_tick_after_shutdown_always_returns_false() {
    let mut shard = Shard::new(small_config());
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.tick(), Ok(false));
    assert_eq!(shard.tick(), Ok(false));
    assert_eq!(shard.tick(), Ok(false));
}

#[test]
fn ut_tick_processes_commands_in_fifo_order() {
    let mut shard = Shard::new(small_config());
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: RunId::new(1),
            workflow: finished_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: RunId::new(2),
            workflow: finished_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_submitted, 2);
    assert_eq!(shard.counters().snapshot().runs_completed, 2);
}

#[test]
fn ut_tick_processes_at_most_one_command() {
    let mut shard = Shard::new(small_config());
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: RunId::new(1),
            workflow: finished_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: RunId::new(2),
            workflow: finished_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.command_queue_len(), 1);
}

#[test]
fn ut_tick_idempotent_on_empty_queue() {
    let mut shard = Shard::new(small_config());
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.command_queue_len(), 0);
}

// =======================================================================
// RED PHASE: Tick Submit tests
// =======================================================================

#[test]
fn ut_tick_submit_increments_runs_submitted() {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 2,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: RunId::new(1),
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
}

#[test]
fn ut_tick_submit_finishes_synchronous_workflow() {
    let mut shard = Shard::new(small_config());
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: RunId::new(1),
            workflow: finished_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.runs.len(), 0);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
}

#[test]
fn ut_tick_submit_returns_run_already_exists() {
    let mut shard = Shard::new(small_config());
    let workflow = suspended_workflow().unwrap();
    let run = RunId::new(42);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunAlreadyExists));
}

#[test]
fn ut_tick_submit_returns_active_run_capacity_exceeded() {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 1,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: RunId::new(1),
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: RunId::new(2),
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
    );
}

#[test]
fn ut_tick_submit_inserts_run_into_runs_map() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(10);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert!(shard.runs.contains_key(&run));
}

// =======================================================================
// RED PHASE: Tick Resume tests
// =======================================================================

#[test]
fn ut_tick_resume_continues_suspended_run() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(90);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert!(shard.runs.contains_key(&run));
}

#[test]
fn ut_tick_resume_returns_run_not_found() {
    let mut shard = Shard::new(small_config());
    assert_eq!(
        shard.enqueue(ShardCommand::Resume {
            run: RunId::new(999),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

// =======================================================================
// RED PHASE: Tick ActionCompleted tests
// =======================================================================

#[test]
fn ut_tick_action_completed_returns_run_not_found() {
    let mut shard = Shard::new(small_config());
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run: RunId::new(888),
            step: StepIdx::new(0),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn ut_tick_action_completed_advances_frame() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(55);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run,
            step: StepIdx::new(0),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
}

#[test]
fn ut_tick_action_completed_emits_trace_event() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(56);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run,
            step: StepIdx::new(0),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let events = shard.trace_ring_mut().drain();
    assert!(events.iter().any(|e| *e == TraceEvent::ActionCompleted { run, step: StepIdx::new(0) }));
}

// =======================================================================
// RED PHASE: Tick ActionFailed tests
// =======================================================================

fn make_action_ticket(run: RunId, step: StepIdx) -> vb_core::action::ActionTicket {
    vb_core::action::ActionTicket {
        run,
        step,
        seq: vb_core::ids::SeqNo::ZERO,
        action: ActionId::new(0),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
    }
}

fn make_timeout_failure() -> vb_core::action::ActionFailure {
    vb_core::action::ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: vb_core::value::Taint::Clean,
        detail: None,
        encoded_len: 0,
    }
}

#[test]
fn ut_tick_action_failed_fails_run_without_handler() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(302);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: make_action_ticket(run, StepIdx::ZERO),
            failure: make_timeout_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
}

#[test]
fn ut_tick_action_failed_routes_to_error_handler() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(301);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: action_with_error_handler_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: make_action_ticket(run, StepIdx::new(1)),
            failure: make_timeout_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
}

#[test]
fn ut_tick_action_failed_increments_failed_counter() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(303);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: make_action_ticket(run, StepIdx::ZERO),
            failure: make_timeout_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
}

// =======================================================================
// RED PHASE: Tick TimerFired tests
// =======================================================================

#[test]
fn ut_tick_timer_fired_returns_run_not_found() {
    let mut shard = Shard::new(small_config());
    assert_eq!(
        shard.enqueue(ShardCommand::TimerFired {
            run: RunId::new(777),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn ut_tick_timer_fired_returns_invalid_timer_fire() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(60);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
}

#[test]
fn ut_tick_timer_fired_consumes_pending_timer() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(62);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: timed_wait_then_finish_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 1);
    assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 0);
}

#[test]
fn ut_tick_timer_fired_advances_run() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(63);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: timed_wait_then_finish_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
}

// =======================================================================
// RED PHASE: Tick Cancel tests
// =======================================================================

#[test]
fn ut_tick_cancel_removes_run_from_runs() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(70);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert!(!shard.runs.contains_key(&run));
}

#[test]
fn ut_tick_cancel_increments_runs_failed() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(72);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
}

#[test]
fn ut_tick_cancel_emits_run_cancelled_event() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(71);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    let events = shard.trace_ring_mut().drain();
    assert!(events.iter().any(|e| *e == TraceEvent::RunCancelled { run }));
}

#[test]
fn ut_tick_cancel_removes_pending_timer() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(12);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: timed_wait_then_finish_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 1);
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 0);
}

#[test]
fn ut_tick_cancel_returns_frame_to_pool() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(11);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.frame_pools.get(&(1, 1)).map(|p| p.available()), Some(0));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.frame_pools.get(&(1, 1)).map(|p| p.available()), Some(1));
}

#[test]
fn ut_tick_cancel_is_idempotent_for_unknown_run() {
    let mut shard = Shard::new(small_config());
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel {
            run: RunId::new(999),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
}

// =======================================================================
// RED PHASE: Tick Inspect tests
// =======================================================================

#[test]
fn ut_tick_inspect_returns_found_for_active_run() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(80);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 10,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    match shard.take_inspect_response() {
        Some(InspectResponse::Found(snapshot)) => {
            assert_eq!(snapshot.run, run);
        }
        _ => panic!("Expected Found response"),
    }
}

#[test]
fn ut_tick_inspect_returns_not_found_for_missing_run() {
    let mut shard = Shard::new(small_config());
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run: RunId::new(999),
            correlation: 42,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.take_inspect_response(),
        Some(InspectResponse::NotFound {
            run: RunId::new(999),
            correlation: 42,
        })
    );
}

#[test]
fn ut_tick_inspect_does_not_mutate_run_state() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(81);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: finished_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 1,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
}

// =======================================================================
// RED PHASE: drain_for_shutdown tests
// =======================================================================

#[test]
fn ut_drain_clears_pending_timers() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(13);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: timed_wait_then_finish_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 1);
    assert_eq!(shard.drain_for_shutdown(), Ok(()));
    assert_eq!(shard.pending_timer_count(), 0);
}

#[test]
fn ut_drain_sets_shutting_down_flag() {
    let mut shard = Shard::new(small_config());
    assert_eq!(shard.drain_for_shutdown(), Ok(()));
    assert_eq!(shard.is_shutting_down(), true);
}

#[test]
fn ut_drain_returns_shutdown_in_progress_at_capacity_limit() {
    let config = ShardConfig {
        command_queue_capacity: 2,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    assert_eq!(
        shard.drain_for_shutdown(),
        Err(RuntimeError::ShutdownInProgress)
    );
}

#[test]
fn ut_drain_returns_ok_when_shutdown_command_processed() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: RunId::new(1),
            workflow: finished_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.drain_for_shutdown(), Ok(()));
    assert_eq!(shard.is_shutting_down(), true);
}

// =======================================================================
// RED PHASE: flush_evidence Evidence Chain tests (Phase 40/44)
// =======================================================================

#[test]
fn ut_flush_evidence_emits_step_started_before_slot_written() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let journal = std::sync::Arc::new(vb_runtime::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(config, shared);

    let mut collector = EvidenceCollector::new();
    collector.push(EvidenceEvent::StepStarted { step: StepIdx::ZERO });
    collector.push(EvidenceEvent::SlotWritten {
        slot: SlotIdx::ZERO,
        value: vb_core::value::SlotValue::Bool(true),
        extra: None,
    });

    let result = shard.flush_evidence(RunId::new(1), &mut collector);
    assert!(result.is_ok());

    let events = shard.trace_ring_mut().drain();
    let step_started_idx = events.iter().position(|e| matches!(e, TraceEvent::StepStarted { .. }));
    let slot_written_idx = events.iter().position(|e| matches!(e, TraceEvent::SlotWritten { .. }));
    assert!(step_started_idx.is_some());
    assert!(slot_written_idx.is_some());
    assert!(step_started_idx < slot_written_idx);
}

#[test]
fn ut_flush_evidence_emits_step_succeeded_after_slot_written() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let journal = std::sync::Arc::new(vb_runtime::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(config, shared);

    let mut collector = EvidenceCollector::new();
    collector.push(EvidenceEvent::StepStarted { step: StepIdx::ZERO });
    collector.push(EvidenceEvent::SlotWritten {
        slot: SlotIdx::ZERO,
        value: vb_core::value::SlotValue::Bool(true),
        extra: None,
    });
    collector.push(EvidenceEvent::StepSucceeded {
        step: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
    });

    let result = shard.flush_evidence(RunId::new(1), &mut collector);
    assert!(result.is_ok());

    let journal_events = journal.snapshot().unwrap();
    let step_started_idx = journal_events.iter().position(|e| matches!(e, RuntimeJournalEvent::StepStarted { .. }));
    let slot_written_idx = journal_events.iter().position(|e| matches!(e, RuntimeJournalEvent::SlotWritten { .. }));
    let step_succeeded_idx = journal_events.iter().position(|e| matches!(e, RuntimeJournalEvent::StepSucceeded { .. }));
    assert!(step_started_idx.is_some());
    assert!(slot_written_idx.is_some());
    assert!(step_succeeded_idx.is_some());
    assert!(step_started_idx < slot_written_idx);
    assert!(slot_written_idx < step_succeeded_idx);
}

#[test]
fn ut_flush_evidence_drains_collector_completely() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let journal = std::sync::Arc::new(vb_runtime::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(config, shared);

    let mut collector = EvidenceCollector::new();
    collector.push(EvidenceEvent::StepStarted { step: StepIdx::ZERO });

    let result = shard.flush_evidence(RunId::new(1), &mut collector);
    assert!(result.is_ok());
}

#[test]
fn ut_flush_slot_written_encodes_with_postcard() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let journal = std::sync::Arc::new(vb_runtime::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(config, shared);

    let original_value = vb_core::value::SlotValue::I64(42);
    let mut collector = EvidenceCollector::new();
    collector.push(EvidenceEvent::SlotWritten {
        slot: SlotIdx::ZERO,
        value: original_value.clone(),
        extra: None,
    });

    let result = shard.flush_evidence(RunId::new(1), &mut collector);
    assert!(result.is_ok());

    let journal_events = journal.snapshot().unwrap();
    if let Some(RuntimeJournalEvent::SlotWritten { value, .. }) = journal_events.first() {
        let decoded: vb_core::value::SlotValue = postcard::from_bytes(value).unwrap();
        assert_eq!(decoded, original_value);
    } else {
        panic!("Expected SlotWritten event");
    }
}

// =======================================================================
// RED PHASE: Frame Pool tests
// =======================================================================

#[test]
fn ut_take_frame_for_creates_pool_if_absent() {
    let mut shard = Shard::new(small_config());
    let workflow = finished_workflow().unwrap();
    let run = RunId::new(1);
    let result = shard.take_frame_for(run, &workflow);
    assert!(result.is_ok());
    assert!(shard.frame_pools.contains_key(&(2, 1)));
}

#[test]
fn ut_take_frame_for_returns_frame_with_correct_dimensions() {
    let mut shard = Shard::new(small_config());
    let workflow = finished_workflow().unwrap();
    let run = RunId::new(1);
    let frame = shard.take_frame_for(run, &workflow).unwrap();
    assert_eq!(frame.step_count(), 2);
    assert_eq!(frame.slot_count(), 1);
}

#[test]
fn ut_take_frame_for_reuses_existing_pool() {
    let mut shard = Shard::new(small_config());
    let workflow = finished_workflow().unwrap();
    let frame1 = shard.take_frame_for(RunId::new(1), &workflow).unwrap();
    let frame2 = shard.take_frame_for(RunId::new(2), &workflow).unwrap();
    assert_eq!(frame1.step_count(), frame2.step_count());
    assert_eq!(frame1.slot_count(), frame2.slot_count());
}

#[test]
fn ut_release_frame_returns_to_correct_pool() {
    let mut shard = Shard::new(small_config());
    let workflow = finished_workflow().unwrap();
    let run = RunId::new(1);
    let frame = shard.take_frame_for(run, &workflow).unwrap();
    shard.release_frame(frame);
    assert_eq!(shard.frame_pools.get(&(2, 1)).map(|p| p.available()), Some(1));
}

#[test]
fn ut_release_frame_increments_available_count() {
    let mut shard = Shard::new(small_config());
    let workflow = finished_workflow().unwrap();
    let frame = shard.take_frame_for(RunId::new(1), &workflow).unwrap();
    assert_eq!(shard.frame_pools.get(&(2, 1)).map(|p| p.available()), Some(0));
    shard.release_frame(frame);
    assert_eq!(shard.frame_pools.get(&(2, 1)).map(|p| p.available()), Some(1));
}

#[test]
fn ut_frame_pool_metrics_zero_initially() {
    let shard = Shard::new(small_config());
    let (free, total) = shard.frame_pool_metrics();
    assert_eq!(free, 0);
    assert_eq!(total, 0);
}

#[test]
fn ut_release_frame_ignores_unknown_dimension() {
    let mut shard = Shard::new(small_config());
    let dummy_frame = vb_core::frame::RunFrame::new(RunId::new(1), StepIdx::ZERO, 99, 99).unwrap();
    shard.release_frame(dummy_frame);
    assert!(shard.frame_pools.get(&(99, 99)).is_none());
}

// =======================================================================
// RED PHASE: Invariant assertion tests
// =======================================================================

#[test]
fn ut_invariant_i1_runs_len_never_exceeds_max_active_runs() {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 2,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);

    for i in 0..5 {
        let _ = shard.enqueue(ShardCommand::Submit {
            run: RunId::new(i),
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        });
        let _ = shard.tick();
    }

    assert!(shard.runs.len() <= shard.status().max_active_runs);
}

#[test]
fn ut_invariant_i2_queue_len_never_exceeds_capacity() {
    let config = ShardConfig {
        command_queue_capacity: 2,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    for _ in 0..5 {
        let _ = shard.enqueue(ShardCommand::Shutdown);
    }
    assert!(shard.command_queue_len() <= shard.command_queue_capacity());
}

#[test]
fn ut_invariant_i3_run_id_unique_in_runs() {
    let mut shard = Shard::new(small_config());
    let workflow = finished_workflow().unwrap();
    for i in 0..10 {
        let _ = shard.enqueue(ShardCommand::Submit {
            run: RunId::new(i),
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty(),
        });
        let _ = shard.tick();
    }
    let mut seen = std::collections::HashSet::new();
    for run_id in shard.runs.keys() {
        assert!(!seen.contains(run_id), "Duplicate RunId found: {:?}", run_id);
        seen.insert(*run_id);
    }
}

#[test]
fn ut_invariant_i4_run_id_unique_in_pending_timers() {
    let mut shard = Shard::new(small_config());
    let workflow = timed_wait_then_finish_workflow().unwrap();
    for i in 0..5 {
        let _ = shard.enqueue(ShardCommand::Submit {
            run: RunId::new(i),
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty(),
        });
        let _ = shard.tick();
    }
    let mut seen = std::collections::HashSet::new();
    for run_id in shard.pending_timers.keys() {
        assert!(!seen.contains(run_id), "Duplicate RunId in pending_timers: {:?}", run_id);
        seen.insert(*run_id);
    }
}

#[test]
fn ut_invariant_i7_shutting_down_is_permanent() {
    let mut shard = Shard::new(small_config());
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.tick(), Ok(false));
    for _ in 0..5 {
        assert_eq!(shard.tick(), Ok(false));
    }
    assert!(shard.is_shutting_down());
}

// =======================================================================
// RED PHASE: Integration tests
// =======================================================================

#[test]
fn it_enqueue_dequeue_cycle_exhausts_capacity_then_refuses() {
    let config = ShardConfig {
        command_queue_capacity: 3,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);

    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Err(RuntimeError::QueueFull));

    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Err(RuntimeError::QueueFull));
}

#[test]
fn it_tick_resumes_command_processing_after_drain() {
    let mut shard = Shard::new(small_config());
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.tick(), Ok(false));
    assert_eq!(shard.command_queue_len(), 0);

    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.tick(), Ok(false));
}

#[test]
fn it_drain_for_shutdown_respects_capacity_limit() {
    let config = ShardConfig {
        command_queue_capacity: 2,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    assert_eq!(
        shard.drain_for_shutdown(),
        Err(RuntimeError::ShutdownInProgress)
    );
}

#[test]
fn it_submit_and_cancel_cycles_frame_through_pool() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(1);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.frame_pools.get(&(1, 1)).map(|p| p.available()), Some(0));

    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.frame_pools.get(&(1, 1)).map(|p| p.available()), Some(1));
}

#[test]
fn it_multiple_runs_same_dimension_share_pool() {
    let mut shard = Shard::new(small_config());
    let workflow = suspended_workflow().unwrap();

    for i in 0..3 {
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(i),
                workflow: workflow.clone(),
                caps: vb_core::capability::CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
    }

    assert_eq!(shard.frame_pools.get(&(1, 1)).map(|p| p.available()), Some(0));
    assert_eq!(shard.frame_pools.get(&(1, 1)).map(|p| p.capacity()), Some(3));
}

#[test]
fn it_different_dimensions_create_separate_pools() {
    let mut shard = Shard::new(small_config());
    let workflow1 = suspended_workflow().unwrap();
    let workflow2 = finished_workflow().unwrap();

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: RunId::new(1),
            workflow: workflow1,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: RunId::new(2),
            workflow: workflow2,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert!(shard.frame_pools.contains_key(&(1, 1)));
    assert!(shard.frame_pools.contains_key(&(2, 1)));
}

#[test]
fn it_submit_finishes_workflow_releases_frame() {
    let mut shard = Shard::new(small_config());
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: RunId::new(1),
            workflow: finished_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.frame_pools.get(&(2, 1)).map(|p| p.available()), Some(1));
}

#[test]
fn it_submit_suspended_workflow_retains_frame_until_suspend() {
    let mut shard = Shard::new(small_config());
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: RunId::new(1),
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.frame_pools.get(&(1, 1)).map(|p| p.available()), Some(0));
    assert!(shard.runs.contains_key(&RunId::new(1)));
}

#[test]
fn it_evidence_chain_step_started_before_slot_written_in_journal() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let journal = std::sync::Arc::new(vb_runtime::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(config, shared);

    let mut collector = EvidenceCollector::new();
    collector.push(EvidenceEvent::StepStarted { step: StepIdx::ZERO });
    collector.push(EvidenceEvent::SlotWritten {
        slot: SlotIdx::ZERO,
        value: vb_core::value::SlotValue::Bool(true),
        extra: None,
    });
    collector.push(EvidenceEvent::StepSucceeded {
        step: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
    });

    let _ = shard.flush_evidence(RunId::new(1), &mut collector);
    let events = journal.snapshot().unwrap();

    let step_started_pos = events.iter().position(|e| matches!(e, RuntimeJournalEvent::StepStarted { .. }));
    let slot_written_pos = events.iter().position(|e| matches!(e, RuntimeJournalEvent::SlotWritten { .. }));
    let step_succeeded_pos = events.iter().position(|e| matches!(e, RuntimeJournalEvent::StepSucceeded { .. }));

    assert!(step_started_pos.is_some() && slot_written_pos.is_some() && step_succeeded_pos.is_some());
    assert!(step_started_pos < slot_written_pos);
    assert!(slot_written_pos < step_succeeded_pos);
}

#[test]
fn it_evidence_flushed_before_tick_returns() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let journal = std::sync::Arc::new(vb_runtime::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(config, shared);

    let mut collector = EvidenceCollector::new();
    collector.push(EvidenceEvent::StepStarted { step: StepIdx::ZERO });

    let _ = shard.flush_evidence(RunId::new(1), &mut collector);
    let events = shard.trace_ring_mut().drain();
    assert!(!events.is_empty());
}

#[test]
fn it_multiple_steps_produce_ordered_evidence() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 32,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let journal = std::sync::Arc::new(vb_runtime::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(config, shared);

    let mut collector = EvidenceCollector::new();
    collector.push(EvidenceEvent::StepStarted { step: StepIdx::ZERO });
    collector.push(EvidenceEvent::StepSucceeded { step: StepIdx::ZERO, output: None });
    collector.push(EvidenceEvent::StepStarted { step: StepIdx::new(1) });
    collector.push(EvidenceEvent::StepSucceeded { step: StepIdx::new(1), output: None });
    collector.push(EvidenceEvent::StepStarted { step: StepIdx::new(2) });
    collector.push(EvidenceEvent::StepSucceeded { step: StepIdx::new(2), output: None });

    let _ = shard.flush_evidence(RunId::new(1), &mut collector);
    let events = journal.snapshot().unwrap();

    let step0_started = events.iter().position(|e| matches!(e, RuntimeJournalEvent::StepStarted { step, .. } if *step == StepIdx::ZERO));
    let step1_started = events.iter().position(|e| matches!(e, RuntimeJournalEvent::StepStarted { step, .. } if *step == StepIdx::new(1)));
    let step2_started = events.iter().position(|e| matches!(e, RuntimeJournalEvent::StepStarted { step, .. } if *step == StepIdx::new(2)));

    assert!(step0_started < step1_started);
    assert!(step1_started < step2_started);
}

#[test]
fn it_wait_workflow_timer_fires_and_continues() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(62);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: timed_wait_then_finish_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 1);
    assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 0);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
}

#[test]
fn it_cancel_clears_wait_timer() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(12);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: timed_wait_then_finish_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 1);
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 0);
}

#[test]
fn it_submit_resume_cancel_full_lifecycle() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(90);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert!(shard.runs.contains_key(&run));

    assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert!(shard.runs.contains_key(&run));

    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert!(!shard.runs.contains_key(&run));
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
}

#[test]
fn it_runs_submitted_incremented_on_submit() {
    let mut shard = Shard::new(small_config());
    let workflow = suspended_workflow().unwrap();
    for i in 0..3 {
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(i),
                workflow: workflow.clone(),
                caps: vb_core::capability::CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
    }
    assert_eq!(shard.counters().snapshot().runs_submitted, 3);
}

#[test]
fn it_runs_failed_incremented_on_cancel() {
    let mut shard = Shard::new(small_config());
    let workflow = suspended_workflow().unwrap();
    for i in 0..2 {
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(i),
                workflow: workflow.clone(),
                caps: vb_core::capability::CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
    }

    for i in 0..2 {
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run: RunId::new(i) }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
    }
    assert_eq!(shard.counters().snapshot().runs_failed, 2);
}

#[test]
fn it_runs_completed_incremented_on_sync_finish() {
    let mut shard = Shard::new(small_config());
    let workflow = finished_workflow().unwrap();
    for i in 0..2 {
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(i),
                workflow: workflow.clone(),
                caps: vb_core::capability::CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
    }
    assert_eq!(shard.counters().snapshot().runs_completed, 2);
}

#[test]
fn it_inspect_returns_fresh_data_after_tick() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(81);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: finished_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
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
    match shard.take_inspect_response() {
        Some(InspectResponse::NotFound { .. }) => {}
        other => panic!("Expected NotFound, got {:?}", other),
    }
}

#[test]
fn it_status_reports_health_without_mutation() {
    let mut shard = Shard::new(small_config());
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    let before_len = shard.command_queue_len();
    let status = shard.status();
    assert_eq!(status.health, vb_runtime::shard::types::ShardHealth::Running);
    assert_eq!(shard.command_queue_len(), before_len);
}

// =======================================================================
// RED PHASE: BDD Scenarios
// =======================================================================

#[test]
fn scenario_shard_initializes_with_empty_state_and_correct_configuration() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 8,
        max_active_runs: 2,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.pending_timer_count(), 0);
    assert_eq!(shard.command_queue_capacity(), 4);
    assert_eq!(shard.is_shutting_down(), false);
    assert_eq!(shard.status().max_active_runs, 2);
    assert_eq!(shard.status().step_budget_per_tick, 8);
}

#[test]
fn scenario_shard_config_rejects_invalid_capacity_at_construction() {
    let result = ShardConfig::new(0, 1, 1, 1, vb_core::policy::RuntimePolicy::Relaxed);
    assert_eq!(
        result,
        Err(RuntimeError::CommandQueueCapacityExceeded {
            capacity: 0,
            max: MAX_COMMAND_QUEUE_CAPACITY,
        })
    );
}

#[test]
fn scenario_enqueue_adds_command_and_decrements_remaining_capacity() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    assert_eq!(shard.remaining_capacity(), 4);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.remaining_capacity(), 3);
}

#[test]
fn scenario_enqueue_rejects_when_queue_is_at_capacity() {
    let config = ShardConfig {
        command_queue_capacity: 2,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Err(RuntimeError::QueueFull));
    assert_eq!(shard.command_queue_len(), 2);
}

#[test]
fn scenario_shutdown_command_sets_permanent_shutting_down_flag() {
    let mut shard = Shard::new(small_config());
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.tick(), Ok(false));
    assert!(shard.is_shutting_down());
}

#[test]
fn scenario_tick_returns_false_permanently_after_shutdown() {
    let mut shard = Shard::new(small_config());
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.tick(), Ok(false));
    assert_eq!(shard.tick(), Ok(false));
    assert_eq!(shard.tick(), Ok(false));
}

#[test]
fn scenario_drain_for_shutdown_processes_all_pending_commands() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: RunId::new(1),
            workflow: finished_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.drain_for_shutdown(), Ok(()));
    assert!(shard.is_shutting_down());
    assert_eq!(shard.pending_timer_count(), 0);
}

#[test]
fn scenario_submit_adds_run_to_runs_map_under_capacity() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(1);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert!(shard.runs.contains_key(&run));
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
}

#[test]
fn scenario_submit_returns_run_already_exists_for_duplicate() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(42);
    let workflow = suspended_workflow().unwrap();
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunAlreadyExists));
}

#[test]
fn scenario_submit_returns_active_run_capacity_exceeded_when_at_limit() {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 1,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    let run1 = RunId::new(1);
    let run2 = RunId::new(2);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: run1,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: run2,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
    );
    assert!(shard.runs.contains_key(&run1));
    assert!(!shard.runs.contains_key(&run2));
}

#[test]
fn scenario_cancel_removes_run_and_increments_failed_counter() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(70);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert!(!shard.runs.contains_key(&run));
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
}

#[test]
fn scenario_cancel_emits_run_cancelled_trace_event() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(71);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    let events = shard.trace_ring_mut().drain();
    assert!(events.iter().any(|e| *e == TraceEvent::RunCancelled { run }));
}

#[test]
fn scenario_cancel_returns_frame_to_dimension_pool() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(11);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.frame_pools.get(&(1, 1)).map(|p| p.available()), Some(0));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.frame_pools.get(&(1, 1)).map(|p| p.available()), Some(1));
}

#[test]
fn scenario_cancel_clears_pending_timer_if_present() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(12);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: timed_wait_then_finish_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 1);
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 0);
}

#[test]
fn scenario_cancel_is_idempotent_for_unknown_run() {
    let mut shard = Shard::new(small_config());
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel {
            run: RunId::new(999),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
}

#[test]
fn scenario_timer_fired_advances_waiting_run() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(62);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: timed_wait_then_finish_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 1);
    assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 0);
}

#[test]
fn scenario_timer_fired_returns_run_not_found_for_unknown_run() {
    let mut shard = Shard::new(small_config());
    assert_eq!(
        shard.enqueue(ShardCommand::TimerFired {
            run: RunId::new(999),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn scenario_timer_fired_returns_invalid_timer_fire_when_no_timer_pending() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(60);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
}

#[test]
fn scenario_take_frame_for_creates_dimension_pool_on_first_use() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(1);
    let workflow = finished_workflow().unwrap();
    let frame = shard.take_frame_for(run, &workflow).unwrap();
    assert!(shard.frame_pools.contains_key(&(2, 1)));
    assert_eq!(frame.step_count(), 2);
    assert_eq!(frame.slot_count(), 1);
}

#[test]
fn scenario_release_frame_returns_frame_to_correct_pool() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(1);
    let workflow = finished_workflow().unwrap();
    let frame = shard.take_frame_for(run, &workflow).unwrap();
    shard.release_frame(frame);
    assert_eq!(shard.frame_pools.get(&(2, 1)).map(|p| p.available()), Some(1));
}

#[test]
fn scenario_multiple_runs_share_dimension_pool() {
    let mut shard = Shard::new(small_config());
    let workflow = suspended_workflow().unwrap();

    for i in 0..3 {
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(i),
                workflow: workflow.clone(),
                caps: vb_core::capability::CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
    }

    let pool = shard.frame_pools.get(&(1, 1)).expect("Pool should exist");
    assert_eq!(pool.capacity(), 3);
    assert_eq!(pool.available(), 0);
}

#[test]
fn scenario_single_step_produces_correct_evidence_order() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let journal = std::sync::Arc::new(vb_runtime::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(config, shared);

    let mut collector = EvidenceCollector::new();
    collector.push(EvidenceEvent::StepStarted { step: StepIdx::ZERO });
    collector.push(EvidenceEvent::SlotWritten {
        slot: SlotIdx::ZERO,
        value: vb_core::value::SlotValue::Bool(true),
        extra: None,
    });
    collector.push(EvidenceEvent::StepSucceeded {
        step: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
    });

    let _ = shard.flush_evidence(RunId::new(1), &mut collector);
    let events = journal.snapshot().unwrap();

    let step_started_pos = events.iter().position(|e| matches!(e, RuntimeJournalEvent::StepStarted { .. }));
    let slot_written_pos = events.iter().position(|e| matches!(e, RuntimeJournalEvent::SlotWritten { .. }));
    let step_succeeded_pos = events.iter().position(|e| matches!(e, RuntimeJournalEvent::StepSucceeded { .. }));

    assert!(step_started_pos.is_some() && slot_written_pos.is_some() && step_succeeded_pos.is_some());
    assert!(step_started_pos < slot_written_pos);
    assert!(slot_written_pos < step_succeeded_pos);
}

#[test]
fn scenario_inspect_returns_found_for_active_run() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(80);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 42,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    match shard.take_inspect_response() {
        Some(InspectResponse::Found(snapshot)) => {
            assert_eq!(snapshot.run, run);
            assert_eq!(snapshot.correlation, 42);
        }
        other => panic!("Expected Found, got {:?}", other),
    }
}

#[test]
fn scenario_inspect_returns_not_found_for_missing_run() {
    let mut shard = Shard::new(small_config());
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run: RunId::new(999),
            correlation: 99,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.take_inspect_response(),
        Some(InspectResponse::NotFound {
            run: RunId::new(999),
            correlation: 99,
        })
    );
}

#[test]
fn scenario_action_failed_without_error_handler_fails_the_run() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(302);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: suspended_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: make_action_ticket(run, StepIdx::ZERO),
            failure: make_timeout_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert!(!shard.runs.contains_key(&run));
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
}

#[test]
fn scenario_action_failed_with_error_handler_routes_to_handler() {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(301);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: action_with_error_handler_workflow().unwrap(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: make_action_ticket(run, StepIdx::new(1)),
            failure: make_timeout_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert!(shard.runs.contains_key(&run));
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
}