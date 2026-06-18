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
//! Test chunk 013 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 3178–3447 of the original. Semantic content is
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
fn repeat_start_max_attempts_tracks_maximum_not_sum() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 3,
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
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 10,
                body: StepIdx::new(4),
                done: StepIdx::new(5),
            },
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(6, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_repeat_attempts, 10)
}

// -------------------------------------------------------------------------
// Additional coverage: StepBudget::MAX is const
// -------------------------------------------------------------------------

#[test]
fn step_budget_max_is_const_compatible() -> Result<(), String> {
    const _MAX: StepBudget = StepBudget::MAX;
    ensure_equal(_MAX.remaining(), crate::limits::MAX_STEP_BUDGET)
}

// -------------------------------------------------------------------------
// Additional coverage: WholeWorkflowBudget debug format
// -------------------------------------------------------------------------

#[test]
fn whole_workflow_budget_debug_format() -> Result<(), String> {
    let budget = test_budget(10, 20, 4, 2);
    let debug = format!("{budget:?}");
    ensure_equal(debug.contains("WholeWorkflowBudget"), true)?;
    ensure_equal(debug.contains("max_total_steps"), true)
}

#[test]
fn boundedness_policy_debug_format() -> Result<(), String> {
    let policy = BoundednessPolicy::DEFAULT;
    let debug = format!("{policy:?}");
    ensure_equal(debug.contains("BoundednessPolicy"), true)?;
    ensure_equal(debug.contains("max_total_steps"), true)
}

#[test]
fn budget_error_debug_format() -> Result<(), String> {
    let err = BudgetError::FanoutExceeded {
        actual: 5,
        limit: 3,
    };
    let debug = format!("{err:?}");
    ensure_equal(debug.contains("FanoutExceeded"), true)
}

// =========================================================================
// Additional edge-case tests — Budget construction, checked operations
// =========================================================================

#[test]
fn whole_workflow_budget_zero_fields_is_valid() -> Result<(), String> {
    let budget = test_budget(0, 0, 0, 0);
    ensure_equal(budget.max_total_steps, 0)?;
    ensure_equal(budget.max_total_slots, 0)?;
    ensure_equal(budget.max_fanout, 0)?;
    ensure_equal(budget.max_nesting_depth, 0)
}

#[test]
fn whole_workflow_budget_max_fields() -> Result<(), String> {
    let budget = WholeWorkflowBudget {
        max_total_steps: u64::MAX,
        max_total_slots: u64::MAX,
        max_fanout: u16::MAX,
        max_nesting_depth: u16::MAX,
        max_steps_executable: u32::MAX,
        max_action_tickets: u32::MAX,
        max_parallel_in_flight: u16::MAX,
        max_retries_per_action: u16::MAX,
        max_gather_pages: u32::MAX,
        max_gather_items: u32::MAX,
        max_for_each_iterations: u32::MAX,
        max_together_branches: u16::MAX,
        max_repeat_attempts: u16::MAX,
        max_run_time_seconds: u64::MAX,
        max_result_bytes: u32::MAX,
        max_total_slots_written: u32::MAX,
        max_timer_entries: u32::MAX,
        max_trace_events: u64::MAX,
        max_journal_batch_bytes: u32::MAX,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_queue_depth: u32::MAX,
    };
    ensure_equal(budget.max_total_steps, u64::MAX)?;
    ensure_equal(budget.max_total_slots, u64::MAX)?;
    ensure_equal(budget.max_fanout, u16::MAX)?;
    ensure_equal(budget.max_nesting_depth, u16::MAX)
}

#[test]
fn boundedness_policy_default_values_are_sensible() -> Result<(), String> {
    let p = BoundednessPolicy::DEFAULT;
    ensure_equal(p.max_total_steps, 1_000)?;
    ensure_equal(p.max_total_slots, 65_535)?;
    ensure_equal(p.max_fanout, 64)?;
    ensure_equal(p.max_nesting_depth, 8)?;
    ensure_equal(p.absolute_max_action_tickets, 100_000)?;
    ensure_equal(p.absolute_max_parallel, 256)?;
    ensure_equal(p.absolute_max_run_time_seconds, 2_592_000)?;
    ensure_equal(p.absolute_max_result_bytes, 262_144)?;
    ensure_equal(p.absolute_max_steps_executable, 1_000_000)
}

#[test]
fn budget_error_all_variants_display_non_empty() -> Result<(), String> {
    let errors = [
        BudgetError::TotalStepsExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::TotalSlotsExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::FanoutExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::NestingDepthExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::ParallelExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::ActionTicketsExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::RunTimeExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::ResultBytesExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::StepsExecutableExceeded {
            actual: 1,
            limit: 0,
        },
    ];
    for err in &errors {
        let display = format!("{err}");
        if display.is_empty() {
            return Err(format!("BudgetError display is empty for {err:?}"));
        }
    }
    Ok(())
}

#[test]
fn budget_error_equality_same_variants() -> Result<(), String> {
    let a = BudgetError::TotalStepsExceeded {
        actual: 5,
        limit: 3,
    };
    let b = BudgetError::TotalStepsExceeded {
        actual: 5,
        limit: 3,
    };
    ensure_equal(a, b)
}

#[test]
fn budget_error_inequality_different_actual() -> Result<(), String> {
    let a = BudgetError::TotalStepsExceeded {
        actual: 5,
        limit: 3,
    };
    let b = BudgetError::TotalStepsExceeded {
        actual: 6,
        limit: 3,
    };
    assert_ne!(a, b);
    Ok(())
}

#[test]
fn budget_error_clone_preserves_equality() -> Result<(), String> {
    let a = BudgetError::FanoutExceeded {
        actual: 10,
        limit: 5,
    };
    let b = a.clone();
    ensure_equal(a, b)
}

#[test]
fn budget_error_from_workflow_error_preserves_variant() -> Result<(), String> {
    let wf_err = WorkflowError::EntryOutOfBounds {
        entry: StepIdx::new(0),
    };
    let budget_err: BudgetError = wf_err.into();
    match budget_err {
        BudgetError::TotalStepsExceeded { actual, limit } => {
            ensure_equal(actual, u64::MAX)?;
            ensure_equal(limit, u64::MAX)
        }
        other => Err(format!(
            "expected TotalStepsExceeded sentinel for workflow error, got {other:?}"
        )),
    }
}
