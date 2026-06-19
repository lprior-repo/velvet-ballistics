#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::borrow_deref_ref,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::cloned_ref_to_slice_refs,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::explicit_counter_loop,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::implicit_saturating_sub,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::items_after_test_module,
    clippy::iter_count,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_saturating_arithmetic,
    clippy::manual_strip,
    clippy::manual_unwrap_or,
    clippy::map_clone,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::type_complexity,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_fallible_conversions,
    clippy::unnecessary_map_or,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_sort_by,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_asref,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]
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
        coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
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
        coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
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
    use vb_core::action::{RetrySafety, is_idempotent};
    assert!(
        is_idempotent(RetrySafety::Idempotent),
        "Idempotent must be considered idempotent (C6)"
    );
}
