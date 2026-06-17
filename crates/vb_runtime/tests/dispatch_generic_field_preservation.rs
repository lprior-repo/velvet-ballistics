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

//! Dispatch generic field preservation tests (with mock field).
//!
//! Verifies that dispatch_generic preserves all input fields and that
//! calling it twice with the same input produces identical results.
//! Gated behind `vb-rxru0-mock-marker` feature.
//!
//! Obligations: RFO-010, RFO-013, RFO-014
//! Contract clauses: PST-DISPATCH-2 (field preservation),
//!                   I-DISPATCH-3 (idempotence)

#![forbid(unsafe_code)]
#![cfg(feature = "vb-rxru0-mock-marker")]

use vb_core::action::MockMarker;
use vb_core::action::{
    ActionContract, ActionInput, ActionName, ActionOutcome, ActionTicket, Idempotency, RetrySafety,
    SideEffect,
};
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};
use vb_runtime::action::dispatch_generic;

fn make_contract(id: u16, name: &str) -> ActionContract {
    ActionContract {
        id: ActionId::new(id),
        name: ActionName::new(name).unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    }
}

fn make_input_with_action_name(action_id: u16, _name: &str) -> ActionInput {
    ActionInput {
        run: RunId::new(1),
        step: StepIdx::new(0),
        action: ActionId::new(action_id),
        input: SlotIdx::new(0),
        ticket: ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(42),
            action: ActionId::new(action_id),
            attempt: 3,
            idempotency_key: 0xBEEF,
            capacity: 999,
            mock: MockMarker::GithubIssueCreate,
        },
    }
}

#[test]
fn test_dispatch_preserves_all_fields() {
    let input = make_input_with_action_name(1, "github.issue.create");
    let contract = make_contract(1, "github.issue.create");
    let outcome = dispatch_generic(&input, &contract).unwrap();

    match outcome {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(ticket.run, input.run, "run must be preserved from input");
            assert_eq!(ticket.step, input.step, "step must be preserved from input");
            assert_eq!(
                ticket.seq, input.ticket.seq,
                "seq must be preserved from input ticket"
            );
            assert_eq!(
                ticket.action, input.action,
                "action must be preserved from input"
            );
            assert_eq!(
                ticket.attempt, input.ticket.attempt,
                "attempt must be preserved from input ticket"
            );
            assert_eq!(
                ticket.idempotency_key, input.ticket.idempotency_key,
                "idempotency_key must be preserved from input ticket"
            );
            assert_eq!(ticket.capacity, 1, "capacity must be set to 1");
            assert_eq!(
                ticket.mock,
                MockMarker::GithubIssueCreate,
                "mock must be derived from contract.name"
            );
        }
        unexpected => panic!("Expected Suspended, got {unexpected:?}"),
    }
}

#[test]
fn test_dispatch_is_pure() {
    let input = make_input_with_action_name(1, "ai.classify_ticket");
    let contract = make_contract(1, "ai.classify_ticket");

    let result1 = dispatch_generic(&input, &contract).unwrap();
    let result2 = dispatch_generic(&input, &contract).unwrap();

    match (result1, result2) {
        (ActionOutcome::Suspended(t1), ActionOutcome::Suspended(t2)) => {
            assert_eq!(t1, t2, "dispatch_generic must be idempotent");
        }
        _ => panic!("Expected two Suspended outcomes"),
    }
}
