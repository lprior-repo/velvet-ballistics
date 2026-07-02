//! Typed error assertions shared by journal fuzz targets.

pub(super) fn assert_typed_journal_error(error: vb_storage::JournalError) {
    use vb_storage::JournalError;
    match error {
        JournalError::UnexpectedEof
        | JournalError::HeaderChecksumMismatch
        | JournalError::PayloadDigestMismatch
        | JournalError::PostcardDecodeFailed(_)
        | JournalError::InvalidEvent
        | JournalError::BadMagic { .. }
        | JournalError::PayloadTooLarge { .. }
        | JournalError::RecordKindFamilyMismatch { .. }
        | JournalError::UnknownRecordKind { .. }
        | JournalError::UnsupportedSchemaVersion { .. }
        | JournalError::HeaderLengthMismatch { .. }
        | JournalError::SequenceOverflow
        | JournalError::WrongRun { .. }
        | JournalError::SequenceGap { .. }
        | JournalError::Fjall(_)
        | JournalError::Encode(_)
        | JournalError::KeyCapacity
        | JournalError::DuplicateEvent { .. }
        | JournalError::WriteLockPoisoned
        | JournalError::QueueCapacity
        | JournalError::QueueFull
        | JournalError::JournalBatchBytesExceeded { .. }
        | JournalError::QueueShutdown
        | JournalError::MigrationRequired { .. }
        | JournalError::ArtifactMalformed
        | JournalError::ArtifactChecksumMismatch
        | JournalError::InvalidGateCount { .. }
        | JournalError::MissingRequiredProofFlag { .. }
        | JournalError::ArtifactNotFound { .. }
        | JournalError::AdmissionRequired
        | JournalError::ArtifactInvalid { .. }
        | JournalError::InputTooLarge { .. }
        | JournalError::InputSchemaMismatch
        | JournalError::CapabilityDenied
        | JournalError::SecretUnavailable
        | JournalError::RunAlreadyExists
        | JournalError::InvalidRunId { .. }
        | JournalError::ActiveRunCapacityExceeded
        | JournalError::FrameAllocationFailed
        | JournalError::AdmissionJournalFailed
        | JournalError::StrictDurabilityFailed
        | JournalError::TooManyEvents { .. }
        | JournalError::ReplayAllocationFailed { .. }
        | JournalError::ClockUnavailable
        | JournalError::ProcessLockHeld { .. }
        | JournalError::ProcessLockIo { .. }
        | JournalError::Trim(_) => {}
        _ => {}
    }
}

pub(super) fn assert_typed_recovery_error(error: vb_storage::recovery::RecoveryError) {
    use vb_storage::recovery::RecoveryError;
    match error {
        RecoveryError::Journal(_)
        | RecoveryError::WorkflowSourceDigestMismatch { .. }
        | RecoveryError::CompiledIrDigestMismatch { .. }
        | RecoveryError::ActionAbiMismatch { .. }
        | RecoveryError::PolicyDigestMismatch { .. }
        | RecoveryError::NonIdempotentActionBlocked { .. }
        | RecoveryError::ReplayDivergence { .. }
        | RecoveryError::SlotTaintReadFailed { .. }
        | RecoveryError::CorruptSlotTaint { .. }
        | RecoveryError::NoRecoveryData { .. } => {}
        _ => {}
    }
}
