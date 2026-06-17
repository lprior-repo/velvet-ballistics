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

//! Dispatch generic determinism tests (with mock field).
//!
//! Verifies that dispatch is deterministic: same input always produces
//! the same output.
//! Gated behind `vb-rxru0-mock-marker` feature.
//!
//! Obligations: RFO-011, RFO-020
//! Contract clauses: I-DISPATCH-3 (idempotence + determinism)

#![forbid(unsafe_code)]
#![cfg(feature = "vb-rxru0-mock-marker")]

use proptest::prelude::{prop_assert_eq, proptest};
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
fn test_dispatch_deterministic() {
    let names = [
        "github.issue.create",
        "ai.classify_ticket",
        "http.request",
        "random.action.name",
    ];

    for name in &names {
        let id = name.len() as u16;
        let results: Vec<ActionOutcome> = (0..10)
            .map(|_| {
                let input = make_input_with_action_name(id, name);
                let contract = make_contract(id, name);
                dispatch_generic(&input, &contract).unwrap()
            })
            .collect();

        // All 10 executions produce identical outcomes.
        for result in &results[1..] {
            assert_eq!(
                result, &results[0],
                "dispatch_generic must be deterministic for '{name}'"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Proptest: property-based determinism
// ---------------------------------------------------------------------------

#[cfg(test)]
proptest! {
    /// Property-based determinism test for dispatch_generic.
    #[test]
    fn test_dispatch_determinism(name in "[a-z\\.]{1,64}") {
        let id = name.len() as u16;
        let input = make_input_with_action_name(id, &name);
        let contract = make_contract(id, &name);
        let result1 = dispatch_generic(&input, &contract).unwrap();
        let result2 = dispatch_generic(&input, &contract).unwrap();
        prop_assert_eq!(result1, result2, "dispatch must be deterministic");
    }

    /// Property-based non-mock names default to HttpGet.
    #[test]
    fn test_dispatch_non_mock_defaults_to_http_get(name in "[a-z]{3,20}") {
        let known_names = ["github.issue.create", "ai.classify_ticket", "http.request"];
        if !known_names.contains(&name.as_str()) {
            let id = name.len() as u16;
            let input = make_input_with_action_name(id, &name);
            let contract = make_contract(id, &name);
            let outcome = dispatch_generic(&input, &contract).unwrap();
            match outcome {
                ActionOutcome::Suspended(ticket) => {
                    prop_assert_eq!(
                        ticket.mock,
                        MockMarker::HttpGet,
                        "Non-mock names must default to HttpGet"
                    );
                }
                _ => panic!("Expected Suspended"),
            }
        }
    }
}
