#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports
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
