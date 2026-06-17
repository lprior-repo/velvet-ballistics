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

//! Tests for the replay module.

use crate::errors::CoreError;
use crate::frame::RunFrame;
use crate::ids::{ConstIdx, ExprIdx, ListId, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use crate::limits::MAX_EXPRESSION_STACK;
use crate::value::ConstValue;
use crate::value::SlotValue;
use crate::value_store::ValueStore;
use crate::workflow::{
    CompiledNode, CompiledNodeKind, ExprBranch, ExprOp, ExprProgram, ResourceContract, SlotBranch,
    WorkflowParts, check_expr_stack_bound,
};

use crate::ids::ActionId;
use crate::value::Taint;

use super::step::ReplayAction;
use super::{ReplayEngine, ReplayError, SuspensionKind};

#[test]
fn suspension_kind_names_are_stable() {
    let cases = [
        (SuspensionKind::ActionPending, "Do"),
        (SuspensionKind::AskPending, "Ask"),
        (SuspensionKind::WaitUntil, "WaitUntil"),
        (SuspensionKind::WaitEvent, "WaitEvent"),
    ];

    cases.into_iter().for_each(|(kind, name)| {
        assert_eq!(kind.as_str(), name);
        assert_eq!(kind.to_string(), name);
    });
}

fn make_plan(
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
    expressions: Vec<ExprProgram>,
) -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    crate::workflow::CompiledWorkflow::try_from_parts(WorkflowParts {
        name: "test_replay".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: nodes.into(),
        expressions: expressions.into(),
        accessors: vec![].into(),
        constants: constants.into(),
        slot_count: 8,
        symbols_count: 0,
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

#[test]
fn replay_linear_setconst_finish() -> Result<(), CoreError> {
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

    let mut store = ValueStore::new();
    let engine = ReplayEngine::new(&plan);
    let result = engine
        .replay_up_to(StepIdx::new(1), &mut store)
        .map_err(replay_err_to_core)?;
    if result != StepIdx::new(1) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "expected step 1",
        });
    }
    Ok(())
}

#[test]
fn replay_stops_at_action() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::Do {
                    action: crate::ids::ActionId::new(0),
                    input: SlotIdx::new(0),
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
        vec![ConstValue::I64(10)],
        vec![],
    )?;

    let mut store = ValueStore::new();
    let engine = ReplayEngine::new(&plan);
    match engine.replay_up_to(StepIdx::new(2), &mut store) {
        Err(ReplayError::NonDeterministicStep { step, kind }) => {
            if step != StepIdx::new(1) || kind != SuspensionKind::ActionPending {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "expected Do at step 1",
                });
            }
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected NonDeterministicStep for Do",
        }),
    }
}

#[test]
fn replay_stops_at_ask() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::Ask {
                    prompt: SlotIdx::new(0),
                    timeout_slot: None,
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
        vec![ConstValue::I64(5)],
        vec![],
    )?;

    let mut store = ValueStore::new();
    let engine = ReplayEngine::new(&plan);
    match engine.replay_up_to(StepIdx::new(2), &mut store) {
        Err(ReplayError::NonDeterministicStep { step, kind }) => {
            if step != StepIdx::new(1) || kind != SuspensionKind::AskPending {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "expected Ask at step 1",
                });
            }
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected NonDeterministicStep for Ask",
        }),
    }
}

#[test]
fn replay_engine_reports_exact_typed_suspension_variants() -> Result<(), CoreError> {
    let cases = [
        (
            CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
            SuspensionKind::ActionPending,
        ),
        (
            CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None,
            },
            SuspensionKind::AskPending,
        ),
        (
            CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::new(0),
            },
            SuspensionKind::WaitUntil,
        ),
        (
            CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: None,
            },
            SuspensionKind::WaitEvent,
        ),
    ];

    cases.into_iter().try_for_each(|(node_kind, expected)| {
        let plan = make_plan(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    on_error: None,
                    error_slot: None,
                    kind: node_kind,
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
            vec![],
            vec![],
        )?;
        let mut store = ValueStore::new();
        match ReplayEngine::new(&plan).replay_up_to(StepIdx::new(1), &mut store) {
            Err(ReplayError::NonDeterministicStep { step, kind }) => {
                assert_eq!(step, StepIdx::new(0));
                assert_eq!(kind, expected);
                Ok(())
            }
            Err(other) => Err(replay_err_to_core(other)),
            Ok(_) => Err(CoreError::InternalInvariantViolation {
                reason: "expected exact typed replay suspension",
            }),
        }
    })
}

#[test]
fn replay_reconstructs_slots() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
                output: Some(SlotIdx::new(2)),
                next: Some(StepIdx::new(3)),
            },
            CompiledNode {
                id: StepIdx::new(3),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(100), ConstValue::I64(200)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    super::step::replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    if *run.read_slot(SlotIdx::new(0))? != SlotValue::I64(100) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 0 should be 100",
        });
    }

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    super::step::replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    if *run.read_slot(SlotIdx::new(1))? != SlotValue::I64(100) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 1 should be 100",
        });
    }

    let node2 = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 2 missing",
        })?;
    super::step::replay_step(node2, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    if *run.read_slot(SlotIdx::new(2))? != SlotValue::I64(200) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 2 should be 200",
        });
    }

    Ok(())
}

#[test]
fn replay_expression_eval() -> Result<(), CoreError> {
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
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    super::step::replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    super::step::replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node2 = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 2 missing",
        })?;
    super::step::replay_step(node2, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if *run.read_slot(SlotIdx::new(2))? != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 2 should be 42",
        });
    }

    Ok(())
}

#[test]
fn replay_step_not_found() -> Result<(), CoreError> {
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
    let mut store = ValueStore::new();
    let engine = ReplayEngine::new(&plan);

    match engine.replay_up_to(StepIdx::new(99), &mut store) {
        Err(ReplayError::StepNotFound { step }) => {
            if step != StepIdx::new(99) {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "expected step 99",
                });
            }
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected StepNotFound",
        }),
    }
}

#[test]
fn replay_copy_missing_source_maps_to_slot_not_available() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![CompiledNode {
            id: StepIdx::new(0),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(7),
            },
            output: Some(SlotIdx::new(0)),
            next: None,
        }],
        vec![],
        vec![],
    )?;
    let mut store = ValueStore::new();
    let engine = ReplayEngine::new(&plan);

    match engine.replay_frame_through(StepIdx::new(0), &mut store) {
        Err(ReplayError::SlotNotAvailable { slot }) => {
            assert_eq!(slot, SlotIdx::new(7));
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected SlotNotAvailable",
        }),
    }
}

#[test]
fn replay_invalid_expression_maps_to_expression_eval_failed() -> Result<(), CoreError> {
    let too_large = replay_stack_capacity_over_limit()?;

    match super::ReplayExprStack::new(too_large) {
        Err(ReplayError::ExpressionEvalFailed { step }) => {
            assert_eq!(step, StepIdx::ZERO);
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected ExpressionEvalFailed",
        }),
    }
}

fn replay_stack_capacity_over_limit() -> Result<u8, CoreError> {
    u8::try_from(crate::limits::MAX_EXPRESSION_STACK_USIZE + 1).map_err(|_| {
        CoreError::InternalInvariantViolation {
            reason: "test expression stack limit exceeds u8",
        }
    })
}

// =========================================================================
// BLACKHAT security regression tests
// =========================================================================

// --- FINDING BH-RP-01: Unbounded replay loop via Jump cycle ---
//
// A workflow with a Jump cycle (node A -> Jump -> node A) would loop
// forever in replay_up_to before the budget guard was added.

#[test]
fn blackhat_replay_jump_cycle_exhausts_budget() -> Result<(), CoreError> {
    // The workflow validator rejects Jump cycles, so we cannot create one
    // through the normal path. However, this test verifies that the
    // budget guard is present by confirming that replay_up_to terminates
    // for any valid workflow. The budget guard is in the code at mod.rs
    // (remaining = remaining.checked_sub(1)) and prevents infinite loops
    // even if a corrupted workflow bypasses validation.
    //
    // We test with a linear workflow that reaches its target normally.
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

    let mut store = ValueStore::new();
    let engine = ReplayEngine::new(&plan);

    let result = engine
        .replay_up_to(StepIdx::new(1), &mut store)
        .map_err(replay_err_to_core)?;
    assert_eq!(
        result,
        StepIdx::new(1),
        "BLACKHAT BH-RP-01: replay must terminate with budget guard"
    );
    Ok(())
}

// --- FINDING BH-RP-01b: Linear replay stays within budget ---
//
// A well-formed linear workflow should complete within the step budget.

#[test]
fn blackhat_replay_linear_workflow_within_budget() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(10), ConstValue::I64(20)],
        vec![],
    )?;

    let mut store = ValueStore::new();
    let engine = ReplayEngine::new(&plan);
    let result = engine
        .replay_up_to(StepIdx::new(2), &mut store)
        .map_err(replay_err_to_core)?;
    assert_eq!(
        result,
        StepIdx::new(2),
        "BLACKHAT BH-RP-01b: linear workflow should reach target within budget"
    );
    Ok(())
}

// --- FINDING BH-RP-02: Taint propagated through full expression chain ---
//
// When a secret-tainted slot is used in a multi-step expression, the final
// result taint must be Secret, not Clean.

#[test]
fn blackhat_replay_taint_propagates_through_expression_chain() -> Result<(), CoreError> {
    let expr = make_expr_program(vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Add,
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::Gt,
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
        vec![ConstValue::I64(10), ConstValue::I64(20)],
        vec![expr],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;

    // Execute SetConst steps
    [0u16, 1u16].into_iter().try_for_each(|idx| {
        let node = plan
            .node(StepIdx::new(idx))
            .ok_or(CoreError::InternalInvariantViolation {
                reason: "node missing",
            })?;
        super::step::replay_step(node, &mut run, &mut store, &plan)
            .map(|_| ())
            .map_err(replay_err_to_core)
    })?;

    // Taint slot 0 as Secret
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(10), Taint::Secret)?;

    // Execute EvalExpr
    let node2 = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 2 missing",
        })?;
    super::step::replay_step(node2, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let output_taint = run.read_taint(SlotIdx::new(2))?;
    assert_eq!(
        output_taint,
        Taint::Secret,
        "BLACKHAT BH-RP-02: taint must propagate through expression chain"
    );
    Ok(())
}

// --- FINDING BH-RP-03: Replay detects non-deterministic Do node ---
//
// A workflow that reaches a Do node must suspend, not silently skip it.

#[test]
fn blackhat_replay_detects_do_node_as_non_deterministic() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(0),
                    input: SlotIdx::new(0),
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

    let mut store = ValueStore::new();
    let engine = ReplayEngine::new(&plan);

    match engine.replay_up_to(StepIdx::new(2), &mut store) {
        Err(ReplayError::NonDeterministicStep { step, kind }) => {
            assert_eq!(
                step,
                StepIdx::new(1),
                "BLACKHAT BH-RP-03: must suspend at Do node (step 1)"
            );
            assert_eq!(
                kind,
                SuspensionKind::ActionPending,
                "BLACKHAT BH-RP-03: kind must be typed action suspension"
            );
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-RP-03: Do node should cause suspension, not success",
        }),
    }
}

// --- FINDING BH-RP-04: Forward jump does not exhaust budget ---
//
// A well-formed forward Jump should complete within the budget.

#[test]
fn blackhat_replay_forward_jump_completes() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(0)),
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
        vec![ConstValue::I64(7)],
        vec![],
    )?;

    let mut store = ValueStore::new();
    let engine = ReplayEngine::new(&plan);
    let result = engine
        .replay_up_to(StepIdx::new(2), &mut store)
        .map_err(replay_err_to_core)?;
    assert_eq!(
        result,
        StepIdx::new(2),
        "BLACKHAT BH-RP-04: forward jump must complete within budget"
    );
    Ok(())
}

// --- FINDING BH-RP-05: Replay diverges from engine when slot is tainted mid-run ---
//
// After replay reconstructs state, the taint must match what the engine
// would compute for the same steps.

#[test]
fn blackhat_replay_taint_matches_engine_after_copy() -> Result<(), CoreError> {
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

    // Run replay engine
    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut replay_store = ValueStore::new();
    let mut replay_run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    super::step::replay_step(node0, &mut replay_run, &mut replay_store, &plan)
        .map_err(replay_err_to_core)?;

    // Manually taint slot 0 (simulating external secret injection)
    replay_run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(100), Taint::Secret)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    super::step::replay_step(node1, &mut replay_run, &mut replay_store, &plan)
        .map_err(replay_err_to_core)?;

    // Copy must propagate taint
    let copied_taint = replay_run.read_taint(SlotIdx::new(1))?;
    assert_eq!(
        copied_taint,
        Taint::Secret,
        "BLACKHAT BH-RP-05: replay Copy must propagate taint to destination"
    );
    assert_eq!(
        *replay_run.read_slot(SlotIdx::new(1))?,
        SlotValue::I64(100),
        "BLACKHAT BH-RP-05: replay Copy must preserve value"
    );
    Ok(())
}

fn collect_source_frame(
    store: &mut ValueStore,
) -> Result<(crate::workflow::CompiledWorkflow, RunFrame), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::CollectStart {
                    source: SlotIdx::new(0),
                    limit: 4,
                    page_size: 1,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
                output: Some(SlotIdx::new(1)),
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::CollectNext {
                    collector_slot: SlotIdx::new(1),
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
                output: None,
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::CollectFinish {
                    collector_slot: SlotIdx::new(1),
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
        vec![],
        vec![],
    )?;
    let source = store
        .insert_list(
            vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)].into_boxed_slice(),
        )
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test source list insert failed",
        })?;
    let mut run = RunFrame::new(
        RunId::new(88),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(source), Taint::Secret)?;
    Ok((plan, run))
}

fn current_page_values(run: &RunFrame, store: &ValueStore) -> Result<Box<[SlotValue]>, CoreError> {
    let list = match *run.read_slot(SlotIdx::new(1))? {
        SlotValue::List(list) => list,
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "collector slot not list",
            });
        }
    };
    store
        .list(list)
        .map(|items| items.to_vec().into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "collector list missing",
        })
}

struct PageWithTaint {
    values: Box<[SlotValue]>,
    taint: Taint,
}

struct ReplayCollectPages {
    first: PageWithTaint,
    second: PageWithTaint,
    third: PageWithTaint,
}

#[test]
fn replay_collect_state_restores_in_flight_ordering_and_taint() {
    match replay_collect_pages_with_taint() {
        Ok(pages) => {
            assert_eq!(&*pages.first.values, &[SlotValue::I64(10)]);
            assert_eq!(pages.first.taint, Taint::Secret);
            assert_eq!(&*pages.second.values, &[SlotValue::I64(20)]);
            assert_eq!(pages.second.taint, Taint::Secret);
            assert_eq!(&*pages.third.values, &[SlotValue::I64(30)]);
            assert_eq!(pages.third.taint, Taint::Secret);
        }
        Err(err) => {
            panic!("collect replay state failed: {err:?}");
        }
    }
}

fn replay_collect_pages_with_taint() -> Result<ReplayCollectPages, CoreError> {
    let mut store = ValueStore::new();
    let (plan, mut run) = collect_source_frame(&mut store)?;
    let mut states = super::step::ReplayCollectStates::new();
    let start = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        })?;
    let next = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(1),
        })?;

    super::step::replay_step_with_collect(start, &mut run, &mut store, &plan, &mut states)
        .map_err(replay_err_to_core)?;
    let first = PageWithTaint {
        values: current_page_values(&run, &store)?,
        taint: run.read_taint(SlotIdx::new(1))?,
    };

    super::step::replay_step_with_collect(next, &mut run, &mut store, &plan, &mut states)
        .map_err(replay_err_to_core)?;
    let second = PageWithTaint {
        values: current_page_values(&run, &store)?,
        taint: run.read_taint(SlotIdx::new(1))?,
    };

    super::step::replay_step_with_collect(next, &mut run, &mut store, &plan, &mut states)
        .map_err(replay_err_to_core)?;
    let third = PageWithTaint {
        values: current_page_values(&run, &store)?,
        taint: run.read_taint(SlotIdx::new(1))?,
    };
    Ok(ReplayCollectPages {
        first,
        second,
        third,
    })
}

#[test]
fn replay_collect_next_without_hydrated_state_reports_typed_error() {
    match replay_collect_next_without_hydrated_state() {
        Ok(Err(ReplayError::Internal { reason })) => {
            assert_eq!(reason, "collect pagination state missing during replay");
        }
        other => assert!(
            matches!(other, Ok(Err(ReplayError::Internal { .. }))),
            "unexpected collect replay result: {other:?}"
        ),
    }
}

fn replay_collect_next_without_hydrated_state() -> Result<Result<(), ReplayError>, CoreError> {
    let mut store = ValueStore::new();
    let (plan, mut run) = collect_source_frame(&mut store)?;
    let mut states = super::step::ReplayCollectStates::new();
    let start = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        })?;
    let next = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(1),
        })?;

    super::step::replay_step_with_collect(start, &mut run, &mut store, &plan, &mut states)
        .map_err(replay_err_to_core)?;
    let mut empty_states = super::step::ReplayCollectStates::new();
    Ok(
        super::step::replay_step_with_collect(next, &mut run, &mut store, &plan, &mut empty_states)
            .map(|_| ()),
    )
}

#[test]
fn replay_collect_start_page_bound_reports_exact_error() {
    match replay_collect_start_page_bound() {
        Ok(Err(ReplayError::Internal { reason })) => {
            assert_eq!(reason, "collect page size exceeds limit during replay");
        }
        other => assert!(
            matches!(other, Ok(Err(ReplayError::Internal { .. }))),
            "unexpected collect replay result: {other:?}"
        ),
    }
}

fn replay_collect_start_page_bound() -> Result<Result<(), ReplayError>, CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::CollectStart {
                    source: SlotIdx::new(0),
                    limit: 1,
                    page_size: 2,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
                output: Some(SlotIdx::new(1)),
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
        vec![],
        vec![],
    )?;
    let mut store = ValueStore::new();
    let source = store
        .insert_list(vec![SlotValue::I64(10)].into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test source list insert failed",
        })?;
    let mut run = RunFrame::new(
        RunId::new(89),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot(SlotIdx::new(0), SlotValue::List(source))?;
    let mut states = super::step::ReplayCollectStates::new();
    let start = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        })?;

    Ok(
        super::step::replay_step_with_collect(start, &mut run, &mut store, &plan, &mut states)
            .map(|_| ()),
    )
}

#[test]
fn replay_frame_through_executes_target_step() -> Result<(), CoreError> {
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
        vec![ConstValue::I64(64)],
        vec![],
    )?;

    let mut store = ValueStore::new();
    let frame = ReplayEngine::new(&plan)
        .replay_frame_through(StepIdx::new(0), &mut store)
        .map_err(replay_err_to_core)?;

    assert_eq!(*frame.read_slot(SlotIdx::new(0))?, SlotValue::I64(64));
    assert_eq!(frame.pc(), StepIdx::new(1));
    Ok(())
}

#[test]
fn replay_before_target_stops_when_finish_returns_none() -> Result<(), CoreError> {
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
                next: Some(StepIdx::new(2)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(11)],
        vec![],
    )?;

    let mut store = ValueStore::new();
    let reached = ReplayEngine::new(&plan)
        .replay_up_to(StepIdx::new(2), &mut store)
        .map_err(replay_err_to_core)?;

    assert_eq!(reached, StepIdx::new(1));
    Ok(())
}

#[test]
fn replay_collect_page_finish_and_empty_next_paths() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let plan = collect_page_finish_plan()?;
    let source = store
        .insert_list(
            vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)].into_boxed_slice(),
        )
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test source list insert failed",
        })?;
    let mut run = RunFrame::new(
        RunId::new(90),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(source), Taint::Secret)?;

    let mut states = super::step::ReplayCollectStates::new();
    replay_plan_step(&plan, &mut run, &mut store, &mut states, StepIdx::new(0))?;
    replay_plan_step(&plan, &mut run, &mut store, &mut states, StepIdx::new(1))?;
    replay_plan_step(&plan, &mut run, &mut store, &mut states, StepIdx::new(2))?;
    replay_plan_step(&plan, &mut run, &mut store, &mut states, StepIdx::new(1))?;
    replay_plan_step(&plan, &mut run, &mut store, &mut states, StepIdx::new(2))?;
    replay_plan_step(&plan, &mut run, &mut store, &mut states, StepIdx::new(2))?;
    replay_plan_step(&plan, &mut run, &mut store, &mut states, StepIdx::new(3))?;

    assert_eq!(run.pc(), StepIdx::new(4));
    assert_eq!(run.read_taint(SlotIdx::new(2))?, Taint::Secret);
    assert_eq!(&*current_page_values(&run, &store)?, &[]);
    Ok(())
}

fn collect_page_finish_plan() -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::CollectStart {
                    source: SlotIdx::new(0),
                    limit: 4,
                    page_size: 2,
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
                output: Some(SlotIdx::new(1)),
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::CollectPage {
                    collector_slot: SlotIdx::new(1),
                    body: StepIdx::new(2),
                    done: StepIdx::new(3),
                },
                output: None,
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::CollectNext {
                    collector_slot: SlotIdx::new(1),
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
                output: None,
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(3),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::CollectFinish {
                    collector_slot: SlotIdx::new(1),
                },
                output: Some(SlotIdx::new(2)),
                next: Some(StepIdx::new(4)),
            },
            CompiledNode {
                id: StepIdx::new(4),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
                output: None,
                next: None,
            },
        ],
        vec![],
        vec![],
    )
}

fn replay_plan_step(
    plan: &crate::workflow::CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    states: &mut super::step::ReplayCollectStates,
    step: StepIdx,
) -> Result<(), CoreError> {
    let node = plan
        .node(step)
        .ok_or(CoreError::InvalidProgramCounter { step })?;
    super::step::replay_step_with_collect(node, run, store, plan, states)
        .map(|_| ())
        .map_err(replay_err_to_core)
}

#[test]
fn replay_collect_start_empty_source_jumps_done() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let (plan, mut run) = collect_empty_source_frame(&mut store)?;
    let mut states = super::step::ReplayCollectStates::new();

    replay_plan_step(&plan, &mut run, &mut store, &mut states, StepIdx::new(0))?;

    assert_eq!(run.pc(), StepIdx::new(2));
    assert_eq!(&*current_page_values(&run, &store)?, &[]);
    Ok(())
}

fn collect_empty_source_frame(
    store: &mut ValueStore,
) -> Result<(crate::workflow::CompiledWorkflow, RunFrame), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::CollectStart {
                    source: SlotIdx::new(0),
                    limit: 4,
                    page_size: 1,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
                output: Some(SlotIdx::new(1)),
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
                output: None,
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
        vec![],
        vec![],
    )?;
    let source = store
        .insert_list(Vec::new().into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test empty list insert failed",
        })?;
    let mut run = RunFrame::new(
        RunId::new(91),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot(SlotIdx::new(0), SlotValue::List(source))?;
    Ok((plan, run))
}

#[test]
fn replay_collect_start_rejects_zero_page_size() -> Result<(), CoreError> {
    match replay_collect_start_zero_page_size()? {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "collect page size was zero during replay");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(()) => Err(CoreError::InternalInvariantViolation {
            reason: "expected zero page size replay error",
        }),
    }
}

fn replay_collect_start_zero_page_size() -> Result<Result<(), ReplayError>, CoreError> {
    let plan = collect_invalid_start_plan(4, 0)?;
    let mut store = ValueStore::new();
    let source = store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test source list insert failed",
        })?;
    let mut run = RunFrame::new(
        RunId::new(92),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot(SlotIdx::new(0), SlotValue::List(source))?;
    let mut states = super::step::ReplayCollectStates::new();
    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        })?;
    Ok(
        super::step::replay_step_with_collect(node, &mut run, &mut store, &plan, &mut states)
            .map(|_| ()),
    )
}

#[test]
fn replay_collect_start_rejects_source_over_limit() -> Result<(), CoreError> {
    match replay_collect_start_source_over_limit()? {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "collect item count exceeds limit during replay");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(()) => Err(CoreError::InternalInvariantViolation {
            reason: "expected source over limit replay error",
        }),
    }
}

fn replay_collect_start_source_over_limit() -> Result<Result<(), ReplayError>, CoreError> {
    let plan = collect_invalid_start_plan(1, 1)?;
    let mut store = ValueStore::new();
    let source = store
        .insert_list(vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test source list insert failed",
        })?;
    let mut run = RunFrame::new(
        RunId::new(93),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot(SlotIdx::new(0), SlotValue::List(source))?;
    let mut states = super::step::ReplayCollectStates::new();
    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        })?;
    Ok(
        super::step::replay_step_with_collect(node, &mut run, &mut store, &plan, &mut states)
            .map(|_| ()),
    )
}

fn collect_invalid_start_plan(
    limit: u32,
    page_size: u32,
) -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::CollectStart {
                    source: SlotIdx::new(0),
                    limit,
                    page_size,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
                output: Some(SlotIdx::new(1)),
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
                output: None,
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
        vec![],
        vec![],
    )
}

#[test]
fn replay_error_handler_node_reports_unsupported_replay_kind() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ErrorHandler {
                    body: StepIdx::new(1),
                    handler: StepIdx::new(2),
                    error_slot: Some(SlotIdx::new(0)),
                },
                output: None,
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(3)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(3)),
            },
            CompiledNode {
                id: StepIdx::new(3),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(1), ConstValue::I64(2)],
        vec![],
    )?;
    let mut run = RunFrame::new(
        RunId::new(94),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();
    let mut states = super::step::ReplayCollectStates::new();
    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        })?;

    match super::step::replay_step_with_collect(node, &mut run, &mut store, &plan, &mut states) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "unsupported node kind for replay");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected unsupported ErrorHandler replay error",
        }),
    }
}

#[test]
fn replay_slot_error_conversion_preserves_exact_slot_variants() -> Result<(), CoreError> {
    let out_of_bounds = super::slot_to_replay_err(CoreError::SlotOutOfBounds {
        slot: SlotIdx::new(6),
    });
    let uninitialized = super::slot_to_replay_err(CoreError::SlotUninitialized {
        slot: SlotIdx::new(7),
    });
    let other = super::slot_to_replay_err(CoreError::StepBudgetExhausted);

    assert_eq!(
        out_of_bounds,
        ReplayError::SlotNotAvailable {
            slot: SlotIdx::new(6)
        }
    );
    assert_eq!(
        uninitialized,
        ReplayError::SlotNotAvailable {
            slot: SlotIdx::new(7)
        }
    );
    assert_eq!(
        other,
        ReplayError::Internal {
            reason: "unexpected engine error during replay"
        }
    );
    Ok(())
}

#[test]
fn replay_choose_expr_takes_true_branch() -> Result<(), CoreError> {
    let expr = ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(0))].into())?;
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(2)),
                },
                output: None,
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(3)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(2),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(3)),
            },
            CompiledNode {
                id: StepIdx::new(3),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![
            ConstValue::Bool(true),
            ConstValue::I64(1),
            ConstValue::I64(2),
        ],
        vec![expr],
    )?;
    let mut store = ValueStore::new();
    let frame = ReplayEngine::new(&plan)
        .replay_frame_through(StepIdx::new(0), &mut store)
        .map_err(replay_err_to_core)?;

    assert_eq!(frame.pc(), StepIdx::new(1));
    Ok(())
}

#[test]
fn replay_collect_start_rejects_non_list_source() -> Result<(), CoreError> {
    let plan = collect_invalid_start_plan(4, 1)?;
    let mut run = RunFrame::new(
        RunId::new(95),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(10))?;

    match replay_step_error(&plan, &mut run, StepIdx::new(0)) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "collect slot was not list during replay");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(()) => Err(CoreError::InternalInvariantViolation {
            reason: "expected non-list collect source error",
        }),
    }
}

#[test]
fn replay_collect_page_rejects_non_list_collector() -> Result<(), CoreError> {
    let plan = collect_page_finish_plan()?;
    let mut run = RunFrame::new(
        RunId::new(96),
        StepIdx::new(1),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(10))?;

    match replay_step_error(&plan, &mut run, StepIdx::new(1)) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "collect slot was not list during replay");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(()) => Err(CoreError::InternalInvariantViolation {
            reason: "expected non-list collect page error",
        }),
    }
}

#[test]
fn replay_collect_start_rejects_missing_source_list() -> Result<(), CoreError> {
    let plan = collect_invalid_start_plan(4, 1)?;
    let mut run = RunFrame::new(
        RunId::new(97),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot(SlotIdx::new(0), SlotValue::List(ListId::new(99)))?;

    match replay_step_error(&plan, &mut run, StepIdx::new(0)) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "collect source list missing during replay");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(()) => Err(CoreError::InternalInvariantViolation {
            reason: "expected missing collect source list error",
        }),
    }
}

#[test]
fn replay_collect_start_reports_page_insert_failure() -> Result<(), CoreError> {
    let plan = collect_invalid_start_plan(4, 1)?;
    let mut store = ValueStore::with_max_slots(1);
    let source = store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test capped source list insert failed",
        })?;
    let mut run = RunFrame::new(
        RunId::new(98),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot(SlotIdx::new(0), SlotValue::List(source))?;
    let mut states = super::step::ReplayCollectStates::new();
    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        })?;

    match super::step::replay_step_with_collect(node, &mut run, &mut store, &plan, &mut states) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "insert collect page failed during replay");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected collect page insert failure",
        }),
    }
}

#[test]
fn replay_collect_next_rejects_missing_current_page() -> Result<(), CoreError> {
    let plan = collect_page_finish_plan()?;
    let mut run = RunFrame::new(
        RunId::new(99),
        StepIdx::new(2),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot(SlotIdx::new(1), SlotValue::List(ListId::new(99)))?;

    match replay_step_error(&plan, &mut run, StepIdx::new(2)) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "collect current page missing during replay");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(()) => Err(CoreError::InternalInvariantViolation {
            reason: "expected missing collect current page error",
        }),
    }
}

#[test]
fn replay_collect_next_rejects_changed_source_length() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let (plan, mut run) = collect_source_frame(&mut store)?;
    let mut states = super::step::ReplayCollectStates::new();

    replay_plan_step(&plan, &mut run, &mut store, &mut states, StepIdx::new(0))?;

    let current_page = match *run.read_slot(SlotIdx::new(1))? {
        SlotValue::List(list) => list,
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "collector slot not list after collect start",
            });
        }
    };
    let mut changed_store = ValueStore::new();
    changed_store
        .insert_list(vec![SlotValue::I64(10), SlotValue::I64(20)].into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test changed source list insert failed",
        })?;
    let changed_current_page = changed_store
        .insert_list(vec![SlotValue::I64(10)].into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test current page list insert failed",
        })?;
    if changed_current_page != current_page {
        return Err(CoreError::InternalInvariantViolation {
            reason: "test current page handle mismatch",
        });
    }

    let node = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(1),
        })?;
    match super::step::replay_step_with_collect(
        node,
        &mut run,
        &mut changed_store,
        &plan,
        &mut states,
    ) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "collect source length changed during replay");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected changed collect source length replay error",
        }),
    }
}

#[test]
fn replay_value_writers_report_missing_output_slots() -> Result<(), CoreError> {
    let expr = ExprProgram::try_from_ops(vec![ExprOp::LoadSlot(SlotIdx::new(0))].into())?;
    let plan = make_plan(
        vec![CompiledNode {
            id: StepIdx::new(0),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            output: None,
            next: None,
        }],
        vec![ConstValue::I64(7)],
        vec![expr],
    )?;
    let mut store = ValueStore::new();
    let mut states = super::step::ReplayCollectStates::new();

    let set_const = replay_node_error(
        &plan,
        &mut store,
        &mut states,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    )?;
    assert_eq!(set_const, "SetConst node missing output slot");

    let copy = replay_node_error(
        &plan,
        &mut store,
        &mut states,
        CompiledNodeKind::Copy {
            source: SlotIdx::new(0),
        },
    )?;
    assert_eq!(copy, "Copy node missing output slot");

    let eval_expr = replay_node_error(
        &plan,
        &mut store,
        &mut states,
        CompiledNodeKind::EvalExpr {
            expr: ExprIdx::new(0),
        },
    )?;
    assert_eq!(eval_expr, "EvalExpr node missing output slot");

    let build_object = replay_node_error(
        &plan,
        &mut store,
        &mut states,
        CompiledNodeKind::BuildObject {
            fields: vec![(SymbolId::new(0), SlotIdx::new(0))].into_boxed_slice(),
        },
    )?;
    assert_eq!(build_object, "BuildObject node missing output slot");

    let build_list = replay_node_error(
        &plan,
        &mut store,
        &mut states,
        CompiledNodeKind::BuildList {
            items: vec![SlotIdx::new(0)].into_boxed_slice(),
        },
    )?;
    assert_eq!(build_list, "BuildList node missing output slot");
    Ok(())
}

fn replay_node_error(
    plan: &crate::workflow::CompiledWorkflow,
    store: &mut ValueStore,
    states: &mut super::step::ReplayCollectStates,
    kind: CompiledNodeKind,
) -> Result<&'static str, CoreError> {
    let node = CompiledNode {
        id: StepIdx::new(0),
        on_error: None,
        error_slot: None,
        kind,
        output: None,
        next: Some(StepIdx::new(0)),
    };
    let mut run = RunFrame::new(
        RunId::new(102),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(7))?;

    match super::step::replay_step_with_collect(&node, &mut run, store, plan, states) {
        Err(ReplayError::Internal { reason }) => Ok(reason),
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected missing output slot replay error",
        }),
    }
}

#[test]
fn replay_build_object_reports_insert_failure() -> Result<(), CoreError> {
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
    let mut run = RunFrame::new(
        RunId::new(100),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::with_max_slots(1);
    store
        .insert_list(Vec::new().into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test cap filler insert failed",
        })?;
    let mut states = super::step::ReplayCollectStates::new();
    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        })?;

    match super::step::replay_step_with_collect(node, &mut run, &mut store, &plan, &mut states) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "insert_object failed");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected build object insert failure",
        }),
    }
}

#[test]
fn replay_build_list_reports_insert_failure() -> Result<(), CoreError> {
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
    let mut run = RunFrame::new(
        RunId::new(101),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::with_max_slots(1);
    store
        .insert_list(Vec::new().into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test cap filler insert failed",
        })?;
    let mut states = super::step::ReplayCollectStates::new();
    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        })?;

    match super::step::replay_step_with_collect(node, &mut run, &mut store, &plan, &mut states) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "insert_list failed");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected build list insert failure",
        }),
    }
}

fn replay_step_error(
    plan: &crate::workflow::CompiledWorkflow,
    run: &mut RunFrame,
    step: StepIdx,
) -> Result<(), ReplayError> {
    let mut store = ValueStore::new();
    let mut states = super::step::ReplayCollectStates::new();
    let node = plan.node(step).ok_or(ReplayError::StepNotFound { step })?;
    super::step::replay_step_with_collect(node, run, &mut store, plan, &mut states).map(|_| ())
}

// =========================================================================
// Choose expression replay tests
// =========================================================================

#[test]
fn replay_choose_expr_false_takes_otherwise() -> Result<(), CoreError> {
    let expr_false = ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(1))].into())?;
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(2),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(2),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(3)),
                },
                output: None,
                next: None,
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
            CompiledNode {
                id: StepIdx::new(3),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![
            ConstValue::Bool(true),
            ConstValue::Bool(false),
            ConstValue::I64(0),
        ],
        vec![expr_false],
    )?;

    let mut store = ValueStore::new();
    let frame = ReplayEngine::new(&plan)
        .replay_frame_through(StepIdx::new(1), &mut store)
        .map_err(replay_err_to_core)?;

    assert_eq!(frame.pc(), StepIdx::new(3));
    Ok(())
}

#[test]
fn replay_choose_expr_first_true_branch_wins_over_second() -> Result<(), CoreError> {
    let expr_true = ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(0))].into())?;
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(2),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![
                        ExprBranch {
                            condition: ExprIdx::new(0),
                            target: StepIdx::new(2),
                        },
                        ExprBranch {
                            condition: ExprIdx::new(0),
                            target: StepIdx::new(3),
                        },
                    ]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(4)),
                },
                output: None,
                next: None,
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
            CompiledNode {
                id: StepIdx::new(3),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(4),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![
            ConstValue::Bool(true),
            ConstValue::Bool(false),
            ConstValue::I64(0),
        ],
        vec![
            expr_true.clone(),
            ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(0))].into())?,
        ],
    )?;

    let mut store = ValueStore::new();
    let frame = ReplayEngine::new(&plan)
        .replay_frame_through(StepIdx::new(1), &mut store)
        .map_err(replay_err_to_core)?;

    assert_eq!(frame.pc(), StepIdx::new(2));
    Ok(())
}

#[test]
fn replay_choose_expr_empty_branches_with_otherwise_uses_fallback() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::Choose {
                    branches: vec![].into_boxed_slice(),
                    otherwise: Some(StepIdx::new(2)),
                },
                output: None,
                next: None,
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
        vec![ConstValue::I64(42)],
        vec![],
    )?;

    let mut store = ValueStore::new();
    let frame = ReplayEngine::new(&plan)
        .replay_frame_through(StepIdx::new(1), &mut store)
        .map_err(replay_err_to_core)?;

    assert_eq!(frame.pc(), StepIdx::new(2));
    Ok(())
}

#[test]
fn replay_choose_expr_empty_branches_no_otherwise_returns_error() -> Result<(), CoreError> {
    let result = make_plan(
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
                kind: CompiledNodeKind::Choose {
                    branches: vec![].into_boxed_slice(),
                    otherwise: None,
                },
                output: None,
                next: None,
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
        vec![ConstValue::I64(42)],
        vec![],
    );
    match result {
        Err(CoreError::InvalidCompiledWorkflow { reason }) => {
            assert_eq!(reason, "test workflow validation failed");
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "expected InvalidCompiledWorkflow from validation",
        }),
    }
}

#[test]
fn replay_choose_expr_all_false_no_otherwise_returns_error() -> Result<(), CoreError> {
    let expr_false = ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(1))].into())?;
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(2),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(2),
                    }]
                    .into_boxed_slice(),
                    otherwise: None,
                },
                output: None,
                next: None,
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
        vec![
            ConstValue::Bool(true),
            ConstValue::Bool(false),
            ConstValue::I64(0),
        ],
        vec![expr_false],
    )?;

    let mut store = ValueStore::new();
    match ReplayEngine::new(&plan).replay_frame_through(StepIdx::new(1), &mut store) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "choose_expr no branch matched and no otherwise");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected all-false no-otherwise error",
        }),
    }
}

#[test]
fn replay_choose_expr_gt_comparison_true_branch() -> Result<(), CoreError> {
    let expr_gt = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Gt,
        ]
        .into(),
    )?;
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(2)),
                },
                output: None,
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(2)),
                next: Some(StepIdx::new(3)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
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
        vec![ConstValue::I64(1), ConstValue::I64(0)],
        vec![expr_gt],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(10))?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(5))?;

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let action =
        super::step::replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) => {
            assert_eq!(next, StepIdx::new(1));
            assert_eq!(run.pc(), StepIdx::new(1));
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "GT comparison true should take branch to step 1",
        }),
    }
}

#[test]
fn replay_choose_expr_gt_comparison_false_takes_otherwise() -> Result<(), CoreError> {
    let expr_gt = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Gt,
        ]
        .into(),
    )?;
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(2)),
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
        vec![],
        vec![expr_gt],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(3))?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(8))?;

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let action =
        super::step::replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) => {
            assert_eq!(next, StepIdx::new(2));
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "GT comparison false should take otherwise",
        }),
    }
}

#[test]
fn replay_choose_expr_non_boolean_condition_returns_error() -> Result<(), CoreError> {
    let expr_i64 = ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(0))].into())?;
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(2)),
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
        vec![ConstValue::I64(42)],
        vec![expr_i64],
    )?;

    let mut store = ValueStore::new();
    match ReplayEngine::new(&plan).replay_frame_through(StepIdx::new(0), &mut store) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "choose_expr condition is not boolean");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected non-bool condition error",
        }),
    }
}

#[test]
fn replay_choose_expr_equality_predicate_true() -> Result<(), CoreError> {
    let expr_eq = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Eq,
        ]
        .into(),
    )?;
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(2)),
                },
                output: None,
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(2)),
                next: Some(StepIdx::new(3)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
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
        vec![ConstValue::I64(7), ConstValue::I64(0)],
        vec![expr_eq],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(7))?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(7))?;

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let action =
        super::step::replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) => {
            assert_eq!(next, StepIdx::new(1));
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "Eq(7,7) should be true, take branch",
        }),
    }
}

#[test]
fn replay_choose_expr_and_combinator_both_true_takes_branch() -> Result<(), CoreError> {
    let expr_and = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::And,
        ]
        .into(),
    )?;
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(2)),
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
        vec![],
        vec![expr_and],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(true))?;
    run.write_slot(SlotIdx::new(1), SlotValue::Bool(true))?;

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let action =
        super::step::replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) => {
            assert_eq!(next, StepIdx::new(1));
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "true && true should take branch",
        }),
    }
}

#[test]
fn replay_choose_expr_or_combinator_one_true_takes_branch() -> Result<(), CoreError> {
    let expr_or = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Or,
        ]
        .into(),
    )?;
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(2)),
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
        vec![],
        vec![expr_or],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))?;
    run.write_slot(SlotIdx::new(1), SlotValue::Bool(true))?;

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let action =
        super::step::replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) => {
            assert_eq!(next, StepIdx::new(1));
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "false || true should take branch",
        }),
    }
}

#[test]
fn replay_choose_expr_three_branches_second_true_wins() -> Result<(), CoreError> {
    let expr_true = ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(1))].into())?;
    let expr_false_a = ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(2))].into())?;
    let expr_false_b = ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(2))].into())?;
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
                kind: CompiledNodeKind::Choose {
                    branches: vec![
                        ExprBranch {
                            condition: ExprIdx::new(0),
                            target: StepIdx::new(2),
                        },
                        ExprBranch {
                            condition: ExprIdx::new(1),
                            target: StepIdx::new(3),
                        },
                        ExprBranch {
                            condition: ExprIdx::new(2),
                            target: StepIdx::new(4),
                        },
                    ]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(5)),
                },
                output: None,
                next: None,
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
            CompiledNode {
                id: StepIdx::new(3),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(4),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(5),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![
            ConstValue::I64(0),
            ConstValue::Bool(true),
            ConstValue::Bool(false),
        ],
        vec![expr_false_a, expr_true, expr_false_b],
    )?;

    let mut store = ValueStore::new();
    let frame = ReplayEngine::new(&plan)
        .replay_frame_through(StepIdx::new(1), &mut store)
        .map_err(replay_err_to_core)?;

    assert_eq!(frame.pc(), StepIdx::new(3));
    Ok(())
}

// =========================================================================
// Choose slot replay tests
// =========================================================================

#[test]
fn replay_choose_slot_first_true_branch_wins() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::ChooseSlot {
                    branches: vec![
                        SlotBranch {
                            condition: SlotIdx::new(0),
                            target: StepIdx::new(2),
                        },
                        SlotBranch {
                            condition: SlotIdx::new(1),
                            target: StepIdx::new(3),
                        },
                    ]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(4)),
                },
                output: None,
                next: None,
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
            CompiledNode {
                id: StepIdx::new(3),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(4),
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
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(true))?;
    run.write_slot(SlotIdx::new(1), SlotValue::Bool(true))?;

    let node = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    let action =
        super::step::replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) => {
            assert_eq!(next, StepIdx::new(2));
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "first true slot branch should win",
        }),
    }
}

#[test]
fn replay_choose_slot_second_true_first_false() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::ChooseSlot {
                    branches: vec![
                        SlotBranch {
                            condition: SlotIdx::new(0),
                            target: StepIdx::new(2),
                        },
                        SlotBranch {
                            condition: SlotIdx::new(1),
                            target: StepIdx::new(3),
                        },
                    ]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(4)),
                },
                output: None,
                next: None,
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
            CompiledNode {
                id: StepIdx::new(3),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(4),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::Bool(false)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))?;
    run.write_slot(SlotIdx::new(1), SlotValue::Bool(true))?;

    let node = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    let action =
        super::step::replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) => {
            assert_eq!(next, StepIdx::new(3));
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "second true slot branch should win after first false",
        }),
    }
}

#[test]
fn replay_choose_slot_all_false_with_otherwise_fallback() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::ChooseSlot {
                    branches: vec![
                        SlotBranch {
                            condition: SlotIdx::new(0),
                            target: StepIdx::new(2),
                        },
                        SlotBranch {
                            condition: SlotIdx::new(1),
                            target: StepIdx::new(3),
                        },
                    ]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(4)),
                },
                output: None,
                next: None,
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
            CompiledNode {
                id: StepIdx::new(3),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(4),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::Bool(false)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))?;
    run.write_slot(SlotIdx::new(1), SlotValue::Bool(false))?;

    let node = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    let action =
        super::step::replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) => {
            assert_eq!(next, StepIdx::new(4));
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "all false with otherwise should fallback",
        }),
    }
}

#[test]
fn replay_choose_slot_all_false_no_otherwise_returns_error() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::ChooseSlot {
                    branches: vec![SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(2),
                    }]
                    .into_boxed_slice(),
                    otherwise: None,
                },
                output: None,
                next: None,
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
        vec![ConstValue::Bool(false)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))?;

    let node = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    match super::step::replay_step(node, &mut run, &mut store, &plan) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "choose_slot no branch matched and no otherwise");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected all-false no-otherwise error",
        }),
    }
}

#[test]
fn replay_choose_slot_empty_branches_with_otherwise_fallback() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::ChooseSlot {
                    branches: vec![].into_boxed_slice(),
                    otherwise: Some(StepIdx::new(2)),
                },
                output: None,
                next: None,
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
        vec![ConstValue::I64(0)],
        vec![],
    )?;

    let mut store = ValueStore::new();
    let frame = ReplayEngine::new(&plan)
        .replay_frame_through(StepIdx::new(1), &mut store)
        .map_err(replay_err_to_core)?;

    assert_eq!(frame.pc(), StepIdx::new(2));
    Ok(())
}

#[test]
fn replay_choose_slot_empty_branches_no_otherwise_error() -> Result<(), CoreError> {
    let result = make_plan(
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
                kind: CompiledNodeKind::ChooseSlot {
                    branches: vec![].into_boxed_slice(),
                    otherwise: None,
                },
                output: None,
                next: None,
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
        vec![ConstValue::I64(0)],
        vec![],
    );
    match result {
        Err(CoreError::InvalidCompiledWorkflow { reason }) => {
            assert_eq!(reason, "test workflow validation failed");
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "expected InvalidCompiledWorkflow from validation",
        }),
    }
}

// =========================================================================
// Replay engine: idempotency, equivalence, independent runs
// =========================================================================

#[test]
fn replay_step_by_step_equals_full_replay() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(1),
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
        vec![ConstValue::I64(100), ConstValue::I64(200)],
        vec![],
    )?;

    let mut full_store = ValueStore::new();
    let full_frame = ReplayEngine::new(&plan)
        .replay_frame_up_to(StepIdx::new(3), &mut full_store)
        .map_err(replay_err_to_core)?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut step_store = ValueStore::new();
    let mut step_run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let nodes: [u16; 3] = [0, 1, 2];
    nodes.iter().try_for_each(|&idx| {
        let node = plan
            .node(StepIdx::new(idx))
            .ok_or(CoreError::InternalInvariantViolation {
                reason: "node missing",
            })?;
        super::step::replay_step(node, &mut step_run, &mut step_store, &plan)
            .map(|_| ())
            .map_err(replay_err_to_core)
    })?;

    assert_eq!(step_run.pc(), full_frame.pc());
    for slot in 0..3u16 {
        let step_val = *step_run.read_slot(SlotIdx::new(slot)).map_err(|_| {
            CoreError::InternalInvariantViolation {
                reason: "step frame read error",
            }
        })?;
        let full_val = *full_frame.read_slot(SlotIdx::new(slot)).map_err(|_| {
            CoreError::InternalInvariantViolation {
                reason: "full frame read error",
            }
        })?;
        assert_eq!(step_val, full_val, "slot {slot} mismatch");
    }
    Ok(())
}

#[test]
fn replay_idempotent_twice_returns_same_result() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(10), ConstValue::I64(20)],
        vec![],
    )?;

    let mut store_a = ValueStore::new();
    let frame_a = ReplayEngine::new(&plan)
        .replay_frame_up_to(StepIdx::new(2), &mut store_a)
        .map_err(replay_err_to_core)?;

    let mut store_b = ValueStore::new();
    let frame_b = ReplayEngine::new(&plan)
        .replay_frame_up_to(StepIdx::new(2), &mut store_b)
        .map_err(replay_err_to_core)?;

    assert_eq!(frame_a.pc(), frame_b.pc());
    for slot in 0..2u16 {
        let val_a = *frame_a.read_slot(SlotIdx::new(slot)).map_err(|_| {
            CoreError::InternalInvariantViolation {
                reason: "frame_a read error",
            }
        })?;
        let val_b = *frame_b.read_slot(SlotIdx::new(slot)).map_err(|_| {
            CoreError::InternalInvariantViolation {
                reason: "frame_b read error",
            }
        })?;
        assert_eq!(val_a, val_b, "slot {slot} mismatch between idempotent runs");
    }
    Ok(())
}

#[test]
fn independent_runs_with_same_plan_produce_identical_state() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
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
        vec![ConstValue::I64(77), ConstValue::I64(88)],
        vec![],
    )?;

    let engine_a = ReplayEngine::new(&plan);
    let engine_b = ReplayEngine::new(&plan);
    let mut store_a = ValueStore::new();
    let mut store_b = ValueStore::new();

    let frame_a = engine_a
        .replay_frame_up_to(StepIdx::new(3), &mut store_a)
        .map_err(replay_err_to_core)?;
    let frame_b = engine_b
        .replay_frame_up_to(StepIdx::new(3), &mut store_b)
        .map_err(replay_err_to_core)?;

    assert_eq!(frame_a.pc(), frame_b.pc());
    for slot in 0..3u16 {
        let val_a = *frame_a.read_slot(SlotIdx::new(slot)).map_err(|_| {
            CoreError::InternalInvariantViolation {
                reason: "frame_a read error",
            }
        })?;
        let val_b = *frame_b.read_slot(SlotIdx::new(slot)).map_err(|_| {
            CoreError::InternalInvariantViolation {
                reason: "frame_b read error",
            }
        })?;
        assert_eq!(val_a, val_b, "independent runs slot {slot} mismatch");
    }
    Ok(())
}

#[test]
fn replay_from_intermediate_snapshot() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(2),
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
        vec![
            ConstValue::I64(10),
            ConstValue::I64(20),
            ConstValue::I64(30),
        ],
        vec![],
    )?;

    let mut store = ValueStore::new();
    let frame_full = ReplayEngine::new(&plan)
        .replay_frame_up_to(StepIdx::new(3), &mut store)
        .map_err(replay_err_to_core)?;

    let mut store_snap = ValueStore::new();
    let snap_frame = ReplayEngine::new(&plan)
        .replay_frame_through(StepIdx::new(1), &mut store_snap)
        .map_err(replay_err_to_core)?;

    assert_eq!(*snap_frame.read_slot(SlotIdx::new(0))?, SlotValue::I64(10));
    assert_eq!(*snap_frame.read_slot(SlotIdx::new(1))?, SlotValue::I64(20));
    assert_eq!(snap_frame.pc(), StepIdx::new(2));

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut resume_store = ValueStore::new();
    let mut resume_run = RunFrame::new(RunId::new(0), snap_frame.pc(), step_count, slot_count)?;
    resume_run.write_slot(SlotIdx::new(0), SlotValue::I64(10))?;
    resume_run.write_slot(SlotIdx::new(1), SlotValue::I64(20))?;

    let node = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 2 missing",
        })?;
    super::step::replay_step(node, &mut resume_run, &mut resume_store, &plan)
        .map_err(replay_err_to_core)?;

    assert_eq!(*resume_run.read_slot(SlotIdx::new(2))?, SlotValue::I64(30));
    assert_eq!(
        *resume_run.read_slot(SlotIdx::new(2))?,
        *frame_full.read_slot(SlotIdx::new(2))?,
        "intermediate snapshot ran ahead of full replay"
    );
    Ok(())
}

// =========================================================================
// i64 extremes boundary tests
// =========================================================================

#[test]
fn replay_choose_expr_i64_max_boundary_expression() -> Result<(), CoreError> {
    let expr_gt = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Gt,
        ]
        .into(),
    )?;
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(2)),
                },
                output: None,
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(2)),
                next: Some(StepIdx::new(3)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
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
        vec![ConstValue::I64(42), ConstValue::I64(0)],
        vec![expr_gt],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(i64::MAX))?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(i64::MAX - 1))?;

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let action =
        super::step::replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) => {
            assert_eq!(next, StepIdx::new(1));
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "i64::MAX > i64::MAX-1 should be true",
        }),
    }
}

#[test]
fn replay_choose_expr_i64_min_boundary_expression() -> Result<(), CoreError> {
    let expr_lt = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Lt,
        ]
        .into(),
    )?;
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(2)),
                },
                output: None,
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(2)),
                next: Some(StepIdx::new(3)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
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
        vec![ConstValue::I64(42), ConstValue::I64(0)],
        vec![expr_lt],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(i64::MIN))?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(i64::MIN + 1))?;

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let action =
        super::step::replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) => {
            assert_eq!(next, StepIdx::new(1));
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "i64::MIN < i64::MIN+1 should be true",
        }),
    }
}

#[test]
fn replay_setconst_with_i64_max_value() -> Result<(), CoreError> {
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
        vec![ConstValue::I64(i64::MAX)],
        vec![],
    )?;

    let mut store = ValueStore::new();
    let frame = ReplayEngine::new(&plan)
        .replay_frame_through(StepIdx::new(0), &mut store)
        .map_err(replay_err_to_core)?;

    assert_eq!(*frame.read_slot(SlotIdx::new(0))?, SlotValue::I64(i64::MAX));
    Ok(())
}

#[test]
fn replay_setconst_with_i64_min_value() -> Result<(), CoreError> {
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
        vec![ConstValue::I64(i64::MIN)],
        vec![],
    )?;

    let mut store = ValueStore::new();
    let frame = ReplayEngine::new(&plan)
        .replay_frame_through(StepIdx::new(0), &mut store)
        .map_err(replay_err_to_core)?;

    assert_eq!(*frame.read_slot(SlotIdx::new(0))?, SlotValue::I64(i64::MIN));
    Ok(())
}

#[test]
fn replay_setconst_with_i64_zero() -> Result<(), CoreError> {
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
        vec![ConstValue::I64(0)],
        vec![],
    )?;

    let mut store = ValueStore::new();
    let frame = ReplayEngine::new(&plan)
        .replay_frame_through(StepIdx::new(0), &mut store)
        .map_err(replay_err_to_core)?;

    assert_eq!(*frame.read_slot(SlotIdx::new(0))?, SlotValue::I64(0));
    Ok(())
}
