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
    unused_variables
)]
#![forbid(unsafe_code)]

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use std::io::Cursor;
use std::num::NonZeroUsize;
use vb_ipc::{
    IPC_MAGIC, IPC_VERSION, IpcCommand, IpcError, IpcFrameHeader, MaxPayloadBytes,
    decode_frame_payload, read_frame_payload_bounded,
};
use vb_storage::constants::{
    CURRENT_SCHEMA_VERSION, MAGIC_JOURNAL_EVENT, RECORD_HEADER_BYTES, RECORD_HEADER_LEN,
};
use vb_storage::{
    JournalError, RecordKind, decode_record, decode_record_header, encode_record_header,
};

fn test_fail(message: String) -> TestCaseError {
    TestCaseError::fail(message)
}

fn storage_header(payload: &[u8], max: u32) -> Result<[u8; RECORD_HEADER_BYTES], TestCaseError> {
    encode_record_header(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        1,
        payload,
        max,
    )
    .map_err(|error| test_fail(format!("storage header fixture failed: {error:?}")))
}

fn ipc_header_bytes() -> Result<[u8; 24], TestCaseError> {
    IpcFrameHeader::new(IpcCommand::Health, 0, 7, 0)
        .encode()
        .map_err(|error| test_fail(format!("ipc header fixture failed: {error:?}")))
}

fn decode_ipc_header(bytes: &[u8; 24]) -> Result<IpcFrameHeader, TestCaseError> {
    IpcFrameHeader::decode(bytes, MaxPayloadBytes::DEFAULT)
        .map_err(|error| test_fail(format!("ipc header decode fixture failed: {error:?}")))
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn storage_decode_order_proptest(mut selector in 0_u8..8, bad in any::<u32>()) {
        selector %= 8;
        let payload = [0_u8];
        let mut header = storage_header(&payload, 8)?;
        match selector {
            0 => { let magic = if bad == MAGIC_JOURNAL_EVENT { 0 } else { bad }; write_u32(&mut header, 0, magic); let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::BadMagic { .. })); prop_assert!(ok); }
            1 => { write_u16(&mut header, 4, CURRENT_SCHEMA_VERSION.saturating_add(1)); let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::UnsupportedSchemaVersion { .. })); prop_assert!(ok); }
            2 => { write_u16(&mut header, 6, 9_000); let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::UnknownRecordKind { .. })); prop_assert!(ok); }
            3 => { write_u16(&mut header, 6, RecordKind::WorkflowSource.id()); let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::RecordKindFamilyMismatch { .. })); prop_assert!(ok); }
            4 => { write_u32(&mut header, 8, RECORD_HEADER_LEN.saturating_add(1)); let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::HeaderLengthMismatch { .. })); prop_assert!(ok); }
            5 => { write_u32(&mut header, 12, 9); let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::PayloadTooLarge { len: 9, max: 8 })); prop_assert!(ok); }
            6 => { write_u32(&mut header, 56, bad); let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::HeaderChecksumMismatch)); prop_assert!(ok); }
            _ => { let bad_postcard = [1_u8, 255_u8]; let header = storage_header(&bad_postcard, 8)?; let result = decode_record::<String>(&[header.as_slice(), bad_postcard.as_slice()].concat(), MAGIC_JOURNAL_EVENT, 8); let ok = matches!(result, Err(JournalError::PostcardDecodeFailed)); prop_assert!(ok); }
        }
    }

    #[test]
    fn storage_payload_too_large_precedes_read_property(len in 9_u32..4096) {
        let payload = [0_u8];
        let mut header = storage_header(&payload, 8)?;
        write_u32(&mut header, 12, len);
        let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::PayloadTooLarge { len: observed, max: 8 }) if observed == len);
        prop_assert!(ok);
    }

    #[test]
    fn storage_numeric_fields_are_observable(found in any::<u32>(), version in (CURRENT_SCHEMA_VERSION + 1)..u16::MAX, kind in 9000_u16..u16::MAX) {
        let payload = [0_u8];
        let mut header = storage_header(&payload, 8)?;
        write_u32(&mut header, 0, found);
        let expected = if found == MAGIC_JOURNAL_EVENT { 0 } else { found };
        write_u32(&mut header, 0, expected);
        let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::BadMagic { found: observed }) if observed == expected);
        prop_assert!(ok);
        let mut header = storage_header(&payload, 8)?;
        write_u16(&mut header, 4, version);
        let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::UnsupportedSchemaVersion { version: observed }) if observed == version);
        prop_assert!(ok);
        let mut header = storage_header(&payload, 8)?;
        write_u16(&mut header, 6, kind);
        let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::UnknownRecordKind { kind: observed }) if observed == kind);
        prop_assert!(ok);
    }

    #[test]
    // SEC-01 repurposed wire offset 10..12 from a hard-zero reserved field to
    // the caller-capabilities envelope. The original case 3 ("non-zero at
    // offset 10..12 → ReservedNonZero") is therefore obsolete: writing 1 at
    // that offset is the ROOT capability envelope and decode MUST accept it.
    // The `ReservedNonZero` error variant still exists in the IpcError enum
    // (with diagnostic code 0x3007) for forward compatibility, but the
    // proptest no longer exercises it. Coverage for the post-SEC-01 reserved
    // semantics lives in `restate_ipc_flag_matrix_tests.rs`.
    fn ipc_decode_order_proptest(selector in 0_u8..5, value in any::<u32>()) {
        let mut bytes = ipc_header_bytes()?;
        match selector {
            0 => { let magic = if value == IPC_MAGIC { 0 } else { value }; write_u32(&mut bytes, 0, magic); let ok = matches!(IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT), Err(IpcError::InvalidMagic { .. })); prop_assert!(ok); }
            1 => { write_u16(&mut bytes, 4, IPC_VERSION.saturating_add(1)); let ok = matches!(IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT), Err(IpcError::UnsupportedVersion { .. })); prop_assert!(ok); }
            2 => { write_u16(&mut bytes, 6, 9000); let ok = matches!(IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT), Ok(header) if header.command == IpcCommand::UnknownCommand(9000)); prop_assert!(ok); }
            3 => { write_u32(&mut bytes, 20, u32::MAX); let ok = matches!(IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT), Err(IpcError::PayloadTooLarge { .. })); prop_assert!(ok); }
            _ => { let header = decode_ipc_header(&bytes)?; let ok = matches!(decode_frame_payload(&header, &[255]), Err(IpcError::PayloadLengthMismatch { .. }) | Err(IpcError::PayloadDecodeFailed)); prop_assert!(ok); }
        }
    }

    #[test]
    fn ipc_payload_too_large_precedes_read_property(len in 2_u32..4096) {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, len);
        let mut cursor = Cursor::new(Vec::<u8>::new());
        let result = read_frame_payload_bounded(&mut cursor, &header, MaxPayloadBytes::new(NonZeroUsize::MIN));
        let ok = matches!(result, Err(IpcError::PayloadTooLarge { actual, limit: 1 }) if actual == len as usize);
        prop_assert!(ok);
    }
}

#[test]
fn ipc_header_constants_are_current_public_contract() {
    assert_eq!(IPC_MAGIC, 0x5642_4c54);
    assert_eq!(IPC_VERSION, 1);
}
