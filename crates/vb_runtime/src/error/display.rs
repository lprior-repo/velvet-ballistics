use super::RuntimeError;

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(message) = runtime_error_static_message(self) {
            f.write_str(message)
        } else {
            write_runtime_error_dynamic(self, f)
        }
    }
}

fn runtime_error_static_message(error: &RuntimeError) -> Option<&'static str> {
    match error {
        RuntimeError::QueueFull => Some("queue full"),
        RuntimeError::RunNotFound => Some("run not found"),
        RuntimeError::RunAlreadyExists => Some("run already exists"),
        RuntimeError::ShutdownInProgress => Some("shutdown in progress"),
        RuntimeError::JournalPoisoned => Some("runtime journal lock poisoned"),
        RuntimeError::UnsupportedAsyncStrictAck => {
            Some("queued strict journal ack is unsupported without persisted-before-ack proof")
        }
        RuntimeError::FramePoolUnavailable => Some("frame pool unavailable"),
        RuntimeError::InvalidActionCompletion => Some("invalid action completion"),
        RuntimeError::InvalidTimerFire => Some("invalid timer fire"),
        RuntimeError::UnsupportedFullRecoveryHydration => {
            Some("full run frame recovery hydration is unsupported")
        }
        RuntimeError::InvalidRecoveryHydration => Some("invalid recovery frame hydration"),
        RuntimeError::ActiveRunCapacityZero => Some("active run capacity cannot be zero"),
        RuntimeError::AdmissionArtifactNotFound { .. } => {
            Some("admission rejected: artifact not found")
        }
        RuntimeError::AdmissionArtifactInvalid { .. } => {
            Some("admission rejected: artifact invalid")
        }
        RuntimeError::AdmissionArtifactDigestMismatch { .. } => {
            Some("admission rejected: artifact digest mismatch")
        }
        RuntimeError::AdmissionArtifactStale { .. } => {
            Some("admission rejected: artifact certificate is stale")
        }
        RuntimeError::AdmissionDigestMismatch { .. } => {
            Some("admission rejected: artifact digest mismatch")
        }
        RuntimeError::AdmissionCapabilityDenied { .. } => {
            Some("admission rejected: capability denied")
        }
        RuntimeError::AdmissionCapabilityCountMismatch { .. } => {
            Some("admission rejected: capability count mismatch")
        }
        RuntimeError::AdmissionHeaderPersistenceFailed { .. } => {
            Some("admission durability failed: header persistence failed")
        }
        RuntimeError::EncodeFailed => Some("slot value encoding failed"),
        RuntimeError::SecretResultNotAllowed => {
            Some("secret-tainted answer payload is not allowed by resource contract")
        }
        RuntimeError::EngineDriveFailed { .. } => Some("deterministic engine drive failed"),
        RuntimeError::MigrateSelf => Some("migration target is the source shard"),
        RuntimeError::IntrospectionEpochExhausted => {
            Some("introspection registry epoch space exhausted (u64::MAX)")
        }
        _ => None,
    }
}

fn write_runtime_error_dynamic(
    error: &RuntimeError,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    match error {
        RuntimeError::ActiveRunCapacityExceeded { capacity } => {
            write!(f, "active run capacity exceeded: {capacity}")
        }
        RuntimeError::JournalFull { capacity } => {
            write!(f, "runtime journal capacity exhausted: {capacity}")
        }
        RuntimeError::UnsupportedOperation { operation } => {
            write!(f, "unsupported runtime operation: {operation}")
        }
        RuntimeError::Core { source } => write!(f, "runtime core error: {source}"),
        RuntimeError::StorageJournalAppend { source } => {
            write!(f, "storage journal append failed: {source}")
        }
        RuntimeError::RollbackFailed {
            operation,
            primary,
            rollback,
        } => write!(
            f,
            "rollback failed during {operation}: primary={primary}; rollback={rollback}"
        ),
        RuntimeError::AdmissionHeaderPersistenceFailed { source } => {
            write!(f, "admission header persistence failed: {source}")
        }
        RuntimeError::AdmissionArtifactStale { digest } => {
            write!(
                f,
                "admission rejected: artifact certificate is stale: {digest:?}"
            )
        }
        RuntimeError::CommandQueueCapacityExceeded { capacity, max } => {
            write!(f, "command queue capacity {capacity} exceeds maximum {max}")
        }
        RuntimeError::StaleAttempt { incoming, current } => {
            write!(
                f,
                "stale action attempt: incoming {incoming}, current {current}"
            )
        }
        RuntimeError::AttemptBeyondMax { attempt, max } => {
            write!(f, "action attempt {attempt} exceeds max attempts {max}")
        }
        RuntimeError::IpcPayloadSizeExceeded { size, max } => {
            write!(f, "IPC payload size {size} exceeds maximum {max}")
        }
        RuntimeError::ActionOutputLengthMismatch { declared, actual } => write!(
            f,
            "action output declared encoded length {declared} does not match actual {actual}"
        ),
        RuntimeError::ActionOutputTooLarge { size, max } => {
            write!(f, "action output size {size} exceeds maximum {max}")
        }
        RuntimeError::ActionOutputBlobTooLarge { size, max } => {
            write!(f, "action output blob size {size} exceeds maximum {max}")
        }
        RuntimeError::ActionTaintDowngrade { required, supplied } => write!(
            f,
            "action output taint {supplied:?} is below required {required:?}"
        ),
        RuntimeError::UnsupportedRuntimeJournalEventMapping { event_kind } => {
            write!(f, "unsupported runtime journal event mapping: {event_kind}")
        }
        RuntimeError::RuntimeJournalTimestampOutOfRange {
            event_kind,
            timestamp,
        } => write!(
            f,
            "runtime journal timestamp out of range for {event_kind}: {timestamp}"
        ),
        RuntimeError::EngineDriveFailed { run, source } => {
            write!(f, "engine drive failed for run {run:?}: {source}")
        }
        RuntimeError::ShardNotFound { shard } => {
            write!(f, "shard {shard} not found")
        }
        _ => Ok(()),
    }
}

impl std::error::Error for RuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Core { source } => Some(source.as_ref()),
            Self::StorageJournalAppend { source } => Some(source.as_ref()),
            Self::RollbackFailed { primary, .. } => Some(primary.as_ref()),
            Self::AdmissionHeaderPersistenceFailed { source } => Some(source.as_ref()),
            Self::EngineDriveFailed { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}
