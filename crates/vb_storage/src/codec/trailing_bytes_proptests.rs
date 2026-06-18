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

use crate::{
    BlobRecord, CompiledIrRecord, EventSeq, JournalError, JournalEvent, WorkflowSourceRecord,
    constants::{
        DIGEST_BYTES, MAGIC_BLOB, MAGIC_COMPILED_ARTIFACT, MAGIC_JOURNAL_EVENT,
        MAGIC_WORKFLOW_SOURCE, MAX_BLOB_BYTES, MAX_COMPILED_IR_BYTES,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES, MAX_WORKFLOW_SOURCE_BYTES,
    },
    records::RecordKind,
};
use proptest::prelude::*;
use serde::de::DeserializeOwned;
use vb_core::{RunId, WorkflowDigest};

use super::{decode_journal_event, decode_record, encode_record};

fn typed_trailing_error_matches<T: DeserializeOwned>(
    mut bytes: Vec<u8>,
    expected_magic: u32,
    max_payload_len: u32,
    trailing: &[u8],
) -> bool {
    let declared_end = bytes.len();
    bytes.extend_from_slice(trailing);
    matches!(
        decode_record::<T>(&bytes, expected_magic, max_payload_len),
        Err(JournalError::UnexpectedTrailingBytes {
            declared_end: found_declared_end,
            actual_len,
        }) if found_declared_end == declared_end && actual_len == bytes.len()
    )
}

proptest! {
    #[test]
    fn vb_e7tl_trailing_bytes(
        run_value in 1_u64..=10_000,
        seq_value in 0_u64..=10_000,
        trailing in proptest::collection::vec(any::<u8>(), 1..=128),
    ) {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(run_value),
            seq: EventSeq::new(seq_value),
            attempt: 1,
            reason: None,
        };
        let mut bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            seq_value,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("valid event encodes");
        let declared_end = bytes.len();
        bytes.extend_from_slice(&trailing);

        let result = decode_record::<JournalEvent>(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );

        let exact_error = matches!(
            result,
            Err(JournalError::UnexpectedTrailingBytes {
                declared_end: found_declared_end,
                actual_len,
            }) if found_declared_end == declared_end && actual_len == bytes.len()
        );
        prop_assert!(exact_error, "trailing bytes return exact typed error");
    }

    #[test]
    fn vb_e7tl_call_site_propagation(
        run_value in 1_u64..=10_000,
        seq_value in 0_u64..=10_000,
        trailing in proptest::collection::vec(any::<u8>(), 1..=128),
    ) {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(run_value),
            seq: EventSeq::new(seq_value),
            attempt: 1,
            reason: None,
        };
        let mut bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            seq_value,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("valid event encodes");
        let declared_end = bytes.len();
        bytes.extend_from_slice(&trailing);

        let generic_result = decode_record::<JournalEvent>(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        let journal_result = decode_journal_event(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );

        let generic_exact_error = matches!(
            generic_result,
            Err(JournalError::UnexpectedTrailingBytes {
                declared_end: found_declared_end,
                actual_len,
            }) if found_declared_end == declared_end && actual_len == bytes.len()
        );
        let journal_exact_error = matches!(
            journal_result,
            Err(JournalError::UnexpectedTrailingBytes {
                declared_end: found_declared_end,
                actual_len,
            }) if found_declared_end == declared_end && actual_len == bytes.len()
        );
        prop_assert!(generic_exact_error, "generic decode propagates typed error");
        prop_assert!(journal_exact_error, "journal decode propagates typed error");
    }

    #[test]
    fn vb_e7tl_record_kind_magic_trailing_bytes(
        family in 0_u8..=3,
        seq_value in 0_u64..=10_000,
        payload in proptest::collection::vec(any::<u8>(), 0..=64),
        trailing in proptest::collection::vec(any::<u8>(), 1..=128),
    ) {
        let exact_error = match family {
            0 => {
                let event = JournalEvent::RunCancelled {
                    run: RunId::new(1),
                    seq: EventSeq::new(seq_value),
                    attempt: 1,
                    reason: None,
                };
                let bytes = encode_record(
                    MAGIC_JOURNAL_EVENT,
                    RecordKind::RunCancelled,
                    seq_value,
                    &event,
                    MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
                ).expect("valid journal event encodes");
                typed_trailing_error_matches::<JournalEvent>(
                    bytes,
                    MAGIC_JOURNAL_EVENT,
                    MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
                    &trailing,
                )
            }
            1 => {
                let record = WorkflowSourceRecord {
                    digest: WorkflowDigest::from_bytes([0x11; DIGEST_BYTES]),
                    source: payload.clone(),
                };
                let bytes = encode_record(
                    MAGIC_WORKFLOW_SOURCE,
                    RecordKind::WorkflowSource,
                    seq_value,
                    &record,
                    MAX_WORKFLOW_SOURCE_BYTES,
                ).expect("valid workflow source encodes");
                typed_trailing_error_matches::<WorkflowSourceRecord>(
                    bytes,
                    MAGIC_WORKFLOW_SOURCE,
                    MAX_WORKFLOW_SOURCE_BYTES,
                    &trailing,
                )
            }
            2 => {
                let record = CompiledIrRecord {
                    digest: WorkflowDigest::from_bytes([0x22; DIGEST_BYTES]),
                    ir: payload.clone(),
                    ..Default::default()
                };
                let bytes = encode_record(
                    MAGIC_COMPILED_ARTIFACT,
                    RecordKind::CompiledIr,
                    seq_value,
                    &record,
                    MAX_COMPILED_IR_BYTES,
                ).expect("valid compiled IR encodes");
                typed_trailing_error_matches::<CompiledIrRecord>(
                    bytes,
                    MAGIC_COMPILED_ARTIFACT,
                    MAX_COMPILED_IR_BYTES,
                    &trailing,
                )
            }
            _ => {
                let record = BlobRecord {
                    digest: [0x33; DIGEST_BYTES],
                    bytes: payload.clone(),
                };
                let bytes = encode_record(
                    MAGIC_BLOB,
                    RecordKind::Blob,
                    seq_value,
                    &record,
                    MAX_BLOB_BYTES,
                ).expect("valid blob encodes");
                typed_trailing_error_matches::<BlobRecord>(
                    bytes,
                    MAGIC_BLOB,
                    MAX_BLOB_BYTES,
                    &trailing,
                )
            }
        };
        prop_assert!(exact_error, "record-kind/magic trailing bytes return exact typed error");
    }
}
