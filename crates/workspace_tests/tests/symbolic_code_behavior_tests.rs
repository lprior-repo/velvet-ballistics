//! Behavior tests for symbolic diagnostic codes — HasSymbolicCode trait.
//!
//! Covers the HasSymbolicCode trait implementation on all six error types:
//! ValidationError, CompileError, YamlError, CoreError, RuntimeError, JournalError.
//!
//! Each test verifies:
//!   - The trait method returns a valid SymbolicCode.
//!   - The symbolic code is registered in CODE_REGISTRY.
//!   - The symbolic code is deterministic.
//!   - The symbolic code matches the expected display string.

use vb_core::diagnostic::{DiagnosticCode, HasSymbolicCode, SymbolicCode};
use vb_core::errors::CoreError;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_runtime::RuntimeError;
use vb_storage::JournalError;
use vb_validate::ValidationError;
use vb_yaml::YamlError;

// ---------------------------------------------------------------------------
// ValidationError symbolic code coverage
// ---------------------------------------------------------------------------

#[test]
fn validation_error_duplicate_key() {
    let code = HasSymbolicCode::symbolic_code(&ValidationError::DuplicateKey);
    assert_eq!(code.as_str(), "DUPLICATE_KEY");
}

#[test]
fn validation_error_missing_required_field() {
    let error = ValidationError::MissingRequiredField {
        field: "version".into(),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "MISSING_REQUIRED_FIELD");
}

#[test]
fn validation_error_type_mismatch() {
    let error = ValidationError::TypeMismatch {
        expected: "a".into(),
        found: "b".into(),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "TYPE_MISMATCH");
}

#[test]
fn validation_error_expression_stack_exceeded() {
    let error = ValidationError::ExpressionStackExceeded {
        declared: 65,
        limit: 64,
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "EXPRESSION_STACK_EXCEEDED");
}

#[test]
fn validation_error_missing_schema_version() {
    let code = HasSymbolicCode::symbolic_code(&ValidationError::MissingSchemaVersion);
    assert_eq!(code.as_str(), "MISSING_SCHEMA_VERSION");
}

// ---------------------------------------------------------------------------
// CompileError symbolic code coverage
// ---------------------------------------------------------------------------

#[test]
fn compile_error_empty_source_returns_missing_required_field() {
    let error = vb_compile::CompileError::EmptySource;
    let code = error.code();
    assert_eq!(code.as_str(), "MISSING_REQUIRED_FIELD");
}

// ---------------------------------------------------------------------------
// YamlError symbolic code coverage
// ---------------------------------------------------------------------------

#[test]
fn yaml_error_duplicate_key() {
    let error = YamlError::DuplicateKey { key: "test".into() };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "DUPLICATE_KEY");
}

#[test]
fn yaml_error_forbidden_feature() {
    let error = YamlError::ForbiddenFeature { detail: "test" };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "FORBIDDEN_YAML_FEATURE");
}

#[test]
fn yaml_error_empty_source() {
    let error = YamlError::EmptySource;
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "MISSING_REQUIRED_FIELD");
}

#[test]
fn yaml_error_field_shape() {
    let error = YamlError::FieldShape {
        field: "x",
        expected: "y",
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "TYPE_MISMATCH");
}

#[test]
fn yaml_error_nesting_too_deep() {
    let error = YamlError::NestingTooDeep { depth: 64, max: 63 };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "LIMIT_EXCEEDED");
}

#[test]
fn yaml_error_unknown_field() {
    let error = YamlError::UnknownField {
        field: "extra".into(),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "UNKNOWN_TOP_LEVEL_FIELD");
}

#[test]
fn yaml_error_source_too_large() {
    let error = YamlError::SourceTooLarge {
        size: 1024,
        max: 512,
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "PAYLOAD_TOO_LARGE");
}

#[test]
fn yaml_error_unsupported_trigger() {
    let error = YamlError::UnsupportedTrigger { trigger: "cron" };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "UNSUPPORTED_TRIGGER");
}

// ---------------------------------------------------------------------------
// CoreError symbolic_code() — 15+ variant coverage
// ---------------------------------------------------------------------------

#[test]
fn core_error_invalid_program_counter() {
    let error = CoreError::InvalidProgramCounter {
        step: StepIdx::new(5),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "INVALID_PROGRAM_COUNTER");
}

#[test]
fn core_error_missing_next_step() {
    let error = CoreError::MissingNextStep {
        step: StepIdx::new(3),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "MISSING_NEXT_STEP");
}

#[test]
fn core_error_slot_out_of_bounds() {
    let error = CoreError::SlotOutOfBounds {
        slot: SlotIdx::new(0),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "SLOT_OUT_OF_BOUNDS");
}

#[test]
fn core_error_slot_uninitialized() {
    let error = CoreError::SlotUninitialized {
        slot: SlotIdx::new(0),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "SLOT_UNINITIALIZED");
}

#[test]
fn core_error_const_out_of_bounds() {
    let error = CoreError::ConstOutOfBounds {
        index: vb_core::ids::ConstIdx::new(0),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "CONST_OUT_OF_BOUNDS");
}

#[test]
fn core_error_type_mismatch() {
    let error = CoreError::TypeMismatch {
        expected: "u64".into(),
        found: "string".into(),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    // Maps through diagnostic_code().symbolic_code() → CODE_REGISTRY
    // 0x1101 → "CORE_TYPE_MISMATCH" in the workspace registry
    assert_eq!(code.as_str(), "CORE_TYPE_MISMATCH");
}

#[test]
fn core_error_non_bool_condition() {
    let error = CoreError::NonBoolCondition {
        slot: SlotIdx::new(1),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "NON_BOOL_CONDITION");
}

#[test]
fn core_error_non_finite_number() {
    let code = HasSymbolicCode::symbolic_code(&CoreError::NonFiniteNumber);
    assert_eq!(code.as_str(), "NON_FINITE_NUMBER");
}

#[test]
fn core_error_division_by_zero() {
    let code = HasSymbolicCode::symbolic_code(&CoreError::DivisionByZero);
    assert_eq!(code.as_str(), "DIVISION_BY_ZERO");
}

#[test]
fn core_error_step_budget_exhausted() {
    let code = HasSymbolicCode::symbolic_code(&CoreError::StepBudgetExhausted);
    assert_eq!(code.as_str(), "STEP_BUDGET_EXHAUSTED");
}

#[test]
fn core_error_queue_full() {
    let code = HasSymbolicCode::symbolic_code(&CoreError::QueueFull);
    assert_eq!(code.as_str(), "CORE_QUEUE_FULL");
}

#[test]
fn core_error_allocation_failed() {
    let code = HasSymbolicCode::symbolic_code(&CoreError::AllocationFailed);
    assert_eq!(code.as_str(), "ALLOCATION_FAILED");
}

#[test]
fn core_error_capability_denied() {
    let error = CoreError::CapabilityDenied {
        action: vb_core::ids::ActionId::new(1),
        required: vb_core::capability::Capability::ALL,
        granted: vb_core::capability::CapabilitySet::empty(),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "CAPABILITY_DENIED");
}

#[test]
fn core_error_expression_stack_overflow() {
    let error = CoreError::ExpressionStackOverflow { max: 4 };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "EXPRESSION_STACK_OVERFLOW");
}

#[test]
fn core_error_lifecycle_storage_unavailable() {
    let error = CoreError::LifecycleStorageUnavailable {
        code: DiagnosticCode::new(0x1501),
        context: String::from("test"),
        timestamp: chrono::Utc::now(),
        bead_id: None,
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "CORE_LIFECYCLE_STORAGE_UNAVAILABLE");
}

#[test]
fn core_error_all_registered_codes_roundtrip() {
    let errors: Vec<CoreError> = vec![
        CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        },
        CoreError::MissingNextStep {
            step: StepIdx::new(0),
        },
        CoreError::SlotOutOfBounds {
            slot: SlotIdx::new(0),
        },
        CoreError::SlotUninitialized {
            slot: SlotIdx::new(0),
        },
        CoreError::ConstOutOfBounds {
            index: vb_core::ids::ConstIdx::new(0),
        },
        CoreError::TypeMismatch {
            expected: "x".into(),
            found: "y".into(),
        },
        CoreError::NonBoolCondition {
            slot: SlotIdx::new(0),
        },
        CoreError::NonFiniteNumber,
        CoreError::DivisionByZero,
    ];
    for error in &errors {
        let code = HasSymbolicCode::symbolic_code(error);
        let reconstructed = SymbolicCode::from_static(code.as_str());
        assert!(
            reconstructed.is_some(),
            "CoreError code '{}' must be registered in CODE_REGISTRY",
            code.as_str()
        );
    }
}

// ---------------------------------------------------------------------------
// RuntimeError symbolic_code() — 25+ variant coverage
// ---------------------------------------------------------------------------

#[test]
fn runtime_error_queue_full() {
    let code = HasSymbolicCode::symbolic_code(&RuntimeError::QueueFull);
    assert_eq!(code.as_str(), "QUEUE_FULL");
}

#[test]
fn runtime_error_run_not_found() {
    let code = HasSymbolicCode::symbolic_code(&RuntimeError::RunNotFound);
    assert_eq!(code.as_str(), "RUN_NOT_FOUND");
}

#[test]
fn runtime_error_shutdown_in_progress() {
    let code = HasSymbolicCode::symbolic_code(&RuntimeError::ShutdownInProgress);
    assert_eq!(code.as_str(), "SHUTDOWN_IN_PROGRESS");
}

#[test]
fn runtime_error_journal_poisoned() {
    let code = HasSymbolicCode::symbolic_code(&RuntimeError::JournalPoisoned);
    assert_eq!(code.as_str(), "JOURNAL_POISONED");
}

#[test]
fn runtime_error_frame_pool_unavailable() {
    let code = HasSymbolicCode::symbolic_code(&RuntimeError::FramePoolUnavailable);
    assert_eq!(code.as_str(), "FRAME_POOL_UNAVAILABLE");
}

#[test]
fn runtime_error_encode_failed() {
    let code = HasSymbolicCode::symbolic_code(&RuntimeError::EncodeFailed);
    assert_eq!(code.as_str(), "ENCODE_FAILED");
}

#[test]
fn runtime_error_migrate_self() {
    let code = HasSymbolicCode::symbolic_code(&RuntimeError::MigrateSelf);
    assert_eq!(code.as_str(), "MIGRATE_SELF");
}

#[test]
fn runtime_error_invalid_action_completion() {
    let code = HasSymbolicCode::symbolic_code(&RuntimeError::InvalidActionCompletion);
    assert_eq!(code.as_str(), "INVALID_ACTION_COMPLETION");
}

#[test]
fn runtime_error_invalid_timer_fire() {
    let code = HasSymbolicCode::symbolic_code(&RuntimeError::InvalidTimerFire);
    assert_eq!(code.as_str(), "INVALID_TIMER_FIRE");
}

#[test]
fn runtime_error_unsupported_async_strict_ack() {
    let code = HasSymbolicCode::symbolic_code(&RuntimeError::UnsupportedAsyncStrictAck);
    assert_eq!(code.as_str(), "UNSUPPORTED_ASYNC_STRICT_ACK");
}

#[test]
fn runtime_error_unsupported_operation() {
    let error = RuntimeError::UnsupportedOperation {
        operation: "test_op".into(),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "UNSUPPORTED_OPERATION");
}

#[test]
fn runtime_error_active_run_capacity_exceeded() {
    let error = RuntimeError::ActiveRunCapacityExceeded { capacity: 5 };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "ACTIVE_RUN_CAPACITY_EXCEEDED");
}

#[test]
fn runtime_error_journal_full() {
    let error = RuntimeError::JournalFull { capacity: 100 };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "JOURNAL_FULL");
}

#[test]
fn runtime_error_command_queue_capacity_exceeded() {
    let error = RuntimeError::CommandQueueCapacityExceeded {
        capacity: 256,
        max: 128,
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "COMMAND_QUEUE_CAPACITY_EXCEEDED");
}

#[test]
fn runtime_error_engine_drive_failed() {
    let error = RuntimeError::EngineDriveFailed {
        run: vb_core::ids::RunId::new(1),
        source: Box::new(CoreError::NonFiniteNumber),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "ENGINE_DRIVE_FAILED");
}

#[test]
fn runtime_error_shard_not_found() {
    let error = RuntimeError::ShardNotFound { shard: 42 };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "SHARD_NOT_FOUND");
}

#[test]
fn runtime_error_admission_artifact_not_found() {
    let error = RuntimeError::AdmissionArtifactNotFound {
        digest: vb_core::ids::WorkflowDigest::new([0u8; 32]),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "ADMISSION_ARTIFACT_NOT_FOUND");
}

#[test]
fn runtime_error_invalid_recovery_hydration() {
    let code = HasSymbolicCode::symbolic_code(&RuntimeError::InvalidRecoveryHydration);
    assert_eq!(code.as_str(), "INVALID_RECOVERY_HYDRATION");
}

#[test]
fn runtime_error_unsupported_full_recovery_hydration() {
    let code = HasSymbolicCode::symbolic_code(&RuntimeError::UnsupportedFullRecoveryHydration);
    assert_eq!(code.as_str(), "UNSUPPORTED_FULL_RECOVERY_HYDRATION");
}

#[test]
fn runtime_error_secret_result_not_allowed() {
    let code = HasSymbolicCode::symbolic_code(&RuntimeError::SecretResultNotAllowed);
    assert_eq!(code.as_str(), "SECRET_RESULT_NOT_ALLOWED");
}

#[test]
fn runtime_error_ipc_payload_size_exceeded() {
    let error = RuntimeError::IpcPayloadSizeExceeded {
        size: 2048,
        max: 1024,
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "IPC_PAYLOAD_SIZE_EXCEEDED");
}

#[test]
fn runtime_error_admission_artifact_digest_mismatch() {
    let error = RuntimeError::AdmissionArtifactDigestMismatch {
        requested: vb_core::ids::WorkflowDigest::new([1u8; 32]),
        found: vb_core::ids::WorkflowDigest::new([2u8; 32]),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "ADMISSION_ARTIFACT_DIGEST_MISMATCH");
}

#[test]
fn runtime_error_admission_digest_mismatch() {
    let error = RuntimeError::AdmissionDigestMismatch {
        requested: vb_core::ids::WorkflowDigest::new([1u8; 32]),
        record: vb_core::ids::WorkflowDigest::new([2u8; 32]),
        envelope: vb_core::ids::WorkflowDigest::new([3u8; 32]),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "ADMISSION_DIGEST_MISMATCH");
}

#[test]
fn runtime_error_all_registered_codes_roundtrip() {
    let errors: Vec<RuntimeError> = vec![
        RuntimeError::QueueFull,
        RuntimeError::RunNotFound,
        RuntimeError::ShutdownInProgress,
        RuntimeError::JournalPoisoned,
        RuntimeError::FramePoolUnavailable,
        RuntimeError::EncodeFailed,
        RuntimeError::MigrateSelf,
        RuntimeError::InvalidActionCompletion,
        RuntimeError::InvalidTimerFire,
        RuntimeError::UnsupportedAsyncStrictAck,
        RuntimeError::InvalidRecoveryHydration,
        RuntimeError::UnsupportedFullRecoveryHydration,
        RuntimeError::SecretResultNotAllowed,
    ];
    for error in &errors {
        let code = HasSymbolicCode::symbolic_code(error);
        let reconstructed = SymbolicCode::from_static(code.as_str());
        assert!(
            reconstructed.is_some(),
            "RuntimeError code '{}' must be registered in CODE_REGISTRY",
            code.as_str()
        );
    }
}

// ---------------------------------------------------------------------------
// JournalError symbolic_code() — 28+ variant coverage
// ---------------------------------------------------------------------------

#[test]
fn journal_error_fjall() {
    let code = HasSymbolicCode::symbolic_code(&JournalError::WriteLockPoisoned);
    assert_eq!(code.as_str(), "JOURNAL_WRITE_LOCK_POISONED");
}

#[test]
fn journal_error_key_capacity() {
    let code = HasSymbolicCode::symbolic_code(&JournalError::KeyCapacity);
    assert_eq!(code.as_str(), "JOURNAL_KEY_CAPACITY");
}

#[test]
fn journal_error_duplicate_event() {
    let error = JournalError::DuplicateEvent {
        seq: vb_core::ids::EventSeq::new(1),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "JOURNAL_DUPLICATE_EVENT");
}

#[test]
fn journal_error_queue_full() {
    let code = HasSymbolicCode::symbolic_code(&JournalError::QueueFull);
    assert_eq!(code.as_str(), "JOURNAL_QUEUE_FULL");
}

#[test]
fn journal_error_wrong_run() {
    let error = JournalError::WrongRun {
        expected: vb_core::ids::RunId::new(1),
        actual: vb_core::ids::RunId::new(2),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "JOURNAL_WRONG_RUN");
}

#[test]
fn journal_error_sequence_gap() {
    let error = JournalError::SequenceGap {
        expected: vb_core::ids::EventSeq::new(5),
        actual: vb_core::ids::EventSeq::new(7),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "JOURNAL_SEQUENCE_GAP");
}

#[test]
fn journal_error_sequence_overflow() {
    let code = HasSymbolicCode::symbolic_code(&JournalError::SequenceOverflow);
    assert_eq!(code.as_str(), "JOURNAL_SEQUENCE_OVERFLOW");
}

#[test]
fn journal_error_bad_magic() {
    let error = JournalError::BadMagic {
        expected: 0xDEAD,
        found: 0xBEEF,
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "JOURNAL_BAD_MAGIC");
}

#[test]
fn journal_error_unsupported_schema() {
    let error = JournalError::UnsupportedSchemaVersion {
        found: 42,
        max_supported: 5,
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "JOURNAL_UNSUPPORTED_SCHEMA");
}

#[test]
fn journal_error_unknown_record_kind() {
    let error = JournalError::UnknownRecordKind { kind: 99u8 };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "JOURNAL_UNKNOWN_RECORD_KIND");
}

#[test]
fn journal_error_header_length_mismatch() {
    let error = JournalError::HeaderLengthMismatch {
        declared: 64,
        actual: 48,
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "JOURNAL_HEADER_LENGTH_MISMATCH");
}

#[test]
fn journal_error_payload_too_large() {
    let error = JournalError::PayloadTooLarge {
        size: 10_000,
        max: 1_000,
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "JOURNAL_PAYLOAD_TOO_LARGE");
}

#[test]
fn journal_error_header_checksum() {
    let code = HasSymbolicCode::symbolic_code(&JournalError::HeaderChecksumMismatch);
    assert_eq!(code.as_str(), "JOURNAL_HEADER_CHECKSUM");
}

#[test]
fn journal_error_payload_digest_mismatch() {
    let code = HasSymbolicCode::symbolic_code(&JournalError::PayloadDigestMismatch);
    assert_eq!(code.as_str(), "JOURNAL_PAYLOAD_DIGEST_MISMATCH");
}

#[test]
fn journal_error_unexpected_eof() {
    let code = HasSymbolicCode::symbolic_code(&JournalError::UnexpectedEof);
    assert_eq!(code.as_str(), "JOURNAL_UNEXPECTED_EOF");
}

#[test]
fn journal_error_postcard_decode() {
    let code = HasSymbolicCode::symbolic_code(&JournalError::PostcardDecodeFailed);
    assert_eq!(code.as_str(), "JOURNAL_POSTCARD_DECODE");
}

#[test]
fn journal_error_queue_shutdown() {
    let code = HasSymbolicCode::symbolic_code(&JournalError::QueueShutdown);
    assert_eq!(code.as_str(), "JOURNAL_QUEUE_SHUTDOWN");
}

#[test]
fn journal_error_artifact_malformed() {
    let code = HasSymbolicCode::symbolic_code(&JournalError::ArtifactMalformed);
    assert_eq!(code.as_str(), "JOURNAL_ARTIFACT_MALFORMED");
}

#[test]
fn journal_error_artifact_checksum() {
    let code = HasSymbolicCode::symbolic_code(&JournalError::ArtifactChecksumMismatch);
    assert_eq!(code.as_str(), "JOURNAL_ARTIFACT_CHECKSUM");
}

#[test]
fn journal_error_artifact_not_found() {
    let error = JournalError::ArtifactNotFound {
        digest: vb_core::ids::WorkflowDigest::new([0u8; 32]),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "JOURNAL_ARTIFACT_NOT_FOUND");
}

#[test]
fn journal_error_too_many_events() {
    let error = JournalError::TooManyEvents { limit: 10_000 };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "JOURNAL_TOO_MANY_EVENTS");
}

#[test]
fn journal_error_replay_alloc_fail() {
    let error = JournalError::ReplayAllocationFailed { requested: 4096 };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "JOURNAL_REPLAY_ALLOC_FAIL");
}

#[test]
fn journal_error_invalid_event() {
    let code = HasSymbolicCode::symbolic_code(&JournalError::InvalidEvent);
    assert_eq!(code.as_str(), "JOURNAL_INVALID_EVENT");
}

#[test]
fn journal_error_invalid_gate_count() {
    let error = JournalError::InvalidGateCount { count: 0 };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "JOURNAL_INVALID_GATE_COUNT");
}

#[test]
fn journal_error_missing_proof_flag() {
    let error = JournalError::MissingRequiredProofFlag {
        expected: "signature".into(),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "JOURNAL_MISSING_PROOF_FLAG");
}

#[test]
fn journal_error_process_lock_held() {
    let error = JournalError::ProcessLockHeld {
        pid: std::process::id(),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "JOURNAL_PROCESS_LOCK_HELD");
}

#[test]
fn journal_error_process_lock_io() {
    let code = HasSymbolicCode::symbolic_code(&JournalError::ProcessLockIo);
    assert_eq!(code.as_str(), "JOURNAL_PROCESS_LOCK_IO");
}

#[test]
fn journal_error_all_registered_codes_roundtrip() {
    let errors: Vec<JournalError> = vec![
        JournalError::WriteLockPoisoned,
        JournalError::KeyCapacity,
        JournalError::QueueFull,
        JournalError::SequenceOverflow,
        JournalError::HeaderChecksumMismatch,
        JournalError::PayloadDigestMismatch,
        JournalError::UnexpectedEof,
        JournalError::PostcardDecodeFailed,
        JournalError::QueueShutdown,
        JournalError::ArtifactMalformed,
        JournalError::ArtifactChecksumMismatch,
        JournalError::InvalidEvent,
    ];
    for error in &errors {
        let code = HasSymbolicCode::symbolic_code(error);
        let reconstructed = SymbolicCode::from_static(code.as_str());
        assert!(
            reconstructed.is_some(),
            "JournalError code '{}' must be registered in CODE_REGISTRY",
            code.as_str()
        );
    }
}

// ---------------------------------------------------------------------------
// HasSymbolicCode trait interface tests
// ---------------------------------------------------------------------------

#[test]
fn has_symbolic_code_implemented_by_validation_error() {
    let code: SymbolicCode = HasSymbolicCode::symbolic_code(&ValidationError::DuplicateKey);
    assert_eq!(code.as_str(), "DUPLICATE_KEY");
}

#[test]
fn has_symbolic_code_implemented_by_yaml_error() {
    let error = YamlError::EmptySource;
    let code: SymbolicCode = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "MISSING_REQUIRED_FIELD");
}

#[test]
fn has_symbolic_code_implemented_by_core_error() {
    let code: SymbolicCode = HasSymbolicCode::symbolic_code(&CoreError::DivisionByZero);
    assert_eq!(code.as_str(), "DIVISION_BY_ZERO");
}

#[test]
fn has_symbolic_code_implemented_by_runtime_error() {
    let code: SymbolicCode = HasSymbolicCode::symbolic_code(&RuntimeError::RunNotFound);
    assert_eq!(code.as_str(), "RUN_NOT_FOUND");
}

#[test]
fn has_symbolic_code_implemented_by_journal_error() {
    let code: SymbolicCode = HasSymbolicCode::symbolic_code(&JournalError::KeyCapacity);
    assert_eq!(code.as_str(), "JOURNAL_KEY_CAPACITY");
}

// ---------------------------------------------------------------------------
// HasSymbolicCode determinism tests
// ---------------------------------------------------------------------------

#[test]
fn has_symbolic_code_determinism_validation_error() {
    let error = ValidationError::TypeMismatch {
        expected: "a".into(),
        found: "b".into(),
    };
    let code1 = HasSymbolicCode::symbolic_code(&error);
    let code2 = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code1, code2);
}

#[test]
fn has_symbolic_code_determinism_core_error() {
    let error = CoreError::SlotOutOfBounds {
        slot: SlotIdx::new(0),
    };
    let code1 = HasSymbolicCode::symbolic_code(&error);
    let code2 = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code1, code2);
}

#[test]
fn has_symbolic_code_determinism_runtime_error() {
    let error = RuntimeError::MigrateSelf;
    let code1 = HasSymbolicCode::symbolic_code(&error);
    let code2 = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code1, code2);
}

#[test]
fn has_symbolic_code_determinism_journal_error() {
    let error = JournalError::KeyCapacity;
    let code1 = HasSymbolicCode::symbolic_code(&error);
    let code2 = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code1, code2);
}
