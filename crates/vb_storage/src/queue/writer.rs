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

#[derive(Debug, Clone)]
struct QueuedJournalEvent {
    event: JournalEvent,
    profile: DurabilityProfile,
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
    ///
    /// vb-1rqz7.31 / SA-005: this function holds the queue mutex only across
    /// the queue bookkeeping (take + pop) and the post-IO requeue. The slow
    /// journal writes and `persist_strict` fsync now run without holding the
    /// mutex, so concurrent `enqueue_*` calls and other producers do not block
    /// behind the IO.
    pub fn flush_batch(
        &self,
        journal: &FjallJournal,
    ) -> Result<JournalWriterFlushReport, JournalError> {
        // 1) Under the mutex: compute batch_len and move the
        //    batch out of the pending deque. Release the mutex before doing
        //    any journal IO so concurrent enqueues are not serialized
        //    behind the slow fsync. The `has_strict` flag is computed
        //    after the lock is released via `batch.iter().any(...)`.
        let mut batch: Vec<QueuedJournalEvent> = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| JournalError::WriteLockPoisoned)?;
            let mut batch_len = 0usize;

            while batch_len < self.batch_size {
                let Some(_item) = state.pending.get(batch_len) else {
                    break;
                };
                batch_len = batch_len.saturating_add(1);
            }

            if batch_len == 0 {
                return Ok(JournalWriterFlushReport {
                    drained: 0,
                    written: 0,
                    pending_after: state.pending.len(),
                });
            }

            let mut taken = Vec::with_capacity(batch_len);
            for _ in 0..batch_len {
                let Some(item) = state.pending.pop_front() else {
                    // LOGIC INVARIANT: `batch_len` was bounded above by
                    // `state.pending.len()`, so pop_front cannot return None
                    // here without an upstream bug. Using WriteLockPoisoned
                    // is intentional because no dedicated invariant-error
                    // variant exists.
                    return Err(JournalError::WriteLockPoisoned);
                };
                taken.push(item);
            }
            taken
        }; // lock released here

        let batch_len = batch.len();
        let has_strict = batch
            .iter()
            .any(|item| item.profile == DurabilityProfile::Strict);

        // 2) Outside the mutex: write the batch to the journal.
        let write_result: Result<usize, JournalError> = (|| {
            let mut written = 0usize;
            while written < batch_len {
                let Some(item) = batch.get(written) else {
                    break;
                };
                journal.append_queued_indexed_unpersisted(&item.event)?;
                written = written.saturating_add(1);
            }
            if has_strict {
                journal.persist_strict()?;
            }
            Ok(written)
        })();

        let written = match write_result {
            Ok(n) => n,
            Err(e) => {
                // 3) On failure: re-queue the unwritten items at the front
                //    of the deque so the next flush_batch attempt sees them
                //    again. Hold the mutex only long enough to splice.
                let mut state = self
                    .state
                    .lock()
                    .map_err(|_| JournalError::WriteLockPoisoned)?;
                let to_requeue: Vec<QueuedJournalEvent> = batch.drain(..).collect();
                let mut requeued_front: VecDeque<QueuedJournalEvent> =
                    to_requeue.into_iter().collect();
                requeued_front.append(&mut state.pending);
                state.pending = requeued_front;
                let pending_after = state.pending.len();
                drop(state);
                let _ = pending_after;
                return Err(e);
            }
        };

        // 4) Non-strict batches skip persist_strict (already done above)
        //    and rely on Fjall's normal durability path.
        let _ = written; // already encoded into Ok arm above

        // 5) Compute pending_after under the mutex for the report.
        let pending_after = {
            let state = self
                .state
                .lock()
                .map_err(|_| JournalError::WriteLockPoisoned)?;
            state.pending.len()
        };

        Ok(JournalWriterFlushReport {
            drained: written,
            written,
            pending_after,
        })
    }

    /// Flushes queued journal writes until the queue is empty.
    ///
    /// Maximum iterations: ceil(capacity / batch_size) + 2.
    /// This is a static bound - the queue is bounded by construction.
    ///
    /// SA-004: when the static bound is exhausted and items remain, the
    /// returned `JournalWriterFlushReport.pending_after` carries the actual
    /// remaining count so the caller can detect a drain-incomplete under
    /// concurrent enqueue instead of receiving a silent success. When the
    /// queue has been shut down (no more enqueues accepted), the function
    /// continues draining past the static bound because the remaining
    /// items must be flushed deterministically — a drain-incomplete under
    /// shutdown is a real bug, not a benign race.
    pub fn drain_all(
        &self,
        journal: &FjallJournal,
    ) -> Result<JournalWriterFlushReport, JournalError> {
        let mut total = JournalWriterFlushReport {
            drained: 0,
            written: 0,
            pending_after: 0,
        };

        // Static bound: queue capacity divided by minimum batch size, plus buffer.
        // batch_size is guaranteed >= 1 by JournalBatchSize constructor invariants.
        // checked_div returns None only on division by zero which is impossible.
        let max_iterations = self
            .capacity
            .checked_div(self.batch_size)
            .ok_or(JournalError::QueueCapacity)?
            .saturating_add(2);
        let mut iterations = 0usize;
        loop {
            let report = self.flush_batch(journal)?;
            total.drained = total.drained.saturating_add(report.drained);
            total.written = total.written.saturating_add(report.written);
            if report.pending_after == 0 {
                return Ok(total);
            }
            iterations = iterations.saturating_add(1);
            if iterations >= max_iterations {
                break;
            }
        }

        // Static iteration bound exhausted with items still pending.
        // If shutdown has been signalled, no new enqueues can arrive so
        // drain MUST complete; loop until empty to make this guarantee
        // explicit. Otherwise (race with concurrent producers) re-acquire
        // the lock and report the actual remaining count so callers can
        // detect drain-incomplete instead of receiving a silent Ok.
        let shutdown = {
            let state = self
                .state
                .lock()
                .map_err(|_| JournalError::WriteLockPoisoned)?;
            state.shutdown
        };
        if shutdown {
            loop {
                let report = self.flush_batch(journal)?;
                total.drained = total.drained.saturating_add(report.drained);
                total.written = total.written.saturating_add(report.written);
                if report.pending_after == 0 {
                    return Ok(total);
                }
            }
        }

        let pending_after = {
            let state = self
                .state
                .lock()
                .map_err(|_| JournalError::WriteLockPoisoned)?;
            state.pending.len()
        };
        total.pending_after = pending_after;
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
