#![forbid(unsafe_code)]

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::capability::CapabilitySet;
use vb_core::engine::StepBudget;
use vb_core::frame::{RunFrame, StepState};
use vb_core::ids::{ActionId, ConstIdx, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::value::{ConstValue, SlotValue};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;
use vb_runtime::shard::{
    InspectResponse, ResumeError, Shard, ShardCommand, ShardConfig, TerminalOutcome,
};

fn one_step_workflow(kind: CompiledNodeKind, slot_count: u16) -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("vb_5m8w_runtime_suspension"),
        digest: WorkflowDigest::from_bytes([0x8b; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind,
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|error| error.to_string())
}

fn const_then_finish_workflow(value: ConstValue) -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("vb_5m8w_runtime_completed_then_exhausted"),
        digest: WorkflowDigest::from_bytes([0x5e; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: Some(SlotIdx::new(0)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![value].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|error| error.to_string())
}

fn new_run(workflow: &CompiledWorkflow, run_id: u64) -> Result<RunFrame, String> {
    RunFrame::new(
        RunId::new(run_id),
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|error| error.to_string())
}

fn drive_with_evidence(
    workflow: &CompiledWorkflow,
    run: &mut RunFrame,
    budget: &mut StepBudget,
    store: &mut ValueStore,
    evidence: &mut EvidenceCollector,
) -> Result<RuntimeSignal, String> {
    let mut collect_states = CollectStates::new();
    drive_deterministic_full(
        workflow,
        run,
        budget,
        store,
        &[],
        RetryPolicy::NEVER,
        evidence,
        &mut collect_states,
        &CapabilitySet::empty(),
    )
    .map_err(|error| error.to_string())
}

fn drive_with_contracts(
    workflow: &CompiledWorkflow,
    run: &mut RunFrame,
    budget: &mut StepBudget,
    store: &mut ValueStore,
    evidence: &mut EvidenceCollector,
    action_contracts: &[ActionContract],
) -> Result<RuntimeSignal, String> {
    let mut collect_states = CollectStates::new();
    drive_deterministic_full(
        workflow,
        run,
        budget,
        store,
        action_contracts,
        RetryPolicy::NEVER,
        evidence,
        &mut collect_states,
        &CapabilitySet::empty(),
    )
    .map_err(|error| error.to_string())
}

fn action_contract(action: ActionId) -> ActionContract {
    ActionContract {
        id: action,
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    }
}

#[test]
fn given_zero_budget_when_drive_runs_then_no_step_started_or_succeeded_evidence()
-> Result<(), String> {
    let workflow = const_then_finish_workflow(ConstValue::I64(10))?;
    let mut run = new_run(&workflow, 5801)?;
    let mut store = ValueStore::new();
    let mut budget = StepBudget::new(0);
    let mut evidence = EvidenceCollector::new();

    let signal = drive_with_evidence(&workflow, &mut run, &mut budget, &mut store, &mut evidence)?;
    let events = evidence.drain();

    assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
    assert_eq!(events, Vec::<EvidenceEvent>::new());
    assert_eq!(run.pc(), StepIdx::new(0));
    assert_eq!(run.executed(), 0);
    assert_eq!(run.step_state(StepIdx::new(0)), Ok(StepState::Pending));
    Ok(())
}

#[test]
fn given_one_step_completed_when_next_budget_exhausts_then_completed_step_remains_succeeded()
-> Result<(), String> {
    let workflow = const_then_finish_workflow(ConstValue::I64(77))?;
    let mut run = new_run(&workflow, 5802)?;
    let mut store = ValueStore::new();
    let mut positive_budget = StepBudget::new(1);
    let mut first_evidence = EvidenceCollector::new();

    let first_signal = drive_with_evidence(
        &workflow,
        &mut run,
        &mut positive_budget,
        &mut store,
        &mut first_evidence,
    )?;
    assert_eq!(first_signal, RuntimeSignal::StepBudgetExhausted);
    assert_eq!(run.pc(), StepIdx::new(1));
    assert_eq!(run.executed(), 1);
    assert_eq!(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded));
    assert_eq!(
        run.read_slot(SlotIdx::new(0)).map(|value| *value),
        Ok(SlotValue::I64(77))
    );

    let mut zero_budget = StepBudget::new(0);
    let mut second_evidence = EvidenceCollector::new();
    let second_signal = drive_with_evidence(
        &workflow,
        &mut run,
        &mut zero_budget,
        &mut store,
        &mut second_evidence,
    )?;

    assert_eq!(second_signal, RuntimeSignal::StepBudgetExhausted);
    assert_eq!(second_evidence.drain(), Vec::<EvidenceEvent>::new());
    assert_eq!(run.pc(), StepIdx::new(1));
    assert_eq!(run.executed(), 1);
    assert_eq!(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded));
    assert_eq!(run.step_state(StepIdx::new(1)), Ok(StepState::Pending));
    assert_eq!(
        run.read_slot(SlotIdx::new(0)).map(|value| *value),
        Ok(SlotValue::I64(77))
    );
    Ok(())
}

#[test]
fn given_action_wait_or_ask_suspension_when_drive_returns_then_signal_is_not_step_budget_exhausted_and_no_false_success()
-> Result<(), String> {
    let scenarios = [
        (
            "wait_until",
            one_step_workflow(
                CompiledNodeKind::WaitUntil {
                    deadline_slot: SlotIdx::new(0),
                },
                1,
            )?,
            RuntimeSignal::AwaitingWait(SlotIdx::ZERO),
            StepState::Waiting,
            SlotValue::I64(1),
        ),
        (
            "wait_event",
            one_step_workflow(
                CompiledNodeKind::WaitEvent {
                    event: SlotIdx::new(0),
                    timeout_slot: None,
                },
                1,
            )?,
            RuntimeSignal::AwaitingWait(SlotIdx::ZERO),
            StepState::Waiting,
            SlotValue::I64(1),
        ),
        (
            "ask",
            one_step_workflow(
                CompiledNodeKind::Ask {
                    prompt: SlotIdx::new(0),
                    timeout_slot: None,
                },
                1,
            )?,
            RuntimeSignal::AwaitingAsk(None),
            StepState::Asking,
            SlotValue::Symbol(SymbolId::new(1)),
        ),
    ];

    for (name, workflow, expected_signal, expected_state, input) in scenarios {
        let mut run = new_run(&workflow, 5900)?;
        run.write_slot(SlotIdx::new(0), input)
            .map_err(|error| error.to_string())?;
        let mut store = ValueStore::new();
        let mut budget = StepBudget::new(1);
        let mut evidence = EvidenceCollector::new();

        let signal =
            drive_with_evidence(&workflow, &mut run, &mut budget, &mut store, &mut evidence)?;
        let events = evidence.drain();
        let succeeded_count = events
            .iter()
            .filter(|event| matches!(event, EvidenceEvent::StepSucceeded { .. }))
            .count();
        let slot_written_count = events
            .iter()
            .filter(|event| matches!(event, EvidenceEvent::SlotWritten { .. }))
            .count();

        assert_eq!(signal, expected_signal, "scenario {name} signal mismatch");
        assert_eq!(
            run.step_state(StepIdx::new(0)),
            Ok(expected_state),
            "scenario {name} state mismatch"
        );
        assert_eq!(
            succeeded_count, 0,
            "scenario {name} must not emit StepSucceeded"
        );
        assert_eq!(
            slot_written_count, 0,
            "scenario {name} must not emit SlotWritten"
        );
    }
    Ok(())
}

#[test]
fn given_action_suspension_when_drive_returns_then_signal_is_awaiting_action_and_no_false_success()
-> Result<(), String> {
    let action = ActionId::new(0);
    let workflow = one_step_workflow(
        CompiledNodeKind::Do {
            action,
            input: SlotIdx::new(0),
        },
        1,
    )?;
    let mut run = new_run(&workflow, 5901)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))
        .map_err(|error| error.to_string())?;
    let mut store = ValueStore::new();
    let mut budget = StepBudget::new(1);
    let mut evidence = EvidenceCollector::new();
    let contracts = [action_contract(action)];

    let signal = drive_with_contracts(
        &workflow,
        &mut run,
        &mut budget,
        &mut store,
        &mut evidence,
        &contracts,
    )?;
    let events = evidence.drain();
    let started_count = events
        .iter()
        .filter(|event| matches!(event, EvidenceEvent::StepStarted { .. }))
        .count();
    let succeeded_count = events
        .iter()
        .filter(|event| matches!(event, EvidenceEvent::StepSucceeded { .. }))
        .count();
    let slot_written_count = events
        .iter()
        .filter(|event| matches!(event, EvidenceEvent::SlotWritten { .. }))
        .count();

    match signal {
        RuntimeSignal::AwaitingAction(ticket) => {
            assert_eq!(ticket.run, RunId::new(5901));
            assert_eq!(ticket.step, StepIdx::new(0));
            assert_eq!(ticket.action, action);
            assert_eq!(ticket.attempt, 1);
        }
        other => return Err(format!("expected AwaitingAction, got {other:?}")),
    }
    assert_eq!(run.step_state(StepIdx::new(0)), Ok(StepState::Running));
    assert_eq!(started_count, 1);
    assert_eq!(succeeded_count, 0);
    assert_eq!(slot_written_count, 0);
    assert_eq!(budget.remaining(), 0);
    Ok(())
}

#[test]
fn given_runtime_step_budget_exhausted_when_apply_drive_result_then_run_is_kept_and_drive_continue_emitted()
-> Result<(), String> {
    let config = ShardConfig {
        command_queue_capacity: 8,
        trace_capacity: 8,
        step_budget_per_tick: 0,
        max_active_runs: 8,
        policy: RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    let run = RunId::new(5810);
    let workflow = const_then_finish_workflow(ConstValue::I64(33))?;

    shard
        .enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: CapabilitySet::empty(),
        })
        .map_err(|error| error.to_string())?;
    let keep_running = shard.tick().map_err(|error| error.to_string())?;
    let snapshot = shard.snapshot_run(run, 44);

    assert_eq!(keep_running, true);
    assert_eq!(shard.active_run_count(), 1);
    match snapshot {
        InspectResponse::Found(found) => {
            assert_eq!(found.run, run);
            assert_eq!(found.correlation, 44);
            assert_eq!(found.pc, StepIdx::new(0));
            assert_eq!(found.executed, 0);
        }
        InspectResponse::NotFound { .. } => return Err("budget-exhausted run was removed".into()),
        _ => return Err("unexpected inspect response variant".into()),
    }
    Ok(())
}

#[test]
fn given_terminal_run_when_resume_attempted_then_invalid_resume_error() -> Result<(), String> {
    let config = ShardConfig {
        command_queue_capacity: 8,
        trace_capacity: 8,
        step_budget_per_tick: 8,
        max_active_runs: 8,
        policy: RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    let run = RunId::new(5811);
    let workflow = const_then_finish_workflow(ConstValue::I64(34))?;

    shard
        .enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: CapabilitySet::empty(),
        })
        .map_err(|error| error.to_string())?;
    let keep_running = shard.tick().map_err(|error| error.to_string())?;
    let snapshot = shard.snapshot_run(run, 45);
    let resume_result = shard.handle_resume(run);

    assert_eq!(keep_running, true);
    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(
        snapshot,
        InspectResponse::Terminal {
            run,
            correlation: 45,
            outcome: TerminalOutcome::Completed,
        }
    );
    assert_eq!(
        resume_result,
        Err(ResumeError::RunIdNotFound { run_id: run })
    );
    Ok(())
}

// =========================================================================
// vb-u09ai: 4-variant RetrySafety step-budget-suspension test (Tier 1).
// =========================================================================

/// Tier 1: `vb_core::action::is_idempotent(RetrySafety::Idempotent) == true`
/// per the master §65 contract (C6). The `is_idempotent(RetrySafety)` const
/// fn is a TDD target State 11 will add — on 3-variant code this test
/// fails to compile (preserves the failing-first signal).
#[test]
fn step_budget_suspension_idempotent_retry_safety_recognized() {
    use vb_core::action::{is_idempotent, RetrySafety};
    assert!(
        is_idempotent(RetrySafety::Idempotent),
        "Idempotent must be considered idempotent (C6)"
    );
}
