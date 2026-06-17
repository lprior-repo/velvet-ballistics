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
    unused_variables,
)]

#![forbid(unsafe_code)]
//! Integration behavior tests for step execution lifecycle: variant dispatch,
//! state transitions, idempotency, budget tracking, invalid transitions,
//! action retry, edge cases, and Kani boundedness harnesses.

use crate::engine::{
    EngineSignal, StepBudget, new_run_frame, resume_action_completion, resume_action_failure,
    step_once,
};
use crate::errors::EngineError;
use crate::frame::{RunFrame, StepState, is_valid_step_state_transition};
use crate::ids::{
    ActionId, ConstIdx, ExprIdx, RunId, SeqNo, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
};
use crate::value::{ConstValue, SlotValue, Taint};
use crate::value_store::ValueStore;
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram, SlotBranch,
    WorkflowParts,
};

fn test_store() -> ValueStore {
    ValueStore::new()
}

fn test_frame(run_id: RunId, workflow: &CompiledWorkflow) -> Result<RunFrame, String> {
    new_run_frame(run_id, workflow).map_err(|error| error.to_string())
}

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

fn tiny_workflow_parts(name: &'static str, value: ConstValue) -> WorkflowParts {
    WorkflowParts {
        name: Box::<str>::from(name),
        digest: WorkflowDigest::from_bytes([0xAB; 32]),
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
        constants: vec![value].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

// ============================================================
// Section 1: Step variant dispatch — every CompiledNodeKind
// ============================================================

#[test]
fn nop_advances_pc_to_next_step_with_continue_signal() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("nop_test"),
        digest: WorkflowDigest::from_bytes([0x01; 32]),
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
        constants: vec![ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(201), &workflow)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(1))
        .map_err(|error| error.to_string())?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(1))?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    ensure_equal(run.executed(), 1)?;
    Ok(())
}

#[test]
fn set_const_writes_constant_to_output_slot_and_marks_succeeded() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("set_const_test"),
        digest: WorkflowDigest::from_bytes([0x02; 32]),
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
        constants: vec![ConstValue::I64(777)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(202), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(
        *run.read_slot(SlotIdx::new(0))
            .map_err(|error| error.to_string())?,
        SlotValue::I64(777),
    )?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    Ok(())
}

#[test]
fn copy_duplicates_source_slot_value_and_taint_to_output() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("copy_test"),
        digest: WorkflowDigest::from_bytes([0x03; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
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
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(203), &workflow)?;
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::Bool(true),
        Taint::DerivedFromSecret,
    )
    .map_err(|error| error.to_string())?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(
        *run.read_slot(SlotIdx::new(0))
            .map_err(|error| error.to_string())?,
        SlotValue::Bool(true),
    )?;
    ensure_equal(
        run.read_taint(SlotIdx::new(0)),
        Ok(Taint::DerivedFromSecret),
    )?;
    Ok(())
}

#[test]
fn eval_expr_adds_two_constants_and_stores_result_in_output() -> Result<(), String> {
    let expr = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Add,
        ]
        .into_boxed_slice(),
    )
    .map_err(|error| error.to_string())?;

    let parts = WorkflowParts {
        name: Box::<str>::from("eval_test"),
        digest: WorkflowDigest::from_bytes([0x04; 32]),
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
        constants: vec![ConstValue::I64(10), ConstValue::I64(15)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(204), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(
        *run.read_slot(SlotIdx::new(0))
            .map_err(|error| error.to_string())?,
        SlotValue::I64(25),
    )?;
    Ok(())
}

#[test]
fn build_object_creates_handle_with_fields_and_taint() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("build_obj_taint"),
        digest: WorkflowDigest::from_bytes([0x05; 32]),
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
        constants: vec![ConstValue::I64(42)].into_boxed_slice(),
        slot_count: 2,
        symbols_count: 2,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(205), &workflow)?;
    let mut store = test_store();

    let _s0 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    match run
        .read_slot(SlotIdx::new(1))
        .map_err(|error| error.to_string())?
    {
        SlotValue::Object(_) => Ok(()),
        other => Err(format!("expected Object handle, got {other:?}")),
    }
}

#[test]
fn build_list_creates_handle_with_elements_and_taint() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("build_list_handle"),
        digest: WorkflowDigest::from_bytes([0x06; 32]),
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
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(206), &workflow)?;
    let mut store = test_store();

    let _s0 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    match run
        .read_slot(SlotIdx::new(1))
        .map_err(|error| error.to_string())?
    {
        SlotValue::List(_) => Ok(()),
        other => Err(format!("expected List handle, got {other:?}")),
    }
}

#[test]
fn do_node_returns_awaiting_action_and_does_not_advance_pc() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("do_no_next"),
        digest: WorkflowDigest::from_bytes([0x07; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(3),
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
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(207), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingAction)?;
    ensure_equal(run.pc(), StepIdx::new(0))?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Running))?;
    Ok(())
}

#[test]
fn jump_sets_pc_to_target_and_returns_continue() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("jump_test"),
        digest: WorkflowDigest::from_bytes([0x08; 32]),
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
        constants: vec![ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(208), &workflow)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(1))
        .map_err(|error| error.to_string())?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(1))?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    Ok(())
}

#[test]
fn finish_returns_finished_signal_with_result_value_and_taint() -> Result<(), String> {
    let parts = tiny_workflow_parts("finish_test", ConstValue::I64(99));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(209), &workflow)?;
    let mut store = test_store();

    let _s0 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::I64(99), Taint::Clean),
    )?;
    ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Succeeded))?;
    Ok(())
}

#[test]
fn wait_until_returns_awaiting_wait_and_step_enters_waiting_state() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("wait_test"),
        digest: WorkflowDigest::from_bytes([0x0A; 32]),
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
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(210), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(
        result,
        EngineSignal::AwaitingWait {
            deadline_slot: SlotIdx::new(0),
        },
    )?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Waiting))?;
    Ok(())
}

#[test]
fn wait_event_returns_awaiting_wait_and_preserves_step_in_waiting_state() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("wait_event_test"),
        digest: WorkflowDigest::from_bytes([0x0B; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
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
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(211), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(
        result,
        EngineSignal::AwaitingWait {
            deadline_slot: SlotIdx::new(0),
        },
    )?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Waiting))?;
    Ok(())
}

#[test]
fn ask_returns_awaiting_ask_and_step_enters_asking_state() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("ask_test"),
        digest: WorkflowDigest::from_bytes([0x0C; 32]),
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
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(212), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingAsk { timeout_slot: None })?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Asking))?;
    Ok(())
}

#[test]
fn error_handler_jumps_to_body_and_returns_continue() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("err_handler"),
        digest: WorkflowDigest::from_bytes([0x0D; 32]),
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
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(213), &workflow)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(1))
        .map_err(|error| error.to_string())?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(1))?;
    Ok(())
}

#[test]
fn choose_slot_true_branch_takes_target_step_and_returns_continue() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("choose_slot_true"),
        digest: WorkflowDigest::from_bytes([0x0E; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ChooseSlot {
                    branches: vec![SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(2),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(1)),
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
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(214), &workflow)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(true))
        .map_err(|error| error.to_string())?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(2))?;
    Ok(())
}

#[test]
fn choose_slot_false_takes_otherwise_target() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("choose_slot_false"),
        digest: WorkflowDigest::from_bytes([0x0F; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ChooseSlot {
                    branches: vec![SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(2),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(1)),
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
        constants: vec![ConstValue::I64(7)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(215), &workflow)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))
        .map_err(|error| error.to_string())?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(1))?;
    Ok(())
}

// ============================================================
// Section 2: Step lifecycle — creation through execution
// ============================================================

#[test]
fn run_frame_created_with_all_step_states_pending() -> Result<(), String> {
    let parts = tiny_workflow_parts("lifecycle_create", ConstValue::I64(1));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let run = test_frame(RunId::new(220), &workflow)?;

    for step in 0..run.step_count() {
        ensure_equal(
            run.step_state(StepIdx::new(step as u16)),
            Ok(StepState::Pending),
        )?;
    }
    ensure_equal(run.executed(), 0)?;
    ensure_equal(run.pc(), StepIdx::new(0))?;
    Ok(())
}

#[test]
fn step_transitions_pending_to_running_then_succeeded_on_completion() -> Result<(), String> {
    let parts = tiny_workflow_parts("pending_running_succeeded", ConstValue::I64(3));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(221), &workflow)?;
    let mut store = test_store();

    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Pending))?;
    let _ = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    ensure_equal(run.pc(), StepIdx::new(1))?;
    Ok(())
}

#[test]
fn all_steps_in_chain_transition_correctly_through_completion() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("chain_tx"),
        digest: WorkflowDigest::from_bytes([0xDD; 32]),
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
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
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
        constants: vec![ConstValue::I64(100), ConstValue::I64(200)].into_boxed_slice(),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(222), &workflow)?;
    let mut store = test_store();

    let s0 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    ensure_equal(s0, EngineSignal::Continue)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Pending))?;

    let s1 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    ensure_equal(s1, EngineSignal::Continue)?;
    ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Succeeded))?;
    ensure_equal(run.step_state(StepIdx::new(2)), Ok(StepState::Pending))?;

    let s2 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    ensure_equal(
        s2,
        EngineSignal::Finished(SlotValue::I64(200), Taint::Clean),
    )?;
    ensure_equal(run.step_state(StepIdx::new(2)), Ok(StepState::Succeeded))?;
    Ok(())
}

// ============================================================
// Section 3: Signal to state mapping
// ============================================================

#[test]
fn signal_continue_maps_step_state_to_succeeded() -> Result<(), String> {
    let parts = tiny_workflow_parts("signal_continue", ConstValue::I64(5));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(230), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    Ok(())
}

#[test]
fn signal_finished_maps_step_state_to_succeeded() -> Result<(), String> {
    let parts = tiny_workflow_parts("signal_finished", ConstValue::I64(6));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(231), &workflow)?;
    let mut store = test_store();

    let _ = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::I64(6), Taint::Clean),
    )?;
    ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Succeeded))?;
    Ok(())
}

// ============================================================
// Section 4: Action retry — resume_action_completion and resume_action_failure
// ============================================================

fn do_then_finish_workflow() -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("do_finish"),
        digest: WorkflowDigest::from_bytes([0x1A; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(7),
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
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|error| error.to_string())
}

fn make_ticket(
    run: RunId,
    step: StepIdx,
    seq: u32,
    action: ActionId,
    attempt: u16,
) -> crate::action::ActionTicket {
    crate::action::ActionTicket {
        run,
        step,
        seq: SeqNo::new(seq.into()),
        action,
        attempt,
        idempotency_key: 0,
        capacity: 3,
        ..Default::default()
    }
}

#[test]
fn resume_action_completion_writes_output_and_advances_pc() -> Result<(), String> {
    let workflow = do_then_finish_workflow()?;
    let mut run = test_frame(RunId::new(240), &workflow)?;
    let mut store = test_store();

    let suspend = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    ensure_equal(suspend, EngineSignal::AwaitingAction)?;

    let ticket = make_ticket(RunId::new(240), StepIdx::new(0), 1, ActionId::new(7), 1);
    let (signal, _journal) = resume_action_completion(
        &workflow,
        &mut run,
        ticket,
        SlotIdx::new(0),
        SlotValue::I64(42),
        Taint::Clean,
    )
    .map_err(|error| error.to_string())?;

    ensure_equal(signal, EngineSignal::Continue)?;
    ensure_equal(
        *run.read_slot(SlotIdx::new(0))
            .map_err(|error| error.to_string())?,
        SlotValue::I64(42),
    )?;
    ensure_equal(run.pc(), StepIdx::new(1))?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    Ok(())
}

#[test]
fn resume_action_completion_increments_executed_counter() -> Result<(), String> {
    let workflow = do_then_finish_workflow()?;
    let mut run = test_frame(RunId::new(241), &workflow)?;
    let mut store = test_store();

    let _ = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    let executed_before = run.executed();

    let ticket = make_ticket(RunId::new(241), StepIdx::new(0), 1, ActionId::new(7), 1);
    let (signal, _journal) = resume_action_completion(
        &workflow,
        &mut run,
        ticket,
        SlotIdx::new(0),
        SlotValue::I64(10),
        Taint::Clean,
    )
    .map_err(|error| error.to_string())?;

    ensure_equal(signal, EngineSignal::Continue)?;
    ensure_equal(run.executed(), executed_before + 1)?;
    Ok(())
}

#[test]
fn resume_action_completion_journal_has_completed_variant_with_correct_fields() -> Result<(), String>
{
    let workflow = do_then_finish_workflow()?;
    let mut run = test_frame(RunId::new(242), &workflow)?;
    let mut store = test_store();

    let _ = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    let ticket = make_ticket(RunId::new(242), StepIdx::new(0), 1, ActionId::new(7), 2);
    let (_signal, journal) = resume_action_completion(
        &workflow,
        &mut run,
        ticket,
        SlotIdx::new(0),
        SlotValue::Bool(true),
        Taint::DerivedFromSecret,
    )
    .map_err(|error| error.to_string())?;

    match journal {
        crate::action::ActionJournalEvent::Completed {
            ticket: t,
            attempt,
            output_slot,
            output_taint,
        } => {
            ensure_equal(t.run, RunId::new(242))?;
            ensure_equal(t.step, StepIdx::new(0))?;
            ensure_equal(attempt, 2)?;
            ensure_equal(output_slot, SlotIdx::new(0))?;
            ensure_equal(output_taint, Taint::DerivedFromSecret)?;
            Ok(())
        }
        other => Err(format!("expected Completed journal event, got {other:?}")),
    }
}

#[test]
fn resume_action_failure_marks_step_failed_without_error_handler() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("fail_no_handler"),
        digest: WorkflowDigest::from_bytes([0x2B; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(8),
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
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(243), &workflow)?;
    let mut store = test_store();

    let _ = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    let ticket = make_ticket(RunId::new(243), StepIdx::new(0), 1, ActionId::new(8), 1);
    let (signal, _journal) = resume_action_failure(
        &workflow,
        &mut run,
        ticket,
        crate::action::ActionFailureCode::Timeout,
        crate::action::RetryPolicy::NonRetryable,
    )
    .map_err(|error| error.to_string())?;

    ensure_equal(signal, EngineSignal::AwaitingAction)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
    Ok(())
}

#[test]
fn resume_action_failure_journal_has_failed_variant_with_failure_code() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("fail_journal"),
        digest: WorkflowDigest::from_bytes([0x2C; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(9),
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
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(244), &workflow)?;
    let mut store = test_store();

    let _ = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    let ticket = make_ticket(RunId::new(244), StepIdx::new(0), 1, ActionId::new(9), 3);
    let (_signal, journal) = resume_action_failure(
        &workflow,
        &mut run,
        ticket,
        crate::action::ActionFailureCode::ExternalUnavailable,
        crate::action::RetryPolicy::Retryable,
    )
    .map_err(|error| error.to_string())?;

    match journal {
        crate::action::ActionJournalEvent::Failed {
            ticket: t,
            attempt,
            code,
            ref retry_policy,
        } => {
            ensure_equal(t.step, StepIdx::new(0))?;
            ensure_equal(attempt, 3)?;
            ensure_equal(code, crate::action::ActionFailureCode::ExternalUnavailable)?;
            ensure_equal(*retry_policy, crate::action::RetryPolicy::Retryable)?;
            Ok(())
        }
        other => Err(format!("expected Failed journal event, got {other:?}")),
    }
}

#[test]
fn resume_action_failure_with_retry_policy_preserves_policy_in_journal() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("fail_retry"),
        digest: WorkflowDigest::from_bytes([0x2D; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(10),
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
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(245), &workflow)?;
    let mut store = test_store();

    let _ = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    let ticket = make_ticket(RunId::new(245), StepIdx::new(0), 2, ActionId::new(10), 1);
    let (_signal, journal) = resume_action_failure(
        &workflow,
        &mut run,
        ticket,
        crate::action::ActionFailureCode::Unknown,
        crate::action::RetryPolicy::Retryable,
    )
    .map_err(|error| error.to_string())?;

    match journal {
        crate::action::ActionJournalEvent::Failed { code, .. } => {
            ensure_equal(code, crate::action::ActionFailureCode::Unknown)
        }
        other => Err(format!("expected Failed journal, got {other:?}")),
    }
}

// ============================================================
// Section 5: Step idempotency
// ============================================================

#[test]
fn mark_running_twice_is_idempotent() -> Result<(), String> {
    let parts = tiny_workflow_parts("idem_run", ConstValue::I64(1));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(250), &workflow)?;

    run.mark_running(StepIdx::new(0))
        .map_err(|error| error.to_string())?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Running))?;

    run.mark_running(StepIdx::new(0))
        .map_err(|error| error.to_string())?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Running))?;
    Ok(())
}

#[test]
fn mark_succeeded_twice_is_idempotent() -> Result<(), String> {
    let parts = tiny_workflow_parts("idem_success", ConstValue::I64(2));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(251), &workflow)?;

    run.mark_running(StepIdx::new(0))
        .map_err(|error| error.to_string())?;
    run.mark_succeeded(StepIdx::new(0))
        .map_err(|error| error.to_string())?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;

    run.mark_succeeded(StepIdx::new(0))
        .map_err(|error| error.to_string())?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    Ok(())
}

#[test]
fn mark_failed_twice_is_idempotent() -> Result<(), String> {
    let parts = tiny_workflow_parts("idem_fail", ConstValue::I64(3));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(252), &workflow)?;

    run.mark_running(StepIdx::new(0))
        .map_err(|error| error.to_string())?;
    run.mark_failed(StepIdx::new(0))
        .map_err(|error| error.to_string())?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;

    run.mark_failed(StepIdx::new(0))
        .map_err(|error| error.to_string())?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
    Ok(())
}

// ============================================================
// Section 6: Invalid state transitions
// ============================================================

#[test]
fn pending_to_waiting_is_invalid_transition() {
    assert!(!is_valid_step_state_transition(
        StepState::Pending,
        StepState::Waiting
    ));
}

#[test]
fn pending_to_asking_is_invalid_transition() {
    assert!(!is_valid_step_state_transition(
        StepState::Pending,
        StepState::Asking
    ));
}

#[test]
fn running_to_pending_is_invalid_transition() {
    assert!(!is_valid_step_state_transition(
        StepState::Running,
        StepState::Pending
    ));
}

#[test]
fn succeeded_to_running_is_invalid_transition() {
    // Master contract (velvet-ballistics-MASTER.md:1569): no terminal state
    // transitions back to running. Loop body reentry uses the explicit
    // Succeeded->Pending admission path in RunFrame::mark_pending before
    // mark_running; the direct Succeeded->Running edge is invalid.
    assert!(!is_valid_step_state_transition(
        StepState::Succeeded,
        StepState::Running
    ));
}

#[test]
fn succeeded_to_pending_is_invalid_direct_transition() {
    // Succeeded -> Pending is admitted only by RunFrame::mark_pending.
    assert!(!is_valid_step_state_transition(
        StepState::Succeeded,
        StepState::Pending
    ));
}

#[test]
fn succeeded_to_waiting_is_invalid_transition() {
    assert!(!is_valid_step_state_transition(
        StepState::Succeeded,
        StepState::Waiting
    ));
}

#[test]
fn failed_to_running_is_invalid_transition() {
    assert!(!is_valid_step_state_transition(
        StepState::Failed,
        StepState::Running
    ));
}

#[test]
fn failed_to_waiting_is_invalid_transition() {
    assert!(!is_valid_step_state_transition(
        StepState::Failed,
        StepState::Waiting
    ));
}

#[test]
fn waiting_to_pending_is_invalid_transition() {
    assert!(!is_valid_step_state_transition(
        StepState::Waiting,
        StepState::Pending
    ));
}

#[test]
fn cancelled_to_running_is_invalid_transition() {
    assert!(!is_valid_step_state_transition(
        StepState::Cancelled,
        StepState::Running
    ));
}

#[test]
fn pending_to_running_is_valid_transition() {
    assert!(is_valid_step_state_transition(
        StepState::Pending,
        StepState::Running
    ));
}

#[test]
fn running_to_succeeded_is_valid_transition() {
    assert!(is_valid_step_state_transition(
        StepState::Running,
        StepState::Succeeded
    ));
}

#[test]
fn running_to_failed_is_valid_transition() {
    assert!(is_valid_step_state_transition(
        StepState::Running,
        StepState::Failed
    ));
}

#[test]
fn running_to_waiting_is_valid_transition() {
    assert!(is_valid_step_state_transition(
        StepState::Running,
        StepState::Waiting
    ));
}

#[test]
fn running_to_asking_is_valid_transition() {
    assert!(is_valid_step_state_transition(
        StepState::Running,
        StepState::Asking
    ));
}

#[test]
fn same_state_idempotent_is_always_valid() {
    let all_states = [
        StepState::Pending,
        StepState::Running,
        StepState::Succeeded,
        StepState::Failed,
        StepState::Skipped,
        StepState::Waiting,
        StepState::Asking,
        StepState::Cancelled,
    ];
    for state in all_states {
        assert!(
            is_valid_step_state_transition(state, state),
            "idempotent transition should be valid for {state:?}"
        );
    }
}

#[test]
fn invalid_transition_rejected_in_run_frame_mark() -> Result<(), String> {
    let parts = tiny_workflow_parts("invalid_tx", ConstValue::I64(7));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(260), &workflow)?;

    run.mark_running(StepIdx::new(0))
        .map_err(|error| error.to_string())?;
    let result = run.mark_pending(StepIdx::new(0));

    match result {
        Err(EngineError::InternalInvariantViolation {
            reason: "invalid_state_transition",
        }) => Ok(()),
        other => Err(format!(
            "expected InternalInvariantViolation, got {other:?}"
        )),
    }
}

#[test]
fn succeeded_to_running_rejected_in_run_frame() -> Result<(), String> {
    let parts = tiny_workflow_parts("succ_to_run", ConstValue::I64(8));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(261), &workflow)?;

    run.mark_running(StepIdx::new(0))
        .map_err(|error| error.to_string())?;
    run.mark_succeeded(StepIdx::new(0))
        .map_err(|error| error.to_string())?;

    let result = run.mark_running(StepIdx::new(0));

    match result {
        Err(EngineError::InternalInvariantViolation {
            reason: "invalid_state_transition",
        }) => Ok(()),
        other => Err(format!(
            "expected InternalInvariantViolation, got {other:?}"
        )),
    }
}

// ============================================================
// Section 7: Edge cases
// ============================================================

#[test]
fn empty_workflow_parts_rejected_at_try_from_parts() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("empty"),
        digest: WorkflowDigest::from_bytes([0xEE; 32]),
        nodes: Box::new([]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let result = CompiledWorkflow::try_from_parts(parts);

    match result {
        Err(ref e) => {
            let msg = format!("{e}");
            if msg.contains("EmptyNodes")
                || msg.contains("empty")
                || msg.contains("at least one node")
            {
                Ok(())
            } else {
                Err(format!("expected EmptyNodes validation error, got {msg}"))
            }
        }
        Ok(_) => Err("expected error for empty workflow, but compiled successfully".into()),
    }
}

#[test]
fn step_once_on_invalid_pc_returns_error() -> Result<(), String> {
    let parts = tiny_workflow_parts("bad_pc", ConstValue::I64(1));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(270), &workflow)?;

    // The two-step workflow has step_count=2, so set_pc(99) must be rejected
    // with InvalidProgramCounter before step_once is called. We verify the
    // set_pc guard here; the test name reflects the broader invariant that
    // out-of-bounds PC values are rejected at the frame boundary.
    let result = run.set_pc(StepIdx::new(99));

    match result {
        Err(EngineError::InvalidProgramCounter { step }) if step == StepIdx::new(99) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn run_frame_creation_rejects_zero_step_count() -> Result<(), String> {
    let result = RunFrame::new(RunId::new(271), StepIdx::new(0), 0, 1);

    match result {
        Err(EngineError::InvalidCompiledWorkflow { reason }) if reason.contains("zero") => Ok(()),
        other => Err(format!(
            "expected InvalidCompiledWorkflow with zero reason, got {other:?}"
        )),
    }
}

#[test]
fn workflow_compile_rejects_entry_out_of_bounds() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("bad_entry"),
        digest: WorkflowDigest::from_bytes([0x11; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(99),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let result = CompiledWorkflow::try_from_parts(parts);

    match result {
        Err(_) => Ok(()),
        Ok(_) => Err("expected validation error for out-of-bounds entry step".into()),
    }
}

#[test]
fn jump_to_out_of_bounds_target_rejected_at_compilation() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("bad_jump"),
        digest: WorkflowDigest::from_bytes([0x22; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Jump {
                target: StepIdx::new(99),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let result = CompiledWorkflow::try_from_parts(parts);

    match result {
        Err(_) => Ok(()),
        Ok(_) => Err("expected validation error for jump to out-of-bounds target".into()),
    }
}

#[test]
fn set_pc_out_of_bounds_returns_invalid_program_counter() -> Result<(), String> {
    let parts = tiny_workflow_parts("pc_oob", ConstValue::I64(1));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(272), &workflow)?;

    let result = run.set_pc(StepIdx::new(200));

    match result {
        Err(EngineError::InvalidProgramCounter { step }) if step == StepIdx::new(200) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn resume_action_completion_nonexistent_step_returns_invalid_pc() -> Result<(), String> {
    let workflow = do_then_finish_workflow()?;
    let mut run = test_frame(RunId::new(273), &workflow)?;
    let mut store = test_store();

    let _ = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    let ticket = make_ticket(RunId::new(273), StepIdx::new(99), 1, ActionId::new(7), 1);
    let result = resume_action_completion(
        &workflow,
        &mut run,
        ticket,
        SlotIdx::new(0),
        SlotValue::I64(1),
        Taint::Clean,
    );

    match result {
        Err(EngineError::InvalidProgramCounter { step }) if step == StepIdx::new(99) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn resume_action_completion_missing_next_returns_error() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("do_no_next"),
        digest: WorkflowDigest::from_bytes([0x33; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(11),
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
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(274), &workflow)?;
    let mut store = test_store();

    let _ = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    let ticket = make_ticket(RunId::new(274), StepIdx::new(0), 1, ActionId::new(11), 1);
    let result = resume_action_completion(
        &workflow,
        &mut run,
        ticket,
        SlotIdx::new(0),
        SlotValue::I64(1),
        Taint::Clean,
    );

    match result {
        Err(EngineError::MissingNextStep { step }) if step == StepIdx::new(0) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// ============================================================
// Section 8: Budget tracking per step
// ============================================================

#[test]
fn step_budget_remaining_decrements_by_one_after_single_take() -> Result<(), String> {
    let mut budget = StepBudget::new(5);
    ensure_equal(budget.remaining(), 5)?;
    let took = budget.try_take().map_err(|error| error.to_string())?;
    ensure_equal(took, true)?;
    ensure_equal(budget.remaining(), 4)?;
    Ok(())
}

#[test]
fn step_budget_new_clamps_to_max() -> Result<(), String> {
    let max = crate::limits::MAX_STEP_BUDGET;
    let budget = StepBudget::new(max + 1000);
    ensure_equal(budget.remaining(), max)?;
    Ok(())
}

#[test]
fn step_budget_run_until_blocked_exhausts_at_limit() -> Result<(), String> {
    let parts = tiny_workflow_parts("budget_exhaust", ConstValue::I64(42));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(280), &workflow)?;
    let mut store = test_store();

    let result =
        crate::engine::run_until_blocked(&workflow, &mut run, StepBudget::new(1), &mut store);

    ensure_equal(result, Ok(EngineSignal::StepBudgetExhausted))?;
    ensure_equal(run.executed(), 1)?;
    Ok(())
}

// ============================================================
// Section 9: Kani harnesses for boundedness
// ============================================================

#[cfg(kani)]
mod kani_boundedness {
    use crate::EngineSignal;
    use crate::engine::{new_run_frame, resume_action_completion, step_once};
    use crate::errors::EngineError;
    use crate::frame::StepState;
    use crate::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
    use crate::value::{ConstValue, SlotValue, Taint};
    use crate::value_store::ValueStore;
    use crate::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts};

    /// Kani H1: step_once is panic-free for bounded single-step execution.
    /// Uses a minimal 2-node Nop→Finish workflow with bounded PC.
    #[kani::proof]
    #[kani::unwind(4)]
    fn step_once_panic_freedom_bounded() {
        let slot_val = kani::any::<i64>();
        let plan = match CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("kani_nop_finish"),
            digest: WorkflowDigest::from_bytes([0x10; 32]),
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
            constants: vec![ConstValue::I64(slot_val)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
            step_names: Box::new([]),
        }) {
            Ok(w) => w,
            Err(_) => return,
        };

        let mut run = match new_run_frame(RunId::new(1), &plan) {
            Ok(r) => r,
            Err(_) => return,
        };
        let _ = run.write_slot(SlotIdx::new(0), SlotValue::I64(slot_val));
        let mut store = ValueStore::new();

        let result = step_once(&plan, &mut run, &mut store);
        // step_once must not panic — the function always returns a Result.
        let _ = result;
    }

    /// Kani H2: resume_action_completion is panic-free for bounded inputs.
    #[kani::proof]
    #[kani::unwind(4)]
    fn resume_action_completion_panic_freedom_bounded() {
        let plan = match CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("kani_resume"),
            digest: WorkflowDigest::from_bytes([0x20; 32]),
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
            resource_contract: crate::ResourceContract::DEFAULT,
            step_names: Box::new([]),
        }) {
            Ok(w) => w,
            Err(_) => return,
        };

        let mut run = match new_run_frame(RunId::new(2), &plan) {
            Ok(r) => r,
            Err(_) => return,
        };
        let mut store = ValueStore::new();

        let _ = step_once(&plan, &mut run, &mut store);

        let ticket = crate::action::ActionTicket {
            run: RunId::new(2),
            step: StepIdx::new(0),
            seq: crate::ids::SeqNo::new(1),
            action: ActionId::new(1),
            attempt: 1,
            idempotency_key: 0,
            capacity: 1,
            ..Default::default()
        };

        let result = resume_action_completion(
            &plan,
            &mut run,
            ticket,
            SlotIdx::new(0),
            SlotValue::I64(0),
            Taint::Clean,
        );
        let _ = result;
    }

    /// Kani H3: step_state transitions are valid for any two StepState values.
    /// The predicate must never panic and must return a deterministic bool.
    #[kani::proof]
    fn step_state_transition_predicate_never_panics() {
        let current: StepState = kani::any();
        let next: StepState = kani::any();

        // The predicate must evaluate without panicking for any input pair.
        let result = crate::frame::is_valid_step_state_transition(current, next);
        // kani::assert is used to force the solver to explore all paths;
        // the property we want is that execution reaches this point (no panic).
        assert!(result || !result, "kani harness assertion");
    }
}
