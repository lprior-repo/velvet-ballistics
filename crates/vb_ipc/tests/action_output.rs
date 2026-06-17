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
