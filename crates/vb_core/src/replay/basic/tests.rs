#![forbid(unsafe_code)]
//! Tests for basic step handlers.

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

use crate::errors::CoreError;
use crate::frame::RunFrame;
use crate::ids::{
    ActionId, ConstIdx, ExprIdx, ListId, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
};
use crate::limits::MAX_EXPRESSION_STACK;
use crate::replay::{ReplayAction, ReplayError, SuspensionKind};
use crate::value::{ConstValue, SlotValue, Taint};
use crate::value_store::ValueStore;
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram, ResourceContract,
    SlotBranch, WorkflowParts, check_expr_stack_bound,
};

fn make_plan(
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
    expressions: Vec<ExprProgram>,
) -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    crate::workflow::CompiledWorkflow::try_from_parts(WorkflowParts {
        name: "test_step".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: nodes.into(),
        expressions: expressions.into(),
        accessors: vec![].into(),
        constants: constants.into(),
        slot_count: 8,
        symbols_count: 1,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|_| CoreError::InvalidCompiledWorkflow {
        reason: "test workflow validation failed",
    })
}

fn make_expr_program(ops: Vec<ExprOp>) -> Result<ExprProgram, CoreError> {
    let max_stack = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK)?;
    ExprProgram::try_from_parts(ops.into(), max_stack)
}

fn replay_err_to_core(e: ReplayError) -> CoreError {
    match e {
        ReplayError::StepNotFound { step } => CoreError::InvalidProgramCounter { step },
        ReplayError::SlotNotAvailable { slot } => CoreError::SlotOutOfBounds { slot },
        ReplayError::ExpressionEvalFailed { step } => CoreError::InvalidProgramCounter { step },
        ReplayError::NonDeterministicStep { step, .. } => CoreError::InvalidProgramCounter { step },
        ReplayError::Internal { reason } => CoreError::InternalInvariantViolation { reason },
    }
}

fn dispatch(
    node: &CompiledNode,
    run: &mut RunFrame,
    store: &mut ValueStore,
    plan: &CompiledWorkflow,
) -> Result<ReplayAction, ReplayError> {
    let mut collect_states = super::super::collect::ReplayCollectStates::new();
    super::replay_step_kind(node, run, store, plan, &mut collect_states)
}

// ---- Nop ----

#[test]
fn replay_nop_advances_to_next() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
                output: None,
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(0)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();
    run.write_slot(SlotIdx::new(0), SlotValue::I64(0))?;

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let action = dispatch(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) if next == StepIdx::new(1) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Nop should return Continue(1)",
            });
        }
    }
    if run.pc() != StepIdx::new(1) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "PC should be at step 1",
        });
    }
    if run.executed() != 1 {
        return Err(CoreError::InternalInvariantViolation {
            reason: "executed should be 1",
        });
    }
    Ok(())
}

#[test]
fn replay_nop_missing_next_returns_error() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![CompiledNode {
            id: StepIdx::new(0),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
            output: None,
            next: None,
        }],
        vec![],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let result = dispatch(node, &mut run, &mut store, &plan);
    assert!(
        matches!(
            result,
            Err(ReplayError::Internal {
                reason: "Nop node missing next step"
            })
        ),
        "Nop without next must fail"
    );
    Ok(())
}

// ---- SetConst ----

#[test]
fn replay_set_const_writes_slot() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(42)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    dispatch(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if *run.read_slot(SlotIdx::new(0))? != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 0 should be I64(42) after SetConst",
        });
    }
    Ok(())
}

#[test]
fn replay_set_const_bool() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::Bool(true)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    dispatch(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if *run.read_slot(SlotIdx::new(0))? != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 0 should be Bool(true)",
        });
    }
    Ok(())
}

#[test]
fn replay_set_const_null() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::Null],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    dispatch(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if *run.read_slot(SlotIdx::new(0))? != SlotValue::Null {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 0 should be Null",
        });
    }
    Ok(())
}

#[test]
fn replay_set_const_missing_output_returns_error() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: None,
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(1)],
        vec![],
    )?;

    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let result = dispatch(node, &mut run, &mut store, &plan);
    assert!(
        matches!(
            result,
            Err(ReplayError::Internal {
                reason: "SetConst node missing output slot"
            })
        ),
        "SetConst without output must fail"
    );
    Ok(())
}

// ---- Copy ----

#[test]
fn replay_copy_transfers_value_and_taint() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
                },
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(100)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    dispatch(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    run.write_taint(SlotIdx::new(0), Taint::Secret)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    dispatch(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if *run.read_slot(SlotIdx::new(1))? != SlotValue::I64(100) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 1 should be I64(100)",
        });
    }
    if run.read_taint(SlotIdx::new(1))? != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 1 taint should be Secret",
        });
    }
    Ok(())
}

#[test]
fn replay_copy_uninitialized_source_returns_error() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(3),
                },
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                output: None,
                next: None,
            },
        ],
        vec![],
        vec![],
    )?;

    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let result = dispatch(node, &mut run, &mut store, &plan);
    assert!(
        matches!(result, Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(3)),
        "Copy from uninitialized slot must fail with SlotNotAvailable"
    );
    Ok(())
}

#[test]
fn replay_copy_missing_output_returns_error() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
                },
                output: None,
                next: Some(StepIdx::new(2)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(1)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    dispatch(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    let result = dispatch(node1, &mut run, &mut store, &plan);
    assert!(
        matches!(
            result,
            Err(ReplayError::Internal {
                reason: "Copy node missing output slot"
            })
        ),
        "Copy without output must fail"
    );
    Ok(())
}

// ---- EvalExpr ----

#[test]
fn replay_eval_expr_computes_result() -> Result<(), CoreError> {
    let expr = make_expr_program(vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Add,
    ])?;

    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
                output: Some(SlotIdx::new(2)),
                next: Some(StepIdx::new(3)),
            },
            CompiledNode {
                id: StepIdx::new(3),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(30), ConstValue::I64(12)],
        vec![expr],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    dispatch(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    dispatch(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node2 = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 2 missing",
        })?;
    dispatch(node2, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if *run.read_slot(SlotIdx::new(2))? != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 2 should be I64(42)",
        });
    }
    Ok(())
}

// ---- BuildObject ----

#[test]
fn replay_build_object_creates_handle() -> Result<(), CoreError> {
    let field_sym = SymbolId::new(0);
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildObject {
                    fields: vec![(field_sym, SlotIdx::new(0))].into(),
                },
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(42)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    dispatch(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    dispatch(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match *run.read_slot(SlotIdx::new(1))? {
        SlotValue::Object(id) => {
            let obj = store.object(id)?;
            let field = obj.first().ok_or(CoreError::InternalInvariantViolation {
                reason: "object should have a field",
            })?;
            if field.key != field_sym || field.value != SlotValue::I64(42) {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "field mismatch",
                });
            }
        }
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "slot 1 should be Object",
            });
        }
    }
    Ok(())
}

#[test]
fn replay_build_object_empty_fields() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildObject {
                    fields: vec![].into(),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    dispatch(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match *run.read_slot(SlotIdx::new(0))? {
        SlotValue::Object(id) => {
            let obj = store.object(id)?;
            if !obj.is_empty() {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "empty BuildObject should create empty object",
                });
            }
        }
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "slot 0 should be Object",
            });
        }
    }
    Ok(())
}

#[test]
fn replay_build_object_uninitialized_field_returns_error() -> Result<(), CoreError> {
    let field_sym = SymbolId::new(0);
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildObject {
                    fields: vec![(field_sym, SlotIdx::new(5))].into(),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![],
        vec![],
    )?;

    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let result = dispatch(node, &mut run, &mut store, &plan);
    assert!(
        result.is_err(),
        "BuildObject with uninitialized field must fail"
    );
    Ok(())
}

#[test]
fn replay_build_object_propagates_taint() -> Result<(), CoreError> {
    let field_sym = SymbolId::new(0);
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildObject {
                    fields: vec![(field_sym, SlotIdx::new(0))].into(),
                },
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(1)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    dispatch(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    run.write_taint(SlotIdx::new(0), Taint::DerivedFromSecret)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    dispatch(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if run.read_taint(SlotIdx::new(1))? != Taint::DerivedFromSecret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BuildObject output taint should be DerivedFromSecret",
        });
    }
    Ok(())
}

// ---- BuildList ----

#[test]
fn replay_build_list_creates_handle() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildList {
                    items: vec![SlotIdx::new(0), SlotIdx::new(1)].into(),
                },
                output: Some(SlotIdx::new(2)),
                next: Some(StepIdx::new(3)),
            },
            CompiledNode {
                id: StepIdx::new(3),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(10), ConstValue::I64(20)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    dispatch(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    dispatch(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node2 = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 2 missing",
        })?;
    dispatch(node2, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match *run.read_slot(SlotIdx::new(2))? {
        SlotValue::List(id) => {
            let list = store.list(id)?;
            if list.len() != 2 {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "list should have 2 items",
                });
            }
            if list[0] != SlotValue::I64(10) || list[1] != SlotValue::I64(20) {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "list items mismatch",
                });
            }
        }
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "slot 2 should be List",
            });
        }
    }
    Ok(())
}

#[test]
fn replay_build_list_empty_items() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildList {
                    items: vec![].into(),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    dispatch(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match *run.read_slot(SlotIdx::new(0))? {
        SlotValue::List(id) => {
            let list = store.list(id)?;
            if !list.is_empty() {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "empty BuildList should create empty list",
                });
            }
        }
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "slot 0 should be List",
            });
        }
    }
    Ok(())
}

#[test]
fn replay_build_list_uninitialized_item_returns_error() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildList {
                    items: vec![SlotIdx::new(5)].into(),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![],
        vec![],
    )?;

    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let result = dispatch(node, &mut run, &mut store, &plan);
    assert!(
        result.is_err(),
        "BuildList with uninitialized item must fail"
    );
    Ok(())
}

#[test]
fn replay_build_list_propagates_taint() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildList {
                    items: vec![SlotIdx::new(0)].into(),
                },
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(7)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    dispatch(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    run.write_taint(SlotIdx::new(0), Taint::Secret)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    dispatch(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if run.read_taint(SlotIdx::new(1))? != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BuildList output taint should be Secret",
        });
    }
    Ok(())
}

// ---- Finish ----

#[test]
fn replay_finish_returns_finished_action() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(99)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    dispatch(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    let action = dispatch(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Finished => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Finish should return Finished",
            });
        }
    }
    if run.executed() != 2 {
        return Err(CoreError::InternalInvariantViolation {
            reason: "executed should be 2",
        });
    }
    Ok(())
}

#[test]
fn replay_finish_uninitialized_result_returns_error() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![CompiledNode {
            id: StepIdx::new(0),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(5),
            },
            output: None,
            next: None,
        }],
        vec![],
        vec![],
    )?;

    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let result = dispatch(node, &mut run, &mut store, &plan);
    assert!(
        result.is_err(),
        "Finish with uninitialized result must fail"
    );
    Ok(())
}

// ---- Jump ----

#[test]
fn replay_jump_advances_pc_to_target() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Jump {
                    target: StepIdx::new(1),
                },
                output: None,
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(0)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(0))?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let action = dispatch(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) if next == StepIdx::new(1) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Jump should return Continue(1)",
            });
        }
    }
    if run.pc() != StepIdx::new(1) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "PC should be at step 1",
        });
    }
    Ok(())
}

// ---- Suspend ----

#[test]
fn replay_do_suspends() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![CompiledNode {
            id: StepIdx::new(0),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
            output: None,
            next: None,
        }],
        vec![],
        vec![],
    )?;

    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let action = dispatch(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Suspended { step, kind }
            if step == StepIdx::new(0) && kind == SuspensionKind::ActionPending => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Do should return Suspended(0, Do)",
            });
        }
    }
    Ok(())
}

#[test]
fn replay_ask_suspends() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![CompiledNode {
            id: StepIdx::new(0),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None,
            },
            output: None,
            next: None,
        }],
        vec![],
        vec![],
    )?;

    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let action = dispatch(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Suspended { step, kind }
            if step == StepIdx::new(0) && kind == SuspensionKind::AskPending => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Ask should return Suspended(0, Ask)",
            });
        }
    }
    Ok(())
}

#[test]
fn replay_wait_until_suspends() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![CompiledNode {
            id: StepIdx::new(0),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::new(0),
            },
            output: None,
            next: None,
        }],
        vec![],
        vec![],
    )?;

    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let action = dispatch(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Suspended { step, kind }
            if step == StepIdx::new(0) && kind == SuspensionKind::WaitUntil => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "WaitUntil should return Suspended(0, WaitUntil)",
            });
        }
    }
    Ok(())
}
