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

//! Tests for frame_types.

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::IpcCommand;
    use crate::constants::IPC_HEADER_LEN;
    use bytes::Bytes;

    fn make_valid_header_bytes() -> [u8; IPC_HEADER_LEN] {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 42, 0);
        header.encode().expect("encode should succeed")
    }

    #[test]
    fn decode_rejects_invalid_magic() {
        let mut bytes = make_valid_header_bytes();
        bytes[0..4].copy_from_slice(&0xDEADBEEF_u32.to_le_bytes());

        let result = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT);
        assert_eq!(
            result,
            Err(IpcError::InvalidMagic {
                actual: 0xDEADBEEF_u32,
            })
        );
    }

    #[test]
    fn decode_rejects_unsupported_version() {
        let mut bytes = make_valid_header_bytes();
        bytes[4..6].copy_from_slice(&99u16.to_le_bytes());

        let result = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT);
        assert_eq!(result, Err(IpcError::UnsupportedVersion { actual: 99 }));
    }

    #[test]
    fn decode_rejects_nonzero_reserved_field() {
        let mut bytes = make_valid_header_bytes();
        bytes[10..12].copy_from_slice(&7u16.to_le_bytes());

        let result = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT);
        assert_eq!(result, Err(IpcError::ReservedNonZero { actual: 7 }));
    }

    #[test]
    fn decode_rejects_payload_too_large() {
        let mut bytes = make_valid_header_bytes();
        let limit = MaxPayloadBytes::DEFAULT.get() as u32;
        let oversized = limit.saturating_add(1);
        bytes[20..24].copy_from_slice(&oversized.to_le_bytes());

        let result = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT);
        assert_eq!(
            result,
            Err(IpcError::PayloadTooLarge {
                actual: oversized as usize,
                limit: MaxPayloadBytes::DEFAULT.get(),
            })
        );
    }

    #[test]
    fn new_rejects_payload_length_mismatch() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 10);
        let payload = Bytes::from(vec![0u8; 5]);

        let result = IpcFrame::new(header, payload, MaxPayloadBytes::DEFAULT);
        assert_eq!(
            result,
            Err(IpcError::PayloadLengthMismatch {
                header: 10,
                actual: 5,
            })
        );
    }

    #[test]
    fn header_getter_returns_expected_value() {
        let expected = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
        let frame = IpcFrame::new(expected, Bytes::new(), MaxPayloadBytes::DEFAULT)
            .expect("frame should build");

        assert_eq!(frame.header(), expected);
    }

    #[test]
    fn payload_getter_returns_expected_value() {
        let payload_data = vec![0xAB, 0xCD, 0xEF];
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, payload_data.len() as u32);
        let frame = IpcFrame::new(
            header,
            Bytes::from(payload_data.clone()),
            MaxPayloadBytes::DEFAULT,
        )
        .expect("frame should build");

        assert_eq!(frame.payload().bytes().as_ref(), payload_data.as_slice());
    }

    #[test]
    fn decode_frame_propagates_header_errors() {
        let mut bytes = make_valid_header_bytes();
        bytes[0..4].copy_from_slice(&0u32.to_le_bytes());

        let result = decode_frame(&bytes, Bytes::new(), MaxPayloadBytes::DEFAULT);
        assert_eq!(result, Err(IpcError::InvalidMagic { actual: 0 }));
    }

    #[test]
    fn decode_frame_succeeds_with_valid_header_and_payload() {
        let payload_data = vec![0x01, 0x02, 0x03];
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 7, payload_data.len() as u32);
        let header_bytes = header.encode().expect("encode should succeed");

        let result = decode_frame(
            &header_bytes,
            Bytes::from(payload_data.clone()),
            MaxPayloadBytes::DEFAULT,
        );
        let frame = result.expect("decode_frame should succeed");
        assert_eq!(frame.header(), header);
        assert_eq!(frame.payload().bytes().as_ref(), payload_data.as_slice());
    }
}
