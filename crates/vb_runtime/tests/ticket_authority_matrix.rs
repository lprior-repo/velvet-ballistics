#![forbid(unsafe_code)]
use std::error::Error;
use std::sync::Arc;
use vb_core::action::{
    ActionFailure, ActionFailureCode, ActionOutputReady, ActionTicket, RetryPolicy as VbRetryPolicy,
};
use vb_core::capability::CapabilitySet;
use vb_core::ids::{ActionId, ConstIdx, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::RuntimeError;
use vb_runtime::journal::VolatileRuntimeJournal;
use vb_runtime::shard::{Shard, ShardCommand, ShardConfig};
type TestResult<T> = Result<T, Box<dyn Error>>;

fn make_contract(
    id: u16,
    name: &str,
) -> Result<vb_core::action::ActionContract, Box<dyn std::error::Error>> {
    Ok(vb_core::action::ActionContract {
        id: ActionId::new(id),
        name: vb_core::action::ActionName::new(name)?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: vb_core::action::Idempotency::DeterministicPure,
        side_effect: vb_core::action::SideEffect::None,
        retry_safety: vb_core::action::RetrySafety::Safe,
        required_capabilities: Box::new([]),
    })
}

fn setconst_do_workflow(action_id: u16) -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    let set = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let do_node = CompiledNode {
        id: StepIdx::new(1),
        output: Some(SlotIdx::new(1)),
        next: Some(StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(action_id),
            input: SlotIdx::ZERO,
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(2),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
    };
    CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("setconst-do"),
        digest: WorkflowDigest::from_bytes([2; 32]),
        nodes: Box::from([set, do_node, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(42)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    })
    .map_err(|e| format!("workflow must compile: {}", e).into())
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
#[derive(Clone, Copy, Debug)]
enum Tamper {
    MissingRun,
    OtherRun,
    Step,
    Action,
    ZeroAttempt,
    FutureAttempt,
    ZeroCapacity,
    Capacity,
    Key,
    Sequence,
}
const CASES: [Tamper; 10] = [
    Tamper::MissingRun,
    Tamper::OtherRun,
    Tamper::Step,
    Tamper::Action,
    Tamper::ZeroAttempt,
    Tamper::FutureAttempt,
    Tamper::ZeroCapacity,
    Tamper::Capacity,
    Tamper::Key,
    Tamper::Sequence,
];
#[derive(Clone, Copy, Debug)]
enum Delivery {
    Completion,
    Failure,
}
fn submit(shard: &mut Shard, run: RunId) -> TestResult<ActionTicket> {
    shard.enqueue(ShardCommand::SubmitWithContracts {
        run,
        workflow: setconst_do_workflow(0)?,
        caps: CapabilitySet::empty(),
        action_contracts: Box::from([make_contract(0, "valid")?]),
    })?;
    assert_eq!(shard.tick(), Ok(true));
    shard
        .pending_action_get(run)
        .ok_or_else(|| "missing issued ticket".into())
}
fn corrupted(mut ticket: ActionTicket, case: Tamper, other: RunId) -> TestResult<ActionTicket> {
    match case {
        Tamper::MissingRun => ticket.run = RunId::new(99999),
        Tamper::OtherRun => ticket.run = other,
        Tamper::Step => ticket.step = StepIdx::new(99),
        Tamper::Action => ticket.action = ActionId::new(99),
        Tamper::ZeroAttempt => ticket.attempt = 0,
        Tamper::FutureAttempt => {
            ticket.attempt = ticket.attempt.checked_add(1).ok_or("attempt overflow")?
        }
        Tamper::ZeroCapacity => ticket.capacity = 0,
        Tamper::Capacity => {
            ticket.capacity = ticket.capacity.checked_add(1).ok_or("capacity overflow")?
        }
        Tamper::Key => ticket.idempotency_key ^= 1,
        Tamper::Sequence => {
            ticket.seq = SeqNo::new(ticket.seq.get().checked_add(1).ok_or("sequence overflow")?);
            ticket.idempotency_key = vb_core::action::compute_action_idempotency_key(
                ticket.run,
                ticket.run_attempt,
                ticket.seq,
                ticket.action,
            );
        }
    }
    Ok(ticket)
}
fn check_case(case: Tamper, delivery: Delivery) -> TestResult<()> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut shard = Shard::new_with_journal(
        ShardConfig {
            command_queue_capacity: 64,
            trace_capacity: 64,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        },
        journal.clone(),
    );
    let run = RunId::new(500);
    let other = RunId::new(501);
    let original = submit(&mut shard, run)?;
    let second = submit(&mut shard, other)?;
    let before = journal.snapshot()?;
    let state = shard
        .run_state_get(run)
        .ok_or("missing run before rejection")?;
    let pc = state.frame.pc();
    let value = *state.frame.read_slot(SlotIdx::ZERO)?;
    let taint = state.frame.read_taint(SlotIdx::ZERO)?;
    let ticket = corrupted(original, case, other)?;
    let command = match delivery {
        Delivery::Completion => ShardCommand::ActionCompleted {
            ticket,
            output: ActionOutputReady {
                output_slot: SlotIdx::new(1),
                value: SlotValue::I64(100),
                taint: Taint::Clean,
                encoded_len: 3,
            },
        },
        Delivery::Failure => ShardCommand::ActionFailed {
            ticket,
            failure: non_retryable_failure(),
        },
    };
    shard.enqueue(command)?;
    let expected = match case {
        Tamper::MissingRun => RuntimeError::RunNotFound,
        _ => RuntimeError::InvalidActionCompletion,
    };
    assert_eq!(shard.tick(), Err(expected), "{case:?} via {delivery:?}");
    assert_eq!(journal.snapshot()?, before, "{case:?} via {delivery:?}");
    assert_eq!(shard.pending_action_get(run), Some(original));
    assert_eq!(shard.pending_action_get(other), Some(second));
    let state = shard
        .run_state_get(run)
        .ok_or("missing run after rejection")?;
    assert_eq!(state.frame.pc(), pc);
    assert_eq!(state.frame.read_slot(SlotIdx::ZERO)?, &value);
    assert_eq!(state.frame.read_taint(SlotIdx::ZERO)?, taint);
    Ok(())
}
#[test]
fn completion_rejects_tampering_without_journal_or_run_mutation() -> TestResult<()> {
    for case in CASES {
        check_case(case, Delivery::Completion)?;
    }
    Ok(())
}
#[test]
fn failure_rejects_tampering_without_journal_or_run_mutation() -> TestResult<()> {
    for case in CASES {
        check_case(case, Delivery::Failure)?;
    }
    Ok(())
}
