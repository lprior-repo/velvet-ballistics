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
//! Test chunk 001 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 13–292 of the original. Semantic content is
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

#[test]
fn budget_simple_linear_workflow() {
    let nodes = vec![
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
            next: Some(StepIdx::new(2)),
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
    let contract = test_contract(3, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_total_steps == 3 && b.max_fanout == 0 && b.max_nesting_depth == 0);

    assert!(budget.is_some(), "linear workflow budget mismatch");
}

#[test]
fn budget_branching_workflow() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ChooseSlot {
                branches: vec![
                    SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(2),
                    },
                ]
                .into_boxed_slice(),
                otherwise: Some(StepIdx::new(3)),
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
    let contract = test_contract(4, 1);
    match WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract) {
        Ok(actual) => {
            assert_eq!(
                actual.max_total_steps, 2,
                "conditional path uses max branch cost"
            );
            assert_eq!(
                actual.max_fanout, 2,
                "choose branch fanout is counted exactly"
            );
        }
        Err(error) => panic!("expected branching budget, got {error:?}"),
    }
}

#[test]
fn budget_nested_loop_depth() {
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
                limit: 10,
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
            output: Some(SlotIdx::new(4)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(4),
            },
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: Some(SlotIdx::new(5)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(5),
            },
        },
        CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(4),
            },
        },
    ];
    let contract = test_contract(6, 6);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_nesting_depth == 2);

    assert!(budget.is_some(), "nested loop depth mismatch");
}

#[test]
fn whole_workflow_budget_multiplies_nested_fanout_loop_body_steps() -> Result<(), String> {
    let nodes = nested_fanout_loop_nodes();
    let contract = test_contract(8, 8);

    let actual = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|error| format!("unexpected budget error: {error:?}"))?;

    ensure_equal(actual.max_total_steps, 11)?;
    ensure_equal(actual.max_fanout, 2)?;
    ensure_equal(actual.max_for_each_iterations, 2)?;
    ensure_equal(actual.max_together_branches, 2)
}

#[test]
fn whole_workflow_budget_accumulates_sequential_collect_reduce_repeat_and_resources()
-> Result<(), String> {
    let nodes = sequential_collect_reduce_repeat_wait_nodes();
    let contract = ResourceContract {
        max_collect_items: 3,
        max_retry_attempts: 4,
        max_output_bytes: 123,
        max_blob_bytes: 456,
        max_ipc_payload_bytes: 78,
        max_queue_depth: 9,
        max_journal_batch_bytes: 10,
        max_input_bytes: 11,
        ..test_contract(12, 12)
    };

    let actual = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|error| format!("unexpected budget error: {error:?}"))?;

    ensure_equal(actual.max_gather_pages, 1)?;
    ensure_equal(actual.max_gather_items, 3)?;
    ensure_equal(actual.max_repeat_attempts, 2)?;
    ensure_equal(actual.max_timer_entries, 1)?;
    ensure_equal(actual.max_result_bytes, 123)?;
    ensure_equal(actual.max_blob_bytes, 456)?;
    ensure_equal(actual.max_ipc_payload_bytes, 78)?;
    ensure_equal(actual.max_queue_depth, 9)?;
    ensure_equal(actual.max_journal_batch_bytes, 10)?;
    ensure_equal(actual.max_input_bytes, 11)
}

#[test]
fn whole_workflow_budget_uses_conditional_max_instead_of_branch_sum() -> Result<(), String> {
    let nodes = conditional_max_nodes();
    let contract = test_contract(8, 4);

    let actual = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|error| format!("unexpected budget error: {error:?}"))?;

    ensure_equal(actual.max_total_steps, 4)?;
    ensure_equal(actual.max_fanout, 2)
}

#[test]
fn whole_workflow_budget_rejects_unbounded_jump_cycle_with_exact_error() -> Result<(), String> {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Jump {
            target: StepIdx::new(0),
        },
    }];
    let contract = test_contract(1, 1);

    match WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract) {
        Err(WorkflowError::JumpCycle { step, target }) => {
            ensure_equal(step, StepIdx::new(0))?;
            ensure_equal(target, StepIdx::new(0))
        }
        other => Err(format!("expected JumpCycle for self jump, got {other:?}")),
    }
}

#[test]
fn budget_rejects_excessive_steps() {
    let budget = test_budget(3, 10, 1, 0);
    let policy = test_policy(2, 10, 64, 8);

    match policy.validate(&budget) {
        Err(BudgetError::TotalStepsExceeded {
            actual: 3,
            limit: 2,
        }) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}
