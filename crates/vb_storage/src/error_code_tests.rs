#![allow(
    unused_imports,
    dead_code,
    clippy::assertions_on_constants,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used,
    clippy::let_underscore_must_use,
    clippy::len_zero,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::needless_return,
    clippy::needless_bool,
    clippy::single_match,
    clippy::single_match_else,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_locals,
    clippy::manual_let_else,
    clippy::or_fun_call,
    clippy::needless_borrow,
    clippy::needless_pass_by_value,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::module_inception,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::uninlined_format_args,
    clippy::large_digit_groups,
    clippy::unreadable_literal,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::vec_init_then_push,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::trivially_copy_pass_by_ref,
    clippy::wildcard_imports,
    clippy::wrong_self_convention,
    clippy::needless_range_loop,
    clippy::nonminimal_bool,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::should_implement_trait,
    clippy::result_large_err,
    clippy::missing_const_for_fn,
    clippy::use_self,
    clippy::items_after_statements,
    clippy::option_if_let_else,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::comparison_chain,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::explicit_counter_loop,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::needless_update,
    clippy::let_and_return,
    clippy::manual_div_ceil,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::match_like_matches_macro,
    clippy::wildcard_enum_match_arm,
    clippy::large_types_passed_by_value,
    clippy::large_futures,
    clippy::type_complexity,
    clippy::needless_collect,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::suspicious_operation_groupings,
    clippy::field_reassign_with_default,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::borrow_deref_ref,
    clippy::cloned_ref_to_slice_refs,
    clippy::inefficient_to_string,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::get_first,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::implicit_saturating_sub,
    clippy::unwrap_or_default,
    clippy::default_trait_access
)]

#![forbid(unsafe_code)]
#[cfg(test)]
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod error_code_tests {
    use crate::{
        EventSeq, JournalError, TrimError,
        recovery::RecoveryError,
    };
    use vb_core::{DiagnosticCode, RunId, WorkflowDigest};

    #[test]
    fn fjall_code_is_correct() {
        assert_eq!(
            JournalError::FJALL_CODE,
            DiagnosticCode::new(0x4001)
        );
    }

    #[test]
    fn encode_code_is_correct() {
        assert_eq!(
            JournalError::ENCODE_CODE,
            DiagnosticCode::new(0x4002)
        );
    }

    #[test]
    fn key_capacity_code_is_correct() {
        assert_eq!(
            JournalError::KEY_CAPACITY_CODE,
            DiagnosticCode::new(0x4003)
        );
    }

    #[test]
    fn duplicate_event_code_is_correct() {
        assert_eq!(
            JournalError::DUPLICATE_EVENT_CODE,
            DiagnosticCode::new(0x4004)
        );
    }

    #[test]
    fn write_lock_poisoned_code_is_correct() {
        assert_eq!(
            JournalError::WRITE_LOCK_POISONED_CODE,
            DiagnosticCode::new(0x4005)
        );
    }

    #[test]
    fn queue_full_code_is_correct() {
        assert_eq!(
            JournalError::QUEUE_FULL_CODE,
            DiagnosticCode::new(0x4007)
        );
    }

    #[test]
    fn wrong_run_code_is_correct() {
        assert_eq!(
            JournalError::WRONG_RUN_CODE,
            DiagnosticCode::new(0x4008)
        );
    }

    #[test]
    fn sequence_gap_code_is_correct() {
        assert_eq!(
            JournalError::SEQUENCE_GAP_CODE,
            DiagnosticCode::new(0x4009)
        );
    }

    #[test]
    fn bad_magic_code_is_correct() {
        assert_eq!(
            JournalError::BAD_MAGIC_CODE,
            DiagnosticCode::new(0x400B)
        );
    }

    #[test]
    fn payload_digest_mismatch_code_is_correct() {
        assert_eq!(
            JournalError::PAYLOAD_DIGEST_MISMATCH_CODE,
            DiagnosticCode::new(0x4011)
        );
    }

    #[test]
    fn duplicate_event_error_has_correct_diagnostic_code() {
        let err = JournalError::DuplicateEvent {
            run: RunId::new(1),
            seq: EventSeq::new(0),
        };
        assert_eq!(err.diagnostic_code(), JournalError::DUPLICATE_EVENT_CODE);
    }

    #[test]
    fn wrong_run_error_has_correct_diagnostic_code() {
        let err = JournalError::WrongRun {
            expected: RunId::new(1),
            actual: RunId::new(2),
        };
        assert_eq!(err.diagnostic_code(), JournalError::WRONG_RUN_CODE);
    }

    #[test]
    fn sequence_gap_error_has_correct_diagnostic_code() {
        let err = JournalError::SequenceGap {
            expected: EventSeq::new(1),
            actual: EventSeq::new(3),
        };
        assert_eq!(err.diagnostic_code(), JournalError::SEQUENCE_GAP_CODE);
    }

    #[test]
    fn bad_magic_error_has_correct_diagnostic_code() {
        let err = JournalError::BadMagic { found: 0xDEAD_BEEF };
        assert_eq!(err.diagnostic_code(), JournalError::BAD_MAGIC_CODE);
    }

    #[test]
    fn payload_digest_mismatch_error_has_correct_diagnostic_code() {
        let err = JournalError::PayloadDigestMismatch;
        assert_eq!(
            err.diagnostic_code(),
            JournalError::PAYLOAD_DIGEST_MISMATCH_CODE
        );
    }

    #[test]
    fn unsupported_schema_version_error_has_correct_code() {
        let err = JournalError::UnsupportedSchemaVersion { version: 99 };
        assert_eq!(
            err.diagnostic_code(),
            JournalError::UNSUPPORTED_SCHEMA_VERSION_CODE
        );
    }

    #[test]
    fn header_length_mismatch_error_has_correct_code() {
        let err = JournalError::HeaderLengthMismatch { found: 99 };
        assert_eq!(
            err.diagnostic_code(),
            JournalError::HEADER_LENGTH_MISMATCH_CODE
        );
    }

    #[test]
    fn header_checksum_mismatch_error_has_correct_code() {
        let err = JournalError::HeaderChecksumMismatch;
        assert_eq!(
            err.diagnostic_code(),
            JournalError::HEADER_CHECKSUM_MISMATCH_CODE
        );
    }

    #[test]
    fn payload_too_large_error_has_correct_code() {
        let err = JournalError::PayloadTooLarge { len: 1000, max: 500 };
        assert_eq!(
            err.diagnostic_code(),
            JournalError::PAYLOAD_TOO_LARGE_CODE
        );
    }

    #[test]
    fn postcard_decode_failed_error_has_correct_code() {
        let err = JournalError::PostcardDecodeFailed;
        assert_eq!(
            err.diagnostic_code(),
            JournalError::POSTCARD_DECODE_FAILED_CODE
        );
    }

    #[test]
    fn unexpected_eof_error_has_correct_code() {
        let err = JournalError::UnexpectedEof;
        assert_eq!(
            err.diagnostic_code(),
            JournalError::UNEXPECTED_EOF_CODE
        );
    }

    #[test]
    fn recovery_error_no_recovery_data_displays_correctly() {
        let err = RecoveryError::NoRecoveryData { run: RunId::new(7) };
        let msg = format!("{err}");
        assert!(msg.contains("no recovery data"), "message should mention recovery: {msg}");
    }

    #[test]
    fn recovery_error_digest_mismatch_displays_correctly() {
        let expected = WorkflowDigest::from_bytes([0x11; 32]);
        let found = WorkflowDigest::from_bytes([0x22; 32]);
        let err = RecoveryError::WorkflowSourceDigestMismatch { expected, found };
        let msg = format!("{err}");
        assert!(
            msg.contains("digest mismatch"),
            "message should mention digest mismatch: {msg}"
        );
    }

    #[test]
    fn journal_error_artifact_not_found_has_correct_code() {
        let err = JournalError::ArtifactNotFound {
            digest: WorkflowDigest::from_bytes([0; 32]),
        };
        assert_eq!(
            err.diagnostic_code(),
            JournalError::ARTIFACT_NOT_FOUND_CODE
        );
    }

    #[test]
    fn trim_error_diagnostic_codes_are_correct() {
        assert_eq!(
            TrimError::NO_DURABLE_SNAPSHOT_CODE,
            DiagnosticCode::new(0x4101)
        );
        assert_eq!(
            TrimError::INCOMPLETE_TRIM_CODE,
            DiagnosticCode::new(0x4102)
        );
        assert_eq!(
            TrimError::RETENTION_POLICY_BLOCKS_CODE,
            DiagnosticCode::new(0x4103)
        );
    }
}
