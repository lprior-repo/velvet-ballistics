#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
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
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
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

//! Unit tests for the step engine.

use crate::action::{ActionFailureCode, ActionTicket};
use crate::engine::step::{
    EngineSignal, RetryPolicy, RunFrame, ValueStore, resume_action_completion,
    resume_action_failure, step_once,
};
use crate::frame::StepState;
use crate::ids::{
    ActionId, ConstIdx, ExprIdx, RunId, SeqNo, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
};
use crate::value::{ConstValue, SlotValue, Taint};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram, ResourceContract,
    WorkflowParts,
};

fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
where
    T: core::fmt::Debug + PartialEq,
{
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, found {actual:?}"))
    }
}

fn test_frame(workflow: &CompiledWorkflow) -> Result<RunFrame, String> {
    RunFrame::new(
        RunId::new(1),
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|e| e.to_string())
}

fn nop_then_finish_workflow() -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("nop_finish"),
        digest: WorkflowDigest::from_bytes([0x11; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
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
        constants: vec![ConstValue::I64(42)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

// ===== Nop dispatch =====

#[test]
fn step_once_nop_advances_pc_and_returns_continue() -> Result<(), String> {
    let workflow = nop_then_finish_workflow()?;
    let mut run = test_frame(&workflow)?;
    // Initialize slot 0 so finish succeeds later
    run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
        .map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(1))?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))
}

// ===== Finish dispatch =====

#[test]
fn step_once_finish_returns_finished_with_value_and_taint() -> Result<(), String> {
    let workflow = nop_then_finish_workflow()?;
    let mut run = test_frame(&workflow)?;
    // Advance past the Nop
    run.set_pc(StepIdx::new(1)).map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Clean)
        .map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::I64(42), Taint::Clean),
    )?;
    ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Succeeded))
}

// ===== Do node dispatch =====

#[test]
fn step_once_do_returns_awaiting_action() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("do_test"),
        digest: WorkflowDigest::from_bytes([0x22; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(5),
                input: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    ensure_equal(
        result,
        EngineSignal::AwaitingAction {
            step: StepIdx::new(0),
            seq: SeqNo::ZERO,
            action: ActionId::new(5),
        },
    )
}

// ===== WaitUntil dispatch =====

#[test]
fn step_once_wait_returns_awaiting_wait() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("wait_test"),
        digest: WorkflowDigest::from_bytes([0x33; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    ensure_equal(
        result,
        EngineSignal::AwaitingWait {
            deadline_slot: SlotIdx::new(0),
        },
    )?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Waiting))
}

// ===== Ask dispatch =====

#[test]
fn step_once_ask_returns_awaiting_ask() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("ask_test"),
        digest: WorkflowDigest::from_bytes([0x44; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None,
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingAsk { timeout_slot: None })?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Asking))
}

// ===== Jump dispatch =====

#[test]
fn step_once_jump_advances_pc_to_target() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("jump_test"),
        digest: WorkflowDigest::from_bytes([0x55; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Jump {
                    target: StepIdx::new(1),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
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
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(1))
}

// ===== EvalExpr dispatch =====

#[test]
fn step_once_eval_expr_writes_result_to_output_slot() -> Result<(), String> {
    let expr = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Add,
        ]
        .into_boxed_slice(),
    )
    .map_err(crate::WorkflowError::Expression)
    .map_err(|e| e.to_string())?;

    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("eval_step_test"),
        digest: WorkflowDigest::from_bytes([0x66; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: vec![expr].into_boxed_slice(),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(19), ConstValue::I64(23)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(
        *run.read_slot(SlotIdx::new(0)).map_err(|e| e.to_string())?,
        SlotValue::I64(42),
    )
}

// ===== BuildObject dispatch =====

#[test]
fn step_once_build_object_writes_object_to_output_slot() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("build_obj_step"),
        digest: WorkflowDigest::from_bytes([0x77; 32]),
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
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildObject {
                    fields: vec![(SymbolId::new(1), SlotIdx::new(0))].into_boxed_slice(),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(100)].into_boxed_slice(),
        slot_count: 2,
        symbols_count: 2,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();

    // Step 0: SetConst
    let s0 = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
    ensure_equal(s0, EngineSignal::Continue)?;

    // Step 1: BuildObject
    let s1 = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
    ensure_equal(s1, EngineSignal::Continue)?;

    // Verify the output slot contains an Object handle
    match *run.read_slot(SlotIdx::new(1)).map_err(|e| e.to_string())? {
        SlotValue::Object(_) => Ok(()),
        ref other => Err(format!("expected Object, got {other:?}")),
    }
}

// ===== BuildList dispatch =====

#[test]
fn step_once_build_list_writes_list_to_output_slot() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("build_list_step"),
        digest: WorkflowDigest::from_bytes([0x88; 32]),
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
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildList {
                    items: vec![SlotIdx::new(0)].into_boxed_slice(),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Bool(true)].into_boxed_slice(),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();

    // Step 0: SetConst
    let s0 = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
    ensure_equal(s0, EngineSignal::Continue)?;

    // Step 1: BuildList
    let s1 = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
    ensure_equal(s1, EngineSignal::Continue)?;

    match *run.read_slot(SlotIdx::new(1)).map_err(|e| e.to_string())? {
        SlotValue::List(_) => Ok(()),
        ref other => Err(format!("expected List, got {other:?}")),
    }
}

// ===== resume_action_completion =====

#[test]
fn resume_action_completion_writes_output_and_advances_pc() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("resume_ok"),
        digest: WorkflowDigest::from_bytes([0xA1; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(1),
                    input: SlotIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
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
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();

    // Execute the Do node to suspend
    let suspend = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
    ensure_equal(
        suspend,
        EngineSignal::AwaitingAction {
            step: StepIdx::new(0),
            seq: SeqNo::ZERO,
            action: ActionId::new(1),
        },
    )?;

    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
        ..Default::default()
    };

    let (signal, _journal) = resume_action_completion(
        &workflow,
        &mut run,
        ticket,
        SlotIdx::new(0),
        SlotValue::I64(99),
        Taint::Clean,
    )
    .map_err(|e| e.to_string())?;

    ensure_equal(signal, EngineSignal::Continue)?;
    ensure_equal(
        *run.read_slot(SlotIdx::new(0)).map_err(|e| e.to_string())?,
        SlotValue::I64(99),
    )?;
    ensure_equal(run.pc(), StepIdx::new(1))?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))
}

// ===== resume_action_failure =====

#[test]
fn resume_action_failure_marks_step_failed() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("resume_fail"),
        digest: WorkflowDigest::from_bytes([0xA2; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();

    // Execute the Do node to suspend
    let _suspend = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
        ..Default::default()
    };

    let (signal, _journal) = resume_action_failure(
        &workflow,
        &mut run,
        ticket,
        ActionFailureCode::Timeout,
        RetryPolicy::NonRetryable,
    )
    .map_err(|e| e.to_string())?;

    // No error handler, so the signal should be AwaitingAction for external handling
    ensure_equal(
        signal,
        EngineSignal::AwaitingAction {
            step: StepIdx::new(0),
            seq: SeqNo::ZERO,
            action: ActionId::new(1),
        },
    )?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))
}

// ===== journal_action_suspended =====

#[test]
fn journal_action_suspended_captures_all_fields() -> Result<(), String> {
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(5),
        attempt: 1,
        idempotency_key: 12345,
        capacity: 1,
        ..Default::default()
    };
    let event = crate::journal_action_suspended(
        ticket,
        ActionId::new(5),
        SlotIdx::new(0),
        SlotIdx::new(1),
        StepIdx::new(0),
    );

    match event {
        crate::action::ActionJournalEvent::Suspended {
            ticket: t,
            attempt,
            action,
            input_slot,
            output_slot,
            step,
        } => {
            ensure_equal(t.run, RunId::new(1))?;
            ensure_equal(attempt, 1)?;
            ensure_equal(action, ActionId::new(5))?;
            ensure_equal(input_slot, SlotIdx::new(0))?;
            ensure_equal(output_slot, SlotIdx::new(1))?;
            ensure_equal(step, StepIdx::new(0))
        }
        other => Err(format!("unexpected event: {other:?}")),
    }
}

// ===== ErrorHandler dispatch =====

#[test]
fn step_once_error_handler_jumps_to_body() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("error_handler_test"),
        digest: WorkflowDigest::from_bytes([0xB1; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ErrorHandler {
                    body: StepIdx::new(1),
                    handler: StepIdx::new(2),
                    error_slot: None,
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
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
        constants: vec![ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(&workflow)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(1))
        .map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    // ErrorHandler should jump to its body step
    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(1))
}

// ===== AwaitingAction preserves PC and keeps step in Running state =====

/// VB-POST003/INV-004: AwaitingAction signal means PC does NOT advance.
/// The step remains in Running state waiting for external action completion.
#[test]
fn step_once_awaiting_action_preserves_pc() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("await_action_preserves_pc"),
        digest: WorkflowDigest::from_bytes([0x55; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(&workflow)?;
    // Precondition: PC is at step 0
    assert_eq!(run.pc(), StepIdx::new(0), "initial PC should be 0");
    let mut store = ValueStore::new();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    // Verify AwaitingAction is returned
    ensure_equal(
        result,
        EngineSignal::AwaitingAction {
            step: StepIdx::new(0),
            seq: SeqNo::ZERO,
            action: ActionId::new(7),
        },
    )?;
    // PC must NOT advance for AwaitingAction
    ensure_equal(run.pc(), StepIdx::new(0))?;
    // Step should be in Running state (not Succeeded)
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Running))
}

// ===== Signal→State mapping verification =====

/// VB-INV-002: Verify EngineSignal→StepState mapping after step_once.
/// - Continue | Finished → Succeeded
/// - AwaitingAction | StepBudgetExhausted → Running (PC unchanged)
/// - AwaitingWait → Waiting
/// - AwaitingAsk → Asking
#[test]
fn step_once_signal_maps_to_correct_state() -> Result<(), String> {
    // Test Continue → Succeeded via Nop node
    {
        let workflow = nop_then_finish_workflow()?;
        let mut run = test_frame(&workflow)?;
        let mut store = ValueStore::new();
        let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
        ensure_equal(result, EngineSignal::Continue)?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    }

    // Test AwaitingAction → Running (PC unchanged)
    {
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("signal_map_awaiting_action"),
            digest: WorkflowDigest::from_bytes([0x66; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(1),
                    input: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())?;
        let mut run = test_frame(&workflow)?;
        let mut store = ValueStore::new();
        let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
        ensure_equal(
            result,
            EngineSignal::AwaitingAction {
                step: StepIdx::new(0),
                seq: SeqNo::ZERO,
                action: ActionId::new(1),
            },
        )?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Running))?;
    }

    // Test AwaitingWait → Waiting
    {
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("signal_map_awaiting_wait"),
            digest: WorkflowDigest::from_bytes([0x77; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::WaitUntil {
                    deadline_slot: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())?;
        let mut run = test_frame(&workflow)?;
        let mut store = ValueStore::new();
        let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
        ensure_equal(
            result,
            EngineSignal::AwaitingWait {
                deadline_slot: SlotIdx::new(0),
            },
        )?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Waiting))?;
    }

    // Test AwaitingAsk → Asking
    {
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("signal_map_awaiting_ask"),
            digest: WorkflowDigest::from_bytes([0x88; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Ask {
                    prompt: SlotIdx::new(0),
                    timeout_slot: None,
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())?;
        let mut run = test_frame(&workflow)?;
        let mut store = ValueStore::new();
        let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
        ensure_equal(result, EngineSignal::AwaitingAsk { timeout_slot: None })?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Asking))?;
    }

    Ok(())
}
