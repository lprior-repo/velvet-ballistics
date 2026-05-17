use super::RuntimeError;

impl PartialEq for RuntimeError {
    fn eq(&self, other: &Self) -> bool {
        runtime_error_unit_eq(self, other) || runtime_error_field_eq(self, other)
    }
}

fn runtime_error_unit_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    match runtime_error_unit_tag(left) {
        Some(tag) => runtime_error_unit_tag(right) == Some(tag),
        None => false,
    }
}

fn runtime_error_unit_tag(error: &RuntimeError) -> Option<u8> {
    match error {
        RuntimeError::QueueFull => Some(0),
        RuntimeError::RunNotFound => Some(1),
        RuntimeError::RunAlreadyExists => Some(2),
        RuntimeError::ShutdownInProgress => Some(3),
        RuntimeError::JournalPoisoned => Some(4),
        RuntimeError::UnsupportedAsyncStrictAck => Some(5),
        RuntimeError::FramePoolUnavailable => Some(6),
        RuntimeError::InvalidActionCompletion => Some(7),
        RuntimeError::InvalidTimerFire => Some(8),
        RuntimeError::UnsupportedFullRecoveryHydration => Some(9),
        RuntimeError::InvalidRecoveryHydration => Some(10),
        RuntimeError::ActiveRunCapacityZero => Some(11),
        RuntimeError::EncodeFailed => Some(12),
        RuntimeError::SecretResultNotAllowed => Some(13),
        RuntimeError::ShardNotFound { .. } => Some(14),
        RuntimeError::MigrateSelf => Some(15),
        _ => None,
    }
}

fn runtime_error_field_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    runtime_error_core_field_eq(left, right) || runtime_error_admission_field_eq(left, right)
}

fn runtime_error_core_field_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    match (left, right) {
        (
            RuntimeError::ActiveRunCapacityExceeded { capacity: a },
            RuntimeError::ActiveRunCapacityExceeded { capacity: b },
        ) => a == b,
        (
            RuntimeError::UnsupportedOperation { operation: a },
            RuntimeError::UnsupportedOperation { operation: b },
        ) => a == b,
        (RuntimeError::Core { source: a }, RuntimeError::Core { source: b }) => a == b,
        (
            RuntimeError::StorageJournalAppend { source: a },
            RuntimeError::StorageJournalAppend { source: b },
        ) => a.diagnostic_code() == b.diagnostic_code(),
        (
            RuntimeError::AdmissionHeaderPersistenceFailed { source: a },
            RuntimeError::AdmissionHeaderPersistenceFailed { source: b },
        ) => a.diagnostic_code() == b.diagnostic_code(),
        (
            RuntimeError::CommandQueueCapacityExceeded {
                capacity: a,
                max: b,
            },
            RuntimeError::CommandQueueCapacityExceeded {
                capacity: c,
                max: d,
            },
        ) => a == c && b == d,
        (
            RuntimeError::StaleAttempt {
                incoming: a,
                current: b,
            },
            RuntimeError::StaleAttempt {
                incoming: c,
                current: d,
            },
        ) => a == c && b == d,
        (
            RuntimeError::AttemptBeyondMax { attempt: a, max: b },
            RuntimeError::AttemptBeyondMax { attempt: c, max: d },
        ) => a == c && b == d,
        (
            RuntimeError::IpcPayloadSizeExceeded { size: a, max: b },
            RuntimeError::IpcPayloadSizeExceeded { size: c, max: d },
        ) => a == c && b == d,
        (RuntimeError::ShardNotFound { shard: a }, RuntimeError::ShardNotFound { shard: b }) => a == b,
        _ => false,
    }
}

fn runtime_error_admission_field_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    runtime_error_admission_digest_eq(left, right)
        || runtime_error_admission_capability_eq(left, right)
}

fn runtime_error_admission_digest_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    match (left, right) {
        (
            RuntimeError::AdmissionArtifactNotFound { digest: a },
            RuntimeError::AdmissionArtifactNotFound { digest: b },
        ) => a == b,
        (
            RuntimeError::AdmissionArtifactInvalid { digest: a },
            RuntimeError::AdmissionArtifactInvalid { digest: b },
        ) => a == b,
        (
            RuntimeError::AdmissionArtifactDigestMismatch {
                requested: a,
                found: c,
            },
            RuntimeError::AdmissionArtifactDigestMismatch {
                requested: b,
                found: d,
            },
        ) => a == b && c == d,
        (
            RuntimeError::AdmissionArtifactStale { digest: a },
            RuntimeError::AdmissionArtifactStale { digest: b },
        ) => a == b,
        _ => false,
    }
}

fn runtime_error_admission_capability_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    match (left, right) {
        (
            RuntimeError::AdmissionCapabilityDenied {
                action: a,
                required: b,
                granted: c,
            },
            RuntimeError::AdmissionCapabilityDenied {
                action: d,
                required: e,
                granted: f,
            },
        ) => a == d && b == e && c == f,
        _ => false,
    }
}

impl Eq for RuntimeError {}
