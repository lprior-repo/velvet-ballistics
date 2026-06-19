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
#![forbid(unsafe_code)]
//! Choice-related replay step handlers.

use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::StepIdx;
use crate::value::SlotValue;

use super::{ReplayAction, ReplayError, eval_expr_for_replay, slot_to_replay_err};

/// Replays a ChooseSlot node which selects a branch based on boolean slot values.
pub fn replay_choose_slot(
    run: &mut RunFrame,
    branches: &[crate::workflow::SlotBranch],
    otherwise: Option<StepIdx>,
) -> Result<ReplayAction, ReplayError> {
    let mut index = 0usize;
    while index < branches.len() {
        let branch = branches.get(index).ok_or(ReplayError::Internal {
            reason: "choose_slot branch index checked by loop bound",
        })?;
        let value = run.read_slot(branch.condition).map_err(|e| match e {
            EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
            EngineError::SlotUninitialized { slot } => ReplayError::SlotNotAvailable { slot },
            _ => ReplayError::Internal {
                reason: "unexpected error reading choose_slot condition",
            },
        })?;
        match value {
            SlotValue::Bool(true) => {
                run.set_pc(branch.target).map_err(slot_to_replay_err)?;
                run.increment_executed()
                    .map_err(|_| ReplayError::Internal {
                        reason: "executed counter overflow",
                    })?;
                return Ok(ReplayAction::Continue(branch.target));
            }
            SlotValue::Bool(false) => {}
            _ => {
                return Err(ReplayError::Internal {
                    reason: "choose_slot condition is not boolean",
                });
            }
        }
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "choose_slot branch index overflow",
        })?;
    }
    let target = otherwise.ok_or(ReplayError::Internal {
        reason: "choose_slot no branch matched and no otherwise",
    })?;
    run.set_pc(target).map_err(slot_to_replay_err)?;
    run.increment_executed()
        .map_err(|_| ReplayError::Internal {
            reason: "executed counter overflow",
        })?;
    Ok(ReplayAction::Continue(target))
}

/// Replays a ChooseExpr node which selects a branch based on evaluated expressions.
pub fn replay_choose_expr(
    plan: &crate::workflow::CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut crate::value_store::ValueStore,
    branches: &[crate::workflow::ExprBranch],
    otherwise: Option<StepIdx>,
) -> Result<ReplayAction, ReplayError> {
    let mut index = 0usize;
    while index < branches.len() {
        let branch = branches.get(index).ok_or(ReplayError::Internal {
            reason: "choose_expr branch index checked by loop bound",
        })?;
        let (value, _taint) = eval_expr_for_replay(plan, run, store, branch.condition)
            .map_err(|_| ReplayError::ExpressionEvalFailed { step: run.pc() })?;
        match value {
            SlotValue::Bool(true) => {
                run.set_pc(branch.target).map_err(slot_to_replay_err)?;
                run.increment_executed()
                    .map_err(|_| ReplayError::Internal {
                        reason: "executed counter overflow",
                    })?;
                return Ok(ReplayAction::Continue(branch.target));
            }
            SlotValue::Bool(false) => {}
            _ => {
                return Err(ReplayError::Internal {
                    reason: "choose_expr condition is not boolean",
                });
            }
        }
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "choose_expr branch index overflow",
        })?;
    }
    let target = otherwise.ok_or(ReplayError::Internal {
        reason: "choose_expr no branch matched and no otherwise",
    })?;
    run.set_pc(target).map_err(slot_to_replay_err)?;
    run.increment_executed()
        .map_err(|_| ReplayError::Internal {
            reason: "executed counter overflow",
        })?;
    Ok(ReplayAction::Continue(target))
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::ok_expect,
        clippy::as_conversions,
        clippy::arithmetic_side_effects,
        clippy::indexing_slicing,
        clippy::let_underscore_must_use,
        clippy::panic,
        clippy::panic_in_result_fn,
        clippy::todo,
        clippy::unimplemented,
        clippy::assertions_on_constants,
        clippy::needless_range_loop,
        clippy::bool_assert_comparison,
        clippy::approx_constant,
        clippy::field_reassign_with_default,
        clippy::redundant_guards,
        clippy::redundant_closure,
        clippy::useless_conversion,
        clippy::unnecessary_unwrap,
        clippy::unnecessary_cast,
        clippy::needless_update,
        clippy::bool_comparison,
        clippy::manual_div_ceil,
        clippy::clone_on_copy,
        clippy::len_zero,
        clippy::redundant_clone,
        clippy::collapsible_if,
        clippy::needless_return,
        clippy::needless_borrow,
        clippy::useless_format,
        clippy::redundant_pub_crate,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::missing_safety_doc,
        clippy::wildcard_enum_match_arm,
        clippy::large_futures,
        clippy::unused_async,
        clippy::unused_self,
        let_underscore_drop,
        clippy::filter_map_next,
        clippy::from_iter_instead_of_collect,
        clippy::if_not_else,
        clippy::implicit_clone,
        clippy::inefficient_to_string,
        clippy::inconsistent_struct_constructor,
        clippy::iter_filter_is_ok,
        clippy::iter_filter_is_some,
        clippy::iter_not_returning_iterator,
        clippy::iter_over_hash_type,
        clippy::iter_without_into_iter,
        clippy::large_digit_groups,
        clippy::large_types_passed_by_value,
        clippy::let_and_return,
        clippy::misnamed_getters,
        clippy::mutable_key_type,
        clippy::needless_collect,
        clippy::nonminimal_bool,
        clippy::option_if_let_else,
        clippy::or_fun_call,
        clippy::path_buf_push_overwrite,
        clippy::print_stderr,
        clippy::print_stdout,
        clippy::pub_with_shorthand,
        clippy::range_minus_one,
        clippy::range_plus_one,
        clippy::ref_binding_to_reference,
        clippy::ref_option_ref,
        clippy::single_match_else,
        clippy::suspicious_operation_groupings,
        clippy::trivially_copy_pass_by_ref,
        clippy::uninlined_format_args,
        clippy::unnecessary_wraps,
        clippy::unnested_or_patterns,
        clippy::unreadable_literal,
        clippy::unused_io_amount,
        clippy::unused_trait_names,
        clippy::vec_init_then_push,
        clippy::wildcard_imports,
        clippy::absurd_extreme_comparisons,
        clippy::expect_fun_call,
        clippy::useless_vec,
        clippy::redundant_locals,
        clippy::too_many_lines,
        clippy::cast_lossless,
        clippy::cast_precision_loss,
        clippy::cast_possible_wrap,
        clippy::cast_abs_to_unsigned,
        clippy::similar_names,
        clippy::shadow_unrelated,
        clippy::needless_pass_by_value,
        unused_imports,
        dead_code,
        unused_variables
    )]

    use super::*;
    use crate::frame::RunFrame;
    use crate::ids::{RunId, SlotIdx, StepIdx};
    use crate::value::SlotValue;
    use crate::workflow::SlotBranch;

    fn make_frame(slot_count: u16) -> RunFrame {
        RunFrame::new(RunId::new(1), StepIdx::new(0), 5, slot_count).expect("valid frame")
    }

    #[test]
    fn choose_slot_true_branch_advances_pc() {
        let mut run = make_frame(2);
        run.write_slot(SlotIdx::new(0), SlotValue::Bool(true))
            .expect("write slot");
        let branches = [SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(2),
        }];
        let result = replay_choose_slot(&mut run, &branches, Some(StepIdx::new(3)));
        assert!(
            matches!(result, Ok(ReplayAction::Continue(s)) if s == StepIdx::new(2)),
            "expected Continue(2)"
        );
        assert_eq!(run.pc(), StepIdx::new(2));
    }

    #[test]
    fn choose_slot_false_branch_falls_through_to_otherwise() {
        let mut run = make_frame(2);
        run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))
            .expect("write slot");
        let branches = [SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(2),
        }];
        let result = replay_choose_slot(&mut run, &branches, Some(StepIdx::new(3)));
        assert!(
            matches!(result, Ok(ReplayAction::Continue(s)) if s == StepIdx::new(3)),
            "expected Continue(3)"
        );
        assert_eq!(run.pc(), StepIdx::new(3));
    }

    #[test]
    fn choose_slot_out_of_bounds_slot_returns_slot_not_available() {
        let mut run = make_frame(1);
        let branches = [SlotBranch {
            condition: SlotIdx::new(5),
            target: StepIdx::new(2),
        }];
        let result = replay_choose_slot(&mut run, &branches, Some(StepIdx::new(3)));
        assert!(
            matches!(result, Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(5)),
            "expected SlotNotAvailable(5)"
        );
    }

    #[test]
    fn choose_slot_uninitialized_slot_returns_slot_not_available() {
        let mut run = make_frame(2);
        // Slot 0 is never written — remains uninitialized
        let branches = [SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(2),
        }];
        let result = replay_choose_slot(&mut run, &branches, Some(StepIdx::new(3)));
        assert!(
            matches!(result, Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(0)),
            "expected SlotNotAvailable(0)"
        );
    }

    #[test]
    fn choose_slot_non_boolean_condition_returns_internal_error() {
        let mut run = make_frame(2);
        run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
            .expect("write slot");
        let branches = [SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(2),
        }];
        let result = replay_choose_slot(&mut run, &branches, Some(StepIdx::new(3)));
        assert!(
            matches!(
                result,
                Err(ReplayError::Internal { reason }) if reason == "choose_slot condition is not boolean"
            ),
            "expected Internal(non-bool)"
        );
    }

    #[test]
    fn choose_slot_no_otherwise_and_all_false_returns_internal_error() {
        let mut run = make_frame(2);
        run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))
            .expect("write slot");
        let branches = [SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(2),
        }];
        let result = replay_choose_slot(&mut run, &branches, None);
        assert!(
            matches!(
                result,
                Err(ReplayError::Internal { reason }) if reason == "choose_slot no branch matched and no otherwise"
            ),
            "expected Internal(no otherwise)"
        );
    }

    #[test]
    fn choose_slot_set_pc_failure_returns_slot_to_replay_err() {
        let mut run = make_frame(1);
        run.write_slot(SlotIdx::new(0), SlotValue::Bool(true))
            .expect("write slot");
        let branches = [SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(99), // out of bounds for frame with 1 step
        }];
        let result = replay_choose_slot(&mut run, &branches, Some(StepIdx::new(3)));
        assert!(
            matches!(
                result,
                Err(ReplayError::Internal { reason }) if reason == "unexpected engine error during replay"
            ),
            "expected Internal(engine error)"
        );
    }
}

// HVR-PO-CORE-004: exclude legacy replay-choose Kani modules from vb-god2f resource lane discovery.
#[cfg(all(kani, not(feature = "kani-vb-god2f-proof-kernels")))]
mod kani;
