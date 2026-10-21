use thiserror::Error;

/// Typed failures preserve uncertain I/O outcomes for receipt reconciliation.
#[derive(Debug, Error)]
pub enum ExecutorError {
    /// Storage engine failure; an atomic commit may already have happened.
    #[error("executor storage: {0}")]
    Storage(#[from] fjall::Error),
    /// Scratch or production filesystem operation failed.
    #[error("executor filesystem: {0}")]
    Io(#[from] std::io::Error),
    /// Wire encoding or decoding failed.
    #[error("executor codec: {0}")]
    Codec(#[from] postcard::Error),
    /// Artifact verifier failed.
    #[error("artifact verification: {0}")]
    Artifact(#[from] crate::JournalError),
    /// Registered identity, ABI, scope, timeout, schema or classification disagrees.
    #[error("unsupported or mismatched action pack")]
    PackMismatch,
    /// Raw packed evidence cannot establish a trusted catalog.
    #[error("registered executor catalog required")]
    CatalogRequired,
    /// A persisted logical ticket has another input, resource or pack.
    #[error("durable receipt conflict")]
    ReceiptConflict,
    /// The bounded store cannot accept a new logical ticket.
    #[error("durable receipt capacity exhausted")]
    Capacity,
    /// Grant does not match the exact artifact/run/action/ticket/resource.
    #[error("execution authority mismatch")]
    Unauthorized,
    /// Ticket key or attempt bounds are invalid.
    #[error("invalid action ticket")]
    InvalidTicket,
    /// Adapter accepts only one I64 value within its byte bound.
    #[error("invalid register input")]
    InvalidInput,
    /// Existing metadata or receipt is inconsistent.
    #[error("executor store is corrupt or incompatible")]
    Corrupt,
    /// Concrete duplicate/conflict/reopen observations failed their contract.
    #[error("registered executor conformance failed")]
    Conformance,
    /// Deadline expired; reconcile a receipt before attempting another effect.
    #[error("executor deadline expired; receipt reconciliation required")]
    Timeout,
    /// Recovery requires an existing receipt and never executes an effect.
    #[error("no durable receipt; execution is blocked")]
    ReceiptMissing,
}
