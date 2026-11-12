use vb_core::value_store::transport::CheckpointTransportError;

/// Typed cold-wire refusal; none of these outcomes grants replay authority.
#[derive(Debug, thiserror::Error)]
pub enum CarrierError {
    /// Existing envelope, key, checksum or family validation failed.
    #[error(transparent)]
    Storage(#[from] crate::JournalError),
    /// A resource charge or allocation failed before its corresponding work.
    #[error(transparent)]
    Bounds(#[from] CheckpointTransportError),
    /// Canonical scalar or exact-consumption rule failed.
    #[error("malformed continuation carrier: {0}")]
    Malformed(&'static str),
    /// Unsupported explicit version.
    #[error("unsupported continuation {field} version {version}")]
    Version { field: &'static str, version: u16 },
    /// Independent key, envelope or sequence identity disagrees.
    #[error("continuation identity mismatch: {0}")]
    Identity(&'static str),
    /// Opaque component integrity disagrees with the explicit digest.
    #[error("continuation component digest mismatch")]
    Digest,
    /// An abandoned, completed or already used account cannot mint allowances.
    #[error("continuation operation account closed or already used")]
    AccountClosed,
}
