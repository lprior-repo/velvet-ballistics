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
// JOURNALERROR TEST COVERAGE AUDIT
//
// Tested variants:
// - AdmissionRequired: admission_required_variant_and_display
// - ArtifactInvalid: artifact_invalid_variant_and_fields, artifact_invalid_display_format, artifact_invalid_error_code
// - InputTooLarge: input_too_large_variant_and_fields, input_too_large_display_format, input_too_large_error_code
// - InputSchemaMismatch: input_schema_mismatch_variant_and_display, input_schema_mismatch_error_code
// - CapabilityDenied: capability_denied_variant_and_display, capability_denied_error_code
// - SecretUnavailable: secret_unavailable_variant_and_display, secret_unavailable_error_code
// - RunAlreadyExists: run_already_exists_variant_and_display, run_already_exists_error_code
// - ActiveRunCapacityExceeded: active_run_capacity_exceeded_variant_and_display, active_run_capacity_exceeded_error_code
// - FrameAllocationFailed: frame_allocation_failed_variant_and_display, frame_allocation_failed_error_code
// - AdmissionJournalFailed: admission_journal_failed_variant_and_display, admission_journal_failed_error_code
// - TooManyEvents: too_many_events_variant_and_fields, too_many_events_display_format, too_many_events_error_code
// - ReplayAllocationFailed: replay_allocation_failed_variant_and_fields, replay_allocation_failed_display_format, replay_allocation_failed_error_code
// - ClockUnavailable: clock_unavailable_variant_and_display, clock_unavailable_error_code
// - InvalidGateCount: invalid_gate_count_variant_and_fields, invalid_gate_count_display_format, invalid_gate_count_error_code
// - MissingRequiredProofFlag: missing_required_proof_flag_variant_and_fields, missing_required_proof_flag_display_format, missing_required_proof_flag_error_code
//
// Untested variants:
// - Fjall: no direct test (requires fjall mock/integration)
// - Encode: no direct test (requires postcard mock)
// - KeyCapacity: no direct test
// - DuplicateEvent: no direct test
// - WriteLockPoisoned: no direct test
// - QueueCapacity: no direct test
// - QueueFull: no direct test
// - QueueShutdown: no direct test
// - WrongRun: no direct test
// - SequenceGap: no direct test
// - SequenceOverflow: no direct test
// - BadMagic: no direct test
// - UnsupportedSchemaVersion: no direct test
// - MigrationRequired: no direct test
// - UnknownRecordKind: no direct test
// - RecordKindFamilyMismatch: no direct test
// - HeaderLengthMismatch: no direct test
// - PayloadTooLarge: no direct test
// - HeaderChecksumMismatch: no direct test
// - PayloadDigestMismatch: no direct test
// - UnexpectedEof: no direct test
// - PostcardDecodeFailed: no direct test
// - InvalidEvent: no direct test
// - ArtifactMalformed: no direct test
// - ArtifactChecksumMismatch: no direct test
// - ArtifactNotFound: no direct test
// - InvalidRunId: no direct test
// - StrictDurabilityFailed: no direct test
// - ProcessLockHeld: no direct test
// - ProcessLockIo: no direct test
// - Trim: no direct test
//
mod error_tests {
    use crate::JournalError;
    use crate::error::ArtifactInvalidSource;
    use vb_core::{DiagnosticCode, RunId};

    // -----------------------------------------------------------------------
    // AdmissionRequired
    // -----------------------------------------------------------------------

    #[test]
    fn admission_required_variant_and_display() {
        let err = JournalError::AdmissionRequired;
        assert!(
            matches!(err, JournalError::AdmissionRequired),
            "variant must be AdmissionRequired"
        );
        let display = format!("{err}");
        assert!(
            display.contains("admission is required"),
            "Display must mention admission: {display}"
        );
        assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
    }

    // -----------------------------------------------------------------------
    // ArtifactInvalid
    // -----------------------------------------------------------------------

    #[test]
    fn artifact_invalid_variant_and_fields() {
        let source = ArtifactInvalidSource::PayloadDigestMismatch;
        let err = JournalError::ArtifactInvalid { source };
        match err {
            JournalError::ArtifactInvalid { source: got } => {
                assert_eq!(got, ArtifactInvalidSource::PayloadDigestMismatch);
            }
            other => panic!("expected ArtifactInvalid, got {other:?}"),
        }
    }

    #[test]
    fn artifact_invalid_display_format() {
        let source = ArtifactInvalidSource::PayloadDigestMismatch;
        let err = JournalError::ArtifactInvalid { source };
        let display = format!("{err}");
        assert!(
            display.contains("artifact invalid"),
            "Display must mention artifact invalid: {display}"
        );
        assert!(
            display.contains("PayloadDigestMismatch"),
            "Display must contain source variant: {display}"
        );
    }

    #[test]
    fn artifact_invalid_error_code() {
        let err = JournalError::ArtifactInvalid {
            source: ArtifactInvalidSource::PayloadDigestMismatch,
        };
        assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
    }

    // -----------------------------------------------------------------------
    // InputTooLarge
    // -----------------------------------------------------------------------

    #[test]
    fn input_too_large_variant_and_fields() {
        let err = JournalError::InputTooLarge {
            len: 1024,
            max: 512,
        };
        match err {
            JournalError::InputTooLarge { len, max } => {
                assert_eq!(len, 1024);
                assert_eq!(max, 512);
            }
            other => panic!("expected InputTooLarge, got {other:?}"),
        }
    }

    #[test]
    fn input_too_large_display_format() {
        let err = JournalError::InputTooLarge { len: 999, max: 100 };
        let display = format!("{err}");
        assert!(
            display.contains("input too large"),
            "Display must mention input too large: {display}"
        );
        assert!(
            display.contains("999") && display.contains("100"),
            "Display must contain both len and max: {display}"
        );
    }

    #[test]
    fn input_too_large_error_code() {
        let err = JournalError::InputTooLarge { len: 1, max: 0 };
        assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
    }

    // -----------------------------------------------------------------------
    // InputSchemaMismatch
    // -----------------------------------------------------------------------

    #[test]
    fn input_schema_mismatch_variant_and_display() {
        let err = JournalError::InputSchemaMismatch;
        assert!(
            matches!(err, JournalError::InputSchemaMismatch),
            "variant must be InputSchemaMismatch"
        );
        let display = format!("{err}");
        assert!(
            display.contains("schema mismatch"),
            "Display must mention schema mismatch: {display}"
        );
    }

    #[test]
    fn input_schema_mismatch_error_code() {
        let err = JournalError::InputSchemaMismatch;
        assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
    }

    // -----------------------------------------------------------------------
    // CapabilityDenied
    // -----------------------------------------------------------------------

    #[test]
    fn capability_denied_variant_and_display() {
        let err = JournalError::CapabilityDenied;
        assert!(
            matches!(err, JournalError::CapabilityDenied),
            "variant must be CapabilityDenied"
        );
        let display = format!("{err}");
        assert!(
            display.contains("capability denied"),
            "Display must mention capability denied: {display}"
        );
    }

    #[test]
    fn capability_denied_error_code() {
        let err = JournalError::CapabilityDenied;
        assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
    }

    // -----------------------------------------------------------------------
    // SecretUnavailable
    // -----------------------------------------------------------------------

    #[test]
    fn secret_unavailable_variant_and_display() {
        let err = JournalError::SecretUnavailable;
        assert!(
            matches!(err, JournalError::SecretUnavailable),
            "variant must be SecretUnavailable"
        );
        let display = format!("{err}");
        assert!(
            display.contains("secret unavailable"),
            "Display must mention secret unavailable: {display}"
        );
    }

    #[test]
    fn secret_unavailable_error_code() {
        let err = JournalError::SecretUnavailable;
        assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
    }

    // -----------------------------------------------------------------------
    // RunAlreadyExists
    // -----------------------------------------------------------------------

    #[test]
    fn run_already_exists_variant_and_display() {
        let err = JournalError::RunAlreadyExists;
        assert!(
            matches!(err, JournalError::RunAlreadyExists),
            "variant must be RunAlreadyExists"
        );
        let display = format!("{err}");
        assert!(
            display.contains("run already exists"),
            "Display must mention run already exists: {display}"
        );
    }

    #[test]
    fn run_already_exists_error_code() {
        let err = JournalError::RunAlreadyExists;
        assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
    }

    // -----------------------------------------------------------------------
    // ActiveRunCapacityExceeded
    // -----------------------------------------------------------------------

    #[test]
    fn active_run_capacity_exceeded_variant_and_display() {
        let err = JournalError::ActiveRunCapacityExceeded;
        assert!(
            matches!(err, JournalError::ActiveRunCapacityExceeded),
            "variant must be ActiveRunCapacityExceeded"
        );
        let display = format!("{err}");
        assert!(
            display.contains("active run capacity exceeded"),
            "Display must mention capacity exceeded: {display}"
        );
    }

    #[test]
    fn active_run_capacity_exceeded_error_code() {
        let err = JournalError::ActiveRunCapacityExceeded;
        assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
    }

    // -----------------------------------------------------------------------
    // FrameAllocationFailed
    // -----------------------------------------------------------------------

    #[test]
    fn frame_allocation_failed_variant_and_display() {
        let err = JournalError::FrameAllocationFailed;
        assert!(
            matches!(err, JournalError::FrameAllocationFailed),
            "variant must be FrameAllocationFailed"
        );
        let display = format!("{err}");
        assert!(
            display.contains("frame allocation failed"),
            "Display must mention frame allocation failed: {display}"
        );
    }

    #[test]
    fn frame_allocation_failed_error_code() {
        let err = JournalError::FrameAllocationFailed;
        assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
    }

    // -----------------------------------------------------------------------
    // AdmissionJournalFailed
    // -----------------------------------------------------------------------

    #[test]
    fn admission_journal_failed_variant_and_display() {
        let err = JournalError::AdmissionJournalFailed;
        assert!(
            matches!(err, JournalError::AdmissionJournalFailed),
            "variant must be AdmissionJournalFailed"
        );
        let display = format!("{err}");
        assert!(
            display.contains("admission journal failed"),
            "Display must mention admission journal failed: {display}"
        );
    }

    #[test]
    fn admission_journal_failed_error_code() {
        let err = JournalError::AdmissionJournalFailed;
        assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
    }

    // -----------------------------------------------------------------------
    // TooManyEvents
    // -----------------------------------------------------------------------

    #[test]
    fn too_many_events_variant_and_fields() {
        let run = RunId::new(42);
        let err = JournalError::TooManyEvents {
            run,
            limit: 100,
            observed: 200,
        };
        match err {
            JournalError::TooManyEvents {
                run: got_run,
                limit: got_limit,
                observed: got_observed,
            } => {
                assert_eq!(got_run, RunId::new(42));
                assert_eq!(got_limit, 100);
                assert_eq!(got_observed, 200);
            }
            other => panic!("expected TooManyEvents, got {other:?}"),
        }
    }

    #[test]
    fn too_many_events_display_format() {
        let run = RunId::new(7);
        let err = JournalError::TooManyEvents {
            run,
            limit: 50,
            observed: 150,
        };
        let display = format!("{err}");
        assert!(
            display.contains("exceeded event limit"),
            "Display must mention exceeded event limit: {display}"
        );
        assert!(
            display.contains("observed") && display.contains("limit"),
            "Display must contain observed and limit: {display}"
        );
    }

    #[test]
    fn too_many_events_error_code() {
        let err = JournalError::TooManyEvents {
            run: RunId::new(1),
            limit: 10,
            observed: 20,
        };
        assert_eq!(err.diagnostic_code(), JournalError::TOO_MANY_EVENTS_CODE,);
        assert_eq!(
            JournalError::TOO_MANY_EVENTS_CODE,
            DiagnosticCode::new(0x401E),
        );
    }

    // -----------------------------------------------------------------------
    // ReplayAllocationFailed
    // -----------------------------------------------------------------------

    #[test]
    fn replay_allocation_failed_variant_and_fields() {
        let run = RunId::new(99);
        let err = JournalError::ReplayAllocationFailed {
            run,
            requested: 1024,
        };
        match err {
            JournalError::ReplayAllocationFailed {
                run: got_run,
                requested: got_requested,
            } => {
                assert_eq!(got_run, RunId::new(99));
                assert_eq!(got_requested, 1024);
            }
            other => panic!("expected ReplayAllocationFailed, got {other:?}"),
        }
    }

    #[test]
    fn replay_allocation_failed_display_format() {
        let run = RunId::new(3);
        let err = JournalError::ReplayAllocationFailed {
            run,
            requested: 500,
        };
        let display = format!("{err}");
        assert!(
            display.contains("allocation failed"),
            "Display must mention allocation failed: {display}"
        );
        assert!(
            display.contains("requested"),
            "Display must mention requested: {display}"
        );
    }

    #[test]
    fn replay_allocation_failed_error_code() {
        let err = JournalError::ReplayAllocationFailed {
            run: RunId::new(1),
            requested: 10,
        };
        assert_eq!(
            err.diagnostic_code(),
            JournalError::REPLAY_ALLOCATION_FAILED_CODE,
        );
        assert_eq!(
            JournalError::REPLAY_ALLOCATION_FAILED_CODE,
            DiagnosticCode::new(0x401F),
        );
    }

    // -----------------------------------------------------------------------
    // ClockUnavailable
    // -----------------------------------------------------------------------

    #[test]
    fn clock_unavailable_variant_and_display() {
        let err = JournalError::ClockUnavailable;
        assert!(
            matches!(err, JournalError::ClockUnavailable),
            "variant must be ClockUnavailable"
        );
        let display = format!("{err}");
        assert!(
            display.contains("clock unavailable"),
            "Display must mention clock unavailable: {display}"
        );
    }

    #[test]
    fn clock_unavailable_error_code() {
        let err = JournalError::ClockUnavailable;
        assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
    }

    // -----------------------------------------------------------------------
    // InvalidGateCount
    // -----------------------------------------------------------------------

    #[test]
    fn invalid_gate_count_variant_and_fields() {
        let err = JournalError::InvalidGateCount { found: 42 };
        match err {
            JournalError::InvalidGateCount { found } => {
                assert_eq!(found, 42);
            }
            other => panic!("expected InvalidGateCount, got {other:?}"),
        }
    }

    #[test]
    fn invalid_gate_count_display_format() {
        let err = JournalError::InvalidGateCount { found: 7 };
        let display = format!("{err}");
        assert!(
            display.contains("invalid gate count"),
            "Display must mention invalid gate count: {display}"
        );
        assert!(
            display.contains("7"),
            "Display must contain the found value: {display}"
        );
    }

    #[test]
    fn invalid_gate_count_error_code() {
        let err = JournalError::InvalidGateCount { found: 1 };
        assert_eq!(err.diagnostic_code(), JournalError::INVALID_GATE_COUNT_CODE,);
        assert_eq!(
            JournalError::INVALID_GATE_COUNT_CODE,
            DiagnosticCode::new(0x401C),
        );
    }

    // -----------------------------------------------------------------------
    // MissingRequiredProofFlag
    // -----------------------------------------------------------------------

    #[test]
    fn missing_required_proof_flag_variant_and_fields() {
        let err = JournalError::MissingRequiredProofFlag {
            flag: "contract_seal",
        };
        match err {
            JournalError::MissingRequiredProofFlag { flag } => {
                assert_eq!(flag, "contract_seal");
            }
            other => panic!("expected MissingRequiredProofFlag, got {other:?}"),
        }
    }

    #[test]
    fn missing_required_proof_flag_display_format() {
        let err = JournalError::MissingRequiredProofFlag {
            flag: "integrity_chain",
        };
        let display = format!("{err}");
        assert!(
            display.contains("missing required proof flag"),
            "Display must mention missing required proof flag: {display}"
        );
        assert!(
            display.contains("integrity_chain"),
            "Display must contain the flag name: {display}"
        );
    }

    #[test]
    fn missing_required_proof_flag_error_code() {
        let err = JournalError::MissingRequiredProofFlag { flag: "test_flag" };
        assert_eq!(
            err.diagnostic_code(),
            JournalError::MISSING_REQUIRED_PROOF_FLAG_CODE,
        );
        assert_eq!(
            JournalError::MISSING_REQUIRED_PROOF_FLAG_CODE,
            DiagnosticCode::new(0x401D),
        );
    }
}
