#![forbid(unsafe_code)]
#![deny(unused_must_use)]
//! Bounded in-memory [`BoundaryTranscript`] with typed [`BoundaryTranscriptError`].
//!
//! # Capacity policy
//!
//! [`BoundaryTranscript`] uses a **FIFO** eviction policy: when the buffer
//! reaches capacity, the **oldest** entry is dropped and the `dropped()`
//! counter is incremented. LIFO (drop newest) was rejected because older
//! entries carry the schedule/setup state required for replay and must
//! not be the first entries evicted.
//!
//! The shared mutex-guarded handle [`crate::boundary_transcript::SharedBoundaryTranscript`]
//! lives in the sibling `shared_transcript` module.

use std::collections::VecDeque;

use crate::boundary_transcript::event::{BoundaryEvent, TranscriptSeq};

/// Entry stored in the transcript with its monotonic sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryTranscriptEntry {
    /// Stable monotonic sequence assigned at capture time.
    pub seq: TranscriptSeq,
    /// Captured boundary event.
    pub event: BoundaryEvent,
}

/// Cold-path in-memory boundary transcript.
///
/// # Capacity policy
///
/// When the transcript reaches `capacity`, the **oldest** entry is dropped
/// (FIFO) so that the most recent boundary events remain visible. Surviving
/// readers can observe the dropped count via [`BoundaryTranscript::dropped`].
#[derive(Debug)]
pub struct BoundaryTranscript {
    entries: VecDeque<BoundaryTranscriptEntry>,
    capacity: usize,
    dropped: u64,
    next_seq: TranscriptSeq,
}

impl BoundaryTranscript {
    /// Default capacity for a boundary transcript.
    pub const DEFAULT_CAPACITY: usize = 4096;

    /// Creates an empty transcript with the default capacity.
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(Self::DEFAULT_CAPACITY)
    }

    /// Creates an empty transcript with explicit bounded capacity.
    ///
    /// The capacity is clamped to at least 1 so the buffer always has room
    /// for the most recent event.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        let bounded = capacity.max(1);
        Self {
            entries: VecDeque::with_capacity(bounded),
            capacity: bounded,
            dropped: 0,
            next_seq: 0,
        }
    }

    /// Pushes a boundary event, returning the assigned sequence.
    ///
    /// If the buffer is at capacity, the oldest entry is dropped first.
    /// Sequence numbers are monotonic across the full transcript lifetime
    /// (dropped entries still consume a sequence number).
    ///
    /// Returns `Err(BoundaryTranscriptError::AllocationFailed)` if the
    /// allocator cannot satisfy the capacity reservation after the FIFO
    /// eviction has freed a slot. The next sequence number is **not**
    /// consumed on allocation failure (the failure path returns
    /// `Err` without bumping `next_seq`), so a subsequent retry does
    /// not skip a sequence.
    ///
    /// Returns `Err(BoundaryTranscriptError::SequenceSaturated)` if the
    /// monotonic sequence has reached `u64::MAX`. In practice the limit
    /// is unreachable (2^64 pushes per runtime lifetime).
    pub fn push(
        &mut self,
        event: BoundaryEvent,
    ) -> Result<Option<TranscriptSeq>, BoundaryTranscriptError> {
        let seq = self.next_seq;
        let next = self
            .next_seq
            .checked_add(1)
            .ok_or(BoundaryTranscriptError::SequenceSaturated)?;
        // Capacity overflow path: if the buffer is at capacity, pop the
        // oldest entry. This must run before `try_reserve` so the
        // reservation never needs to grow the deque beyond its bounded cap.
        if self.entries.len() >= self.capacity {
            let _ = self.entries.pop_front();
            self.dropped = self.dropped.saturating_add(1);
        }
        // Propagate allocation failure rather than silently discarding the
        // fallible result (`let _reserve = ...` was MEDIUM-1 in the prior
        // review). Capacity overflow already freed a slot, so the reservation
        // should succeed in normal operation; any failure is a hard resource
        // exhaustion signal that the runtime must observe.
        self.entries
            .try_reserve(1)
            .map_err(|_| BoundaryTranscriptError::AllocationFailed)?;
        self.next_seq = next;
        self.entries
            .push_back(BoundaryTranscriptEntry { seq, event });
        Ok(Some(seq))
    }

    /// Returns the number of entries currently retained.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if no entries are retained.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the configured capacity.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the cumulative number of entries dropped due to capacity overflow.
    #[must_use]
    pub const fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Returns a snapshot of the retained entries in insertion order.
    #[must_use]
    pub fn snapshot(&self) -> Vec<BoundaryTranscriptEntry> {
        self.entries.iter().cloned().collect()
    }

    /// Returns the retained entries with seq >= `from_seq`, in insertion order.
    #[must_use]
    pub fn snapshot_from(&self, from_seq: TranscriptSeq) -> Vec<BoundaryTranscriptEntry> {
        self.entries
            .iter()
            .filter(|e| e.seq >= from_seq)
            .cloned()
            .collect()
    }

    /// Returns the entries with seq in `[from_seq, to_seq)`, in insertion order.
    #[must_use]
    pub fn snapshot_range(
        &self,
        from_seq: TranscriptSeq,
        to_seq: TranscriptSeq,
    ) -> Vec<BoundaryTranscriptEntry> {
        self.entries
            .iter()
            .filter(|e| e.seq >= from_seq && e.seq < to_seq)
            .cloned()
            .collect()
    }
}

impl Default for BoundaryTranscript {
    fn default() -> Self {
        Self::new()
    }
}

/// Error type for shared boundary transcript operations.
///
/// Cold-path capture may receive events from any shard / worker / IPC
/// dispatcher. Mutex poisoning, allocator exhaustion, and sequence
/// saturation surface as typed errors rather than panics so the runtime
/// can choose to ignore, log, or recover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BoundaryTranscriptError {
    /// The shared mutex was poisoned by a previous panic.
    MutexPoisoned,
    /// The allocator could not satisfy a `try_reserve` request.
    ///
    /// The boundary transcript is bounded, so this is a hard resource
    /// exhaustion signal: the runtime should surface it as a typed error
    /// rather than retry indefinitely.
    AllocationFailed,
    /// The monotonic sequence has reached `u64::MAX`.
    ///
    /// In practice this is unreachable (2^64 pushes per runtime lifetime);
    /// included as a fail-closed guard against sequence collisions.
    SequenceSaturated,
}

impl core::fmt::Display for BoundaryTranscriptError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MutexPoisoned => f.write_str("boundary transcript mutex poisoned"),
            Self::AllocationFailed => f.write_str("boundary transcript allocator failed"),
            Self::SequenceSaturated => f.write_str("boundary transcript sequence saturated"),
        }
    }
}

impl std::error::Error for BoundaryTranscriptError {}

impl BoundaryTranscriptError {
    /// Converts this transcript error into the runtime-level
    /// [`crate::RuntimeError::BoundaryTranscript`] variant so cold-path
    /// capture failures can flow through the same `RuntimeResult`
    /// channel as the journal's authoritative writes.
    ///
    /// The runtime journal is the source of truth for replay; a
    /// transcript failure does not invalidate the journal entry.
    /// Callers that treat the cold-path transcript as best-effort may
    /// log this error and continue; callers that require cold-path
    /// fidelity must surface the typed error to the operator.
    pub fn into_runtime_err(self) -> crate::RuntimeError {
        let error = self.to_string();
        crate::RuntimeError::BoundaryTranscript { error }
    }
}
