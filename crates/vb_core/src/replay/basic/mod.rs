#![forbid(unsafe_code)]
//! Basic deterministic step handlers.
//!
//! Handles the simple node kinds: Nop, SetConst, Copy, EvalExpr, BuildObject,
//! BuildList, Finish, Jump, and the non-deterministic suspend passthrough.
//!
//! Also provides shared step-advance helpers (`advance_to_next`,
//! `increment_replay_executed`) used by this module and the dispatcher.

use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx};
use crate::value::{join_taint, SlotValue, Taint};
use crate::value_store::{ObjectField, ValueStore};
use crate::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use super::{ReplayAction, ReplayError, eval_expr_for_replay, slot_to_replay_err};

// ---------------------------------------------------------------------------
// Shared advancement helpers (used by basic handlers + collect module)
// ---------------------------------------------------------------------------

/// Advances the PC and increments the executed counter.
///
/// Extracted from `replay_nop` and `advance_to_next` to avoid duplication.
pub(super) fn advance_to_next(run: &mut RunFrame, node: &CompiledNode) -> Result<StepIdx, ReplayError> {
    let next = node.next.ok_or(ReplayError::Internal {
        reason: "node missing next step",
    })?;
    run.set_pc(next).map_err(slot_to_replay_err)?;
    increment_replay_executed(run)?;
    Ok(next)
}

/// Increments the run's executed counter with overflow guard.
pub(super) fn increment_replay_executed(run: &mut RunFrame) -> Result<(), ReplayError> {
    run.increment_executed().map_err(|_| ReplayError::Internal {
        reason: "executed counter overflow",
    })
}

// ---------------------------------------------------------------------------
// Nop
// ---------------------------------------------------------------------------

fn replay_nop(node: &CompiledNode, run: &mut RunFrame) -> Result<ReplayAction, ReplayError> {
    let next = node.next.ok_or(ReplayError::Internal {
        reason: "Nop node missing next step",
    })?;
    run.set_pc(next).map_err(slot_to_replay_err)?;
    increment_replay_executed(run)?;
    Ok(ReplayAction::Continue(next))
}

// ---------------------------------------------------------------------------
// Finish
// ---------------------------------------------------------------------------

fn replay_finish(run: &mut RunFrame, result: SlotIdx) -> Result<ReplayAction, ReplayError> {
    let _value = *run.read_slot(result).map_err(|e| match e {
        EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
        EngineError::SlotUninitialized { slot } => ReplayError::SlotNotAvailable { slot },
        _ => ReplayError::Internal {
            reason: "unexpected error reading finish result slot",
        },
    })?;
    increment_replay_executed(run)?;
    Ok(ReplayAction::Finished)
}

// ---------------------------------------------------------------------------
// Jump
// ---------------------------------------------------------------------------

fn replay_jump(run: &mut RunFrame, target: StepIdx) -> Result<ReplayAction, ReplayError> {
    run.set_pc(target).map_err(slot_to_replay_err)?;
    increment_replay_executed(run)?;
    Ok(ReplayAction::Continue(target))
}

// ---------------------------------------------------------------------------
// Suspend
// ---------------------------------------------------------------------------

fn replay_suspend(node: &CompiledNode, kind: super::step::SuspensionKind) -> ReplayAction {
    ReplayAction::Suspended {
        step: node.id,
        kind,
    }
}

// ---------------------------------------------------------------------------
// SetConst
// ---------------------------------------------------------------------------

fn replay_set_const(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    node: &CompiledNode,
    value: ConstIdx,
) -> Result<ReplayAction, ReplayError> {
    let constant = plan.constant(value).copied().ok_or(ReplayError::Internal {
        reason: "constant out of bounds",
    })?;
    let slot_value = constant
        .to_slot_value()
        .map_err(|_| ReplayError::Internal {
            reason: "constant to slot value failed",
        })?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "SetConst node missing output slot",
    })?;
    run.write_slot(output, slot_value)
        .map_err(slot_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

// ---------------------------------------------------------------------------
// Copy
// ---------------------------------------------------------------------------

fn replay_copy(
    run: &mut RunFrame,
    node: &CompiledNode,
    source: SlotIdx,
) -> Result<ReplayAction, ReplayError> {
    let value = *run.read_slot(source).map_err(|e| match e {
        EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
        EngineError::SlotUninitialized { slot } => ReplayError::SlotNotAvailable { slot },
        _ => ReplayError::Internal {
            reason: "unexpected error reading copy source slot",
        },
    })?;
    let taint = run.read_taint(source).map_err(slot_to_replay_err)?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "Copy node missing output slot",
    })?;
    run.write_slot_with_taint(output, value, taint)
        .map_err(slot_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

// ---------------------------------------------------------------------------
// EvalExpr
// ---------------------------------------------------------------------------

fn replay_eval_expr(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    expr: ExprIdx,
) -> Result<ReplayAction, ReplayError> {
    let (value, taint) = eval_expr_for_replay(plan, run, store, expr)
        .map_err(|_| ReplayError::ExpressionEvalFailed { step: node.id })?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "EvalExpr node missing output slot",
    })?;
    run.write_slot_with_taint(output, value, taint)
        .map_err(slot_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

// ---------------------------------------------------------------------------
// BuildObject
// ---------------------------------------------------------------------------

fn replay_build_object(
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    fields: &[(crate::ids::SymbolId, SlotIdx)],
) -> Result<ReplayAction, ReplayError> {
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(fields.len())
        .map_err(|_| ReplayError::Internal {
            reason: "allocation failed",
        })?;
    let mut accumulated_taint = Taint::Clean;
    let mut index = 0usize;
    while index < fields.len() {
        let (key, slot) = fields.get(index).ok_or(ReplayError::Internal {
            reason: "build_object field index checked by loop bound",
        })?;
        let value = *run.read_slot(*slot).map_err(|e| match e {
            EngineError::SlotOutOfBounds { slot: s } => ReplayError::SlotNotAvailable { slot: s },
            EngineError::SlotUninitialized { slot: s } => ReplayError::SlotNotAvailable { slot: s },
            _ => ReplayError::Internal {
                reason: "unexpected error reading build_object field slot",
            },
        })?;
        let slot_taint = run.read_taint(*slot).map_err(slot_to_replay_err)?;
        accumulated_taint = join_taint(accumulated_taint, slot_taint);
        entries.push(ObjectField {
            key: *key,
            value,
            taint: slot_taint,
        });
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "build_object field index overflow",
        })?;
    }
    let handle = store
        .insert_object(entries.into_boxed_slice())
        .map_err(|_| ReplayError::Internal {
            reason: "insert_object failed",
        })?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "BuildObject node missing output slot",
    })?;
    run.write_slot_with_taint(output, SlotValue::Object(handle), accumulated_taint)
        .map_err(slot_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

// ---------------------------------------------------------------------------
// BuildList
// ---------------------------------------------------------------------------

fn replay_build_list(
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    items: &[SlotIdx],
) -> Result<ReplayAction, ReplayError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(items.len())
        .map_err(|_| ReplayError::Internal {
            reason: "allocation failed",
        })?;
    let mut accumulated_taint = Taint::Clean;
    let mut index = 0usize;
    while index < items.len() {
        let slot = items.get(index).ok_or(ReplayError::Internal {
            reason: "build_list item index checked by loop bound",
        })?;
        let value = *run.read_slot(*slot).map_err(|e| match e {
            EngineError::SlotOutOfBounds { slot: s } => ReplayError::SlotNotAvailable { slot: s },
            EngineError::SlotUninitialized { slot: s } => ReplayError::SlotNotAvailable { slot: s },
            _ => ReplayError::Internal {
                reason: "unexpected error reading build_list item slot",
            },
        })?;
        let slot_taint = run.read_taint(*slot).map_err(slot_to_replay_err)?;
        accumulated_taint = join_taint(accumulated_taint, slot_taint);
        values.push(value);
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "build_list item index overflow",
        })?;
    }
    let handle =
        store
            .insert_list(values.into_boxed_slice())
            .map_err(|_| ReplayError::Internal {
                reason: "insert_list failed",
            })?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "BuildList node missing output slot",
    })?;
    run.write_slot_with_taint(output, SlotValue::List(handle), accumulated_taint)
        .map_err(slot_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

// ---------------------------------------------------------------------------
// Step-kind dispatcher (called by the outer `step.rs` dispatch)
// ---------------------------------------------------------------------------

/// Dispatches a single deterministic step by node kind.
///
/// This function is the single entry point for replaying one compiled node.
/// Non-deterministic node kinds return a `SuspensionKind`-wrapped action
/// rather than executing inline.
pub(crate) fn replay_step_kind(
    node: &CompiledNode,
    run: &mut RunFrame,
    store: &mut ValueStore,
    plan: &CompiledWorkflow,
    collect_states: &mut super::collect::ReplayCollectStates,
) -> Result<ReplayAction, ReplayError> {
    match &node.kind {
        CompiledNodeKind::Nop => replay_nop(node, run),
        CompiledNodeKind::SetConst { value } => replay_set_const(plan, run, node, *value),
        CompiledNodeKind::Copy { source } => replay_copy(run, node, *source),
        CompiledNodeKind::EvalExpr { expr } => replay_eval_expr(plan, run, store, node, *expr),
        CompiledNodeKind::BuildObject { fields } => replay_build_object(run, store, node, fields),
        CompiledNodeKind::BuildList { items } => replay_build_list(run, store, node, items),
        CompiledNodeKind::Finish { result } => replay_finish(run, *result),
        CompiledNodeKind::Jump { target } => replay_jump(run, *target),
        CompiledNodeKind::Do { .. } => Ok(replay_suspend(node, super::step::SuspensionKind::ActionPending)),
        CompiledNodeKind::Ask { .. } => Ok(replay_suspend(node, super::step::SuspensionKind::AskPending)),
        CompiledNodeKind::WaitUntil { .. } => Ok(replay_suspend(node, super::step::SuspensionKind::WaitUntil)),
        CompiledNodeKind::WaitEvent { .. } => Ok(replay_suspend(node, super::step::SuspensionKind::WaitEvent)),
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => super::choose::replay_choose_slot(run, branches, *otherwise),
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => super::choose::replay_choose_expr(plan, run, store, branches, *otherwise),
        CompiledNodeKind::CollectStart {
            source,
            limit,
            page_size,
            body,
            done,
        } => super::collect::replay_collect_start(
            run,
            store,
            collect_states,
            node,
            super::collect::ReplayCollectStartArgs {
                source: *source,
                limit: *limit,
                page_size: *page_size,
                body: *body,
                done: *done,
            },
        ),
        CompiledNodeKind::CollectPage {
            collector_slot,
            body,
            ..
        } => super::collect::replay_collect_page(run, *collector_slot, *body),
        CompiledNodeKind::CollectNext {
            collector_slot,
            body,
            done,
        } => super::collect::replay_collect_next(run, store, collect_states, *collector_slot, *body, *done),
        CompiledNodeKind::CollectFinish { collector_slot } => {
            super::collect::replay_collect_finish(run, collect_states, node, *collector_slot)
        }
        _ => Err(ReplayError::Internal {
            reason: "unsupported node kind for replay",
        }),
    }
}

#[cfg(test)]
mod tests {
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
}
