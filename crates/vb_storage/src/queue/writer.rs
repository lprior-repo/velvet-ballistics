use std::collections::VecDeque;
use std::sync::Mutex;

use crate::{
    codec::{decode_journal_event, encode_record},
    constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES},
    error::JournalError,
    events::JournalEvent,
    journal::FjallJournal,
    keys::run_event_key,
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

    /// Flushes at most one configured batch to the journal.
    ///
    /// Atomicity: events drained in a single `flush_batch` either all
    /// become durable together or none do. Per master §49
    /// Crash-Consistency Rule, a partial prefix is forbidden: a process
    /// crash mid-flush must leave no durable-visible record set. We
    /// stage every event into one `fjall::OwnedWriteBatch` (acquired
    /// directly via `journal.database.batch()`) and commit the batch
    /// with `PersistMode::SyncAll` when any staged event requested the
    /// `Strict` durability profile. The non-strict branch leaves
    /// durability to Fjall's lazy WAL flush but is still atomic at the
    /// OwnedWriteBatch boundary.
    ///
    /// Idempotency: an event already present in the durable store at
    /// the same `(run, seq)` is treated as an idempotent retry — the
    /// queued bytes are compared to the existing value, and the event is
    /// skipped on match (so retries after a partial flush succeed) or
    /// surface `DuplicateEvent` on mismatch.
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

        // Stage every event into a single OwnedWriteBatch so the commit
        // is atomic. Either all `written` events become durable or none
        // do. We accumulate errors into a single typed return without
        // touching the durable store, preserving the §49 rule that a
        // partial prefix is never observable.
        let mut owned_batch = journal.database.batch();
        let mut written = 0usize;
        while written < batch_len {
            let Some(item) = state.pending.get(written) else {
                break;
            };
            stage_queued_event(&mut owned_batch, journal, &item.event)?;
            written = written.saturating_add(1);
        }

        // Apply durability: strict batches force SyncAll; journaled
        // batches rely on Fjall's lazy WAL flush but still commit
        // atomically through the OwnedWriteBatch.
        let owned_batch = if has_strict {
            owned_batch.durability(Some(fjall::PersistMode::SyncAll))
        } else {
            owned_batch.durability(None)
        };
        owned_batch.commit()?;

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
    ///
    /// Maximum iterations: ceil(capacity / batch_size) + 2.
    /// This is a static bound - the queue is bounded by construction.
    pub fn drain_all(
        &self,
        journal: &FjallJournal,
    ) -> Result<JournalWriterFlushReport, JournalError> {
        let mut total = JournalWriterFlushReport {
            drained: 0,
            written: 0,
        };

        // Static bound: queue capacity divided by minimum batch size, plus buffer.
        // batch_size is guaranteed >= 1 by JournalBatchSize constructor invariants.
        // checked_div returns None only on division by zero which is impossible.
        let max_iterations = self
            .capacity
            .checked_div(self.batch_size)
            .ok_or(JournalError::QueueCapacity)?
            .saturating_add(2);
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

/// Stages one queued event into the supplied `OwnedWriteBatch`.
///
/// Idempotency: when the durable events keyspace already holds a value
/// at the same `(run, seq)`, the existing bytes are decoded and compared
/// against the queued event. A match means an idempotent retry — the
/// event is silently skipped so the queue's eventual drain remains
/// correct. A mismatch returns `DuplicateEvent` so the operator can
/// diagnose the divergence.
fn stage_queued_event(
    owned_batch: &mut fjall::OwnedWriteBatch,
    journal: &FjallJournal,
    event: &JournalEvent,
) -> Result<(), JournalError> {
    let key = run_event_key(event.run_id(), event.seq())?;
    if let Some(existing_bytes) = journal.events.get(key.as_slice())? {
        let (_, existing) = decode_journal_event(
            existing_bytes.as_ref(),
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        if existing == *event {
            return Ok(());
        }
        return Err(JournalError::DuplicateEvent {
            run: event.run_id(),
            seq: event.seq(),
        });
    }
    let value = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    owned_batch.insert(&journal.events, key, value);
    Ok(())
}
