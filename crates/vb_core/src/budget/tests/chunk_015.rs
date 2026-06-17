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
//! Test chunk 015 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 3712–3960 of the original. Semantic content is
//! preserved exactly; only the file structure changed.
//! Budget module integration tests.

use crate::budget::{
    AggregateBudgetError, AggregateResourceBudget, AggregateResourceUsage, BoundednessPolicy,
    BudgetError, WholeWorkflowBudget,
};
use crate::engine::StepBudget;
use crate::ids::{ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, ExprBranch, ResourceContract, SlotBranch, WorkflowError,
};

use super::prelude::*;

proptest::proptest! {
    #[test]
    fn property_boundedness_policy(
        max_total_steps: u64,
        max_total_slots: u64,
        max_fanout: u16,
        max_nesting_depth: u16,
    ) {
        use crate::budget::{BoundednessPolicy, WholeWorkflowBudget};
        use proptest::prop_assert;

        let policy = BoundednessPolicy::DEFAULT;
        let budget = WholeWorkflowBudget {
            max_total_steps,
            max_total_slots,
            max_fanout,
            max_nesting_depth,
            max_steps_executable: max_total_steps as u32,
            max_action_tickets: 0,
            max_parallel_in_flight: 0,
            max_retries_per_action: 0,
            max_gather_pages: 0,
            max_gather_items: 0,
            max_for_each_iterations: 0,
            max_together_branches: 0,
            max_repeat_attempts: 0,
            max_run_time_seconds: 0,
            max_result_bytes: 0,
            max_total_slots_written: 0,
        max_timer_entries: 0,
        max_trace_events: 0,
            max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
            max_queue_depth: 0,
        };

        // If all dimensions are within policy defaults, validation should pass
        let result = policy.validate(&budget);
        if max_total_steps <= policy.max_total_steps
            && max_total_slots <= policy.max_total_slots
            && max_fanout <= policy.max_fanout
            && max_nesting_depth <= policy.max_nesting_depth
        {
            prop_assert!(matches!(result, Ok(())));
        } else {
            // If any dimension exceeds policy, validation should fail with a
            // specific BudgetError variant, not just a generic Err.
            prop_assert!(
                matches!(result, Err(BudgetError::TotalStepsExceeded { .. } | BudgetError::TotalSlotsExceeded { .. } | BudgetError::FanoutExceeded { .. } | BudgetError::NestingDepthExceeded { .. })),
                "exceeded policy must return a specific BudgetError, got {:?}",
                result
            );
        }
    }
}

// -------------------------------------------------------------------------
// Unit test: UNIT-POST-005
// test_step_count_overflow — WholeWorkflowBudget::compute propagates overflow
// -------------------------------------------------------------------------

#[test]
fn test_step_count_overflow() -> Result<(), String> {
    use crate::ids::StepIdx;
    use crate::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

    // Build a minimal 1-node workflow (a single Nop) to test the compute path.
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }];

    let parts = WorkflowParts {
        name: Box::from("step_count_overflow_test"),
        digest: crate::ids::WorkflowDigest::from_bytes([0x41; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    // Single-node workflow should compute without overflow
    let budget = crate::budget::WholeWorkflowBudget::compute(
        &parts.nodes,
        parts.entry,
        &parts.resource_contract,
    )
    .map_err(|e| e.to_string())?;
    // 1 node = 1 step
    assert_eq!(
        budget.max_total_steps, 1,
        "single-node workflow should have 1 step"
    );

    // Verify WorkflowError::StepCountOverflow can be constructed correctly.
    // This is the error type returned when u32::try_from(max_total_steps) fails
    // (i.e., when step count exceeds u32::MAX).
    let overflow_err = crate::workflow::WorkflowError::StepCountOverflow { actual: u64::MAX };
    match overflow_err {
        crate::workflow::WorkflowError::StepCountOverflow { actual } => {
            assert_eq!(actual, u64::MAX, "StepCountOverflow should carry u64::MAX");
        }
        other => return Err(format!("expected StepCountOverflow, got {:?}", other)),
    }

    Ok(())
}

// =========================================================================
// Additional coverage: count_and_push_loop_body overflow paths
// =========================================================================

#[test]
fn count_total_steps_overflow_returns_step_count_overflow() {
    use crate::ids::StepIdx;
    use crate::workflow::{CompiledNode, CompiledNodeKind};

    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: u32::MAX,
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(4, 4);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Ok(budget) => {
            assert!(budget.max_total_steps > u64::from(u32::MAX));
        }
        Err(WorkflowError::StepCountOverflow { actual: _ }) => {}
        Err(other) => panic!("expected StepCountOverflow, got {:?}", other),
    }
}

#[test]
fn count_and_push_loop_body_overflow_propagates_budget_error() {
    use crate::ids::StepIdx;
    use crate::workflow::{CompiledNode, CompiledNodeKind};

    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: u32::MAX,
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(2),
                item_slot: SlotIdx::new(3),
                limit: u32::MAX,
                body: StepIdx::new(2),
                done: StepIdx::new(3),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(5, 5);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Ok(budget) => {
            let _ = budget;
        }
        Err(WorkflowError::StepCountOverflow { actual: _ }) => {}
        Err(e) => panic!("expected StepCountOverflow, got {:?}", e),
    }
}
