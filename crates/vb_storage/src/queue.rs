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

        journal.persist_strict()?;
        let mut drained = 0usize;
        while drained < written {
            match state.pending.pop_front() {
                Some(_) => {
                    drained = drained.saturating_add(1);
                }
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
mod tests {
    use super::*;
    use crate::{
        EventSeq, FjallJournal, JournalEvent, StorageLimits,
        constants::DIGEST_BYTES,
    };
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

        queue.enqueue_journaled(e0).expect("first enqueue should succeed");
        queue.enqueue_journaled(e1).expect("second enqueue should succeed");
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
            queue.enqueue_journaled(make_event(run, i)).unwrap_or_else(|_| {
                panic!("enqueue {} should succeed within capacity", i)
            });
        }
        let counts = queue.pending_profile_counts().expect("counts should succeed");
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
        queue.enqueue_journaled(make_event(run, 0)).expect("enqueue should succeed");
        queue.enqueue_journaled(make_event(run, 1)).expect("enqueue should succeed");

        let counts = queue.pending_profile_counts().expect("counts should succeed");
        assert_eq!(counts.journaled, 2, "should have 2 journaled events");
        assert_eq!(counts.strict, 0, "should have 0 strict events");
    }

    #[test]
    fn pending_profile_counts_tracks_strict_events() {
        let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(20);
        queue.enqueue_strict(make_event(run, 0)).expect("enqueue should succeed");
        queue.enqueue_strict(make_event(run, 1)).expect("enqueue should succeed");

        let counts = queue.pending_profile_counts().expect("counts should succeed");
        assert_eq!(counts.journaled, 0, "should have 0 journaled events");
        assert_eq!(counts.strict, 2, "should have 2 strict events");
    }

    #[test]
    fn pending_profile_counts_tracks_mixed_profiles() {
        let queue = JournalWriterQueue::new(6, 3, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(30);
        queue.enqueue_journaled(make_event(run, 0)).expect("enqueue should succeed");
        queue.enqueue_strict(make_event(run, 1)).expect("enqueue should succeed");
        queue.enqueue_journaled(make_event(run, 2)).expect("enqueue should succeed");
        queue.enqueue_strict(make_event(run, 3)).expect("enqueue should succeed");

        let counts = queue.pending_profile_counts().expect("counts should succeed");
        assert_eq!(counts.journaled, 2, "should have 2 journaled events");
        assert_eq!(counts.strict, 2, "should have 2 strict events");
    }

    #[test]
    fn pending_profile_counts_empty_queue() {
        let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let counts = queue.pending_profile_counts().expect("counts should succeed");
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
        queue.enqueue_journaled(make_event(run, 0)).expect("enqueue before shutdown");

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
            queue.enqueue_journaled(make_event(run, i)).unwrap_or_else(|_| {
                panic!("enqueue {} should succeed", i)
            });
        }

        let report = queue.shutdown(&journal).expect("shutdown should succeed");
        assert_eq!(report.drained, 5, "shutdown should drain all 5 events");
        assert_eq!(report.written, 5, "shutdown should write all 5 events");

        // Verify events are persisted
        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 5, "journal should contain 5 events after shutdown");
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
            queue.enqueue_journaled(make_event(run, i)).unwrap_or_else(|_| {
                panic!("enqueue {} should succeed", i)
            });
        }

        // batch_size is 2, so first flush drains 2
        let report = queue.flush_batch(&journal).expect("flush should succeed");
        assert_eq!(report.drained, 2, "first flush should drain batch_size (2)");
        assert_eq!(report.written, 2);

        // Still 3 remaining
        let counts = queue.pending_profile_counts().expect("counts should succeed");
        assert_eq!(counts.journaled, 3, "should have 3 remaining events");
    }

    #[test]
    fn drain_all_flushes_until_empty() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(8, 2, StorageLimits::DEFAULT)
            .expect("queue creation should succeed");
        let run = RunId::new(70);
        for i in 0..5u64 {
            queue.enqueue_journaled(make_event(run, i)).unwrap_or_else(|_| {
                panic!("enqueue {} should succeed", i)
            });
        }

        let report = queue.drain_all(&journal).expect("drain_all should succeed");
        assert_eq!(report.drained, 5, "drain_all should drain all 5 events");
        assert_eq!(report.written, 5);

        // Queue should now be empty
        let counts = queue.pending_profile_counts().expect("counts should succeed");
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
        queue.enqueue_journaled(make_event(run, 0)).expect("enqueue 0");
        queue.enqueue_journaled(make_event(run, 1)).expect("enqueue 1");

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
        queue.enqueue_journaled(make_event(run, 0)).expect("enqueue journaled");
        queue.enqueue_strict(make_event(run, 1)).expect("enqueue strict");
        queue.enqueue_journaled(make_event(run, 2)).expect("enqueue journaled");

        let report = queue.drain_all(&journal).expect("drain_all should succeed");
        assert_eq!(report.drained, 3);
        assert_eq!(report.written, 3);

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 3);
    }
}
