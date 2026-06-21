use vb_core::ActionFailureCode;
use vb_core::action::RetryPolicy as VbRetryPolicy;
use vb_core::ids::{ActionId, ConstIdx, SlotIdx, WorkflowDigest};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

use crate::counters::ShardCounters;
use crate::frame_pool::FramePool;
use crate::journal::{NoopRuntimeJournal, RuntimeJournalEvent, SharedRuntimeJournal};
use crate::trace::{TraceEvent, TraceRing};
use crate::RuntimeError;

use crate::shard::{
    AskAnswer, AskTicket, InspectResponse, InspectSnapshot, MAX_COMMAND_QUEUE_CAPACITY,
    new_action_attempts, record_retry_attempt, RunId, RunState, Shard, ShardCommand, ShardConfig,
    TimerTick,
};
use crate::shard::types::{PendingTimer, PendingTimerKind};

fn timer_command(shard: &Shard, run: RunId) -> Option<ShardCommand> {
    let entry = shard.timer_entry(run)?;
    Some(ShardCommand::TimerFired {
        run: entry.run,
        generation: entry.generation,
        deadline: entry.deadline,
        kind: entry.kind,
    })
}

fn invalid_timer_command(run: RunId) -> ShardCommand {
    ShardCommand::TimerFired {
        run,
        generation: 0,
        deadline: std::time::Instant::now(),
        kind: PendingTimerKind::Wait,
    }
}

fn suspended_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let node = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
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
            error_slot: None,
        },
    };
    let action = CompiledNode {
        id: vb_core::ids::StepIdx::new(1),
        output: Some(SlotIdx::new(0)),
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

fn action_ticket(run: RunId, step: vb_core::ids::StepIdx) -> vb_core::action::ActionTicket {
    let seq = vb_core::ids::SeqNo::ZERO;
    let action = ActionId::new(0);
    vb_core::action::ActionTicket {
        run,
        step,
        seq,
        action,
        attempt: 1,
        idempotency_key: vb_core::action::compute_action_idempotency_key(run, seq, action),
        capacity: 1,
        ..Default::default()
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
fn retry_attempt_counter_increments_until_policy_exhaustion() -> Result<(), RuntimeError> {
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let frame = match vb_core::frame::RunFrame::new(
        RunId::new(9),
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
        action_attempts: new_action_attempts(1),
        admission: None,
        collect_states: crate::primitives::collect::CollectStates::new(),
        action_contracts: Box::new([]),
        last_snapshot_executed: 0,
    };
    let ticket = vb_core::action::ActionTicket {
        run: RunId::new(9),
        step: vb_core::ids::StepIdx::ZERO,
        seq: vb_core::ids::SeqNo::new(1),
        action: ActionId::new(0),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
            ..Default::default()
    };
    let policy = crate::engine::RetryPolicy {
        max_attempts: 2,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    assert_eq!(
        record_retry_attempt(&mut state, ticket, policy),
        Ok(true)
    );
    assert_eq!(state.action_attempts.get(0).copied(), Some(2));
    assert_eq!(
        record_retry_attempt(&mut state, ticket, policy),
        Ok(false)
    );
    Ok(())
}

#[test]
fn action_failed_routes_to_nearby_error_handler() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = action_with_error_handler_workflow() else {
        return Ok(());
    };
    let run = RunId::new(301);

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
    Ok(())
}

#[test]
fn action_failed_without_error_handler_fails_run() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(302);

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
    Ok(())
}

#[test]
fn submit_rejects_duplicate_run_id() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(451);

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
    assert_eq!(shard.active_run_count(), 1);
    Ok(())
}
