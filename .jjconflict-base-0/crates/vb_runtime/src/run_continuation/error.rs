use crate::primitives::collect::CollectStatesCheckpointV1Error;
use vb_core::frame::RunFrameCheckpointV1Error;
use vb_core::root_bundle::OwnedRootBundleError;

/// Typed refusal of malformed, mismatched or unsupported continuation state.
#[derive(Debug, thiserror::Error)]
pub enum RunContinuationError {
    /// Payload version is unsupported.
    #[error("unsupported continuation version {version}")]
    UnsupportedVersion { version: u16 },
    /// Encoded data was malformed.
    #[error("continuation codec failure: {0}")]
    Codec(#[from] postcard::Error),
    /// Bytes remain after a complete payload.
    #[error("continuation payload has trailing bytes")]
    TrailingBytes,
    /// Complete domain-separated digest did not match.
    #[error("continuation digest mismatch")]
    Digest,
    /// An independently supplied run/artifact/source identity differs.
    #[error("continuation content identity mismatch")]
    Identity,
    /// Attempt identity differs.
    #[error("continuation attempt mismatch")]
    AttemptIdentity,
    /// Prior durable sequence differs.
    #[error("continuation prior sequence mismatch")]
    SequenceIdentity,
    /// Independently accepted workflow or projection differs.
    #[error("continuation workflow mismatch")]
    WorkflowIdentity,
    /// Historical admission cannot be reconstructed exactly from the receipt.
    #[error("continuation historical admission mismatch")]
    Admission,
    /// Accepted full action contracts differ.
    #[error("continuation action contracts mismatch")]
    Contracts,
    /// Canonical aggregate workflow budget could not be computed.
    #[error("continuation aggregate budget validation failed: {0:?}")]
    Budget(#[source] Box<vb_core::budget::AggregateBudgetError>),
    /// Execution or parallel counter exceeds the canonical admitted budget.
    #[error("continuation counters differ from canonical admitted bounds")]
    CounterBounds,
    /// Frame dimensions or counter table differs from the workflow.
    #[error("continuation frame dimensions mismatch")]
    Dimensions,
    /// No unique supported compiled Collect boundary matches.
    #[error("unsupported continuation boundary")]
    UnsupportedBoundary,
    /// An external/wait/running/parallel state lacks reconciliation authority.
    #[error("continuation has unsupported in-flight state")]
    InFlight,
    /// Live Collect pagination or lineage remains at a terminal boundary.
    #[error("continuation terminal Collect state is inconsistent")]
    CollectState,
    /// Compiled edge, terminal state, source or output shape differs.
    #[error("continuation compiled boundary state mismatch")]
    BoundaryState,
    /// Original identifier-ordered arena prefix differs.
    #[error("continuation original owned arena prefix mismatch")]
    OwnedPrefix,
    /// Original initialized root value or taint differs.
    #[error("continuation original owned root mismatch")]
    OwnedRoot,
    /// Frame DTO validation failed; the exact typed error is retained.
    #[error("continuation frame validation failed: {0:?}")]
    Frame(RunFrameCheckpointV1Error),
    /// Collect DTO validation failed; the exact typed error is retained.
    #[error("continuation Collect checkpoint failed: {0:?}")]
    CollectCheckpoint(CollectStatesCheckpointV1Error),
    /// Runtime store graph, cap, references or taint are invalid.
    #[error("continuation runtime store validation failed: {0}")]
    RuntimeStore(#[source] Box<OwnedRootBundleError>),
    /// Local core validation failed.
    #[error("continuation core validation failed: {0}")]
    Core(#[from] vb_core::CoreError),
    /// Accepted artifact or owned input receipt validation failed.
    #[error("continuation storage authority failed: {0}")]
    Storage(#[source] Box<vb_storage::JournalError>),
    /// Existing runtime counter derivation failed.
    #[error("continuation engine validation failed: {0}")]
    Engine(#[source] Box<crate::engine::RuntimeEngineError>),
    /// Existing runtime admission authority validation failed.
    #[error("continuation admission validation failed: {0}")]
    Runtime(#[source] Box<crate::RuntimeError>),
}

impl From<OwnedRootBundleError> for RunContinuationError {
    fn from(error: OwnedRootBundleError) -> Self {
        Self::RuntimeStore(Box::new(error))
    }
}

impl From<vb_storage::JournalError> for RunContinuationError {
    fn from(error: vb_storage::JournalError) -> Self {
        Self::Storage(Box::new(error))
    }
}

impl From<crate::RuntimeError> for RunContinuationError {
    fn from(error: crate::RuntimeError) -> Self {
        Self::Runtime(Box::new(error))
    }
}
