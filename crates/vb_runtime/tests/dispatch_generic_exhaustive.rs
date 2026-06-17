#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::let_underscore_must_use, clippy::panic, clippy::panic_in_result_fn, clippy::bool_comparison, clippy::manual_div_ceil, clippy::clone_on_copy, clippy::len_zero, clippy::redundant_clone, clippy::collapsible_if, clippy::needless_return, clippy::needless_borrow, clippy::useless_format, clippy::redundant_pub_crate, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::missing_safety_doc, clippy::wildcard_enum_match_arm, clippy::large_futures, clippy::unused_async, clippy::unused_self, clippy::let_underscore_drop, clippy::filter_map_next, clippy::from_iter_instead_of_collect, clippy::if_not_else, clippy::implicit_clone, clippy::inefficient_to_string, clippy::inconsistent_struct_constructor, clippy::iter_filter_is_ok, clippy::iter_filter_is_some, clippy::iter_not_returning_iterator, clippy::iter_over_hash_type, clippy::iter_without_into_iter, clippy::large_digit_groups, clippy::large_types_passed_by_value, clippy::let_and_return, clippy::misnamed_getters, clippy::mutable_key_type, clippy::needless_collect, clippy::nonminimal_bool, clippy::option_if_let_else, clippy::or_fun_call, clippy::path_buf_push_overwrite, clippy::print_stderr, clippy::print_stdout, clippy::pub_with_shorthand, clippy::range_minus_one, clippy::range_plus_one, clippy::ref_binding_to_reference, clippy::ref_option_ref, clippy::single_match_else, clippy::suspicious_operation_groupings, clippy::trivially_copy_pass_by_ref, clippy::uninlined_format_args, clippy::unnecessary_wraps, clippy::unnested_or_patterns, clippy::unreadable_literal, clippy::unused_io_amount, clippy::unused_trait_names, clippy::vec_init_then_push, clippy::wildcard_imports)]

//! Dispatch generic exhaustive match test.
//!
//! Verifies that all match arms (3 named + default) are exercised.
//! Gated behind `vb-rxru0-mock-marker` feature.
//!
//! Obligations: RFO-019
//! Contract clauses: I-DISPATCH-1 (exhaustiveness)

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
fn test_dispatch_generic_exhaustive_match() {
    // All 3 named arms plus the default arm are exercised.
    let all_names = [
        ("github.issue.create", MockMarker::GithubIssueCreate),
        ("ai.classify_ticket", MockMarker::AiClassifyTicket),
        ("http.request", MockMarker::HttpGet),
        ("unknown.name", MockMarker::HttpGet), // defaults to HttpGet
    ];

    for (name, expected_mock) in all_names {
        let id = name.len() as u16;
        let input = make_input_with_action_name(id, name);
        let contract = make_contract(id, name);
        let outcome = dispatch_generic(&input, &contract).unwrap();

        match outcome {
            ActionOutcome::Suspended(ticket) => {
                assert_eq!(
                    ticket.mock, expected_mock,
                    "dispatch_generic must handle '{name}' exhaustively (expected {expected_mock:?})"
                );
            }
            unexpected => panic!("Expected Suspended for '{name}', got {unexpected:?}"),
        }
    }

    // Verify that no branch can return an error for valid input.
    // All paths must produce Ok(ActionOutcome::Suspended).
}
