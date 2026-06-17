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

//! Unit tests for [`IpcActionOutputPayload`].

use vb_core::ids::SlotIdx;
use vb_core::value::{SlotValue, Taint};
use vb_ipc::action_output::IpcActionOutputPayload;

fn sample_payload() -> IpcActionOutputPayload {
    IpcActionOutputPayload {
        output_slot: SlotIdx::new(3),
        value: SlotValue::I64(42),
        taint: Taint::Clean,
    }
}

#[test]
fn into_action_output_preserves_output_slot() {
    let payload = sample_payload();
    let action_output = payload.into_action_output(100);
    assert_eq!(action_output.output_slot, SlotIdx::new(3));
}

#[test]
fn into_action_output_preserves_value() {
    let payload = sample_payload();
    let action_output = payload.into_action_output(100);
    assert_eq!(action_output.value, SlotValue::I64(42));
}

#[test]
fn into_action_output_preserves_taint() {
    let payload = sample_payload();
    let action_output = payload.into_action_output(100);
    assert_eq!(action_output.taint, Taint::Clean);
}

#[test]
fn into_action_output_stores_encoded_len() {
    let payload = sample_payload();
    let action_output = payload.into_action_output(256);
    assert_eq!(action_output.encoded_len, 256);
}

#[test]
fn into_action_output_with_zero_encoded_len() {
    let payload = IpcActionOutputPayload {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::Null,
        taint: Taint::Clean,
    };
    let action_output = payload.into_action_output(0);
    assert_eq!(action_output.encoded_len, 0);
    assert_eq!(action_output.output_slot, SlotIdx::ZERO);
}

#[test]
fn into_action_output_with_max_encoded_len() {
    let payload = sample_payload();
    let action_output = payload.into_action_output(u32::MAX);
    assert_eq!(action_output.encoded_len, u32::MAX);
}

#[test]
fn into_action_output_with_secret_taint() {
    let payload = IpcActionOutputPayload {
        output_slot: SlotIdx::new(1),
        value: SlotValue::Bool(true),
        taint: Taint::Secret,
    };
    let action_output = payload.into_action_output(10);
    assert_eq!(action_output.taint, Taint::Secret);
}

#[test]
fn into_action_output_with_derived_from_secret_taint() {
    let payload = IpcActionOutputPayload {
        output_slot: SlotIdx::new(1),
        value: SlotValue::Null,
        taint: Taint::DerivedFromSecret,
    };
    let action_output = payload.into_action_output(5);
    assert_eq!(action_output.taint, Taint::DerivedFromSecret);
}

#[test]
fn postcard_roundtrip_with_null_value() {
    let payload = IpcActionOutputPayload {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::Null,
        taint: Taint::Clean,
    };
    let encoded =
        postcard::to_allocvec(&payload).expect("postcard encoding of null value must succeed");
    let decoded: IpcActionOutputPayload =
        postcard::from_bytes(&encoded).expect("postcard decoding of null value must succeed");
    assert_eq!(decoded.output_slot, payload.output_slot);
    assert_eq!(decoded.value, payload.value);
    assert_eq!(decoded.taint, payload.taint);
}

#[test]
fn postcard_roundtrip_with_bool_value() {
    let payload = IpcActionOutputPayload {
        output_slot: SlotIdx::new(5),
        value: SlotValue::Bool(false),
        taint: Taint::Clean,
    };
    let encoded =
        postcard::to_allocvec(&payload).expect("postcard encoding of bool value must succeed");
    let decoded: IpcActionOutputPayload =
        postcard::from_bytes(&encoded).expect("postcard decoding of bool value must succeed");
    assert_eq!(decoded.value, SlotValue::Bool(false));
    assert_eq!(decoded.output_slot, payload.output_slot);
}

#[test]
fn postcard_roundtrip_with_i64_value() {
    let payload = IpcActionOutputPayload {
        output_slot: SlotIdx::new(2),
        value: SlotValue::I64(-100),
        taint: Taint::DerivedFromSecret,
    };
    let encoded =
        postcard::to_allocvec(&payload).expect("postcard encoding of i64 value must succeed");
    let decoded: IpcActionOutputPayload =
        postcard::from_bytes(&encoded).expect("postcard decoding of i64 value must succeed");
    assert_eq!(decoded.value, SlotValue::I64(-100));
    assert_eq!(decoded.taint, Taint::DerivedFromSecret);
    assert_eq!(decoded.output_slot, payload.output_slot);
}

#[test]
fn ipc_action_output_payload_equality() {
    let a = IpcActionOutputPayload {
        output_slot: SlotIdx::new(1),
        value: SlotValue::Null,
        taint: Taint::Clean,
    };
    let b = IpcActionOutputPayload {
        output_slot: SlotIdx::new(1),
        value: SlotValue::Null,
        taint: Taint::Clean,
    };
    assert_eq!(a, b);
}

#[test]
fn ipc_action_output_payload_inequality_different_slot() {
    let a = IpcActionOutputPayload {
        output_slot: SlotIdx::new(1),
        value: SlotValue::Null,
        taint: Taint::Clean,
    };
    let b = IpcActionOutputPayload {
        output_slot: SlotIdx::new(2),
        value: SlotValue::Null,
        taint: Taint::Clean,
    };
    assert_ne!(a, b);
}

#[test]
fn ipc_action_output_payload_inequality_different_taint() {
    let a = IpcActionOutputPayload {
        output_slot: SlotIdx::new(1),
        value: SlotValue::Null,
        taint: Taint::Clean,
    };
    let b = IpcActionOutputPayload {
        output_slot: SlotIdx::new(1),
        value: SlotValue::Null,
        taint: Taint::Secret,
    };
    assert_ne!(a, b);
}
