#![forbid(unsafe_code)]
#![deny(unused_must_use)]
//! Shared mutex-guarded [`SharedBoundaryTranscript`] handle.
//!
//! The mutex is held only for the duration of a single `push` or
//! `snapshot`, both of which are O(1) for push and O(n) for snapshot.

use std::sync::{Arc, Mutex};

use crate::boundary_transcript::event::{BoundaryEvent, TranscriptSeq};
use crate::boundary_transcript::transcript::BoundaryTranscriptEntry;
use crate::boundary_transcript::transcript::{BoundaryTranscript, BoundaryTranscriptError};

/// Shared cold-path transcript handle.
#[derive(Debug, Clone)]
pub struct SharedBoundaryTranscript {
    inner: Arc<Mutex<BoundaryTranscript>>,
}

impl SharedBoundaryTranscript {
    /// Creates a shared transcript with default capacity.
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(BoundaryTranscript::DEFAULT_CAPACITY)
    }

    /// Creates a shared transcript with explicit capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BoundaryTranscript::with_capacity(capacity))),
        }
    }

    /// Pushes a boundary event. Returns the assigned sequence wrapped in
    /// `Ok(Some(seq))`, or propagates an [`BoundaryTranscriptError`] for
    /// the typed failure modes (poisoned mutex, allocation failure,
    /// sequence saturation).
    pub fn push(
        &self,
        event: BoundaryEvent,
    ) -> Result<Option<TranscriptSeq>, BoundaryTranscriptError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BoundaryTranscriptError::MutexPoisoned)?;
        guard.push(event)
    }

    /// Returns the number of entries currently retained.
    pub fn len(&self) -> Result<usize, BoundaryTranscriptError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| BoundaryTranscriptError::MutexPoisoned)?;
        Ok(guard.len())
    }

    /// Returns true if no entries are retained.
    pub fn is_empty(&self) -> Result<bool, BoundaryTranscriptError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| BoundaryTranscriptError::MutexPoisoned)?;
        Ok(guard.is_empty())
    }

    /// Returns the configured capacity.
    pub fn capacity(&self) -> Result<usize, BoundaryTranscriptError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| BoundaryTranscriptError::MutexPoisoned)?;
        Ok(guard.capacity())
    }

    /// Returns the cumulative number of dropped entries.
    pub fn dropped(&self) -> Result<u64, BoundaryTranscriptError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| BoundaryTranscriptError::MutexPoisoned)?;
        Ok(guard.dropped())
    }

    /// Returns a snapshot of retained entries.
    pub fn snapshot(&self) -> Result<Vec<BoundaryTranscriptEntry>, BoundaryTranscriptError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| BoundaryTranscriptError::MutexPoisoned)?;
        Ok(guard.snapshot())
    }

    /// Returns a range snapshot `[from_seq, to_seq)`.
    pub fn snapshot_range(
        &self,
        from_seq: TranscriptSeq,
        to_seq: TranscriptSeq,
    ) -> Result<Vec<BoundaryTranscriptEntry>, BoundaryTranscriptError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| BoundaryTranscriptError::MutexPoisoned)?;
        Ok(guard.snapshot_range(from_seq, to_seq))
    }
}

impl Default for SharedBoundaryTranscript {
    fn default() -> Self {
        Self::new()
    }
}
