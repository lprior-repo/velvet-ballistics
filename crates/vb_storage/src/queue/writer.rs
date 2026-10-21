use std::collections::VecDeque;
use std::sync::Mutex;

use crate::{
    error::JournalError,
    events::JournalEvent,
    journal::FjallJournal,
    types::{
        DurabilityProfile, JournalBatchSize, JournalQueueCapacity, JournalWriterFlushReport,
        JournalWriterQueueProfileCounts, StorageLimits,
    },
};

mod flush;
mod group;
mod stage;

use self::group::{QueuedJournalGroup, count_profile};

#[derive(Debug, Clone)]
struct QueuedJournalEvent {
    event: JournalEvent,
    profile: DurabilityProfile,
    group: QueuedJournalGroup,
}

#[derive(Debug, Clone)]
struct JournalWriterQueueState {
    pending: VecDeque<QueuedJournalEvent>,
    shutdown: bool,
}

/// Bounded in-memory queue for journal writer batching.
#[derive(Debug)]
pub struct JournalWriterQueue {
    state: Mutex<JournalWriterQueueState>,
    capacity: usize,
    batch_size: usize,
}

impl JournalWriterQueue {
    /// Creates a bounded writer queue.
    pub fn new(
        capacity: usize,
        batch_size: usize,
        limits: StorageLimits,
    ) -> Result<Self, JournalError> {
        let capacity = JournalQueueCapacity::try_from_usize(capacity)?;
        let batch_size = JournalBatchSize::try_from_usize(batch_size)?;
        Self::with_contracts(capacity, batch_size, limits)
    }

    /// Creates a bounded writer queue from validated domain contracts.
    pub fn with_contracts(
        capacity: JournalQueueCapacity,
        batch_size: JournalBatchSize,
        _limits: StorageLimits,
    ) -> Result<Self, JournalError> {
        Ok(Self {
            state: Mutex::new(JournalWriterQueueState {
                pending: VecDeque::with_capacity(capacity.get()),
                shutdown: false,
            }),
            capacity: capacity.get(),
            batch_size: batch_size.get(),
        })
    }

    /// Enqueues an event for journaled append.
    pub fn enqueue_journaled(&self, event: JournalEvent) -> Result<(), JournalError> {
        self.enqueue(event, DurabilityProfile::Journaled)
    }

    /// Enqueues a same-run batch for journaled append without permitting a
    /// later flush to split the batch across durable commits.
    pub fn enqueue_journaled_batch(&self, events: Vec<JournalEvent>) -> Result<(), JournalError> {
        self.enqueue_batch(events, DurabilityProfile::Journaled)
    }

    /// Enqueues an event for strict append.
    pub fn enqueue_strict(&self, event: JournalEvent) -> Result<(), JournalError> {
        self.enqueue(event, DurabilityProfile::Strict)
    }

    fn enqueue(&self, event: JournalEvent, profile: DurabilityProfile) -> Result<(), JournalError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
        if state.shutdown {
            return Err(JournalError::QueueShutdown);
        }
        if state.pending.len() >= self.capacity {
            return Err(JournalError::QueueFull);
        }
        state.pending.push_back(QueuedJournalEvent {
            event,
            profile,
            group: QueuedJournalGroup::Single,
        });
        Ok(())
    }

    /// Returns pending write counts split by durability profile.
    pub fn pending_profile_counts(&self) -> Result<JournalWriterQueueProfileCounts, JournalError> {
        let state = self
            .state
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
        let mut counts = JournalWriterQueueProfileCounts {
            journaled: 0,
            strict: 0,
        };
        for item in &state.pending {
            count_profile(&mut counts, item.profile);
        }
        Ok(counts)
    }

    /// Probes whether the queue can currently accept another journaled write.
    ///
    /// This does not enqueue a sentinel event or mutate queue state.
    pub fn probe_accepting_writes(&self) -> Result<(), JournalError> {
        let state = self
            .state
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
        if state.shutdown {
            return Err(JournalError::QueueShutdown);
        }
        if state.pending.len() >= self.capacity {
            return Err(JournalError::QueueFull);
        }
        Ok(())
    }

    /// Flushes queued journal writes until the queue is empty.
    ///
    /// Maximum iterations: capacity + 1. Atomic queued batches are never split,
    /// so one flush can deliberately drain fewer than `batch_size` events when
    /// the next queued item is an indivisible batch. The queue capacity is still
    /// a static bound, and every non-empty flush drains at least one event.
    pub fn drain_all(
        &self,
        journal: &FjallJournal,
    ) -> Result<JournalWriterFlushReport, JournalError> {
        let mut total = JournalWriterFlushReport {
            drained: 0,
            written: 0,
        };

        let max_iterations = self.capacity.saturating_add(1);
        for _ in 0..max_iterations {
            let report = self.flush_batch(journal)?;
            if report.drained == 0 {
                return Ok(total);
            }
            total.drained = total.drained.saturating_add(report.drained);
            total.written = total.written.saturating_add(report.written);
        }
        Ok(total)
    }

    /// Closes the queue to new writes and drains all accepted writes durably.
    pub fn shutdown(
        &self,
        journal: &FjallJournal,
    ) -> Result<JournalWriterFlushReport, JournalError> {
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| JournalError::WriteLockPoisoned)?;
            state.shutdown = true;
        }
        self.drain_all(journal)
    }
}

#[cfg(test)]
mod flush_tests;
