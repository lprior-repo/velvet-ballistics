//! Domain types for the batch system.
//!
//! - `BatchState` — explicit lifecycle state (open / aborted).
//! - `BatchByteLimit` — strongly-typed byte budget newtype.

/// Default journal batch encoded-byte budget (1 MiB).
///
/// Matches the core `max_journal_batch_bytes` default of `1_048_576`.
pub const DEFAULT_JOURNAL_BATCH_BYTE_LIMIT: u64 = 1_048_576;

// =========================================================================
// Domain types: BatchState and BatchByteLimit
// =========================================================================

/// Lifecycle state of a [`crate::batch::JournalWriteBatch`].
///
/// - [`BatchState::Open`] — the batch accepts operations.
/// - [`BatchState::Aborted`] — an unrecoverable error occurred; the batch
///   must not stage further writes and will silently discard on commit.
///
/// This is an explicit state machine rather than a boolean flag, making
/// illegal states unrepresentable at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BatchState {
    /// The batch is accepting operations.
    #[default]
    Open,
    /// The batch was aborted due to a domain error (e.g. duplicate event).
    Aborted,
}

impl BatchState {
    /// Returns `true` when the batch is in the aborted state.
    #[must_use]
    pub fn is_aborted(&self) -> bool {
        matches!(self, Self::Aborted)
    }
}

/// Strongly-typed byte budget for journal event admission.
///
/// Wraps `u64` so that callers cannot accidentally pass an arbitrary
/// integer where a batch byte limit is expected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BatchByteLimit(u64);

impl BatchByteLimit {
    /// Creates a byte limit from the given capacity.
    #[inline]
    pub const fn bounded(bytes: u64) -> Self {
        Self(bytes)
    }

    /// Returns the limit as a plain `u64`.
    #[inline]
    pub fn as_u64(self) -> u64 {
        self.0
    }
}
