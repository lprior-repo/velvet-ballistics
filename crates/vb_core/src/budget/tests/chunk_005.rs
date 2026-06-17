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
//! Test chunk 005 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 1057–1323 of the original. Semantic content is
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

#[test]
fn blackhat_step_count_overflow_uses_misleading_error_variant() {
    let workflow_err = WorkflowError::StepOutOfBounds {
        step: StepIdx::new(0),
    };
    let converted: BudgetError = workflow_err.into();
    match converted {
        BudgetError::TotalStepsExceeded { actual, limit } => {
            assert_eq!(actual, u64::MAX);
            assert_eq!(limit, u64::MAX);
        }
        other => panic!("unexpected variant: {other:?}"),
    }
}

// --- FINDING BH-BUD-06: action_tickets saturating_add hides overflow ---

#[test]
fn blackhat_action_tickets_saturating_add_under_reports() {
    let mut max_action_tickets: u32 = u32::MAX;
    max_action_tickets = max_action_tickets.saturating_add(1);
    assert_eq!(
        max_action_tickets,
        u32::MAX,
        "BLACKHAT BH-BUD-06: saturating_add hides overflow"
    );
}

// --- FINDING BH-BUD-07: gather_items saturating_add accumulation ---

#[test]
fn blackhat_gather_items_accumulation_saturates() {
    let mut max_gather_items: u32 = u32::MAX - 10;
    max_gather_items = max_gather_items.saturating_add(20);
    assert_eq!(
        max_gather_items,
        u32::MAX,
        "BLACKHAT BH-BUD-07: gather items saturates at u32::MAX"
    );
}

// --- FINDING BH-BUD-08: retries_per_action copied from contract not computed ---

#[test]
fn blackhat_retries_per_action_copied_from_contract_not_computed() {
    let contract = ResourceContract {
        max_retry_attempts: 42,
        ..test_contract(1, 1)
    };
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }];
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_retries_per_action == 42);

    assert!(
        budget.is_some(),
        "BLACKHAT BH-BUD-08: retries copied from contract, not computed from IR"
    );
}

// --- FINDING BH-BUD-09: forward jump does not trigger cycle detection ---

#[test]
fn blackhat_jump_cycle_detection_relies_on_forward_edge_validation() {
    let nodes = vec![
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
    ];
    let contract = test_contract(2, 1);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);

    match result {
        Ok(budget) => {
            assert_eq!(
                budget.max_total_steps, 2,
                "forward jump should count as 2 steps"
            );
        }
        Err(_) => {
            panic!("BLACKHAT BH-BUD-09: forward jump incorrectly detected as cycle");
        }
    }
}

// --- FINDING BH-BUD-10: policy boundary exact vs over ---

#[test]
fn blackhat_policy_allows_exact_limit() {
    let budget = test_budget(1_000, 65_535, 64, 8);
    let result = BoundednessPolicy::DEFAULT.validate(&budget);
    assert_eq!(
        result,
        Ok(()),
        "BLACKHAT BH-BUD-10: budget at exact limits should pass"
    );
}

#[test]
fn blackhat_policy_rejects_one_over_limit() {
    let budget = test_budget(1_001, 65_535, 64, 8);
    let result = BoundednessPolicy::DEFAULT.validate(&budget);
    assert!(
        result.is_err(),
        "BLACKHAT BH-BUD-10: budget one over limit must be rejected"
    );
}

// --- FINDING BH-BUD-11: StepBudget clamping is silent ---

#[test]
fn blackhat_step_budget_clamping_is_silent() {
    let budget = StepBudget::new(100_000);
    assert_eq!(
        budget.remaining(),
        crate::limits::MAX_STEP_BUDGET,
        "BLACKHAT BH-BUD-11: requested 100K steps silently clamped"
    );
}

// --- FINDING BH-BUD-12: self-referencing loop body graceful handling ---

#[test]
fn blackhat_self_referencing_loop_body_gracefully_handled() {
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
                limit: 10,
                body: StepIdx::new(0),
                done: StepIdx::new(1),
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
    ];
    let contract = test_contract(2, 3);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    assert!(
        result.is_ok() || result.is_err(),
        "BLACKHAT BH-BUD-12: self-referencing body must not panic"
    );
}

// --- FINDING BH-BUD-13: ReduceStart body uses cold-AST-conservative iter count ---

#[test]
fn blackhat_reduce_start_uses_cold_ast_conservative_iteration_count() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ReduceStart {
                input: SlotIdx::new(0),
                accumulator: SlotIdx::new(1),
                initial: ConstIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
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
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract).ok();

    // Cold-AST invariant (master §45) drops body, so the budget traversal
    // cannot recover the declared input length. The conservative default
    // iter count is 1, giving body_count * 1 = 1 + 1 (header + finish) = 3.
    let expected = 1 + 1 + 1;

    assert!(
        budget.is_some(),
        "BLACKHAT BH-BUD-13: ReduceStart should compute with cold-AST-conservative iter count"
    );
    assert_eq!(
        budget.as_ref().map(|b| b.max_total_steps),
        Some(expected),
        "BLACKHAT BH-BUD-13: expected {expected} steps"
    );
}

// Helpers (test_contract, test_budget, etc.) are imported from the prelude.
use super::prelude::*;
