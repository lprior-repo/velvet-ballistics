use vb_core::RunId;

/// Monotonic uncertainty retained by the actual journal, including after Drop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnedWriterQuarantine {
    /// A future guarded write may have reached its engine without a certain ack.
    IndeterminateCommit,
    /// The owning caller cannot establish that continued writing is safe.
    OwnershipUncertain,
}

/// Bounded typed writer-registry refusal; diagnostics contain no owned strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OwnedWriterError {
    /// The requested fixed capacity is zero, too large or unrepresentable.
    #[error("invalid owned-writer capacity {requested}; maximum {maximum}")]
    InvalidCapacity { requested: usize, maximum: usize },
    /// This journal has already bound a different fixed capacity.
    #[error("owned-writer capacity {requested} differs from bound {bound}")]
    CapacityMismatch { bound: usize, requested: usize },
    /// Another operation currently holds the registry gate.
    #[error("owned-writer registry contended")]
    Contended,
    /// A previous unwind poisoned the registry gate; it is never cleared.
    #[error("owned-writer registry poisoned")]
    Poisoned,
    /// The initial fixed table could not be reserved fallibly.
    #[error("owned-writer table allocation refused")]
    AllocationRefused,
    /// Every fixed slot remains held or quarantined.
    #[error("owned-writer registry full at capacity {capacity}")]
    Full { capacity: usize },
    /// An existing healthy owner for this exact run remains held.
    #[error("owned writer for run {run:?} already held")]
    Busy { run: RunId },
    /// Store-retained uncertainty prohibits release or new ownership.
    #[error("owned writer for run {run:?} quarantined: {reason:?}")]
    Quarantined {
        run: RunId,
        reason: OwnedWriterQuarantine,
    },
    /// Owner generations have exhausted their domain; the failure is permanent.
    #[error("owned-writer generations exhausted")]
    GenerationExhausted,
    /// A capability belongs to a different actual journal aggregate.
    #[error("owned writer belongs to another store")]
    WrongStore,
    /// The run, slot or generation no longer denotes the held ownership.
    #[error("stale owned writer for run {run:?}")]
    StaleOwnership { run: RunId },
    /// An internal fixed-table invariant failed closed.
    #[error("owned-writer registry invariant failure")]
    RegistryInvariant,
}
