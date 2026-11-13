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

    #[test]
    fn artifact_validation_variant_and_source() {
        let err = JournalError::ArtifactValidation(ValidationError::DuplicateKey);
        match err {
            JournalError::ArtifactValidation(inner) => {
                assert_eq!(inner, ValidationError::DuplicateKey);
            }
            other => panic!("expected ArtifactValidation, got {other:?}"),
        }
    }

    #[test]
    fn artifact_validation_display_format() {
        let err = JournalError::ArtifactValidation(ValidationError::DuplicateKey);
        let display = format!("{err}");
        assert!(
            display.contains("compiled artifact validation failed"),
            "Display must mention artifact validation failure: {display}"
        );
        assert!(
            display.contains("DUPLICATE_KEY"),
            "Display must contain the validation error: {display}"
        );
    }

    #[test]
    fn artifact_validation_error_code() {
        let err = JournalError::ArtifactValidation(ValidationError::DuplicateKey);
        assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
        assert_eq!(
            JournalError::ARTIFACT_MALFORMED_CODE,
            DiagnosticCode::new(0x4017),
        );
    }

    // -----------------------------------------------------------------------
    // ArtifactActionRegistry
    // -----------------------------------------------------------------------

    #[test]
    fn artifact_action_registry_variant_and_source() {
        let err = JournalError::ArtifactActionRegistry(ActionRegistryError::IndexOverflow {
            index: 70000,
        });
        match err {
            JournalError::ArtifactActionRegistry(inner) => {
                assert_eq!(inner, ActionRegistryError::IndexOverflow { index: 70000 });
            }
            other => panic!("expected ArtifactActionRegistry, got {other:?}"),
        }
    }

    #[test]
    fn artifact_action_registry_display_format() {
        let err = JournalError::ArtifactActionRegistry(ActionRegistryError::IndexOverflow {
            index: 70000,
        });
        let display = format!("{err}");
        assert!(
            display.contains("artifact action registry invalid"),
            "Display must mention action registry failure: {display}"
        );
        assert!(
            display.contains("70000"),
            "Display must contain the overflowing index: {display}"
        );
    }

    #[test]
    fn artifact_action_registry_error_code() {
        let err =
            JournalError::ArtifactActionRegistry(ActionRegistryError::IndexOverflow { index: 1 });
        assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
        assert_eq!(
            JournalError::ARTIFACT_MALFORMED_CODE,
            DiagnosticCode::new(0x4017),
        );
    }

    // -----------------------------------------------------------------------
    // WorkflowReconstruction
    // -----------------------------------------------------------------------

    #[test]
    fn workflow_reconstruction_variant_and_source() {
        let err = JournalError::WorkflowReconstruction(WorkflowError::EmptyNodes);
        match err {
            JournalError::WorkflowReconstruction(inner) => {
                assert_eq!(inner, WorkflowError::EmptyNodes);
            }
            other => panic!("expected WorkflowReconstruction, got {other:?}"),
        }
    }

    #[test]
    fn workflow_reconstruction_display_format() {
        let err = JournalError::WorkflowReconstruction(WorkflowError::EmptyNodes);
        let display = format!("{err}");
        assert!(
            display.contains("compiled workflow reconstruction failed"),
            "Display must mention workflow reconstruction failure: {display}"
        );
        assert!(
            display.contains("compiled workflow must contain at least one node"),
            "Display must contain the workflow error: {display}"
        );
    }

    #[test]
    fn workflow_reconstruction_error_code() {
        let err = JournalError::WorkflowReconstruction(WorkflowError::EmptyNodes);
        assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
        assert_eq!(
            JournalError::ARTIFACT_MALFORMED_CODE,
            DiagnosticCode::new(0x4017),
        );
    }

    // -----------------------------------------------------------------------
    // CompiledIrReadback
    // -----------------------------------------------------------------------

    #[test]
    fn compiled_ir_readback_variant_and_source() {
        let err = JournalError::CompiledIrReadback(Box::new(JournalError::BatchAborted));
        match err {
            JournalError::CompiledIrReadback(inner) => {
                assert!(matches!(inner.as_ref(), JournalError::BatchAborted));
            }
            other => panic!("expected CompiledIrReadback, got {other:?}"),
        }
    }

    #[test]
    fn compiled_ir_readback_display_format() {
        let err = JournalError::CompiledIrReadback(Box::new(JournalError::BatchAborted));
        let display = format!("{err}");
        assert!(
            display.contains("compiled_ir readback failed"),
            "Display must mention compiled_ir readback failure: {display}"
        );
        assert!(
            display.contains("journal write batch was aborted"),
            "Display must contain the wrapped error: {display}"
        );
    }

    #[test]
    fn compiled_ir_readback_error_code() {
        let err = JournalError::CompiledIrReadback(Box::new(JournalError::BatchAborted));
        assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
        assert_eq!(
            JournalError::ARTIFACT_MALFORMED_CODE,
            DiagnosticCode::new(0x4017),
        );
    }

    // -----------------------------------------------------------------------
    // AdmissionAllocationFailed
    // -----------------------------------------------------------------------

    #[test]
    fn admission_allocation_failed_variant_and_source() {
        let reserve = Vec::<u8>::new().try_reserve(usize::MAX);
        let source = match reserve {
            Ok(()) => panic!("reserving usize::MAX bytes must fail"),
            Err(err) => err,
        };
        assert_eq!(
            format!("{source}"),
            "memory allocation failed because the computed capacity exceeded the collection's maximum"
        );
        let err = JournalError::AdmissionAllocationFailed(source);
        match err {
            JournalError::AdmissionAllocationFailed(inner) => {
                assert_eq!(
                    format!("{inner}"),
                    "memory allocation failed because the computed capacity exceeded the collection's maximum"
                );
            }
            other => panic!("expected AdmissionAllocationFailed, got {other:?}"),
        }
    }

    #[test]
    fn admission_allocation_failed_display_format() {
        let reserve = Vec::<u8>::new().try_reserve(usize::MAX);
        let source = match reserve {
            Ok(()) => panic!("reserving usize::MAX bytes must fail"),
            Err(err) => err,
        };
        let err = JournalError::AdmissionAllocationFailed(source);
        let display = format!("{err}");
        assert!(
            display.contains("admission pre-allocation failed"),
            "Display must mention pre-allocation failure: {display}"
        );
        assert!(
            display.contains("capacity exceeded the collection's maximum"),
            "Display must contain the reserve failure: {display}"
        );
    }

    #[test]
    fn admission_allocation_failed_error_code() {
        let reserve = Vec::<u8>::new().try_reserve(usize::MAX);
        let source = match reserve {
            Ok(()) => panic!("reserving usize::MAX bytes must fail"),
            Err(err) => err,
        };
        let err = JournalError::AdmissionAllocationFailed(source);
        assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
        assert_eq!(
            JournalError::ARTIFACT_MALFORMED_CODE,
            DiagnosticCode::new(0x4017),
        );
    }

    // -----------------------------------------------------------------------
    // OwnedInputs
    // -----------------------------------------------------------------------

    #[test]
    fn owned_inputs_variant_and_source() {
        let err = JournalError::OwnedInputs(OwnedInputError::IdentityMismatch);
        match err {
            JournalError::OwnedInputs(inner) => {
                assert!(matches!(inner, OwnedInputError::IdentityMismatch));
            }
            other => panic!("expected OwnedInputs, got {other:?}"),
        }
    }

    #[test]
    fn owned_inputs_display_format() {
        let err = JournalError::OwnedInputs(OwnedInputError::IdentityMismatch);
        let display = format!("{err}");
        assert!(
            display.contains("owned input record rejected"),
            "Display must mention owned input rejection: {display}"
        );
        assert!(
            display.contains("owned input identity mismatch"),
            "Display must contain the owned input error: {display}"
        );
    }

    #[test]
    fn owned_inputs_error_code() {
        let err = JournalError::OwnedInputs(OwnedInputError::IdentityMismatch);
        assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
        assert_eq!(
            JournalError::ARTIFACT_MALFORMED_CODE,
            DiagnosticCode::new(0x4017),
        );
    }

    // -----------------------------------------------------------------------
    // IndexStatusStateCollision
    // -----------------------------------------------------------------------

    #[test]
    fn index_status_state_collision_variant_and_fields() {
        let err = JournalError::IndexStatusStateCollision {
            byte: 0x10,
            min: 0x20,
        };
        match err {
            JournalError::IndexStatusStateCollision { byte, min } => {
                assert_eq!(byte, 0x10);
                assert_eq!(min, 0x20);
            }
            other => panic!("expected IndexStatusStateCollision, got {other:?}"),
        }
    }

    #[test]
    fn index_status_state_collision_display_format() {
        let err = JournalError::IndexStatusStateCollision {
            byte: 0x10,
            min: 0x20,
        };
        let display = format!("{err}");
        assert!(
            display.contains("IndexStatusState byte collision"),
            "Display must mention index status collision: {display}"
        );
        assert!(
            display.contains("0x10") && display.contains("0x20"),
            "Display must contain byte and minimum: {display}"
        );
    }

    #[test]
    fn index_status_state_collision_error_code() {
        let err = JournalError::IndexStatusStateCollision { byte: 1, min: 2 };
        assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
        assert_eq!(
            JournalError::ARTIFACT_MALFORMED_CODE,
            DiagnosticCode::new(0x4017),
        );
    }

    // -----------------------------------------------------------------------
    // InvalidConfig
    // -----------------------------------------------------------------------

    #[test]
    fn invalid_config_variant_and_fields() {
        let err = JournalError::InvalidConfig {
            field: "max_bytes",
            reason: "must be positive",
        };
        match err {
            JournalError::InvalidConfig { field, reason } => {
                assert_eq!(field, "max_bytes");
                assert_eq!(reason, "must be positive");
            }
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }

    #[test]
    fn invalid_config_display_format() {
        let err = JournalError::InvalidConfig {
            field: "max_bytes",
            reason: "must be positive",
        };
        let display = format!("{err}");
        assert!(
            display.contains("invalid configuration for field max_bytes"),
            "Display must mention invalid configuration: {display}"
        );
        assert!(
            display.contains("must be positive"),
            "Display must contain the reason: {display}"
        );
    }

    #[test]
    fn invalid_config_error_code() {
        let err = JournalError::InvalidConfig {
            field: "f",
            reason: "r",
        };
        assert_eq!(err.diagnostic_code(), JournalError::INVALID_CONFIG_CODE,);
        assert_eq!(
            JournalError::INVALID_CONFIG_CODE,
            DiagnosticCode::new(0x4033),
        );
    }

    // -----------------------------------------------------------------------
    // UnsupportedReadOnly
    // -----------------------------------------------------------------------

    #[test]
    fn unsupported_read_only_variant_and_display() {
        let err = JournalError::UnsupportedReadOnly;
        assert!(
            matches!(err, JournalError::UnsupportedReadOnly),
            "variant must be UnsupportedReadOnly"
        );
        let display = format!("{err}");
        assert!(
            display.contains("read-only journal open is not supported"),
            "Display must mention unsupported read-only open: {display}"
        );
    }

    #[test]
    fn unsupported_read_only_error_code() {
        let err = JournalError::UnsupportedReadOnly;
        assert_eq!(
            err.diagnostic_code(),
            JournalError::UNSUPPORTED_READ_ONLY_CODE,
        );
        assert_eq!(
            JournalError::UNSUPPORTED_READ_ONLY_CODE,
            DiagnosticCode::new(0x4034),
        );
    }

    // -----------------------------------------------------------------------
    // EmergencyDenyMarkerUnsupportedVersion
    // -----------------------------------------------------------------------

    #[test]
    fn emergency_deny_marker_unsupported_version_variant_and_fields() {
        let err = JournalError::EmergencyDenyMarkerUnsupportedVersion { version: 9 };
        match err {
            JournalError::EmergencyDenyMarkerUnsupportedVersion { version } => {
                assert_eq!(version, 9);
            }
            other => panic!("expected EmergencyDenyMarkerUnsupportedVersion, got {other:?}"),
        }
    }

    #[test]
    fn emergency_deny_marker_unsupported_version_display_format() {
        let err = JournalError::EmergencyDenyMarkerUnsupportedVersion { version: 9 };
        let display = format!("{err}");
        assert!(
            display.contains("emergency deny marker has unsupported version"),
            "Display must mention unsupported marker version: {display}"
        );
        assert!(
            display.contains('9'),
            "Display must contain the version: {display}"
        );
    }

    #[test]
    fn emergency_deny_marker_unsupported_version_error_code() {
        let err = JournalError::EmergencyDenyMarkerUnsupportedVersion { version: 1 };
        assert_eq!(
            err.diagnostic_code(),
            JournalError::EMERGENCY_DENY_UNSUPPORTED_VERSION_CODE,
        );
        assert_eq!(
            JournalError::EMERGENCY_DENY_UNSUPPORTED_VERSION_CODE,
            DiagnosticCode::new(0x4042),
        );
    }

    // -----------------------------------------------------------------------
    // EmergencyDenyMarkerMalformed
    // -----------------------------------------------------------------------

    #[test]
    fn emergency_deny_marker_malformed_variant_and_display() {
        let err = JournalError::EmergencyDenyMarkerMalformed;
        assert!(
            matches!(err, JournalError::EmergencyDenyMarkerMalformed),
            "variant must be EmergencyDenyMarkerMalformed"
        );
        let display = format!("{err}");
        assert!(
            display.contains("emergency deny marker is malformed"),
            "Display must mention malformed marker: {display}"
        );
    }

    #[test]
    fn emergency_deny_marker_malformed_error_code() {
        let err = JournalError::EmergencyDenyMarkerMalformed;
        assert_eq!(
            err.diagnostic_code(),
            JournalError::EMERGENCY_DENY_MALFORMED_CODE,
        );
        assert_eq!(
            JournalError::EMERGENCY_DENY_MALFORMED_CODE,
            DiagnosticCode::new(0x4043),
        );
    }
