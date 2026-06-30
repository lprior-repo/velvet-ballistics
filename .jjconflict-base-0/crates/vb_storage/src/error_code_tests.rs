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
    use crate::{EventSeq, JournalError, TrimError, recovery::RecoveryError};
    use vb_core::{DiagnosticCode, RunId, WorkflowDigest};

    #[test]
    fn fjall_code_is_correct() {
        assert_eq!(JournalError::FJALL_CODE, DiagnosticCode::new(0x4001));
    }

    #[test]
    fn encode_code_is_correct() {
        assert_eq!(JournalError::ENCODE_CODE, DiagnosticCode::new(0x4002));
    }

    #[test]
    fn key_capacity_code_is_correct() {
        assert_eq!(JournalError::KEY_CAPACITY_CODE, DiagnosticCode::new(0x4003));
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
        assert_eq!(JournalError::QUEUE_FULL_CODE, DiagnosticCode::new(0x4007));
    }

    #[test]
    fn wrong_run_code_is_correct() {
        assert_eq!(JournalError::WRONG_RUN_CODE, DiagnosticCode::new(0x4008));
    }

    #[test]
    fn sequence_gap_code_is_correct() {
        assert_eq!(JournalError::SEQUENCE_GAP_CODE, DiagnosticCode::new(0x4009));
    }

    #[test]
    fn bad_magic_code_is_correct() {
        assert_eq!(JournalError::BAD_MAGIC_CODE, DiagnosticCode::new(0x400B));
    }

    #[test]
    fn payload_digest_mismatch_code_is_correct() {
        assert_eq!(
            JournalError::PAYLOAD_DIGEST_MISMATCH_CODE,
            DiagnosticCode::new(0x4013)
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
        let err = JournalError::PayloadTooLarge {
            len: 1000,
            max: 500,
        };
        assert_eq!(err.diagnostic_code(), JournalError::PAYLOAD_TOO_LARGE_CODE);
    }

    #[test]
    fn postcard_decode_failed_error_has_correct_code() {
        let err = JournalError::PostcardDecodeFailed(postcard::Error::DeserializeBadVarint);
        assert_eq!(
            err.diagnostic_code(),
            JournalError::POSTCARD_DECODE_FAILED_CODE
        );
    }

    #[test]
    fn unexpected_eof_error_has_correct_code() {
        let err = JournalError::UnexpectedEof;
        assert_eq!(err.diagnostic_code(), JournalError::UNEXPECTED_EOF_CODE);
    }

    #[test]
    fn recovery_error_no_recovery_data_displays_correctly() {
        let err = RecoveryError::NoRecoveryData { run: RunId::new(7) };
        let msg = format!("{err}");
        assert!(
            msg.contains("no recovery data"),
            "message should mention recovery: {msg}"
        );
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
        assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_NOT_FOUND_CODE);
    }

    #[test]
    fn trim_error_diagnostic_codes_are_correct() {
        assert_eq!(
            TrimError::NO_DURABLE_SNAPSHOT_CODE,
            DiagnosticCode::new(0x4101)
        );
        assert_eq!(TrimError::INCOMPLETE_TRIM_CODE, DiagnosticCode::new(0x4102));
        assert_eq!(
            TrimError::RETENTION_POLICY_BLOCKS_CODE,
            DiagnosticCode::new(0x4103)
        );
    }

    // -----------------------------------------------------------------
    // vb-1rqz7.28 / SC-009: `JournalError::Trim(inner)` must delegate
    // the diagnostic code to the wrapped `TrimError`, so a
    // journal-wrapped trim error still surfaces its underlying
    // failure code (e.g. INCOMPLETE_TRIM, WRONG_RUN) instead of
    // collapsing to the generic FJALL fallback.
    // -----------------------------------------------------------------

    /// `JournalError::Trim(TrimError::Journal(inner))` must propagate
    /// the inner journal diagnostic code through the trim wrapper.
    #[test]
    fn journal_error_trim_wrapper_delegates_to_inner_journal_error_code() {
        let inner = JournalError::WrongRun {
            expected: RunId::new(1),
            actual: RunId::new(2),
        };
        let inner_code = inner.diagnostic_code();
        let wrapped: JournalError = JournalError::Trim(Box::new(TrimError::Journal(inner)));
        assert_eq!(
            wrapped.diagnostic_code(),
            inner_code,
            "JournalError::Trim(TrimError::Journal(inner)) must delegate to inner.diagnostic_code()"
        );
        assert_ne!(
            wrapped.diagnostic_code(),
            JournalError::FJALL_CODE,
            "delegation must not collapse to the generic FJALL_CODE fallback"
        );
    }

    /// `JournalError::Trim(TrimError::IncompleteTrim { .. })` must
    /// surface the `INCOMPLETE_TRIM_CODE` directly rather than the
    /// generic `FJALL_CODE`. This is the code path exercised by
    /// `count_trimmable_events` when it observes a malformed key.
    #[test]
    fn journal_error_trim_wrapper_delegates_incomplete_trim_code() {
        let inner = TrimError::IncompleteTrim { deleted_count: 0 };
        let wrapped: JournalError = JournalError::Trim(Box::new(inner));
        assert_eq!(
            wrapped.diagnostic_code(),
            TrimError::INCOMPLETE_TRIM_CODE,
            "JournalError::Trim(IncompleteTrim) must surface INCOMPLETE_TRIM_CODE"
        );
        assert_ne!(
            wrapped.diagnostic_code(),
            JournalError::FJALL_CODE,
            "delegation must not collapse to FJALL_CODE"
        );
    }

    /// `JournalError::Trim(TrimError::NoDurableSnapshot { .. })` must
    /// surface the `NO_DURABLE_SNAPSHOT_CODE`.
    #[test]
    fn journal_error_trim_wrapper_delegates_no_durable_snapshot_code() {
        let inner = TrimError::NoDurableSnapshot { run: RunId::new(7) };
        let wrapped: JournalError = JournalError::Trim(Box::new(inner));
        assert_eq!(
            wrapped.diagnostic_code(),
            TrimError::NO_DURABLE_SNAPSHOT_CODE,
            "JournalError::Trim(NoDurableSnapshot) must surface NO_DURABLE_SNAPSHOT_CODE"
        );
    }

    /// `JournalError::Trim(TrimError::RetentionPolicyBlocks { .. })` must
    /// surface the `RETENTION_POLICY_BLOCKS_CODE`.
    #[test]
    fn journal_error_trim_wrapper_delegates_retention_policy_code() {
        let inner = TrimError::RetentionPolicyBlocks {
            run: RunId::new(11),
        };
        let wrapped: JournalError = JournalError::Trim(Box::new(inner));
        assert_eq!(
            wrapped.diagnostic_code(),
            TrimError::RETENTION_POLICY_BLOCKS_CODE,
            "JournalError::Trim(RetentionPolicyBlocks) must surface RETENTION_POLICY_BLOCKS_CODE"
        );
    }

    /// `JournalError::symbolic_code()` for `Self::Trim(inner)` must
    /// resolve to the inner's registered symbolic code, not
    /// `INTERNAL_INVARIANT`.
    #[test]
    fn journal_error_trim_wrapper_symbolic_code_is_inner() {
        use vb_core::SymbolicCode;

        let inner = JournalError::SequenceGap {
            expected: EventSeq::new(1),
            actual: EventSeq::new(3),
        };
        let inner_symbolic = inner.symbolic_code();
        let wrapped: JournalError = JournalError::Trim(Box::new(TrimError::Journal(inner)));
        let wrapped_symbolic = wrapped.symbolic_code();
        assert_eq!(
            wrapped_symbolic, inner_symbolic,
            "JournalError::Trim(TrimError::Journal(inner)).symbolic_code() must equal inner.symbolic_code()"
        );
        assert_ne!(
            wrapped_symbolic,
            SymbolicCode::INTERNAL_INVARIANT,
            "delegation must not collapse to INTERNAL_INVARIANT"
        );
    }
}
