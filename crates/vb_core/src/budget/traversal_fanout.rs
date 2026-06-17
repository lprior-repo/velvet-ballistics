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
#![cfg_attr(kani, allow(dead_code))]

use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind};

use super::traversal::BudgetTraversalError;
use super::traversal_successors::{
    branch_count_to_u16, find_node_position, node_at_position, push_successor_targets,
};
use super::traversal_tracking::{insert_tracked_step, remove_tracked_step, tracked_steps_contain};

/// Computes max fanout and max nesting depth via a DFS walk.
#[allow(clippy::too_many_arguments)]
pub(super) fn compute_fanout_and_depth(
    nodes: &[CompiledNode],
    current: StepIdx,
    visited: &mut [bool],
    in_path: &mut Vec<u16>,
    node_count: usize,
    current_depth: u16,
    max_fanout: &mut u16,
    max_nesting_depth: &mut u16,
    max_action_tickets: &mut u32,
    max_parallel_in_flight: &mut u16,
    max_gather_pages: &mut u32,
    max_gather_items: &mut u32,
    max_for_each_iterations: &mut u32,
    max_together_branches: &mut u16,
    max_repeat_attempts: &mut u16,
    max_timer_entries: &mut u32,
) -> Result<(), BudgetTraversalError> {
    let idx = find_node_position(nodes, current, node_count)?;
    if visited.get(idx).copied() == Some(true) {
        return Ok(());
    }
    let Some(flag) = visited.get_mut(idx) else {
        return Err(BudgetTraversalError::StepOutOfBounds { step: current });
    };
    *flag = true;

    let node = node_at_position(nodes, idx, current)?;

    let current_u16 = current.get();
    insert_tracked_step(in_path, current_u16, node_count)?;

    if let CompiledNodeKind::Jump { target } = &node.kind {
        let target_u16 = target.get();
        if tracked_steps_contain(in_path, target_u16) {
            remove_tracked_step(in_path, current_u16);
            return Err(BudgetTraversalError::JumpCycle {
                step: current,
                target: *target,
            });
        }
    }

    let child_depth = compute_child_depth(&node.kind, current_depth, max_nesting_depth)?;
    update_fanout(&node.kind, max_fanout)?;
    update_workflow_metrics(
        &node.kind,
        max_action_tickets,
        max_parallel_in_flight,
        max_gather_pages,
        max_gather_items,
        max_for_each_iterations,
        max_together_branches,
        max_repeat_attempts,
        max_timer_entries,
    )?;

    let mut targets: Vec<StepIdx> = Vec::new();
    push_successor_targets(&node.kind, &mut targets);
    if let Some(next) = node.next {
        targets.push(next);
    }

    for target in targets {
        if find_node_position(nodes, target, node_count).is_ok() {
            compute_fanout_and_depth(
                nodes,
                target,
                visited,
                in_path,
                node_count,
                child_depth,
                max_fanout,
                max_nesting_depth,
                max_action_tickets,
                max_parallel_in_flight,
                max_gather_pages,
                max_gather_items,
                max_for_each_iterations,
                max_together_branches,
                max_repeat_attempts,
                max_timer_entries,
            )?;
        }
    }

    remove_tracked_step(in_path, current_u16);
    Ok(())
}

fn compute_child_depth(
    kind: &CompiledNodeKind,
    current_depth: u16,
    max_nesting_depth: &mut u16,
) -> Result<u16, BudgetTraversalError> {
    match kind {
        CompiledNodeKind::ForEachStart { .. }
        | CompiledNodeKind::ForEachNext { .. }
        | CompiledNodeKind::CollectStart { .. }
        | CompiledNodeKind::CollectPage { .. }
        | CompiledNodeKind::CollectNext { .. }
        | CompiledNodeKind::ReduceStart { .. }
        | CompiledNodeKind::ReduceNext { .. }
        | CompiledNodeKind::RepeatStart { .. }
        | CompiledNodeKind::RepeatAttempt { .. }
        | CompiledNodeKind::TogetherStart { .. }
        | CompiledNodeKind::TogetherBranch { .. } => {
            let new_depth = current_depth
                .checked_add(1)
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
            if new_depth > *max_nesting_depth {
                *max_nesting_depth = new_depth;
            }
            Ok(new_depth)
        }
        _ => Ok(current_depth),
    }
}

fn update_fanout(
    kind: &CompiledNodeKind,
    max_fanout: &mut u16,
) -> Result<(), BudgetTraversalError> {
    match kind {
        CompiledNodeKind::TogetherStart { branches, .. } => {
            let branch_count = branch_count_to_u16(branches.len())?;
            if branch_count > *max_fanout {
                *max_fanout = branch_count;
            }
        }
        CompiledNodeKind::ChooseSlot { branches, .. } => {
            let branch_count = branch_count_to_u16(branches.len())?;
            if branch_count > *max_fanout {
                *max_fanout = branch_count;
            }
        }
        CompiledNodeKind::Choose { branches, .. } => {
            let branch_count = branch_count_to_u16(branches.len())?;
            if branch_count > *max_fanout {
                *max_fanout = branch_count;
            }
        }
        _ => {}
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn update_workflow_metrics(
    kind: &CompiledNodeKind,
    max_action_tickets: &mut u32,
    max_parallel_in_flight: &mut u16,
    max_gather_pages: &mut u32,
    max_gather_items: &mut u32,
    max_for_each_iterations: &mut u32,
    max_together_branches: &mut u16,
    max_repeat_attempts: &mut u16,
    max_timer_entries: &mut u32,
) -> Result<(), BudgetTraversalError> {
    match kind {
        CompiledNodeKind::Do { .. } => {
            *max_action_tickets = max_action_tickets
                .checked_add(1)
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
        }
        CompiledNodeKind::TogetherStart { branches, .. } => {
            let branch_count = branch_count_to_u16(branches.len())?;
            if branch_count > *max_parallel_in_flight {
                *max_parallel_in_flight = branch_count;
            }
            if branch_count > *max_together_branches {
                *max_together_branches = branch_count;
            }
        }
        CompiledNodeKind::CollectStart { limit, .. } => {
            *max_gather_pages = max_gather_pages
                .checked_add(1)
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
            *max_gather_items = max_gather_items
                .checked_add(*limit)
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
        }
        CompiledNodeKind::ForEachStart { limit, .. } => {
            *max_for_each_iterations = max_for_each_iterations
                .checked_add(*limit)
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
        }
        CompiledNodeKind::RepeatStart { max_attempts, .. } => {
            *max_repeat_attempts = (*max_repeat_attempts).max(*max_attempts);
        }
        CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::RetryCheck { .. }
        | CompiledNodeKind::RepeatCheck { .. } => {
            *max_timer_entries = max_timer_entries
                .checked_add(1)
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
        }
        _ => {}
    }
    Ok(())
}

// =========================================================================
// Tests for `compute_child_depth` — the private depth-increment function.
//
// Because this function is private (no `pub` qualifier) and the public API
// (`WholeWorkflowBudget::compute`) always seeds `current_depth` at 0, the
// u16::MAX overflow path cannot be reached through the public surface
// without constructing a 65 535-node workflow.  The unit tests below call
// the private function directly to prove the overflow guard is exercised
// and that `u16::MAX - 1` still advances to `u16::MAX` cleanly.
// =========================================================================

#[cfg(test)]
mod test {
    use crate::ids::{ConstIdx, SlotIdx, StepIdx};

    use super::*;

    /// B-BUDGET-TRAV-DEPTH-001: u16::MAX + ForEachStart overflows to
    /// `StepCountOverflow { actual: u64::MAX }` rather than wrapping.
    #[test]
    fn compute_child_depth_u16_max_overflows() {
        let kind = CompiledNodeKind::ForEachStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(1),
            limit: 1,
            body: StepIdx::new(0),
            done: StepIdx::new(0),
        };
        let mut max_nesting_depth: u16 = 0;

        let result = compute_child_depth(&kind, u16::MAX, &mut max_nesting_depth);

        assert!(
            result.is_err(),
            "compute_child_depth(u16::MAX, ForEachStart) must overflow"
        );
        match result {
            Err(BudgetTraversalError::StepCountOverflow { actual }) => {
                assert_eq!(
                    actual,
                    u64::MAX,
                    "overflow must carry the sentinel u64::MAX as `actual`"
                );
                // max_nesting_depth must NOT be mutated on error.
                assert_eq!(
                    max_nesting_depth, 0,
                    "max_nesting_depth must remain unchanged after overflow"
                );
            }
            other => panic!("expected StepCountOverflow, got {:?}", other),
        }
    }

    /// B-BUDGET-TRAV-DEPTH-002: u16::MAX - 1 + ForEachStart → Ok(u16::MAX)
    /// and correctly records the new depth into `max_nesting_depth`.
    #[test]
    fn compute_child_depth_u16_max_minus_one_succeeds() {
        let kind = CompiledNodeKind::ForEachStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(1),
            limit: 1,
            body: StepIdx::new(0),
            done: StepIdx::new(0),
        };
        let mut max_nesting_depth: u16 = 0;

        let result = compute_child_depth(&kind, u16::MAX - 1, &mut max_nesting_depth);

        assert!(
            result.is_ok(),
            "u16::MAX - 1 must successfully increment to u16::MAX"
        );
        let new_depth = result.unwrap();
        assert_eq!(
            new_depth,
            u16::MAX,
            "depth should be u16::MAX after incrementing u16::MAX - 1"
        );
        assert_eq!(
            max_nesting_depth,
            u16::MAX,
            "max_nesting_depth must be updated to u16::MAX"
        );
    }

    /// B-BUDGET-TRAV-DEPTH-003: non-depth-increasing node types pass
    /// `current_depth` through unchanged regardless of its value.
    #[test]
    fn compute_child_depth_nop_passthrough() {
        let kind = CompiledNodeKind::Nop;
        let mut max_nesting_depth: u16 = 0;

        let result = compute_child_depth(&kind, u16::MAX, &mut max_nesting_depth);

        assert!(result.is_ok(), "Nop must return Ok at any depth");
        assert_eq!(result.unwrap(), u16::MAX);
        assert_eq!(max_nesting_depth, 0, "max_nesting_depth unchanged for Nop");
    }

    /// B-BUDGET-TRAV-DEPTH-004: Every depth-increasing node kind overflows
    /// at u16::MAX and succeeds at u16::MAX - 1.
    #[test]
    fn compute_child_depth_all_depth_increasing_kinds() {
        let kinds: Vec<CompiledNodeKind> = vec![
            // ForEachStart
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 1,
                body: StepIdx::new(0),
                done: StepIdx::new(0),
            },
            // ForEachNext
            CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::new(0),
                body: StepIdx::new(0),
                done: StepIdx::new(0),
            },
            // CollectStart
            CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: 1,
                page_size: 1,
                body: StepIdx::new(0),
                done: StepIdx::new(0),
            },
            // CollectPage
            CompiledNodeKind::CollectPage {
                collector_slot: SlotIdx::new(0),
                body: StepIdx::new(0),
                done: StepIdx::new(0),
            },
            // CollectNext
            CompiledNodeKind::CollectNext {
                collector_slot: SlotIdx::new(0),
                body: StepIdx::new(0),
                done: StepIdx::new(0),
            },
            // ReduceStart
            CompiledNodeKind::ReduceStart {
                input: SlotIdx::new(0),
                accumulator: SlotIdx::new(1),
                initial: ConstIdx::new(0),
                body: StepIdx::new(0),
                done: StepIdx::new(0),
            },
            // ReduceNext
            CompiledNodeKind::ReduceNext {
                iterator_slot: SlotIdx::new(0),
                accumulator: SlotIdx::new(1),
                body: StepIdx::new(0),
                done: StepIdx::new(0),
            },
            // RepeatStart
            CompiledNodeKind::RepeatStart {
                max_attempts: 1,
                body: StepIdx::new(0),
                done: StepIdx::new(0),
            },
            // RepeatAttempt
            CompiledNodeKind::RepeatAttempt {
                attempt_slot: SlotIdx::new(0),
                body: StepIdx::new(0),
                done: StepIdx::new(0),
            },
            // TogetherStart
            CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(0)].into_boxed_slice(),
                join: StepIdx::new(1),
            },
            // TogetherBranch
            CompiledNodeKind::TogetherBranch {
                branch: 0,
                entry: StepIdx::new(0),
                join: StepIdx::new(1),
                accumulator: SlotIdx::new(0),
            },
        ];

        for kind in &kinds {
            let mut max_nesting_depth: u16 = 0;
            let result = compute_child_depth(kind, u16::MAX, &mut max_nesting_depth);
            assert!(
                result.is_err(),
                "depth-increasing kind {:?} must overflow at u16::MAX",
                kind
            );
            match result {
                Err(BudgetTraversalError::StepCountOverflow { actual }) => {
                    assert_eq!(actual, u64::MAX);
                    assert_eq!(max_nesting_depth, 0);
                }
                other => panic!("expected StepCountOverflow for {:?}, got {:?}", kind, other),
            }

            // Re-test: u16::MAX - 1 should succeed for every kind.
            let mut max_nesting_depth2: u16 = 0;
            let result = compute_child_depth(kind, u16::MAX - 1, &mut max_nesting_depth2);
            assert!(
                result.is_ok(),
                "depth-increasing kind {:?} must succeed at u16::MAX - 1",
                kind
            );
            assert_eq!(result.unwrap(), u16::MAX);
            assert_eq!(max_nesting_depth2, u16::MAX);
        }
    }
}
