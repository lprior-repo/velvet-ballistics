//! Async journal writer queue and batch builder.
//!
//! Provides bounded queueing for journal events with durability profiling.

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

/// Ergonomic builder for batching journal events.
#[derive(Debug, Default)]
pub struct BatchBuilder {
    events: Vec<JournalEvent>,
}

impl BatchBuilder {
    /// Creates an empty batch builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an event to the batch.
    pub fn push(&mut self, event: JournalEvent) {
        self.events.push(event);
    }

    /// Returns the number of events in the batch.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Returns true if the batch contains no events.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Returns the built event slice.
    #[must_use]
    pub fn as_slice(&self) -> &[JournalEvent] {
        &self.events
    }
}

#[cfg(test)]
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod tests {
    use super::*;
    use crate::{EventSeq, FjallJournal, JournalEvent, StorageLimits, constants::DIGEST_BYTES};
    use vb_core::{RunId, WorkflowDigest};

    fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
        (temp, journal)
    }

    fn make_event(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(seq),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        }
    }

    // =========================================================================
    // Capacity enforcement
    // =========================================================================

    #[test]
    fn new_rejects_zero_capacity() {
        let result = JournalWriterQueue::new(0, 4, StorageLimits::DEFAULT);
        assert!(
            matches!(result, Err(JournalError::QueueCapacity)),
            "zero capacity must yield QueueCapacity, got {:?}",
            result
        );
    }

    #[test]
    fn new_rejects_zero_batch_size() {
        let result = JournalWriterQueue::new(4, 0, StorageLimits::DEFAULT);
        assert!(
            matches!(result, Err(JournalError::QueueCapacity)),
            "zero batch size must yield QueueCapacity, got {:?}",
            result
        );
    }

    #[test]
    fn new_accepts_valid_capacity_and_batch() {
        let result = JournalWriterQueue::new(8, 4, StorageLimits::DEFAULT);
        assert!(result.is_ok(), "valid capacity and batch should succeed");
    }

    #[test]
    fn enqueue_rejects_when_at_capacity() {
        let queue = JournalWriterQueue::new(2, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(1);
        let e0 = make_event(run, 0);
        let e1 = make_event(run, 1);
        let e2 = make_event(run, 2);

        queue
            .enqueue_journaled(e0)
            .expect("first enqueue should succeed");
        queue
            .enqueue_journaled(e1)
            .expect("second enqueue should succeed");
        let result = queue.enqueue_journaled(e2);
        assert!(
            matches!(result, Err(JournalError::QueueFull)),
            "enqueue beyond capacity must yield QueueFull, got {:?}",
            result
        );
    }

    #[test]
    fn enqueue_at_exact_capacity_succeeds() {
        let queue = JournalWriterQueue::new(3, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(1);
        for i in 0..3u64 {
            queue
                .enqueue_journaled(make_event(run, i))
                .unwrap_or_else(|_| panic!("enqueue {} should succeed within capacity", i));
        }
        let counts = queue
            .pending_profile_counts()
            .expect("counts should succeed");
        assert_eq!(counts.journaled, 3);
    }

    // =========================================================================
    // Profile counting (strict vs journaled durability)
    // =========================================================================

    #[test]
    fn pending_profile_counts_tracks_journaled_events() {
        let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(10);
        queue
            .enqueue_journaled(make_event(run, 0))
            .expect("enqueue should succeed");
        queue
            .enqueue_journaled(make_event(run, 1))
            .expect("enqueue should succeed");

        let counts = queue
            .pending_profile_counts()
            .expect("counts should succeed");
        assert_eq!(counts.journaled, 2, "should have 2 journaled events");
        assert_eq!(counts.strict, 0, "should have 0 strict events");
    }

    #[test]
    fn pending_profile_counts_tracks_strict_events() {
        let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(20);
        queue
            .enqueue_strict(make_event(run, 0))
            .expect("enqueue should succeed");
        queue
            .enqueue_strict(make_event(run, 1))
            .expect("enqueue should succeed");

        let counts = queue
            .pending_profile_counts()
            .expect("counts should succeed");
        assert_eq!(counts.journaled, 0, "should have 0 journaled events");
        assert_eq!(counts.strict, 2, "should have 2 strict events");
    }

    #[test]
    fn pending_profile_counts_tracks_mixed_profiles() {
        let queue = JournalWriterQueue::new(6, 3, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(30);
        queue
            .enqueue_journaled(make_event(run, 0))
            .expect("enqueue should succeed");
        queue
            .enqueue_strict(make_event(run, 1))
            .expect("enqueue should succeed");
        queue
            .enqueue_journaled(make_event(run, 2))
            .expect("enqueue should succeed");
        queue
            .enqueue_strict(make_event(run, 3))
            .expect("enqueue should succeed");

        let counts = queue
            .pending_profile_counts()
            .expect("counts should succeed");
        assert_eq!(counts.journaled, 2, "should have 2 journaled events");
        assert_eq!(counts.strict, 2, "should have 2 strict events");
    }

    #[test]
    fn pending_profile_counts_empty_queue() {
        let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let counts = queue
            .pending_profile_counts()
            .expect("counts should succeed");
        assert_eq!(counts.journaled, 0);
        assert_eq!(counts.strict, 0);
    }

    // =========================================================================
    // Shutdown drain behavior
    // =========================================================================

    #[test]
    fn shutdown_rejects_new_writes_after_call() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(40);
        queue
            .enqueue_journaled(make_event(run, 0))
            .expect("enqueue before shutdown");

        let report = queue.shutdown(&journal).expect("shutdown should succeed");
        assert!(report.drained > 0, "shutdown should drain pending events");

        // After shutdown, new enqueues should fail
        let result = queue.enqueue_journaled(make_event(run, 1));
        assert!(
            matches!(result, Err(JournalError::QueueShutdown)),
            "enqueue after shutdown must yield QueueShutdown, got {:?}",
            result
        );
    }

    #[test]
    fn shutdown_drains_all_pending_events() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(8, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(50);
        for i in 0..5u64 {
            queue
                .enqueue_journaled(make_event(run, i))
                .unwrap_or_else(|_| panic!("enqueue {} should succeed", i));
        }

        let report = queue.shutdown(&journal).expect("shutdown should succeed");
        assert_eq!(report.drained, 5, "shutdown should drain all 5 events");
        assert_eq!(report.written, 5, "shutdown should write all 5 events");

        // Verify events are persisted
        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(
            events.len(),
            5,
            "journal should contain 5 events after shutdown"
        );
    }

    #[test]
    fn shutdown_on_empty_queue_succeeds_with_zero_drained() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");

        let report = queue.shutdown(&journal).expect("shutdown should succeed");
        assert_eq!(report.drained, 0, "empty shutdown should drain 0");
        assert_eq!(report.written, 0, "empty shutdown should write 0");

        // Still rejects new writes
        let result = queue.enqueue_journaled(make_event(RunId::new(1), 0));
        assert!(
            matches!(result, Err(JournalError::QueueShutdown)),
            "enqueue after shutdown must fail, got {:?}",
            result
        );
    }

    // =========================================================================
    // Flush batch behavior
    // =========================================================================

    #[test]
    fn flush_batch_on_empty_queue_returns_zero() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");

        let report = queue.flush_batch(&journal).expect("flush should succeed");
        assert_eq!(report.drained, 0);
        assert_eq!(report.written, 0);
    }

    #[test]
    fn flush_batch_drains_up_to_batch_size() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(8, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(60);
        for i in 0..5u64 {
            queue
                .enqueue_journaled(make_event(run, i))
                .unwrap_or_else(|_| panic!("enqueue {} should succeed", i));
        }

        // batch_size is 2, so first flush drains 2
        let report = queue.flush_batch(&journal).expect("flush should succeed");
        assert_eq!(report.drained, 2, "first flush should drain batch_size (2)");
        assert_eq!(report.written, 2);

        // Still 3 remaining
        let counts = queue
            .pending_profile_counts()
            .expect("counts should succeed");
        assert_eq!(counts.journaled, 3, "should have 3 remaining events");
    }

    #[test]
    fn drain_all_flushes_until_empty() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(8, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(70);
        for i in 0..5u64 {
            queue
                .enqueue_journaled(make_event(run, i))
                .unwrap_or_else(|_| panic!("enqueue {} should succeed", i));
        }

        let report = queue.drain_all(&journal).expect("drain_all should succeed");
        assert_eq!(report.drained, 5, "drain_all should drain all 5 events");
        assert_eq!(report.written, 5);

        // Queue should now be empty
        let counts = queue
            .pending_profile_counts()
            .expect("counts should succeed");
        assert_eq!(counts.journaled, 0, "queue should be empty after drain_all");
    }

    // =========================================================================
    // BatchBuilder tests
    // =========================================================================

    #[test]
    fn batch_builder_starts_empty() {
        let builder = BatchBuilder::new();
        assert!(builder.is_empty());
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn batch_builder_push_adds_events() {
        let mut builder = BatchBuilder::new();
        let event = make_event(RunId::new(1), 0);
        builder.push(event.clone());
        assert!(!builder.is_empty());
        assert_eq!(builder.len(), 1);
        assert_eq!(builder.as_slice().first(), Some(&event));
    }

    #[test]
    fn batch_builder_accumulates_multiple_events() {
        let mut builder = BatchBuilder::new();
        let e1 = make_event(RunId::new(1), 0);
        let e2 = make_event(RunId::new(2), 0);
        let e3 = make_event(RunId::new(3), 0);
        builder.push(e1.clone());
        builder.push(e2.clone());
        builder.push(e3.clone());
        assert_eq!(builder.len(), 3);
        assert_eq!(builder.as_slice(), &[e1, e2, e3]);
    }

    // =========================================================================
    // Flush writes events to journal correctly
    // =========================================================================

    #[test]
    fn flush_batch_writes_events_readable_from_journal() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(8, 4, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(80);
        queue
            .enqueue_journaled(make_event(run, 0))
            .expect("enqueue 0");
        queue
            .enqueue_journaled(make_event(run, 1))
            .expect("enqueue 1");

        let report = queue.flush_batch(&journal).expect("flush should succeed");
        assert_eq!(report.drained, 2);
        assert_eq!(report.written, 2);

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].seq().get(), 0);
        assert_eq!(events[1].seq().get(), 1);
    }

    #[test]
    fn drain_all_with_mixed_profiles_writes_all() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(8, 4, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(90);
        queue
            .enqueue_journaled(make_event(run, 0))
            .expect("enqueue journaled");
        queue
            .enqueue_strict(make_event(run, 1))
            .expect("enqueue strict");
        queue
            .enqueue_journaled(make_event(run, 2))
            .expect("enqueue journaled");

        let report = queue.drain_all(&journal).expect("drain_all should succeed");
        assert_eq!(report.drained, 3);
        assert_eq!(report.written, 3);

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 3);
    }

    // =========================================================================
    // Durability tier distinction: strict vs journaled through flush_batch
    // =========================================================================

    #[test]
    fn flush_batch_persists_strict_events_to_journal() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(8, 4, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(100);
        queue
            .enqueue_strict(make_event(run, 0))
            .expect("enqueue strict 0");
        queue
            .enqueue_strict(make_event(run, 1))
            .expect("enqueue strict 1");

        let report = queue.flush_batch(&journal).expect("flush should succeed");
        assert_eq!(report.drained, 2, "strict flush should drain 2 events");
        assert_eq!(report.written, 2, "strict flush should write 2 events");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(
            events.len(),
            2,
            "journal should contain 2 strict events after flush"
        );
        assert_eq!(events[0].seq().get(), 0, "first event seq should be 0");
        assert_eq!(events[1].seq().get(), 1, "second event seq should be 1");
    }

    #[test]
    fn flush_batch_persists_journaled_events_to_journal() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(8, 4, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(101);
        queue
            .enqueue_journaled(make_event(run, 0))
            .expect("enqueue journaled 0");
        queue
            .enqueue_journaled(make_event(run, 1))
            .expect("enqueue journaled 1");
        queue
            .enqueue_journaled(make_event(run, 2))
            .expect("enqueue journaled 2");

        let report = queue.flush_batch(&journal).expect("flush should succeed");
        assert_eq!(report.drained, 3, "journaled flush should drain 3 events");
        assert_eq!(report.written, 3, "journaled flush should write 3 events");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(
            events.len(),
            3,
            "journal should contain 3 journaled events after flush"
        );
    }

    #[test]
    fn flush_batch_mixed_strict_and_journaled_writes_all() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(8, 4, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(102);
        queue
            .enqueue_journaled(make_event(run, 0))
            .expect("enqueue journaled");
        queue
            .enqueue_strict(make_event(run, 1))
            .expect("enqueue strict");
        queue
            .enqueue_journaled(make_event(run, 2))
            .expect("enqueue journaled");
        queue
            .enqueue_strict(make_event(run, 3))
            .expect("enqueue strict");

        let report = queue.flush_batch(&journal).expect("flush should succeed");
        assert_eq!(report.drained, 4, "mixed batch should drain 4 events");
        assert_eq!(report.written, 4, "mixed batch should write 4 events");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 4, "journal should contain all 4 events");
        assert_eq!(events[0].seq().get(), 0);
        assert_eq!(events[1].seq().get(), 1);
        assert_eq!(events[2].seq().get(), 2);
        assert_eq!(events[3].seq().get(), 3);
    }

    #[test]
    fn flush_batch_strict_only_drains_in_batches() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(8, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(103);
        queue
            .enqueue_strict(make_event(run, 0))
            .expect("enqueue strict 0");
        queue
            .enqueue_strict(make_event(run, 1))
            .expect("enqueue strict 1");
        queue
            .enqueue_strict(make_event(run, 2))
            .expect("enqueue strict 2");

        let r1 = queue.flush_batch(&journal).expect("first flush");
        assert_eq!(r1.drained, 2);
        assert_eq!(r1.written, 2);

        let r2 = queue.flush_batch(&journal).expect("second flush");
        assert_eq!(r2.drained, 1);
        assert_eq!(r2.written, 1);

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 3, "all strict events should be in journal");
    }

    #[test]
    fn flush_batch_journaled_only_drains_in_batches() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(8, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(104);
        queue
            .enqueue_journaled(make_event(run, 0))
            .expect("enqueue journaled 0");
        queue
            .enqueue_journaled(make_event(run, 1))
            .expect("enqueue journaled 1");
        queue
            .enqueue_journaled(make_event(run, 2))
            .expect("enqueue journaled 2");

        let r1 = queue.flush_batch(&journal).expect("first flush");
        assert_eq!(r1.drained, 2);
        assert_eq!(r1.written, 2);

        let r2 = queue.flush_batch(&journal).expect("second flush");
        assert_eq!(r2.drained, 1);
        assert_eq!(r2.written, 1);

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 3, "all journaled events should be in journal");
    }

    // =========================================================================
    // FIFO ordering across durability tiers
    // =========================================================================

    #[test]
    fn flush_batch_returns_items_in_fifo_order() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(110);
        queue
            .enqueue_journaled(make_event(run, 0))
            .expect("enqueue 0");
        queue.enqueue_strict(make_event(run, 1)).expect("enqueue 1");
        queue
            .enqueue_journaled(make_event(run, 2))
            .expect("enqueue 2");
        queue.enqueue_strict(make_event(run, 3)).expect("enqueue 3");
        queue
            .enqueue_journaled(make_event(run, 4))
            .expect("enqueue 4");

        let report = queue.flush_batch(&journal).expect("flush should succeed");
        assert_eq!(report.drained, 5);

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 5, "all 5 events should be present");
        for (i, ev) in events.iter().enumerate() {
            assert_eq!(
                ev.seq().get(),
                i as u64,
                "event at index {} should have seq {}, got {}",
                i,
                i,
                ev.seq().get()
            );
        }
    }

    #[test]
    fn multi_flush_maintains_fifo_across_batches() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(8, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(111);
        for i in 0..6u64 {
            if i % 2 == 0 {
                queue
                    .enqueue_strict(make_event(run, i))
                    .unwrap_or_else(|_| panic!("enqueue {} should succeed", i));
            } else {
                queue
                    .enqueue_journaled(make_event(run, i))
                    .unwrap_or_else(|_| panic!("enqueue {} should succeed", i));
            }
        }

        for _ in 0..3 {
            let r = queue.flush_batch(&journal).expect("flush should succeed");
            assert_eq!(r.drained, 2, "each flush should drain batch_size events");
        }

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 6, "all 6 events should be persisted");
        for (i, ev) in events.iter().enumerate() {
            assert_eq!(
                ev.seq().get(),
                i as u64,
                "event at index {} should have seq {}",
                i,
                i
            );
        }
    }

    // =========================================================================
    // Mixed-tier batch: strict presence triggers strict flush path
    // =========================================================================

    #[test]
    fn mixed_batch_with_one_strict_among_journaled_writes_all() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(8, 4, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(120);
        queue
            .enqueue_journaled(make_event(run, 0))
            .expect("enqueue journaled");
        queue
            .enqueue_journaled(make_event(run, 1))
            .expect("enqueue journaled");
        queue
            .enqueue_strict(make_event(run, 2))
            .expect("enqueue strict");
        queue
            .enqueue_journaled(make_event(run, 3))
            .expect("enqueue journaled");

        let report = queue.flush_batch(&journal).expect("flush should succeed");
        assert_eq!(report.drained, 4, "mixed batch should drain all 4");
        assert_eq!(report.written, 4, "mixed batch should write all 4");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 4);
    }

    #[test]
    fn mixed_batch_followed_by_journaled_only_batch() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(8, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(121);
        // Batch 1: strict + journaled (has_strict = true)
        queue
            .enqueue_strict(make_event(run, 0))
            .expect("enqueue strict");
        queue
            .enqueue_journaled(make_event(run, 1))
            .expect("enqueue journaled");
        // Batch 2: journaled only (has_strict = false)
        queue
            .enqueue_journaled(make_event(run, 2))
            .expect("enqueue journaled");
        queue
            .enqueue_journaled(make_event(run, 3))
            .expect("enqueue journaled");

        let r1 = queue.flush_batch(&journal).expect("first flush");
        assert_eq!(r1.drained, 2);
        assert_eq!(r1.written, 2);

        let r2 = queue.flush_batch(&journal).expect("second flush");
        assert_eq!(r2.drained, 2);
        assert_eq!(r2.written, 2);

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(
            events.len(),
            4,
            "all events from both batches should be persisted"
        );
        for (i, ev) in events.iter().enumerate() {
            assert_eq!(ev.seq().get(), i as u64);
        }
    }

    // =========================================================================
    // Capacity limits across durability tiers
    // =========================================================================

    #[test]
    fn capacity_limit_applies_to_strict_events() {
        let queue = JournalWriterQueue::new(2, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(130);
        queue
            .enqueue_strict(make_event(run, 0))
            .expect("first strict enqueue");
        queue
            .enqueue_strict(make_event(run, 1))
            .expect("second strict enqueue");
        let result = queue.enqueue_strict(make_event(run, 2));
        assert!(
            matches!(result, Err(JournalError::QueueFull)),
            "strict enqueue beyond capacity must yield QueueFull, got {:?}",
            result
        );
    }

    #[test]
    fn capacity_limit_shared_across_strict_and_journaled() {
        let queue = JournalWriterQueue::new(3, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(131);
        queue.enqueue_strict(make_event(run, 0)).expect("strict 0");
        queue
            .enqueue_journaled(make_event(run, 1))
            .expect("journaled 1");
        queue.enqueue_strict(make_event(run, 2)).expect("strict 2");

        let strict_result = queue.enqueue_strict(make_event(run, 3));
        assert!(
            matches!(strict_result, Err(JournalError::QueueFull)),
            "strict beyond shared capacity must fail, got {:?}",
            strict_result
        );
        let journaled_result = queue.enqueue_journaled(make_event(run, 4));
        assert!(
            matches!(journaled_result, Err(JournalError::QueueFull)),
            "journaled beyond shared capacity must fail, got {:?}",
            journaled_result
        );
    }

    // =========================================================================
    // Shutdown drains mixed durability tiers
    // =========================================================================

    #[test]
    fn shutdown_drains_mixed_strict_and_journaled() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(8, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(140);
        queue.enqueue_strict(make_event(run, 0)).expect("strict 0");
        queue
            .enqueue_journaled(make_event(run, 1))
            .expect("journaled 1");
        queue
            .enqueue_journaled(make_event(run, 2))
            .expect("journaled 2");
        queue.enqueue_strict(make_event(run, 3)).expect("strict 3");
        queue
            .enqueue_journaled(make_event(run, 4))
            .expect("journaled 4");

        let report = queue.shutdown(&journal).expect("shutdown should succeed");
        assert_eq!(
            report.drained, 5,
            "shutdown should drain all 5 mixed events"
        );
        assert_eq!(
            report.written, 5,
            "shutdown should write all 5 mixed events"
        );

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(
            events.len(),
            5,
            "journal should have all 5 events after shutdown"
        );
        for (i, ev) in events.iter().enumerate() {
            assert_eq!(ev.seq().get(), i as u64, "FIFO order must be preserved");
        }
    }

    #[test]
    fn shutdown_rejects_strict_and_journaled_after_drain() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(8, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(141);
        queue
            .enqueue_journaled(make_event(run, 0))
            .expect("enqueue before shutdown");

        let _ = queue.shutdown(&journal).expect("shutdown should succeed");

        let strict_result = queue.enqueue_strict(make_event(run, 1));
        assert!(
            matches!(strict_result, Err(JournalError::QueueShutdown)),
            "strict enqueue after shutdown must fail, got {:?}",
            strict_result
        );
        let journaled_result = queue.enqueue_journaled(make_event(run, 2));
        assert!(
            matches!(journaled_result, Err(JournalError::QueueShutdown)),
            "journaled enqueue after shutdown must fail, got {:?}",
            journaled_result
        );
    }

    // =========================================================================
    // drain_all with mixed tiers across multiple batches
    // =========================================================================

    #[test]
    fn drain_all_mixed_tiers_across_multiple_batches() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(16, 3, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(150);
        for i in 0..7u64 {
            if i % 2 == 0 {
                queue
                    .enqueue_strict(make_event(run, i))
                    .unwrap_or_else(|_| panic!("enqueue {} should succeed", i));
            } else {
                queue
                    .enqueue_journaled(make_event(run, i))
                    .unwrap_or_else(|_| panic!("enqueue {} should succeed", i));
            }
        }

        let report = queue.drain_all(&journal).expect("drain_all should succeed");
        assert_eq!(report.drained, 7, "drain_all should drain all 7 events");
        assert_eq!(report.written, 7, "drain_all should write all 7 events");

        let counts = queue
            .pending_profile_counts()
            .expect("counts should succeed");
        assert_eq!(counts.strict, 0, "queue should be empty after drain_all");
        assert_eq!(counts.journaled, 0, "queue should be empty after drain_all");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 7);
        for (i, ev) in events.iter().enumerate() {
            assert_eq!(ev.seq().get(), i as u64);
        }
    }

    // =========================================================================
    // Counts update correctly after partial flush of mixed tiers
    // =========================================================================

    #[test]
    fn counts_after_partial_strict_flush() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(8, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(160);
        queue.enqueue_strict(make_event(run, 0)).expect("strict 0");
        queue
            .enqueue_journaled(make_event(run, 1))
            .expect("journaled 1");
        queue.enqueue_strict(make_event(run, 2)).expect("strict 2");
        queue
            .enqueue_journaled(make_event(run, 3))
            .expect("journaled 3");

        let counts_before = queue.pending_profile_counts().expect("counts before flush");
        assert_eq!(counts_before.strict, 2);
        assert_eq!(counts_before.journaled, 2);

        // batch_size=2, has_strict=true (strict at index 0), drains first 2 items
        let _ = queue.flush_batch(&journal).expect("flush should succeed");

        let counts_after = queue.pending_profile_counts().expect("counts after flush");
        assert_eq!(counts_after.strict, 1, "should have 1 strict remaining");
        assert_eq!(
            counts_after.journaled, 1,
            "should have 1 journaled remaining"
        );
    }

    // =========================================================================
    // Additional edge-case tests
    // =========================================================================

    #[test]
    fn batch_builder_default_trait_yields_empty() {
        let builder = BatchBuilder::default();
        assert!(builder.is_empty(), "default BatchBuilder should be empty");
        assert_eq!(builder.len(), 0, "default BatchBuilder len should be 0");
        assert!(
            builder.as_slice().is_empty(),
            "default BatchBuilder as_slice should be empty"
        );
    }

    #[test]
    fn batch_builder_as_slice_returns_event_order() {
        let mut builder = BatchBuilder::new();
        let e0 = make_event(RunId::new(200), 0);
        let e1 = make_event(RunId::new(201), 0);
        builder.push(e0.clone());
        builder.push(e1.clone());
        let slice = builder.as_slice();
        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0], e0, "first element should be e0");
        assert_eq!(slice[1], e1, "second element should be e1");
    }

    #[test]
    fn queue_with_capacity_one_accepts_single_event() {
        let queue = JournalWriterQueue::new(1, 1, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(210);
        queue
            .enqueue_journaled(make_event(run, 0))
            .expect("single enqueue should succeed");
        let counts = queue
            .pending_profile_counts()
            .expect("counts should succeed");
        assert_eq!(counts.journaled, 1);
    }

    #[test]
    fn queue_with_capacity_one_rejects_second_event() {
        let queue = JournalWriterQueue::new(1, 1, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(211);
        queue
            .enqueue_journaled(make_event(run, 0))
            .expect("first enqueue");
        let result = queue.enqueue_journaled(make_event(run, 1));
        assert!(
            matches!(result, Err(JournalError::QueueFull)),
            "second enqueue on capacity-1 queue must yield QueueFull, got {:?}",
            result
        );
    }

    #[test]
    fn flush_batch_single_event_with_large_batch_size() {
        let (_temp, journal) = temp_journal();
        // batch_size=10 but only 1 event enqueued
        let queue = JournalWriterQueue::new(10, 10, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(220);
        queue
            .enqueue_journaled(make_event(run, 0))
            .expect("enqueue");

        let report = queue.flush_batch(&journal).expect("flush should succeed");
        assert_eq!(report.drained, 1, "should drain the single event");
        assert_eq!(report.written, 1, "should write the single event");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 1, "journal should have 1 event");
    }

    #[test]
    fn drain_all_on_empty_queue_returns_zero() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");

        let report = queue.drain_all(&journal).expect("drain_all should succeed");
        assert_eq!(report.drained, 0, "empty drain_all should drain 0");
        assert_eq!(report.written, 0, "empty drain_all should write 0");
    }

    #[test]
    fn flush_then_enqueue_then_flush_again() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(8, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(230);

        // First round: enqueue 2 events and flush
        queue
            .enqueue_journaled(make_event(run, 0))
            .expect("enqueue 0");
        queue
            .enqueue_journaled(make_event(run, 1))
            .expect("enqueue 1");
        let r1 = queue.flush_batch(&journal).expect("first flush");
        assert_eq!(r1.drained, 2);
        assert_eq!(r1.written, 2);

        // Second round: enqueue 2 more events and flush
        queue
            .enqueue_journaled(make_event(run, 2))
            .expect("enqueue 2");
        queue
            .enqueue_journaled(make_event(run, 3))
            .expect("enqueue 3");
        let r2 = queue.flush_batch(&journal).expect("second flush");
        assert_eq!(r2.drained, 2);
        assert_eq!(r2.written, 2);

        // All 4 events should be in journal
        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 4, "journal should have all 4 events");
        for (i, ev) in events.iter().enumerate() {
            assert_eq!(
                ev.seq().get(),
                i as u64,
                "event at index {} should have seq {}",
                i,
                i
            );
        }
    }

    #[test]
    fn flush_batch_size_equals_queue_size_drains_all() {
        let (_temp, journal) = temp_journal();
        // batch_size=5, capacity=5, enqueue 5 events
        let queue = JournalWriterQueue::new(5, 5, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(240);
        for i in 0..5u64 {
            queue
                .enqueue_journaled(make_event(run, i))
                .unwrap_or_else(|_| panic!("enqueue {} should succeed", i));
        }

        let report = queue.flush_batch(&journal).expect("flush should succeed");
        assert_eq!(report.drained, 5, "should drain all 5 events in one flush");
        assert_eq!(report.written, 5);

        let counts = queue
            .pending_profile_counts()
            .expect("counts should succeed");
        assert_eq!(counts.journaled, 0, "queue should be empty after flush");
    }

    #[test]
    fn pending_profile_counts_reflects_partial_drain() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(8, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(250);

        // Enqueue 4 journaled, 2 strict
        for i in 0..4u64 {
            queue
                .enqueue_journaled(make_event(run, i))
                .expect("journaled enqueue");
        }
        queue.enqueue_strict(make_event(run, 4)).expect("strict 4");
        queue.enqueue_strict(make_event(run, 5)).expect("strict 5");

        let counts_before = queue.pending_profile_counts().expect("counts before");
        assert_eq!(counts_before.journaled, 4);
        assert_eq!(counts_before.strict, 2);

        // batch_size=2, first 2 are journaled (no strict in this batch)
        let _ = queue.flush_batch(&journal).expect("flush");

        let counts_after = queue.pending_profile_counts().expect("counts after");
        assert_eq!(
            counts_after.journaled, 2,
            "should have 2 journaled remaining"
        );
        assert_eq!(
            counts_after.strict, 2,
            "should have 2 strict remaining (not yet flushed)"
        );
    }

    #[test]
    fn shutdown_on_previously_flushed_queue_still_works() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(8, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(260);

        // Enqueue and flush some events
        queue
            .enqueue_journaled(make_event(run, 0))
            .expect("enqueue 0");
        queue
            .enqueue_journaled(make_event(run, 1))
            .expect("enqueue 1");
        let _ = queue.flush_batch(&journal).expect("flush");

        // Add more events then shutdown
        queue.enqueue_strict(make_event(run, 2)).expect("enqueue 2");
        queue
            .enqueue_journaled(make_event(run, 3))
            .expect("enqueue 3");

        let report = queue.shutdown(&journal).expect("shutdown should succeed");
        assert_eq!(
            report.drained, 2,
            "shutdown should drain remaining 2 events"
        );
        assert_eq!(report.written, 2);

        // Verify all 4 events in journal
        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 4);
    }

    #[test]
    fn queue_holds_exact_capacity_without_overflow() {
        let queue = JournalWriterQueue::new(3, 3, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(270);

        // Fill to exact capacity
        for i in 0..3u64 {
            queue
                .enqueue_journaled(make_event(run, i))
                .unwrap_or_else(|_| panic!("enqueue {} should succeed", i));
        }

        let counts = queue
            .pending_profile_counts()
            .expect("counts should succeed");
        assert_eq!(counts.journaled, 3);

        // Next enqueue must fail
        let result = queue.enqueue_journaled(make_event(run, 3));
        assert!(
            matches!(result, Err(JournalError::QueueFull)),
            "enqueue beyond exact capacity must fail, got {:?}",
            result
        );
    }
}
