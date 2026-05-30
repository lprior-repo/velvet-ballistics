use std::sync::Arc;
use vb_core::ids::RunId;

/// Runtime error type.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum RuntimeError {
    /// Bounded queue is full.
    QueueFull,
    /// Run identifier not found.
    RunNotFound,
    /// Active run capacity for a shard has been exhausted.
    ActiveRunCapacityExceeded {
        /// Configured active-run capacity.
        capacity: usize,
    },
    /// Run identifier is already active on the shard.
    RunAlreadyExists,
    /// Runtime API exists, but the durable path is not implemented yet.
    UnsupportedOperation {
        /// Static operation code.
        operation: &'static str,
    },
    /// Shutdown is in progress.
    ShutdownInProgress,
    /// Runtime journal mutex was poisoned.
    JournalPoisoned,
    /// Runtime journal reached its configured capacity without dropping or overwriting entries.
    JournalFull {
        /// Configured journal capacity in events.
        capacity: usize,
    },
    /// Core execution failure propagated through the runtime boundary.
    Core {
        /// Preserved core error source.
        source: Box<vb_core::errors::CoreError>,
    },
    /// Durable storage journal failure propagated through the runtime boundary.
    StorageJournalAppend {
        /// Preserved storage journal source.
        source: Arc<vb_storage::JournalError>,
    },
    /// Durable run header persistence failed before admission acknowledgement.
    AdmissionHeaderPersistenceFailed {
        /// Preserved storage journal source.
        source: Arc<vb_storage::JournalError>,
    },
    /// Queued strict mode cannot acknowledge before persistence.
    UnsupportedAsyncStrictAck,
    /// A run frame could not be taken from or returned to the frame pool.
    FramePoolUnavailable,
    /// Action completion did not match the suspended Do step.
    InvalidActionCompletion,
    /// Action completion belongs to an older attempt than the live suspended attempt.
    StaleAttempt {
        /// Attempt carried by the incoming completion ticket.
        incoming: u16,
        /// Current attempt recorded on the live run state.
        current: u16,
    },
    /// Action completion or scheduling exceeded the bounded retry capacity.
    AttemptBeyondMax {
        /// Attempt carried by the ticket.
        attempt: u16,
        /// Maximum attempt count from the retry policy.
        max: u16,
    },
    /// Timer fired for a run that is not suspended on a registered timer.
    InvalidTimerFire,
    /// Durable recovery can expose a summary, but cannot yet rebuild a live frame.
    UnsupportedFullRecoveryHydration,
    /// Durable recovery frame seed was internally inconsistent.
    InvalidRecoveryHydration,
    /// Command queue capacity exceeds the maximum allowed.
    CommandQueueCapacityExceeded {
        /// Requested capacity.
        capacity: usize,
        /// Maximum allowed capacity.
        max: usize,
    },
    /// Active run capacity cannot be zero.
    ActiveRunCapacityZero,
    /// Admission gate rejected the run because the compiled artifact was not found.
    AdmissionArtifactNotFound {
        /// Digest of the artifact that was expected but not found.
        digest: vb_core::ids::WorkflowDigest,
    },
    /// Admission gate rejected the run because the accepted artifact envelope is invalid.
    AdmissionArtifactInvalid {
        /// Digest of the invalid artifact.
        digest: vb_core::ids::WorkflowDigest,
    },
    /// Admission gate rejected because the artifact digest did not match the requested digest.
    AdmissionArtifactDigestMismatch {
        /// Digest that was requested at admission.
        requested: vb_core::ids::WorkflowDigest,
        /// Digest found inside the loaded artifact envelope.
        found: vb_core::ids::WorkflowDigest,
    },
    /// Admission gate rejected the run because the required capability was not granted.
    AdmissionCapabilityDenied {
        /// Action that required the capability.
        action: vb_core::ids::ActionId,
        /// Capability that was required but not granted.
        required: vb_core::capability::Capability,
        /// Capabilities that were granted at admission time.
        granted: vb_core::capability::CapabilitySet,
    },
    /// Admission gate rejected the run because the artifact certificate is stale.
    AdmissionArtifactStale {
        /// Digest of the stale artifact.
        digest: vb_core::ids::WorkflowDigest,
    },
    /// Admission gate rejected the run because the artifact digest does not match.
    AdmissionDigestMismatch {
        /// Digest that was requested for admission.
        requested: vb_core::ids::WorkflowDigest,
        /// Digest found in the stored record.
        record: vb_core::ids::WorkflowDigest,
        /// Digest in the accepted artifact envelope.
        envelope: vb_core::ids::WorkflowDigest,
    },
    /// Failed to encode a slot value for journal persistence.
    EncodeFailed,
    /// Caller-declared action output length did not match encoded bytes.
    ActionOutputLengthMismatch {
        /// Caller-declared encoded length.
        declared: u32,
        /// Runtime-computed encoded length.
        actual: u32,
    },
    /// Encoded action output exceeded the action contract byte limit.
    ActionOutputTooLarge {
        /// Runtime-computed encoded length.
        size: u32,
        /// Maximum allowed by the action contract.
        max: u32,
    },
    /// Encoded action output exceeded the workflow blob byte limit.
    ActionOutputBlobTooLarge {
        /// Runtime-computed encoded length.
        size: u64,
        /// Maximum allowed by the workflow resource contract.
        max: u64,
    },
    /// Action output taint attempted to downgrade required taint propagation.
    ActionTaintDowngrade {
        /// Minimum taint required by the action input and contract.
        required: vb_core::Taint,
        /// Taint supplied by the completion payload.
        supplied: vb_core::Taint,
    },
    /// Secret-tainted answer payload is not allowed by the resource contract.
    SecretResultNotAllowed,
    /// IPC payload size exceeds the maximum allowed by the resource contract.
    IpcPayloadSizeExceeded {
        /// Actual encoded payload size in bytes.
        size: u32,
        /// Maximum allowed payload size in bytes.
        max: u32,
    },
    /// Deterministic engine drive failed and the cause must survive terminal failure handling.
    EngineDriveFailed {
        /// Run identifier.
        run: RunId,
        /// Preserved error source from the engine.
        source: Box<vb_core::errors::CoreError>,
    },
    /// Target shard does not exist.
    ShardNotFound {
        /// Shard index that was not found.
        shard: u32,
    },
    /// Migrate directive targeted the source shard (self-migrate).
    MigrateSelf,
}

impl From<std::io::Error> for RuntimeError {
    fn from(_: std::io::Error) -> Self {
        RuntimeError::JournalPoisoned
    }
}

/// Result alias for runtime operations.
pub type RuntimeResult<T> = Result<T, RuntimeError>;

mod conversions;
mod diagnostics;
mod display;
mod equality;

#[cfg(test)]
mod tests_basic;
#[cfg(test)]
mod tests_conversion_refinement;
#[cfg(test)]
mod tests_diagnostics;
