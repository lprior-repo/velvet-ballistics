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

use super::*;
use crate::ids::RunId;

// -- Valid transitions --

#[test]
fn cancel_from_active_is_valid() {
    assert!(check_lifecycle_transition(
        LifecycleState::Active,
        LifecycleCommand::Cancel,
    ));
}

#[test]
fn cancel_from_waiting_answer_is_valid() {
    assert!(check_lifecycle_transition(
        LifecycleState::WaitingAnswer,
        LifecycleCommand::Cancel,
    ));
}

#[test]
fn resume_from_waiting_answer_is_valid() {
    assert!(check_lifecycle_transition(
        LifecycleState::WaitingAnswer,
        LifecycleCommand::Resume,
    ));
}

#[test]
fn retry_from_failed_is_valid() {
    assert!(check_lifecycle_transition(
        LifecycleState::Failed,
        LifecycleCommand::Retry,
    ));
}

#[test]
fn answer_from_waiting_answer_is_valid() {
    assert!(check_lifecycle_transition(
        LifecycleState::WaitingAnswer,
        LifecycleCommand::Answer,
    ));
}

// -- Invalid transitions: Cancel is the only command from Active --

#[test]
fn resume_from_active_is_invalid() {
    assert!(!check_lifecycle_transition(
        LifecycleState::Active,
        LifecycleCommand::Resume,
    ));
}

#[test]
fn retry_from_active_is_invalid() {
    assert!(!check_lifecycle_transition(
        LifecycleState::Active,
        LifecycleCommand::Retry,
    ));
}

#[test]
fn answer_from_active_is_invalid() {
    assert!(!check_lifecycle_transition(
        LifecycleState::Active,
        LifecycleCommand::Answer,
    ));
}

// -- Invalid transitions: Failed only accepts Retry --

#[test]
fn cancel_from_failed_is_invalid() {
    assert!(!check_lifecycle_transition(
        LifecycleState::Failed,
        LifecycleCommand::Cancel,
    ));
}

#[test]
fn resume_from_failed_is_invalid() {
    assert!(!check_lifecycle_transition(
        LifecycleState::Failed,
        LifecycleCommand::Resume,
    ));
}

#[test]
fn answer_from_failed_is_invalid() {
    assert!(!check_lifecycle_transition(
        LifecycleState::Failed,
        LifecycleCommand::Answer,
    ));
}

// -- Terminal states: Cancelled and Completed are absorbing --

#[test]
fn cancelled_rejects_cancel() {
    assert!(!check_lifecycle_transition(
        LifecycleState::Cancelled,
        LifecycleCommand::Cancel,
    ));
}

#[test]
fn cancelled_rejects_resume() {
    assert!(!check_lifecycle_transition(
        LifecycleState::Cancelled,
        LifecycleCommand::Resume,
    ));
}

#[test]
fn cancelled_rejects_retry() {
    assert!(!check_lifecycle_transition(
        LifecycleState::Cancelled,
        LifecycleCommand::Retry,
    ));
}

#[test]
fn cancelled_rejects_answer() {
    assert!(!check_lifecycle_transition(
        LifecycleState::Cancelled,
        LifecycleCommand::Answer,
    ));
}

#[test]
fn completed_rejects_cancel() {
    assert!(!check_lifecycle_transition(
        LifecycleState::Completed,
        LifecycleCommand::Cancel,
    ));
}

#[test]
fn completed_rejects_resume() {
    assert!(!check_lifecycle_transition(
        LifecycleState::Completed,
        LifecycleCommand::Resume,
    ));
}

#[test]
fn completed_rejects_retry() {
    assert!(!check_lifecycle_transition(
        LifecycleState::Completed,
        LifecycleCommand::Retry,
    ));
}

#[test]
fn completed_rejects_answer() {
    assert!(!check_lifecycle_transition(
        LifecycleState::Completed,
        LifecycleCommand::Answer,
    ));
}

// -- Pending state: no commands valid --

#[test]
fn pending_rejects_cancel() {
    assert!(!check_lifecycle_transition(
        LifecycleState::Pending,
        LifecycleCommand::Cancel,
    ));
}

#[test]
fn pending_rejects_resume() {
    assert!(!check_lifecycle_transition(
        LifecycleState::Pending,
        LifecycleCommand::Resume,
    ));
}

#[test]
fn pending_rejects_retry() {
    assert!(!check_lifecycle_transition(
        LifecycleState::Pending,
        LifecycleCommand::Retry,
    ));
}

#[test]
fn pending_rejects_answer() {
    assert!(!check_lifecycle_transition(
        LifecycleState::Pending,
        LifecycleCommand::Answer,
    ));
}

// -- Terminal state checks --

#[test]
fn pending_is_not_terminal() {
    assert!(!LifecycleState::Pending.is_terminal());
}

#[test]
fn active_is_not_terminal() {
    assert!(!LifecycleState::Active.is_terminal());
}

#[test]
fn waiting_answer_is_not_terminal() {
    assert!(!LifecycleState::WaitingAnswer.is_terminal());
}

#[test]
fn failed_is_not_terminal() {
    assert!(!LifecycleState::Failed.is_terminal());
}

#[test]
fn cancelled_is_terminal() {
    assert!(LifecycleState::Cancelled.is_terminal());
}

#[test]
fn completed_is_terminal() {
    assert!(LifecycleState::Completed.is_terminal());
}

// -- RunState integration --

#[test]
fn run_state_terminal_propagates() {
    let active_run = RunState {
        lifecycle: LifecycleState::Active,
        run_id: RunId::new(1),
    };
    assert!(!active_run.is_terminal());

    let completed_run = RunState {
        lifecycle: LifecycleState::Completed,
        run_id: RunId::new(2),
    };
    assert!(completed_run.is_terminal());
}

// -- Exhaustive transition matrix --

#[test]
fn exhaustive_transition_matrix() {
    let states = [
        LifecycleState::Pending,
        LifecycleState::Active,
        LifecycleState::WaitingAnswer,
        LifecycleState::Failed,
        LifecycleState::Cancelled,
        LifecycleState::Completed,
    ];
    let commands = [
        LifecycleCommand::Cancel,
        LifecycleCommand::Resume,
        LifecycleCommand::Retry,
        LifecycleCommand::Answer,
    ];

    let expected: [[bool; 4]; 6] = [
        // Pending
        [false, false, false, false],
        // Active
        [true, false, false, false],
        // WaitingAnswer
        [true, true, false, true],
        // Failed
        [false, false, true, false],
        // Cancelled
        [false, false, false, false],
        // Completed
        [false, false, false, false],
    ];

    for (si, state) in states.iter().enumerate() {
        for (ci, cmd) in commands.iter().enumerate() {
            let result = check_lifecycle_transition(*state, *cmd);
            assert_eq!(
                result, expected[si][ci],
                "transition ({state:?}, {cmd:?}) expected {expected:?}, got {result}",
            );
        }
    }
}
