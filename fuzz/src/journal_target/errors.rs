//! Typed error assertions shared by journal fuzz targets.

pub(super) fn assert_typed_journal_error(error: vb_storage::JournalError) {
    use vb_storage::JournalError;
    match error {
        JournalError::Fjall(_)
        | JournalError::Encode(_)
        | JournalError::PostcardEncodeFailed(_)
        | JournalError::KeyCapacity
        | JournalError::DuplicateEvent { .. }
        | JournalError::DuplicateStagedKey { .. }
        | JournalError::WriteLockPoisoned
        | JournalError::QueueCapacity
        | JournalError::QueueFull
        | JournalError::JournalBatchBytesExceeded { .. }
        | JournalError::BatchAborted
        | JournalError::QueueShutdown
        | JournalError::WrongRun { .. }
        | JournalError::SequenceGap { .. }
        | JournalError::ReplayKeyMismatch { .. }
        | JournalError::ReplayEnvelopeSequenceMismatch { .. }
        | JournalError::SequenceOverflow
        | JournalError::BadMagic { .. }
        | JournalError::UnsupportedSchemaVersion { .. }
        | JournalError::MigrationRequired { .. }
        | JournalError::UnknownRecordKind { .. }
        | JournalError::RecordKindFamilyMismatch { .. }
        | JournalError::RecordKindPayloadMismatch { .. }
        | JournalError::HeaderLengthMismatch { .. }
        | JournalError::PayloadTooLarge { .. }
        | JournalError::HeaderChecksumMismatch
        | JournalError::PayloadDigestMismatch
        | JournalError::UnexpectedEof
        | JournalError::MalformedKeyspaceRow { .. }
        | JournalError::PostcardDecodeFailed(_)
        | JournalError::InvalidEvent
        | JournalError::ArtifactMalformed
        | JournalError::WorkflowReconstruction(_)
        | JournalError::CompiledIrReadback(_)
        | JournalError::AdmissionAllocationFailed(_)
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
        | JournalError::IndexStatusStateCollision { .. }
        | JournalError::StrictDurabilityFailed
        | JournalError::TooManyEvents { .. }
        | JournalError::ReplayAllocationFailed { .. }
        | JournalError::ClockUnavailable
        | JournalError::InvalidConfig { .. }
        | JournalError::UnsupportedReadOnly
        | JournalError::ProcessLockHeld { .. }
        | JournalError::ProcessLockIo { .. }
        | JournalError::Trim(_) => {}
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
        | RecoveryError::NoRecoveryData { .. }
        | RecoveryError::CorruptSnapshot { .. }
        | RecoveryError::MissingSnapshot { .. }
        | RecoveryError::TerminalStateMismatch { .. }
        | RecoveryError::FrameDimensionOverflow { .. }
        | RecoveryError::UnsupportedFrameSeed { .. }
        | RecoveryError::ArtifactNotFound { .. }
        | RecoveryError::ArtifactDecodeFailed => {}
        _ => std::process::abort(),
    }
}
