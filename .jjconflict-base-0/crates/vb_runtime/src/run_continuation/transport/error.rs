use super::super::RunContinuationError;
use vb_core::value_store::transport::{CheckpointTransportError, TransportUsage};

/// Typed transport refusal with the exact phase and pre-allocation charges retained.
#[derive(Debug, thiserror::Error)]
#[error("continuation transport failed: {failure}")]
pub struct ContinuationTransportError {
    /// Exact failure, preserving strong semantic validation errors.
    #[source]
    pub failure: TransportFailure,
    /// Accepted charges before failure; these are not measured allocations.
    pub usage: TransportUsage,
    /// True only after complete structural/authority/integrity preflight succeeded.
    pub strong_decode_started: bool,
}

/// Typed structural, resource, codec and strong-validator failures.
#[derive(Debug, thiserror::Error)]
pub enum TransportFailure {
    /// Structural allocation/work limit refusal.
    #[error(transparent)]
    Bounds(#[from] CheckpointTransportError),
    /// Bounded serializer failure, including complete payload exhaustion.
    #[error("bounded continuation encoding failed: {0}")]
    Encoding(#[from] postcard::Error),
    /// An unsupported component version was encountered before decoding.
    #[error("unsupported continuation transport version {0}")]
    UnsupportedVersion(u64),
    /// Complete raw body digest differs from the encoded digest.
    #[error("continuation transport integrity mismatch")]
    Integrity,
    /// Existing ordinary strong validation failed; no validator is bypassed.
    ///
    /// Boxed because the strong-continuation error carries an action resume
    /// report, which carries a full ticket; inline, that chain pushes the
    /// enclosing transport error past the large-error threshold.
    #[error(transparent)]
    Strong(Box<RunContinuationError>),
}

impl From<RunContinuationError> for TransportFailure {
    fn from(error: RunContinuationError) -> Self {
        Self::Strong(Box::new(error))
    }
}
