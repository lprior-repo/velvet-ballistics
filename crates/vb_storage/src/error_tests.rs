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
// - KeyCapacity: key_capacity_variant_and_display, key_capacity_error_code
// - WriteLockPoisoned: write_lock_poisoned_variant_and_display, write_lock_poisoned_error_code
// - QueueCapacity: queue_capacity_variant_and_display, queue_capacity_error_code
// - QueueFull: queue_full_variant_and_display, queue_full_error_code
// - QueueShutdown: queue_shutdown_variant_and_display, queue_shutdown_error_code
// - SequenceOverflow: sequence_overflow_variant_and_display, sequence_overflow_error_code
// - InvalidEvent: invalid_event_variant_and_display, invalid_event_error_code
// - ArtifactMalformed: artifact_malformed_variant_and_display, artifact_malformed_error_code
// - ArtifactChecksumMismatch: artifact_checksum_mismatch_variant_and_display, artifact_checksum_mismatch_error_code
// - InvalidRunId: invalid_run_id_variant_and_fields, invalid_run_id_display_format, invalid_run_id_error_code
// - StrictDurabilityFailed: strict_durability_failed_variant_and_display, strict_durability_failed_error_code
// - ProcessLockHeld: process_lock_held_variant_and_fields, process_lock_held_display_format, process_lock_held_error_code
// - ProcessLockIo: process_lock_io_variant_and_fields, process_lock_io_display_format, process_lock_io_error_code
// - DuplicateStagedKey: duplicate_staged_key_variant_and_fields, duplicate_staged_key_display_format, duplicate_staged_key_error_code
// - JournalBatchBytesExceeded: journal_batch_bytes_exceeded_variant_and_fields, journal_batch_bytes_exceeded_display_format, journal_batch_bytes_exceeded_error_code
// - BatchAborted: batch_aborted_variant_and_display, batch_aborted_error_code
// - ReplayKeyMismatch: replay_key_mismatch_variant_and_fields, replay_key_mismatch_display_format, replay_key_mismatch_error_code
// - ReplayEnvelopeSequenceMismatch: replay_envelope_sequence_mismatch_variant_and_fields, replay_envelope_sequence_mismatch_display_format, replay_envelope_sequence_mismatch_error_code
// - RecordKindPayloadMismatch: record_kind_payload_mismatch_variant_and_fields, record_kind_payload_mismatch_display_format, record_kind_payload_mismatch_error_code
// - PostcardEncodeFailed: postcard_encode_failed_variant_and_source, postcard_encode_failed_display_format, postcard_encode_failed_error_code
// - ArtifactValidation: artifact_validation_variant_and_source, artifact_validation_display_format, artifact_validation_error_code
// - ArtifactActionRegistry: artifact_action_registry_variant_and_source, artifact_action_registry_display_format, artifact_action_registry_error_code
// - WorkflowReconstruction: workflow_reconstruction_variant_and_source, workflow_reconstruction_display_format, workflow_reconstruction_error_code
// - CompiledIrReadback: compiled_ir_readback_variant_and_source, compiled_ir_readback_display_format, compiled_ir_readback_error_code
// - AdmissionAllocationFailed: admission_allocation_failed_variant_and_source, admission_allocation_failed_display_format, admission_allocation_failed_error_code
// - OwnedInputs: owned_inputs_variant_and_source, owned_inputs_display_format, owned_inputs_error_code
// - IndexStatusStateCollision: index_status_state_collision_variant_and_fields, index_status_state_collision_display_format, index_status_state_collision_error_code
// - InvalidConfig: invalid_config_variant_and_fields, invalid_config_display_format, invalid_config_error_code
// - UnsupportedReadOnly: unsupported_read_only_variant_and_display, unsupported_read_only_error_code
// - EmergencyDenyMarkerUnsupportedVersion: emergency_deny_marker_unsupported_version_variant_and_fields, emergency_deny_marker_unsupported_version_display_format, emergency_deny_marker_unsupported_version_error_code
// - EmergencyDenyMarkerMalformed: emergency_deny_marker_malformed_variant_and_display, emergency_deny_marker_malformed_error_code
//
// Tested in sibling suites (direct construction + diagnostic-code assertion
// in error_code_tests.rs, behavioral production in tests/error_variants_and_bdd.rs):
// - DuplicateEvent, SequenceGap, BadMagic, UnsupportedSchemaVersion,
//   HeaderLengthMismatch, PayloadTooLarge, HeaderChecksumMismatch,
//   PayloadDigestMismatch, UnexpectedEof, PostcardDecodeFailed: both suites
// - WrongRun, ArtifactNotFound, Trim, ArtifactVersionIncompatible: error_code_tests.rs
// - MigrationRequired, UnknownRecordKind, RecordKindFamilyMismatch,
//   MalformedKeyspaceRow: tests/error_variants_and_bdd.rs
//
// Untested variants (documented external-wrapper exceptions; see
// docs/error-variant-completeness.md):
// - Fjall: no direct test (wraps fjall::Error, no stable public constructor)
// - Encode: no direct test (wraps postcard::Error, no stable public constructor)
//
mod error_tests {
    use crate::JournalError;
    use crate::OwnedInputError;
    use crate::error::ArtifactInvalidSource;
    use crate::types::EventSeq;
    use std::path::PathBuf;
    use vb_core::WorkflowError;
    use vb_core::action::ActionRegistryError;
    use vb_core::{DiagnosticCode, RunId};
    use vb_validate::ValidationError;

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

    // -----------------------------------------------------------------------
    // KeyCapacity
    // -----------------------------------------------------------------------

    #[test]
    fn key_capacity_variant_and_display() {
        let err = JournalError::KeyCapacity;
        assert!(
            matches!(err, JournalError::KeyCapacity),
            "variant must be KeyCapacity"
        );
        let display = format!("{err}");
        assert!(
            display.contains("journal key capacity exceeded"),
            "Display must mention key capacity: {display}"
        );
    }

    #[test]
    fn key_capacity_error_code() {
        let err = JournalError::KeyCapacity;
        assert_eq!(err.diagnostic_code(), JournalError::KEY_CAPACITY_CODE,);
        assert_eq!(JournalError::KEY_CAPACITY_CODE, DiagnosticCode::new(0x4003),);
    }

    // -----------------------------------------------------------------------
    // WriteLockPoisoned
    // -----------------------------------------------------------------------

    #[test]
    fn write_lock_poisoned_variant_and_display() {
        let err = JournalError::WriteLockPoisoned;
        assert!(
            matches!(err, JournalError::WriteLockPoisoned),
            "variant must be WriteLockPoisoned"
        );
        let display = format!("{err}");
        assert!(
            display.contains("journal write lock is poisoned"),
            "Display must mention poisoned write lock: {display}"
        );
    }

    #[test]
    fn write_lock_poisoned_error_code() {
        let err = JournalError::WriteLockPoisoned;
        assert_eq!(
            err.diagnostic_code(),
            JournalError::WRITE_LOCK_POISONED_CODE,
        );
        assert_eq!(
            JournalError::WRITE_LOCK_POISONED_CODE,
            DiagnosticCode::new(0x4005),
        );
    }

    // -----------------------------------------------------------------------
    // QueueCapacity
    // -----------------------------------------------------------------------

    #[test]
    fn queue_capacity_variant_and_display() {
        let err = JournalError::QueueCapacity;
        assert!(
            matches!(err, JournalError::QueueCapacity),
            "variant must be QueueCapacity"
        );
        let display = format!("{err}");
        assert!(
            display.contains("journal writer queue capacity must be non-zero"),
            "Display must mention queue capacity: {display}"
        );
    }

    #[test]
    fn queue_capacity_error_code() {
        let err = JournalError::QueueCapacity;
        assert_eq!(err.diagnostic_code(), JournalError::QUEUE_CAPACITY_CODE,);
        assert_eq!(
            JournalError::QUEUE_CAPACITY_CODE,
            DiagnosticCode::new(0x4006),
        );
    }

    // -----------------------------------------------------------------------
    // QueueFull
    // -----------------------------------------------------------------------

    #[test]
    fn queue_full_variant_and_display() {
        let err = JournalError::QueueFull;
        assert!(
            matches!(err, JournalError::QueueFull),
            "variant must be QueueFull"
        );
        let display = format!("{err}");
        assert!(
            display.contains("journal writer queue is full"),
            "Display must mention queue full: {display}"
        );
    }

    #[test]
    fn queue_full_error_code() {
        let err = JournalError::QueueFull;
        assert_eq!(err.diagnostic_code(), JournalError::QUEUE_FULL_CODE,);
        assert_eq!(JournalError::QUEUE_FULL_CODE, DiagnosticCode::new(0x4007),);
    }

    // -----------------------------------------------------------------------
    // QueueShutdown
    // -----------------------------------------------------------------------

    #[test]
    fn queue_shutdown_variant_and_display() {
        let err = JournalError::QueueShutdown;
        assert!(
            matches!(err, JournalError::QueueShutdown),
            "variant must be QueueShutdown"
        );
        let display = format!("{err}");
        assert!(
            display.contains("journal writer queue is shut down"),
            "Display must mention queue shutdown: {display}"
        );
    }

    #[test]
    fn queue_shutdown_error_code() {
        let err = JournalError::QueueShutdown;
        assert_eq!(err.diagnostic_code(), JournalError::QUEUE_SHUTDOWN_CODE,);
        assert_eq!(
            JournalError::QUEUE_SHUTDOWN_CODE,
            DiagnosticCode::new(0x4016),
        );
    }

    // -----------------------------------------------------------------------
    // SequenceOverflow
    // -----------------------------------------------------------------------

    #[test]
    fn sequence_overflow_variant_and_display() {
        let err = JournalError::SequenceOverflow;
        assert!(
            matches!(err, JournalError::SequenceOverflow),
            "variant must be SequenceOverflow"
        );
        let display = format!("{err}");
        assert!(
            display.contains("journal event sequence overflow"),
            "Display must mention sequence overflow: {display}"
        );
    }

    #[test]
    fn sequence_overflow_error_code() {
        let err = JournalError::SequenceOverflow;
        assert_eq!(err.diagnostic_code(), JournalError::SEQUENCE_OVERFLOW_CODE,);
        assert_eq!(
            JournalError::SEQUENCE_OVERFLOW_CODE,
            DiagnosticCode::new(0x400A),
        );
    }

    // -----------------------------------------------------------------------
    // InvalidEvent
    // -----------------------------------------------------------------------

    #[test]
    fn invalid_event_variant_and_display() {
        let err = JournalError::InvalidEvent;
        assert!(
            matches!(err, JournalError::InvalidEvent),
            "variant must be InvalidEvent"
        );
        let display = format!("{err}");
        assert!(
            display.contains("semantically invalid"),
            "Display must mention semantic invalidity: {display}"
        );
    }

    #[test]
    fn invalid_event_error_code() {
        let err = JournalError::InvalidEvent;
        assert_eq!(err.diagnostic_code(), JournalError::INVALID_EVENT_CODE,);
        assert_eq!(
            JournalError::INVALID_EVENT_CODE,
            DiagnosticCode::new(0x4020),
        );
    }

    // -----------------------------------------------------------------------
    // ArtifactMalformed
    // -----------------------------------------------------------------------

    #[test]
    fn artifact_malformed_variant_and_display() {
        let err = JournalError::ArtifactMalformed;
        assert!(
            matches!(err, JournalError::ArtifactMalformed),
            "variant must be ArtifactMalformed"
        );
        let display = format!("{err}");
        assert!(
            display.contains("artifact structure validation failed"),
            "Display must mention artifact structure validation: {display}"
        );
    }

    #[test]
    fn artifact_malformed_error_code() {
        let err = JournalError::ArtifactMalformed;
        assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
        assert_eq!(
            JournalError::ARTIFACT_MALFORMED_CODE,
            DiagnosticCode::new(0x4017),
        );
    }

    // -----------------------------------------------------------------------
    // ArtifactChecksumMismatch
    // -----------------------------------------------------------------------

    #[test]
    fn artifact_checksum_mismatch_variant_and_display() {
        let err = JournalError::ArtifactChecksumMismatch;
        assert!(
            matches!(err, JournalError::ArtifactChecksumMismatch),
            "variant must be ArtifactChecksumMismatch"
        );
        let display = format!("{err}");
        assert!(
            display.contains("artifact checksum mismatch"),
            "Display must mention artifact checksum mismatch: {display}"
        );
    }

    #[test]
    fn artifact_checksum_mismatch_error_code() {
        let err = JournalError::ArtifactChecksumMismatch;
        assert_eq!(
            err.diagnostic_code(),
            JournalError::ARTIFACT_CHECKSUM_MISMATCH_CODE,
        );
        assert_eq!(
            JournalError::ARTIFACT_CHECKSUM_MISMATCH_CODE,
            DiagnosticCode::new(0x4018),
        );
    }

    // -----------------------------------------------------------------------
    // InvalidRunId
    // -----------------------------------------------------------------------

    #[test]
    fn invalid_run_id_variant_and_fields() {
        let err = JournalError::InvalidRunId { run: RunId::new(7) };
        match err {
            JournalError::InvalidRunId { run } => {
                assert_eq!(run, RunId::new(7));
            }
            other => panic!("expected InvalidRunId, got {other:?}"),
        }
    }

    #[test]
    fn invalid_run_id_display_format() {
        let err = JournalError::InvalidRunId { run: RunId::new(7) };
        let display = format!("{err}");
        assert!(
            display.contains("invalid run identifier"),
            "Display must mention invalid run identifier: {display}"
        );
    }

    #[test]
    fn invalid_run_id_error_code() {
        let err = JournalError::InvalidRunId { run: RunId::new(1) };
        assert_eq!(err.diagnostic_code(), JournalError::INVALID_RUN_ID_CODE,);
        assert_eq!(
            JournalError::INVALID_RUN_ID_CODE,
            DiagnosticCode::new(0x4021),
        );
    }

    // -----------------------------------------------------------------------
    // StrictDurabilityFailed
    // -----------------------------------------------------------------------

    #[test]
    fn strict_durability_failed_variant_and_display() {
        let err = JournalError::StrictDurabilityFailed;
        assert!(
            matches!(err, JournalError::StrictDurabilityFailed),
            "variant must be StrictDurabilityFailed"
        );
        let display = format!("{err}");
        assert!(
            display.contains("strict durability failed"),
            "Display must mention strict durability: {display}"
        );
    }

    #[test]
    fn strict_durability_failed_error_code() {
        let err = JournalError::StrictDurabilityFailed;
        assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
        assert_eq!(
            JournalError::ARTIFACT_MALFORMED_CODE,
            DiagnosticCode::new(0x4017),
        );
    }

    // -----------------------------------------------------------------------
    // ProcessLockHeld
    // -----------------------------------------------------------------------

    #[test]
    fn process_lock_held_variant_and_fields() {
        let err = JournalError::ProcessLockHeld {
            path: PathBuf::from("/tmp/vb-test-process.lock").into_boxed_path(),
            source: rustix::io::Errno::WOULDBLOCK,
            holder_pid: Some(4242),
        };
        match err {
            JournalError::ProcessLockHeld {
                path,
                source,
                holder_pid,
            } => {
                assert_eq!(
                    path,
                    PathBuf::from("/tmp/vb-test-process.lock").into_boxed_path()
                );
                assert!(matches!(source, rustix::io::Errno::WOULDBLOCK));
                assert_eq!(holder_pid, Some(4242));
            }
            other => panic!("expected ProcessLockHeld, got {other:?}"),
        }
    }

    #[test]
    fn process_lock_held_display_format() {
        let err = JournalError::ProcessLockHeld {
            path: PathBuf::from("/tmp/vb-test-process.lock").into_boxed_path(),
            source: rustix::io::Errno::WOULDBLOCK,
            holder_pid: Some(4242),
        };
        let display = format!("{err}");
        assert!(
            display.contains("process lock held by another process"),
            "Display must mention process lock held: {display}"
        );
        assert!(
            display.contains("4242"),
            "Display must contain the holder pid: {display}"
        );
        assert!(
            display.contains("/tmp/vb-test-process.lock"),
            "Display must contain the lock path: {display}"
        );
    }

    #[test]
    fn process_lock_held_error_code() {
        let err = JournalError::ProcessLockHeld {
            path: PathBuf::from("/tmp/vb-test-process.lock").into_boxed_path(),
            source: rustix::io::Errno::WOULDBLOCK,
            holder_pid: None,
        };
        assert_eq!(err.diagnostic_code(), JournalError::PROCESS_LOCK_HELD_CODE,);
        assert_eq!(
            JournalError::PROCESS_LOCK_HELD_CODE,
            DiagnosticCode::new(0x401A),
        );
    }

    // -----------------------------------------------------------------------
    // ProcessLockIo
    // -----------------------------------------------------------------------

    #[test]
    fn process_lock_io_variant_and_fields() {
        let err = JournalError::ProcessLockIo {
            path: PathBuf::from("/tmp/vb-test-process.lock").into_boxed_path(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "test lock file missing"),
        };
        match err {
            JournalError::ProcessLockIo { path, source } => {
                assert_eq!(
                    path,
                    PathBuf::from("/tmp/vb-test-process.lock").into_boxed_path()
                );
                assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
            }
            other => panic!("expected ProcessLockIo, got {other:?}"),
        }
    }

    #[test]
    fn process_lock_io_display_format() {
        let err = JournalError::ProcessLockIo {
            path: PathBuf::from("/tmp/vb-test-process.lock").into_boxed_path(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "test lock file missing"),
        };
        let display = format!("{err}");
        assert!(
            display.contains("process lock I/O error"),
            "Display must mention process lock I/O error: {display}"
        );
        assert!(
            display.contains("/tmp/vb-test-process.lock"),
            "Display must contain the lock path: {display}"
        );
    }

    #[test]
    fn process_lock_io_error_code() {
        let err = JournalError::ProcessLockIo {
            path: PathBuf::from("/tmp/vb-test-process.lock").into_boxed_path(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "test"),
        };
        assert_eq!(err.diagnostic_code(), JournalError::PROCESS_LOCK_IO_CODE,);
        assert_eq!(
            JournalError::PROCESS_LOCK_IO_CODE,
            DiagnosticCode::new(0x401B),
        );
    }

    // -----------------------------------------------------------------------
    // DuplicateStagedKey
    // -----------------------------------------------------------------------

    #[test]
    fn duplicate_staged_key_variant_and_fields() {
        let err = JournalError::DuplicateStagedKey {
            run: RunId::new(5),
            seq: EventSeq::new(3),
        };
        match err {
            JournalError::DuplicateStagedKey { run, seq } => {
                assert_eq!(run, RunId::new(5));
                assert_eq!(seq, EventSeq::new(3));
            }
            other => panic!("expected DuplicateStagedKey, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_staged_key_display_format() {
        let err = JournalError::DuplicateStagedKey {
            run: RunId::new(5),
            seq: EventSeq::new(3),
        };
        let display = format!("{err}");
        assert!(
            display.contains("duplicate journal event staged in the same batch"),
            "Display must mention duplicate staged key: {display}"
        );
    }

    #[test]
    fn duplicate_staged_key_error_code() {
        let err = JournalError::DuplicateStagedKey {
            run: RunId::new(1),
            seq: EventSeq::new(0),
        };
        assert_eq!(
            err.diagnostic_code(),
            JournalError::DUPLICATE_STAGED_KEY_CODE,
        );
        assert_eq!(
            JournalError::DUPLICATE_STAGED_KEY_CODE,
            DiagnosticCode::new(0x4023),
        );
    }

    // -----------------------------------------------------------------------
    // JournalBatchBytesExceeded
    // -----------------------------------------------------------------------

    #[test]
    fn journal_batch_bytes_exceeded_variant_and_fields() {
        let err = JournalError::JournalBatchBytesExceeded {
            attempted: 2048,
            limit: 1024,
        };
        match err {
            JournalError::JournalBatchBytesExceeded { attempted, limit } => {
                assert_eq!(attempted, 2048);
                assert_eq!(limit, 1024);
            }
            other => panic!("expected JournalBatchBytesExceeded, got {other:?}"),
        }
    }

    #[test]
    fn journal_batch_bytes_exceeded_display_format() {
        let err = JournalError::JournalBatchBytesExceeded {
            attempted: 2048,
            limit: 1024,
        };
        let display = format!("{err}");
        assert!(
            display.contains("journal batch byte budget exceeded"),
            "Display must mention batch byte budget: {display}"
        );
        assert!(
            display.contains("2048") && display.contains("1024"),
            "Display must contain attempted and limit: {display}"
        );
    }

    #[test]
    fn journal_batch_bytes_exceeded_error_code() {
        let err = JournalError::JournalBatchBytesExceeded {
            attempted: 10,
            limit: 5,
        };
        assert_eq!(
            err.diagnostic_code(),
            JournalError::JOURNAL_BATCH_BYTES_EXCEEDED_CODE,
        );
        assert_eq!(
            JournalError::JOURNAL_BATCH_BYTES_EXCEEDED_CODE,
            DiagnosticCode::new(0x4022),
        );
    }

    // -----------------------------------------------------------------------
    // BatchAborted
    // -----------------------------------------------------------------------

    #[test]
    fn batch_aborted_variant_and_display() {
        let err = JournalError::BatchAborted;
        assert!(
            matches!(err, JournalError::BatchAborted),
            "variant must be BatchAborted"
        );
        let display = format!("{err}");
        assert!(
            display.contains("journal write batch was aborted"),
            "Display must mention aborted batch: {display}"
        );
    }

    #[test]
    fn batch_aborted_error_code() {
        let err = JournalError::BatchAborted;
        assert_eq!(err.diagnostic_code(), JournalError::BATCH_ABORTED_CODE,);
        assert_eq!(
            JournalError::BATCH_ABORTED_CODE,
            DiagnosticCode::new(0x4024),
        );
    }

    // -----------------------------------------------------------------------
    // ReplayKeyMismatch
    // -----------------------------------------------------------------------

    #[test]
    fn replay_key_mismatch_variant_and_fields() {
        let err = JournalError::ReplayKeyMismatch {
            run: RunId::new(11),
            key_seq: 7,
            payload_seq: 9,
        };
        match err {
            JournalError::ReplayKeyMismatch {
                run,
                key_seq,
                payload_seq,
            } => {
                assert_eq!(run, RunId::new(11));
                assert_eq!(key_seq, 7);
                assert_eq!(payload_seq, 9);
            }
            other => panic!("expected ReplayKeyMismatch, got {other:?}"),
        }
    }

    #[test]
    fn replay_key_mismatch_display_format() {
        let err = JournalError::ReplayKeyMismatch {
            run: RunId::new(11),
            key_seq: 7,
            payload_seq: 9,
        };
        let display = format!("{err}");
        assert!(
            display.contains("journal replay key/payload mismatch"),
            "Display must mention replay key/payload mismatch: {display}"
        );
        assert!(
            display.contains("key_seq=7") && display.contains("payload_seq=9"),
            "Display must contain key and payload sequences: {display}"
        );
    }

    #[test]
    fn replay_key_mismatch_error_code() {
        let err = JournalError::ReplayKeyMismatch {
            run: RunId::new(1),
            key_seq: 0,
            payload_seq: 1,
        };
        assert_eq!(
            err.diagnostic_code(),
            JournalError::REPLAY_KEY_MISMATCH_CODE,
        );
        assert_eq!(
            JournalError::REPLAY_KEY_MISMATCH_CODE,
            DiagnosticCode::new(0x4040),
        );
    }

    // -----------------------------------------------------------------------
    // ReplayEnvelopeSequenceMismatch
    // -----------------------------------------------------------------------

    #[test]
    fn replay_envelope_sequence_mismatch_variant_and_fields() {
        let err = JournalError::ReplayEnvelopeSequenceMismatch {
            run: RunId::new(12),
            envelope_seq: 4,
            payload_seq: 6,
        };
        match err {
            JournalError::ReplayEnvelopeSequenceMismatch {
                run,
                envelope_seq,
                payload_seq,
            } => {
                assert_eq!(run, RunId::new(12));
                assert_eq!(envelope_seq, 4);
                assert_eq!(payload_seq, 6);
            }
            other => panic!("expected ReplayEnvelopeSequenceMismatch, got {other:?}"),
        }
    }

    #[test]
    fn replay_envelope_sequence_mismatch_display_format() {
        let err = JournalError::ReplayEnvelopeSequenceMismatch {
            run: RunId::new(12),
            envelope_seq: 4,
            payload_seq: 6,
        };
        let display = format!("{err}");
        assert!(
            display.contains("journal replay envelope/payload sequence mismatch"),
            "Display must mention envelope/payload mismatch: {display}"
        );
        assert!(
            display.contains("envelope_seq=4") && display.contains("payload_seq=6"),
            "Display must contain envelope and payload sequences: {display}"
        );
    }

    #[test]
    fn replay_envelope_sequence_mismatch_error_code() {
        let err = JournalError::ReplayEnvelopeSequenceMismatch {
            run: RunId::new(1),
            envelope_seq: 0,
            payload_seq: 1,
        };
        assert_eq!(
            err.diagnostic_code(),
            JournalError::REPLAY_ENVELOPE_SEQUENCE_MISMATCH_CODE,
        );
        assert_eq!(
            JournalError::REPLAY_ENVELOPE_SEQUENCE_MISMATCH_CODE,
            DiagnosticCode::new(0x4041),
        );
    }

    // -----------------------------------------------------------------------
    // RecordKindPayloadMismatch
    // -----------------------------------------------------------------------

    #[test]
    fn record_kind_payload_mismatch_variant_and_fields() {
        let err = JournalError::RecordKindPayloadMismatch {
            envelope_kind: 3,
            payload_kind: 5,
        };
        match err {
            JournalError::RecordKindPayloadMismatch {
                envelope_kind,
                payload_kind,
            } => {
                assert_eq!(envelope_kind, 3);
                assert_eq!(payload_kind, 5);
            }
            other => panic!("expected RecordKindPayloadMismatch, got {other:?}"),
        }
    }

    #[test]
    fn record_kind_payload_mismatch_display_format() {
        let err = JournalError::RecordKindPayloadMismatch {
            envelope_kind: 3,
            payload_kind: 5,
        };
        let display = format!("{err}");
        assert!(
            display.contains("journal event payload kind 5 does not match envelope kind 3"),
            "Display must mention payload/envelope kind mismatch: {display}"
        );
    }

    #[test]
    fn record_kind_payload_mismatch_error_code() {
        let err = JournalError::RecordKindPayloadMismatch {
            envelope_kind: 1,
            payload_kind: 2,
        };
        assert_eq!(err.diagnostic_code(), JournalError::INVALID_EVENT_CODE,);
        assert_eq!(
            JournalError::INVALID_EVENT_CODE,
            DiagnosticCode::new(0x4020),
        );
    }

    // -----------------------------------------------------------------------
    // PostcardEncodeFailed
    // -----------------------------------------------------------------------

    #[test]
    fn postcard_encode_failed_variant_and_source() {
        let err = JournalError::PostcardEncodeFailed(postcard::Error::SerializeBufferFull);
        match err {
            JournalError::PostcardEncodeFailed(inner) => {
                assert!(matches!(inner, postcard::Error::SerializeBufferFull));
            }
            other => panic!("expected PostcardEncodeFailed, got {other:?}"),
        }
    }

    #[test]
    fn postcard_encode_failed_display_format() {
        let err = JournalError::PostcardEncodeFailed(postcard::Error::SerializeBufferFull);
        let display = format!("{err}");
        assert!(
            display.contains("artifact postcard encode failed"),
            "Display must mention postcard encode failure: {display}"
        );
    }

    #[test]
    fn postcard_encode_failed_error_code() {
        let err = JournalError::PostcardEncodeFailed(postcard::Error::SerializeBufferFull);
        assert_eq!(
            err.diagnostic_code(),
            JournalError::POSTCARD_ENCODE_FAILED_CODE,
        );
        assert_eq!(
            JournalError::POSTCARD_ENCODE_FAILED_CODE,
            DiagnosticCode::new(0x4032),
        );
    }

    // -----------------------------------------------------------------------
    // ArtifactValidation
    // -----------------------------------------------------------------------

    include!("error_additional_tests.rs");
}
