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
        RuntimeError::AdmissionResourceCapacityExceeded { .. } => {
            Some("admission rejected: resource capacity exceeded")
        }
        RuntimeError::AdmissionBudgetPolicyExceeded { .. } => {
            Some("admission rejected: budget policy exceeded")
        }
        RuntimeError::AdmissionResourceBudgetOverflow { .. } => {
            Some("admission rejected: resource budget overflow")
        }
        RuntimeError::AdmissionResourceBudgetUnderflow { .. } => {
            Some("admission rejected: resource budget underflow")
        }
        RuntimeError::AdmissionResourceBudgetInvalidCapacity { .. } => {
            Some("admission rejected: resource budget invalid capacity")
        }
        RuntimeError::AdmissionArtifactEnvelopeDecodeFailed => {
            Some("admission rejected: artifact envelope decode failed")
        }
        RuntimeError::AdmissionArtifactInvalidGateCount { .. } => {
            Some("admission rejected: artifact invalid gate count")
        }
        RuntimeError::AdmissionArtifactInvalidProofFlag { .. } => {
            Some("admission rejected: artifact invalid proof flag")
        }
        RuntimeError::AdmissionBudgetExceeded { .. } => {
            Some("admission rejected: workflow step count exceeds per-workflow ceiling")
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
        RuntimeError::InputMappingFailed { kind, .. } => Some(kind.legacy_diagnostic_phrase()),
        RuntimeError::AskTimeout { .. } => Some("ask timer expired before answer arrived"),
        RuntimeError::WaitTimeout { .. } => Some("wait timer expired before deadline reached"),
        _ => None,
    }
}

fn write_runtime_error_dynamic(
    error: &RuntimeError,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    if write_capacity_or_storage_error(error, f)? {
        return Ok(());
    }
    if write_action_state_error(error, f)? {
        return Ok(());
    }
    if write_action_output_error(error, f)? {
        return Ok(());
    }
    if write_flow_error(error, f)? {
        return Ok(());
    }
    Ok(())
}

fn wrote(result: std::fmt::Result) -> Result<bool, std::fmt::Error> {
    result.map(|()| true)
}

fn write_capacity_or_storage_error(
    error: &RuntimeError,
    f: &mut std::fmt::Formatter<'_>,
) -> Result<bool, std::fmt::Error> {
    match error {
        RuntimeError::ActiveRunCapacityExceeded { capacity } => {
            wrote(write!(f, "active run capacity exceeded: {capacity}"))
        }
        RuntimeError::JournalFull { capacity } => {
            wrote(write!(f, "runtime journal capacity exhausted: {capacity}"))
        }
        RuntimeError::TerminalRunsLruFull { capacity } => wrote(write!(
            f,
            "terminal runs LRU capacity exhausted: {capacity}"
        )),
        RuntimeError::UnsupportedOperation { operation } => {
            wrote(write!(f, "unsupported runtime operation: {operation}"))
        }
        RuntimeError::Core { source } => wrote(write!(f, "runtime core error: {source}")),
        RuntimeError::StorageJournalAppend { source } => {
            wrote(write!(f, "storage journal append failed: {source}"))
        }
        RuntimeError::AdmissionHeaderPersistenceFailed { source } => {
            wrote(write!(f, "admission header persistence failed: {source}"))
        }
        RuntimeError::AdmissionArtifactStale { digest } => wrote(write!(
            f,
            "admission rejected: artifact certificate is stale: {digest:?}"
        )),
        _ => Ok(false),
    }
}

fn write_action_state_error(
    error: &RuntimeError,
    f: &mut std::fmt::Formatter<'_>,
) -> Result<bool, std::fmt::Error> {
    match error {
        RuntimeError::CommandQueueCapacityExceeded { capacity, max } => wrote(write!(
            f,
            "command queue capacity {capacity} exceeds maximum {max}"
        )),
        RuntimeError::StaleAttempt { incoming, current } => wrote(write!(
            f,
            "stale action attempt: incoming {incoming}, current {current}"
        )),
        RuntimeError::AttemptBeyondMax { attempt, max } => wrote(write!(
            f,
            "action attempt {attempt} exceeds max attempts {max}"
        )),
        RuntimeError::IpcPayloadSizeExceeded { size, max } => {
            wrote(write!(f, "IPC payload size {size} exceeds maximum {max}"))
        }
        RuntimeError::EngineDriveFailed { run, source } => {
            wrote(write!(f, "engine drive failed for run {run:?}: {source}"))
        }
        RuntimeError::ShardNotFound { shard } => wrote(write!(f, "shard {shard} not found")),
        _ => Ok(false),
    }
}

fn write_action_output_error(
    error: &RuntimeError,
    f: &mut std::fmt::Formatter<'_>,
) -> Result<bool, std::fmt::Error> {
    match error {
        RuntimeError::ActionOutputLengthMismatch { declared, actual } => write!(
            f,
            "action output declared encoded length {declared} does not match actual {actual}"
        )
        .map(|()| true),
        RuntimeError::ActionOutputTooLarge { size, max } => {
            wrote(write!(f, "action output size {size} exceeds maximum {max}"))
        }
        RuntimeError::ActionOutputBlobTooLarge { size, max } => wrote(write!(
            f,
            "action output blob size {size} exceeds maximum {max}"
        )),
        RuntimeError::ActionTaintDowngrade { required, supplied } => write!(
            f,
            "action output taint {supplied:?} is below required {required:?}"
        )
        .map(|()| true),
        _ => Ok(false),
    }
}

fn write_flow_error(
    error: &RuntimeError,
    f: &mut std::fmt::Formatter<'_>,
) -> Result<bool, std::fmt::Error> {
    if write_timer_flow_error(error, f)? {
        return Ok(true);
    }
    if write_collection_flow_error(error, f)? {
        return Ok(true);
    }
    if write_branch_flow_error(error, f)? {
        return Ok(true);
    }
    write_input_mapping_error(error, f)
}

fn write_timer_flow_error(
    error: &RuntimeError,
    f: &mut std::fmt::Formatter<'_>,
) -> Result<bool, std::fmt::Error> {
    match error {
        RuntimeError::AskTimeout { step, ask_id } => wrote(write!(
            f,
            "ask timer expired at step {step:?} for ask {ask_id:?}"
        )),
        RuntimeError::WaitTimeout { step } => {
            wrote(write!(f, "wait timer expired at step {step:?}"))
        }
        _ => Ok(false),
    }
}

fn write_collection_flow_error(
    error: &RuntimeError,
    f: &mut std::fmt::Formatter<'_>,
) -> Result<bool, std::fmt::Error> {
    match error {
        RuntimeError::CollectPageFailed {
            step,
            expected_page,
            found_page,
        } => wrote(write!(
            f,
            "collect page order violation at step {step:?}: expected {expected_page:?}, found {found_page:?}"
        )),
        RuntimeError::ReduceItemFailed {
            step,
            item_index,
            source,
        } => wrote(write!(
            f,
            "reduce body failed at step {step:?} on item {item_index}: {source}"
        )),
        _ => Ok(false),
    }
}

fn write_branch_flow_error(
    error: &RuntimeError,
    f: &mut std::fmt::Formatter<'_>,
) -> Result<bool, std::fmt::Error> {
    match error {
        RuntimeError::TogetherBranchFailed {
            step,
            branch_index,
            source,
        } => wrote(write!(
            f,
            "together branch {branch_index} failed at step {step:?}: {source}"
        )),
        RuntimeError::ForEachItemFailed {
            step,
            item_index,
            source,
        } => wrote(write!(
            f,
            "for-each body failed at step {step:?} on item {item_index}: {source}"
        )),
        _ => Ok(false),
    }
}

fn write_input_mapping_error(
    error: &RuntimeError,
    f: &mut std::fmt::Formatter<'_>,
) -> Result<bool, std::fmt::Error> {
    match error {
        RuntimeError::InputMappingFailed { kind, source } => {
            wrote(write!(f, "{}: {source}", kind.legacy_diagnostic_phrase()))
        }
        _ => Ok(false),
    }
}

impl std::error::Error for RuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Core { source } => Some(source.as_ref()),
            Self::StorageJournalAppend { source } => Some(source.as_ref()),
            Self::AdmissionHeaderPersistenceFailed { source } => Some(source.as_ref()),
            Self::EngineDriveFailed { source, .. } => Some(source.as_ref()),
            Self::ReduceItemFailed { source, .. } => Some(source.as_ref()),
            Self::TogetherBranchFailed { source, .. } => Some(source.as_ref()),
            Self::ForEachItemFailed { source, .. } => Some(source.as_ref()),
            Self::InputMappingFailed { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}
