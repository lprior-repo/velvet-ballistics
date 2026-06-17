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

//! Properties of `dispatch_generic` that hold regardless of feature flags.
//!
//! Verifies that dispatch produces Suspended outcome, preserves fields,
//! sets capacity to 1, is idempotent, and rejects zero max_input_bytes.
//!
//! Tests go through `ActionRegistry::dispatch` (the public API), which
//! internally calls `dispatch_generic`.

#![forbid(unsafe_code)]

use vb_core::action::{
    ActionContract, ActionError, ActionInput, ActionName, ActionOutcome, ActionTicket,
};
use vb_core::action::{Idempotency, RetrySafety, SideEffect};
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

fn make_input(action_id: u16) -> ActionInput {
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
            ..Default::default()
        },
    }
}

fn make_input_with_capacity(action_id: u16, capacity: u16) -> ActionInput {
    let mut input = make_input(action_id);
    input.ticket.capacity = capacity;
    input
}

// ---------------------------------------------------------------------------
// Suspended outcome
// ---------------------------------------------------------------------------

#[test]
fn test_dispatch_produces_suspended_outcome() {
    let contract = make_contract(5, "test.action");
    let input = make_input(5);

    let result = dispatch_generic(&input, &contract);
    match result {
        Ok(ActionOutcome::Suspended(ticket)) => {
            assert_eq!(
                ticket.action,
                ActionId::new(5),
                "dispatched ticket action must match input action"
            );
        }
        other => panic!("Expected Ok(Suspended), got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Field preservation
// ---------------------------------------------------------------------------

#[test]
fn test_dispatch_preserves_run() {
    let contract = make_contract(1, "test.action");
    let input = make_input(1);

    let result = dispatch_generic(&input, &contract).unwrap();
    match result {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(
                ticket.run, input.run,
                "dispatch must preserve run from input"
            );
        }
        other => panic!("Expected Suspended, got {other:?}"),
    }
}

#[test]
fn test_dispatch_preserves_step() {
    let contract = make_contract(1, "test.action");
    let input = make_input(1);

    let result = dispatch_generic(&input, &contract).unwrap();
    match result {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(
                ticket.step, input.step,
                "dispatch must preserve step from input"
            );
        }
        other => panic!("Expected Suspended, got {other:?}"),
    }
}

#[test]
fn test_dispatch_preserves_ticket_seq() {
    let contract = make_contract(1, "test.action");
    let input = make_input(1);

    let result = dispatch_generic(&input, &contract).unwrap();
    match result {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(
                ticket.seq, input.ticket.seq,
                "dispatch must preserve ticket seq from input"
            );
        }
        other => panic!("Expected Suspended, got {other:?}"),
    }
}

#[test]
fn test_dispatch_preserves_ticket_action() {
    let contract = make_contract(1, "test.action");
    let input = make_input(1);

    let result = dispatch_generic(&input, &contract).unwrap();
    match result {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(
                ticket.action, input.action,
                "dispatch must preserve action from input"
            );
        }
        other => panic!("Expected Suspended, got {other:?}"),
    }
}

#[test]
fn test_dispatch_preserves_ticket_attempt() {
    let contract = make_contract(1, "test.action");
    let input = make_input(1);

    let result = dispatch_generic(&input, &contract).unwrap();
    match result {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(
                ticket.attempt, input.ticket.attempt,
                "dispatch must preserve attempt from input ticket"
            );
        }
        other => panic!("Expected Suspended, got {other:?}"),
    }
}

#[test]
fn test_dispatch_preserves_ticket_idempotency_key() {
    let contract = make_contract(1, "test.action");
    let input = make_input(1);

    let result = dispatch_generic(&input, &contract).unwrap();
    match result {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(
                ticket.idempotency_key, input.ticket.idempotency_key,
                "dispatch must preserve idempotency_key from input ticket"
            );
        }
        other => panic!("Expected Suspended, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Capacity handling
// ---------------------------------------------------------------------------

#[test]
fn test_dispatch_capacity_set_to_one() {
    let contract = make_contract(1, "test.action");
    let input = make_input(1);

    let result = dispatch_generic(&input, &contract).unwrap();
    match result {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(
                ticket.capacity, 1,
                "dispatch must set capacity to 1 regardless of input"
            );
        }
        other => panic!("Expected Suspended, got {other:?}"),
    }
}

#[test]
fn test_dispatch_capacity_override_from_high_value() {
    let contract = make_contract(1, "test.action");
    let input = make_input_with_capacity(1, 9999);

    let result = dispatch_generic(&input, &contract).unwrap();
    match result {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(
                ticket.capacity, 1,
                "dispatch must override input capacity to 1"
            );
            assert_ne!(
                ticket.capacity, 9999,
                "capacity must NOT be the input capacity"
            );
        }
        other => panic!("Expected Suspended, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Idempotence
// ---------------------------------------------------------------------------

#[test]
fn test_dispatch_is_idempotent() {
    let contract = make_contract(1, "test.action");
    let input = make_input(1);

    let result1 = dispatch_generic(&input, &contract).unwrap();
    let result2 = dispatch_generic(&input, &contract).unwrap();

    match (result1, result2) {
        (ActionOutcome::Suspended(t1), ActionOutcome::Suspended(t2)) => {
            assert_eq!(
                t1, t2,
                "dispatch must be idempotent: same input produces same outcome"
            );
        }
        (r1, r2) => panic!("Expected two Suspended outcomes, got {r1:?}, {r2:?}"),
    }
}

// ---------------------------------------------------------------------------
// Payload rejection
// ---------------------------------------------------------------------------

#[test]
fn test_dispatch_zero_max_input_bytes_rejects() {
    let input = make_input(1);
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test.action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 0,
        max_input_bytes: 0,
        max_output_bytes: 0,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    };

    let result = dispatch_generic(&input, &contract);
    assert_eq!(
        result,
        Err(ActionError::PayloadTooLarge {
            max_bytes: 0,
            actual_bytes: 0
        }),
        "dispatch must reject when max_input_bytes is 0 and input_slot_count > 0"
    );
}

// ---------------------------------------------------------------------------
// Multiple action names
// ---------------------------------------------------------------------------

#[test]
fn test_dispatch_with_different_action_names() {
    // Explicit IDs to avoid collisions.
    let actions: &[(&str, u16)] = &[
        ("test.action", 100),
        ("github.issue.create", 101),
        ("ai.classify", 102),
        ("http.request", 103),
    ];

    for (name, id) in actions {
        let contract = make_contract(*id, name);

        let input = ActionInput {
            run: RunId::new(1),
            step: StepIdx::new(0),
            action: ActionId::new(*id),
            input: SlotIdx::new(0),
            ticket: ActionTicket {
                run: RunId::new(1),
                step: StepIdx::new(0),
                seq: SeqNo::new(1),
                action: ActionId::new(*id),
                attempt: 1,
                idempotency_key: 0,
                capacity: 1,
                ..Default::default()
            },
        };

        let result = dispatch_generic(&input, &contract).unwrap();
        match result {
            ActionOutcome::Suspended(ticket) => {
                assert_eq!(
                    ticket.action,
                    ActionId::new(*id),
                    "dispatch must work for action name '{name}'"
                );
                assert_eq!(
                    ticket.capacity, 1,
                    "capacity must be 1 for action name '{name}'"
                );
            }
            other => panic!("Expected Suspended for '{name}', got {other:?}"),
        }
    }
}
