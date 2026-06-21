use super::input_mapping::InputMappingFailureKind;
use std::sync::Arc;
use vb_core::ids::{RunId, StepIdx};

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
    /// Input bin could not be mapped to workflow slot values.
    InputMappingFailed {
        /// Specific mapping failure mode.
        kind: InputMappingFailureKind,
        /// Preserved core error source.
        source: Box<vb_core::errors::CoreError>,
    },
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
    /// Ask timer expired before an answer arrived for the suspended ask.
    AskTimeout {
        /// Step that issued the ask.
        step: StepIdx,
        /// Ask identifier (the ask_step from the AskTicket).
        ask_id: StepIdx,
    },
    /// Wait timer expired for a suspended wait/ask-wait step.
    WaitTimeout {
        /// Step that issued the wait.
        step: StepIdx,
    },
    /// Collect step observed an out-of-order page.
    CollectPageFailed {
        /// Step that hosts the Collect primitive.
        step: StepIdx,
        /// Page that the collector expected.
        expected_page: vb_core::ids::ListId,
        /// Page that the body delivered.
        found_page: vb_core::ids::ListId,
    },
    /// Reduce step body failed on a specific item.
    ReduceItemFailed {
        /// Step that hosts the Reduce primitive.
        step: StepIdx,
        /// Zero-based index of the item that failed.
        item_index: u32,
        /// Underlying source error from the body execution.
        source: Box<vb_core::errors::CoreError>,
    },
    /// Together parallel branch failed.
    TogetherBranchFailed {
        /// Step that hosts the Together primitive.
        step: StepIdx,
        /// Zero-based index of the failing branch.
        branch_index: u16,
        /// Underlying source error from the branch execution.
        source: Box<vb_core::errors::CoreError>,
    },
    /// ForEach body failed on a specific item.
    ForEachItemFailed {
        /// Step that hosts the ForEach primitive.
        step: StepIdx,
        /// Zero-based index of the item that failed.
        item_index: u32,
        /// Underlying source error from the body execution.
        source: Box<vb_core::errors::CoreError>,
    },
    /// Admission gate rejected because the workflow step count exceeds the
    /// master contract per-workflow ceiling
    /// (`vb_core::limits::MAX_STEPS_PER_WORKFLOW`).
    AdmissionBudgetExceeded {
        /// Observed step count from the declaration or computed IR budget.
        actual: u32,
        /// Per-workflow ceiling from the master contract.
        limit: u32,
    },
    /// Terminal-runs LRU ring is at capacity and no TTL-expired entry
    /// could be evicted to make room. Returned by
    /// `Shard::terminal_runs_try_insert` when the bounded ring refuses
    /// a new terminal-runs insertion.
    TerminalRunsLruFull {
        /// Configured terminal-runs ring capacity.
        capacity: usize,
    },
    /// `LruRing::try_new` rejected a capacity of zero. A zero-capacity
    /// ring would silently rewrite to capacity 1, which is the
    /// black-hat finding BH-W0-S09; the constructor now surfaces a
    /// typed error so the configuration bug is visible at the runtime
    /// boundary instead of being silently normalised.
    LruRingCapacityZero,
}
