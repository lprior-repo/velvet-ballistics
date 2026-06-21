#![forbid(unsafe_code)]
//! Journal writer queue capacity, batch size, and flush reporting.

use std::num::NonZeroUsize;

/// Non-zero bounded capacity for the journal writer queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct JournalQueueCapacity(NonZeroUsize);

impl JournalQueueCapacity {
    /// Creates a queue-capacity contract from a proven non-zero value.
    #[must_use]
    pub const fn new(value: NonZeroUsize) -> Self {
        Self(value)
    }

    /// Validates a raw queue capacity.
    pub fn try_from_usize(value: usize) -> Result<Self, crate::JournalError> {
        NonZeroUsize::new(value)
            .map(Self::new)
            .ok_or(crate::JournalError::QueueCapacity)
    }

    /// Returns the raw capacity.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0.get()
    }
}

/// Non-zero bounded batch size for the journal writer queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct JournalBatchSize(NonZeroUsize);

impl JournalBatchSize {
    /// Creates a batch-size contract from a proven non-zero value.
    #[must_use]
    pub const fn new(value: NonZeroUsize) -> Self {
        Self(value)
    }

    /// Validates a raw batch size.
    pub fn try_from_usize(value: usize) -> Result<Self, crate::JournalError> {
        NonZeroUsize::new(value)
            .map(Self::new)
            .ok_or(crate::JournalError::QueueCapacity)
    }

    /// Returns the raw batch size.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0.get()
    }
}

/// Counts queued journal writes by durability profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalWriterQueueProfileCounts {
    /// Number of journaled pending writes.
    pub journaled: usize,
    /// Number of strict pending writes.
    pub strict: usize,
}

/// Result of flushing a bounded writer queue batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalWriterFlushReport {
    /// Number of queued events drained from memory.
    pub drained: usize,
    /// Number of events written to Fjall.
    pub written: usize,
    /// Number of events still queued after the operation completed.
    ///
    /// Zero when the queue was fully drained. A non-zero value indicates
    /// the static iteration bound was exhausted before the queue was
    /// empty — typically because concurrent producers replenished the
    /// queue mid-drain. Callers that need a fully-empty drain must
    /// observe `pending_after == 0` and retry or escalate.
    pub pending_after: usize,
}
