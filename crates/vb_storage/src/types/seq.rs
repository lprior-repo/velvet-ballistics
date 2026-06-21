#![forbid(unsafe_code)]
//! Monotonic per-run event sequence.

use crate::error::JournalError;

/// Monotonic per-run event sequence.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[repr(transparent)]
pub struct EventSeq(u64);

impl EventSeq {
    /// Creates an event sequence.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw sequence value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Fallible constructor that rejects the reserved sentinel `u64::MAX`.
    ///
    /// The Fjall key decoder refuses `u64::MAX` (it would otherwise collide
    /// with the absence of a sequence in range queries), so encoders must not
    /// emit it. This constructor is the safe entry point for callers that
    /// plan to persist the sequence into a storage key.
    ///
    /// # Errors
    ///
    /// Returns [`JournalError::ReservedSeqSentinel`] when `value == u64::MAX`.
    pub const fn try_new(value: u64) -> Result<Self, JournalError> {
        if value == u64::MAX {
            Err(JournalError::ReservedSeqSentinel)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns `true` if `value` would be safe to encode into a storage key.
    #[must_use]
    pub const fn is_reserved_sentinel(value: u64) -> bool {
        value == u64::MAX
    }

    /// Zero event sequence.
    pub const ZERO: Self = Self(0);
    /// Minimum event sequence.
    pub const MIN: Self = Self(0);
    /// Maximum event sequence.
    ///
    /// `u64::MAX` is the Fjall key decoder's reserved sentinel and must
    /// not be encoded into a storage key. Use [`EventSeq::MAX_ENCODABLE`]
    /// for the highest value that may appear in a key, or
    /// [`EventSeq::try_new`] to fail closed.
    pub const MAX: Self = Self(u64::MAX);
    /// Maximum encodable event sequence (`u64::MAX - 1`).
    ///
    /// Use this in place of [`EventSeq::MAX`] whenever the value will be
    /// passed to a key encoder, since the Fjall key decoder rejects
    /// `u64::MAX` as a reserved sentinel.
    pub const MAX_ENCODABLE: Self = Self(u64::MAX - 1);
}

#[cfg(kani)]
impl kani::Arbitrary for EventSeq {
    fn any() -> Self {
        Self::new(kani::any())
    }
}
