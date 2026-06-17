#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
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
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cmp_owned,
    clippy::derivable_impls,
    clippy::enum_variant_names,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables,
)]

#![cfg(test)]

use proptest::prelude::*;
use std::io::Cursor;
use std::num::NonZeroUsize;
use vb_ipc::{
    IpcCommand, IpcError, MaxPayloadBytes, encode_frame, read_frame_header_bounded,
    read_frame_payload_bounded,
};

const DEFAULT_MAX_IPC_PAYLOAD_BYTES: usize = 1_048_576;

// =========================================================================
// OBL-IPC-001: IPC frame rejected over payload limit
// =========================================================================
// Given: IPC frame with payload > max_ipc_payload_bytes
// When: Frame decoded
// Then: Decode returns payload size error
#[test]
fn test_ipc_frame_rejected_over_payload_limit() -> Result<(), IpcError> {
    let limit = MaxPayloadBytes::DEFAULT;
    let over_limit =
        DEFAULT_MAX_IPC_PAYLOAD_BYTES
            .checked_add(1)
            .ok_or(IpcError::PayloadTooLarge {
                actual: DEFAULT_MAX_IPC_PAYLOAD_BYTES,
                limit: DEFAULT_MAX_IPC_PAYLOAD_BYTES,
            })?;
    let payload = vec![0u8; over_limit];
    let frame = encode_frame(IpcCommand::Health, 0, 42, &payload)?;
    let mut reader = Cursor::new(frame);

    let decoded = read_frame_header_bounded(&mut reader, limit);

    assert_eq!(
        decoded,
        Err(IpcError::PayloadTooLarge {
            actual: over_limit,
            limit: DEFAULT_MAX_IPC_PAYLOAD_BYTES,
        })
    );
    Ok(())
}

// =========================================================================
// OBL-IPC-002: IPC frame accepted at payload limit
// =========================================================================
// Given: IPC frame with payload == max_ipc_payload_bytes
// When: Frame decoded
// Then: Decode succeeds
#[test]
fn test_ipc_frame_accepted_at_payload_limit() -> Result<(), IpcError> {
    let at_limit = DEFAULT_MAX_IPC_PAYLOAD_BYTES;
    let payload = vec![0u8; at_limit];
    let frame = encode_frame(IpcCommand::Health, 0, 42, &payload)?;
    let mut reader = Cursor::new(frame);

    let header = read_frame_header_bounded(&mut reader, MaxPayloadBytes::DEFAULT)?;
    let decoded_payload =
        read_frame_payload_bounded(&mut reader, &header, MaxPayloadBytes::DEFAULT)?;

    assert_eq!(decoded_payload.len(), at_limit);
    Ok(())
}

proptest! {
    #[test]
    fn proptest_ipc_header_rejects_payloads_above_explicit_limit(limit in 1usize..64, extra in 1usize..16) {
        let Some(non_zero_limit) = NonZeroUsize::new(limit) else {
            return Ok(());
        };
        let max_payload = MaxPayloadBytes::new(non_zero_limit);
        let Some(over_limit) = limit.checked_add(extra) else {
            return Ok(());
        };
        let payload = vec![0u8; over_limit];
        let frame = encode_frame(IpcCommand::Health, 0, 42, &payload)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let mut reader = Cursor::new(frame);

        prop_assert_eq!(
            read_frame_header_bounded(&mut reader, max_payload),
            Err(IpcError::PayloadTooLarge {
                actual: over_limit,
                limit,
            })
        );
    }

    #[test]
    fn proptest_ipc_payload_at_explicit_limit_decodes_without_truncation(limit in 1usize..64) {
        let Some(non_zero_limit) = NonZeroUsize::new(limit) else {
            return Ok(());
        };
        let max_payload = MaxPayloadBytes::new(non_zero_limit);
        let payload = vec![0xA5u8; limit];
        let frame = encode_frame(IpcCommand::Health, 0, 42, &payload)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let mut reader = Cursor::new(frame);

        let header = read_frame_header_bounded(&mut reader, max_payload)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let decoded = read_frame_payload_bounded(&mut reader, &header, max_payload)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;

        prop_assert_eq!(decoded.len(), limit);
        prop_assert_eq!(decoded, payload);
    }
}
