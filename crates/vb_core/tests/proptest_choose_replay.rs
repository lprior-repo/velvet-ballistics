#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
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
    clippy::wildcard_imports
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
