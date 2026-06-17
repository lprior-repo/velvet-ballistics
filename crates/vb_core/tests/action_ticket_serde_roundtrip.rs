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

//! Postcard serialization roundtrip for `ActionTicket`.
//!
//! Verifies that all 7 fields are preserved through serialization and
//! deserialization.

#![forbid(unsafe_code)]

use vb_core::action::ActionTicket;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

#[test]
fn test_action_ticket_postcard_roundtrip_all_fields() {
    let ticket = ActionTicket {
        run: RunId::new(0x0102_0304_0506_0708),
        step: StepIdx::new(0x1011),
        seq: SeqNo::new(0x2021_2223_2425_2627),
        action: ActionId::new(0x3031),
        attempt: 42,
        idempotency_key: 0xAABB_CCDD_0011_2233_4455_6677_8899_AABB,
        capacity: 99,
        ..Default::default()
    };

    let buf = postcard::to_allocvec(&ticket).expect("ActionTicket serialization must succeed");
    let ticket2: ActionTicket =
        postcard::from_bytes(&buf).expect("ActionTicket deserialization must succeed");

    assert_eq!(
        ticket2.run.get(),
        ticket.run.get(),
        "run must be preserved through serialization"
    );
    assert_eq!(
        ticket2.step.get(),
        ticket.step.get(),
        "step must be preserved through serialization"
    );
    assert_eq!(
        ticket2.seq.get(),
        ticket.seq.get(),
        "seq must be preserved through serialization"
    );
    assert_eq!(
        ticket2.action.get(),
        ticket.action.get(),
        "action must be preserved through serialization"
    );
    assert_eq!(
        ticket2.attempt, ticket.attempt,
        "attempt must be preserved through serialization"
    );
    assert_eq!(
        ticket2.idempotency_key, ticket.idempotency_key,
        "idempotency_key must be preserved through serialization"
    );
    assert_eq!(
        ticket2.capacity, ticket.capacity,
        "capacity must be preserved through serialization"
    );
}

#[test]
fn test_action_ticket_postcard_roundtrip_zero_values() {
    let ticket = ActionTicket {
        run: RunId::new(0),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(0),
        attempt: 0,
        idempotency_key: 0,
        capacity: 0,
        ..Default::default()
    };

    let buf = postcard::to_allocvec(&ticket).expect("ActionTicket serialization must succeed");
    let ticket2: ActionTicket =
        postcard::from_bytes(&buf).expect("ActionTicket deserialization must succeed");

    assert_eq!(
        ticket2, ticket,
        "zero-value ticket must roundtrip identically"
    );
}

#[test]
fn test_action_ticket_postcard_roundtrip_max_values() {
    let ticket = ActionTicket {
        run: RunId::new(u64::MAX),
        step: StepIdx::new(u16::MAX),
        seq: SeqNo::new(u64::MAX),
        action: ActionId::new(u16::MAX),
        attempt: u16::MAX,
        idempotency_key: u128::MAX,
        capacity: u16::MAX,
        ..Default::default()
    };

    let buf = postcard::to_allocvec(&ticket).expect("ActionTicket serialization must succeed");
    let ticket2: ActionTicket =
        postcard::from_bytes(&buf).expect("ActionTicket deserialization must succeed");

    assert_eq!(
        ticket2.run.get(),
        u64::MAX,
        "max run must be preserved through serialization"
    );
    assert_eq!(
        ticket2.step.get(),
        u16::MAX,
        "max step must be preserved through serialization"
    );
    assert_eq!(
        ticket2.seq.get(),
        u64::MAX,
        "max seq must be preserved through serialization"
    );
    assert_eq!(
        ticket2.action.get(),
        u16::MAX,
        "max action must be preserved through serialization"
    );
    assert_eq!(
        ticket2.attempt,
        u16::MAX,
        "max attempt must be preserved through serialization"
    );
    assert_eq!(
        ticket2.idempotency_key,
        u128::MAX,
        "max idempotency_key must be preserved through serialization"
    );
    assert_eq!(
        ticket2.capacity,
        u16::MAX,
        "max capacity must be preserved through serialization"
    );
}

#[test]
fn test_action_ticket_postcard_roundtrip_determinism() {
    // Two tickets with different values serialize to different sizes (postcard
    // uses varint), but both must roundtrip correctly and produce deterministic
    // output for the same input.
    let small = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };
    let large = ActionTicket {
        run: RunId::new(u64::MAX),
        step: StepIdx::new(u16::MAX),
        seq: SeqNo::new(u64::MAX),
        action: ActionId::new(u16::MAX),
        attempt: u16::MAX,
        idempotency_key: u128::MAX,
        capacity: u16::MAX,
        ..Default::default()
    };

    let buf_small = postcard::to_allocvec(&small).expect("small ticket serialization must succeed");
    let buf_large = postcard::to_allocvec(&large).expect("large ticket serialization must succeed");

    // Postcard uses variable-length encoding; small values serialize smaller.
    assert!(
        buf_small.len() <= buf_large.len(),
        "small ticket must serialize to <= bytes as large ticket"
    );
    assert!(
        buf_small.len() > 0,
        "serialization must produce non-empty output"
    );

    // Both must roundtrip correctly.
    let restored_small: ActionTicket =
        postcard::from_bytes(&buf_small).expect("small roundtrip must succeed");
    let restored_large: ActionTicket =
        postcard::from_bytes(&buf_large).expect("large roundtrip must succeed");

    assert_eq!(
        restored_small, small,
        "small ticket must roundtrip faithfully"
    );
    assert_eq!(
        restored_large, large,
        "large ticket must roundtrip faithfully"
    );

    // Determinism: same input always produces same output.
    let buf_small_again =
        postcard::to_allocvec(&small).expect("determinism serialization must succeed");
    assert_eq!(
        buf_small, buf_small_again,
        "serialization must be deterministic (same input → same bytes)"
    );
}

#[test]
fn test_action_ticket_postcard_roundtrip_mixed_values() {
    // Test a variety of mixed values to catch encoding edge cases.
    let ticket = ActionTicket {
        run: RunId::new(0xDEAD_BEEF_CAFE_BABE),
        step: StepIdx::new(0x1234),
        seq: SeqNo::new(0x0000_0000_FFFF_FFFF),
        action: ActionId::new(0x00FF),
        attempt: 1,
        idempotency_key: 0x0000_0000_0000_0000_DEAD_BEEF_DEAD_BEEF,
        capacity: 1,
        ..Default::default()
    };

    let buf = postcard::to_allocvec(&ticket).expect("ActionTicket serialization must succeed");
    let ticket2: ActionTicket =
        postcard::from_bytes(&buf).expect("ActionTicket deserialization must succeed");

    assert_eq!(ticket2.run.get(), ticket.run.get(), "run must match");
    assert_eq!(ticket2.step.get(), ticket.step.get(), "step must match");
    assert_eq!(ticket2.seq.get(), ticket.seq.get(), "seq must match");
    assert_eq!(
        ticket2.action.get(),
        ticket.action.get(),
        "action must match"
    );
    assert_eq!(ticket2.attempt, ticket.attempt, "attempt must match");
    assert_eq!(
        ticket2.idempotency_key, ticket.idempotency_key,
        "idempotency_key must match"
    );
    assert_eq!(ticket2.capacity, ticket.capacity, "capacity must match");
}
