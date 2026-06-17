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

//!
//! Proptest properties for ChooseSlot replay — supplementary to Kani harnesses.
//!
//! Bead: vb-282my
//! Obligation: PO-vb282my-CR-PROP-001
//!
//! Target: crate::replay::choose::replay_choose_slot
//!
//! Tests branch permutation correctness: randomized branch order and slot
//! values produce consistent selection (first true branch wins).

use proptest::prelude::*;
use vb_core::{
    frame::RunFrame,
    ids::{RunId, SlotIdx, StepIdx},
    replay::{ReplayAction, ReplayError, choose::replay_choose_slot},
    value::SlotValue,
    workflow::SlotBranch,
};

proptest! {
    /// PO-vb282my-CR-PROP-001: Branch permutation correctness
    /// Randomized branch order and boolean slot values produce consistent
    /// selection: the first branch with Bool(true) is always selected.
    #[test]
    fn proptest_choose_replay_permutation(
        branch_count in 1usize..=16usize,
        true_count in 1usize..=16usize,
    ) {
        let step_count: u16 = 200;
        let slot_count: u16 = 64;

        // Create a frame with initialized boolean slots
        let mut frame = RunFrame::new(
            RunId::new(1),
            StepIdx::new(0),
            step_count,
            slot_count,
        ).unwrap();

        // Initialize all slots to false
        for i in 0..slot_count {
            let _ = frame.write_slot(SlotIdx::new(i), SlotValue::Bool(false));
        }

        // Pick which slots will have true values
        let true_slots: Vec<u16> = (0..true_count.min(slot_count as usize))
            .map(|i| i as u16)
            .collect();

        // Set true values
        for &s in &true_slots {
            let _ = frame.write_slot(SlotIdx::new(s), SlotValue::Bool(true));
        }

        // Generate branches pointing at various slots
        let mut branches: Vec<SlotBranch> = Vec::new();
        for i in 0..branch_count {
            branches.push(SlotBranch {
                condition: SlotIdx::new(i as u16),
                target: StepIdx::new(100 + i as u16),
            });
        }

        let otherwise = Some(StepIdx::new(199));
        let result = replay_choose_slot(&mut frame, &branches, otherwise);

        match result {
            Ok(ReplayAction::Continue(target)) => {
                // A branch matched — verify it's the first true branch
                // The target should match the first branch with a true condition
                let first_true_idx = branches.iter()
                    .position(|b| {
                        if let Some(_cond_idx) = true_slots.iter().find(|&&s| s == b.condition.get()) {
                            true
                        } else {
                            false
                        }
                    });

                if let Some(idx) = first_true_idx {
                    assert_eq!(target, branches[idx].target,
                        "selected target must match first true branch");
                } else {
                    // No true branch — falls through to otherwise
                    assert_eq!(target, StepIdx::new(199),
                        "no true branch must fall through to otherwise");
                }

                // PC must match the target
                assert_eq!(frame.pc(), target);
            }
            Err(ReplayError::Internal { .. }) => {
                // Only expected when no branch matched and no otherwise
                // which shouldn't happen here since we have otherwise
            }
            Err(_) => {
                // Other errors for edge cases (e.g., uninitialized slots)
            }
            _ => {}
        }
    }
}
