use std::collections::VecDeque;
use std::sync::Mutex;

use crate::{
    error::JournalError,
    events::JournalEvent,
    journal::FjallJournal,
    types::{
        DurabilityProfile, JournalWriterFlushReport, JournalWriterQueueProfileCounts, StorageLimits,
    },
};

#[derive(Debug)]
struct QueuedJournalEvent {
    event: JournalEvent,
    profile: DurabilityProfile,
}

#[derive(Debug)]
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
        _limits: StorageLimits,
    ) -> Result<Self, JournalError> {
        if capacity == 0 || batch_size == 0 {
            return Err(JournalError::QueueCapacity);
        }
        Ok(Self {
            state: Mutex::new(JournalWriterQueueState {
                pending: VecDeque::with_capacity(capacity),
                shutdown: false,
            }),
            capacity,
            batch_size,
        })
    }

    /// Enqueues an event for journaled append.
    pub fn enqueue_journaled(&self, event: JournalEvent) -> Result<(), JournalError> {
        self.enqueue(event, DurabilityProfile::Journaled)
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
        state
            .pending
            .push_back(QueuedJournalEvent { event, profile });
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
            match item.profile {
                DurabilityProfile::Journaled => {
                    counts.journaled = counts.journaled.saturating_add(1);
                }
                DurabilityProfile::Strict => {
                    counts.strict = counts.strict.saturating_add(1);
                }
                DurabilityProfile::Volatile => {}
            }
        }
        Ok(counts)
    }

    /// Flushes at most one configured batch to the journal.
    pub fn flush_batch(
        &self,
        journal: &FjallJournal,
    ) -> Result<JournalWriterFlushReport, JournalError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
        let mut batch_len = 0usize;
        let mut has_strict = false;

        while batch_len < self.batch_size {
            let Some(item) = state.pending.get(batch_len) else {
                break;
            };
            if item.profile == DurabilityProfile::Strict {
                has_strict = true;
            }
            batch_len = batch_len.saturating_add(1);
        }

        if batch_len == 0 {
            return Ok(JournalWriterFlushReport {
                drained: 0,
                written: 0,
            });
        }

        if has_strict {
            let mut written = 0usize;
            while written < batch_len {
                let Some(item) = state.pending.get(written) else {
                    break;
                };
                journal.append_queued_unpersisted(&item.event)?;
                written = written.saturating_add(1);
            }
            journal.persist_strict()?;
            let mut drained = 0usize;
            while drained < written {
                match state.pending.pop_front() {
                    Some(_) => {
                        drained = drained.saturating_add(1);
                    }
                    // LOGIC INVARIANT: `written` counts items we just indexed via
                    // `get(index)` on the same deque, so `pop_front` cannot return
                    // None here unless an upstream bug corrupts the counts.  We
                    // use WriteLockPoisoned only because no dedicated
                    // queue-drain-inconsistent variant exists.
                    None => return Err(JournalError::WriteLockPoisoned),
                }
            }
            return Ok(JournalWriterFlushReport { drained, written });
        }

        let mut written = 0usize;
        while written < batch_len {
            let Some(item) = state.pending.get(written) else {
                break;
            };
            journal.append_queued_unpersisted(&item.event)?;
            written = written.saturating_add(1);
        }

        // Non-strict batch: skip persist_strict so journaled items are
        // flushed lazily by the storage engine rather than forcing fsync.
        let mut drained = 0usize;
        while drained < written {
            match state.pending.pop_front() {
                Some(_) => {
                    drained = drained.saturating_add(1);
                }
                // LOGIC INVARIANT: `written` counts items we just indexed via
                // `get(index)` on the same deque, so `pop_front` cannot return
                // None here unless an upstream bug corrupts the counts.  We
                // use WriteLockPoisoned only because no dedicated
                // queue-drain-inconsistent variant exists.
                None => return Err(JournalError::WriteLockPoisoned),
            }
        }

        Ok(JournalWriterFlushReport { drained, written })
    }

    /// Flushes queued journal writes until the queue is empty.
    pub fn drain_all(
        &self,
        journal: &FjallJournal,
    ) -> Result<JournalWriterFlushReport, JournalError> {
        let mut total = JournalWriterFlushReport {
            drained: 0,
            written: 0,
        };

        loop {
            let report = self.flush_batch(journal)?;
            if report.drained == 0 {
                return Ok(total);
            }
            total.drained = total.drained.saturating_add(report.drained);
            total.written = total.written.saturating_add(report.written);
        }
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
