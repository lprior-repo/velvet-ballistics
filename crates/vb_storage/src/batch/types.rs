//! Domain types for the batch system.
//!
//! - `BatchState` — explicit lifecycle state (open / aborted).
//! - `BatchByteLimit` — strongly typed byte budget newtype.

/// Default journal batch encoded-byte budget (1 MiB).
///
/// Matches the core `max_journal_batch_bytes` default of `1_048_576`.
pub const DEFAULT_JOURNAL_BATCH_BYTE_LIMIT: u64 = 1_048_576;

/// Lifecycle state of a [`crate::batch::JournalWriteBatch`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BatchState {
    /// The batch is accepting operations.
    #[default]
    Open,
    /// The batch was aborted by a staging operation that surfaced a typed
    /// error. Carries a stable reason string for diagnostic reporting.
    Aborted {
        /// Stable reason string identifying which staging method aborted
        /// the batch (e.g., `"duplicate_event"`, `"payload_digest_mismatch"`).
        reason: &'static str,
    },
}

impl BatchState {
    /// Returns `true` when the batch is in the aborted state.
    #[must_use]
    pub fn is_aborted(&self) -> bool {
        matches!(self, Self::Aborted { .. })
    }

    /// Returns the reason string when aborted, or `None` if open.
    #[must_use]
    pub fn abort_reason(&self) -> Option<&'static str> {
        match self {
            Self::Aborted { reason } => Some(reason),
            Self::Open => None,
        }
    }
}

/// Strongly typed byte budget for journal event admission.
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
