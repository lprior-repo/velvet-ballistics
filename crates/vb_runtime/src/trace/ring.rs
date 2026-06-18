#![forbid(unsafe_code)]
//! Bounded trace ring using rtrb SPSC ring buffer.

#[cfg(not(kani))]
use std::collections::VecDeque;

#[cfg(not(kani))]
use rtrb::RingBuffer;
use vb_core::ids::RunId;
#[cfg(not(kani))]
use vb_core::limits::MAX_TRACE_RING_CAPACITY;

use super::event::TraceEvent;
#[cfg(all(kani, feature = "kani-trace-ring"))]
use super::kani::KaniDrainSummary;
#[cfg(kani)]
use super::kani::{KaniTraceQueue, KaniTraceRecord};

/// Bounded trace event ring for one shard.
#[derive(Debug)]
pub struct TraceRing {
    #[cfg(not(kani))]
    producer: rtrb::Producer<TraceEvent>,
    #[cfg(not(kani))]
    consumer: rtrb::Consumer<TraceEvent>,
    #[cfg(kani)]
    pending: KaniTraceQueue,
    capacity: usize,
    dropped: u64,
    #[cfg(not(kani))]
    history: VecDeque<TraceEvent>,
    #[cfg(kani)]
    history: KaniTraceQueue,
}

impl TraceRing {
    /// Creates a trace ring with the given bounded capacity.
    ///
    /// # Invariants
    ///
    /// `capacity` is normalized into `1..=MAX_TRACE_RING_CAPACITY`.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        #[cfg(not(kani))]
        let bounded_capacity = capacity.clamp(1, MAX_TRACE_RING_CAPACITY);
        #[cfg(kani)]
        let bounded_capacity = capacity.clamp(1, KaniTraceQueue::capacity());
        #[cfg(not(kani))]
        let (producer, consumer) = RingBuffer::new(bounded_capacity);
        Self {
            #[cfg(not(kani))]
            producer,
            #[cfg(not(kani))]
            consumer,
            #[cfg(kani)]
            pending: KaniTraceQueue::new(),
            capacity: bounded_capacity,
            dropped: 0,
            #[cfg(not(kani))]
            history: VecDeque::with_capacity(bounded_capacity),
            #[cfg(kani)]
            history: KaniTraceQueue::new(),
        }
    }

    /// Returns the ring capacity.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the number of events currently in the ring.
    #[must_use]
    pub fn len(&self) -> usize {
        self.history.len()
    }

    /// Returns true if the ring contains no events.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    /// Attempts to push a trace event. Returns false if the ring is full (drops oldest policy
    /// is not used here; the caller may choose to count the drop).
    pub fn push(&mut self, event: TraceEvent) -> bool {
        #[cfg(not(kani))]
        {
            let remembered = event.clone();
            if self.push_pending(event) {
                self.remember(remembered);
                true
            } else {
                self.dropped = self.dropped.saturating_add(1);
                false
            }
        }
        #[cfg(kani)]
        {
            let record = KaniTraceRecord::from_event(&event);
            if self.push_pending_record(record) {
                self.remember_record(record);
                true
            } else {
                self.dropped = self.dropped.saturating_add(1);
                false
            }
        }
    }

    /// Drains all pending trace events into a vector.
    pub fn drain(&mut self) -> Vec<TraceEvent> {
        let mut events = Vec::with_capacity(self.capacity);
        self.drain_into(self.capacity, &mut events);
        events
    }

    /// Drains at most `limit` events into `events`.
    pub fn drain_into(&mut self, limit: usize, events: &mut Vec<TraceEvent>) {
        let bounded_limit = self.bounded_limit(limit);
        for _ in 0..bounded_limit {
            let Some(event) = self.pop_pending() else {
                return;
            };
            events.push(event);
        }
    }

    /// Drains at most `limit` events for one run into a vector.
    pub fn drain_for_run(&mut self, target: RunId, limit: usize) -> Vec<TraceEvent> {
        let bounded_limit = self.bounded_limit(limit);
        let mut events = Vec::with_capacity(bounded_limit);
        for _ in 0..bounded_limit {
            let Some(event) = self.pop_pending() else {
                return events;
            };
            if event.run_id() == target {
                events.push(event);
            }
        }
        events
    }

    /// Returns at most `limit` remembered trace events for one run without draining the ring.
    #[cfg(not(kani))]
    pub fn snapshot_for_run(&self, target: RunId, limit: usize) -> Vec<TraceEvent> {
        let bounded_limit = self.bounded_limit(limit);
        let mut events = Vec::with_capacity(bounded_limit);
        let mut inspected = 0usize;
        for event in self.history_events() {
            if inspected >= bounded_limit {
                return events;
            }
            if event.run_id() == target {
                events.push(event.clone());
                inspected = match inspected.checked_add(1) {
                    Some(next) => next,
                    None => return events,
                };
            }
        }
        events
    }

    /// Returns at most `limit` remembered trace events for one run without draining the ring.
    #[cfg(kani)]
    pub fn snapshot_for_run(&self, _target: RunId, _limit: usize) -> Vec<TraceEvent> {
        Vec::new()
    }

    /// Returns true when remembered trace evidence shows the run reached a terminal state.
    #[must_use]
    #[cfg(not(kani))]
    pub fn has_terminal_event_for_run(&self, target: RunId) -> bool {
        let mut inspected = 0usize;
        for event in self.history_events() {
            if inspected >= self.capacity {
                return false;
            }
            if event.is_terminal_for_run(target) {
                return true;
            }
            inspected = match inspected.checked_add(1) {
                Some(next) => next,
                None => return false,
            };
        }
        false
    }

    /// Returns true when remembered trace evidence shows the run reached a terminal state.
    #[must_use]
    #[cfg(kani)]
    pub fn has_terminal_event_for_run(&self, target: RunId) -> bool {
        self.history.has_terminal_for_run(target, self.capacity)
    }

    #[cfg(all(kani, feature = "kani-trace-ring"))]
    pub(crate) fn drain_pending_kani_limit_64(&mut self) {
        for _ in 0..KaniTraceQueue::capacity() {
            if self.pop_pending_record().is_none() {
                return;
            }
        }
    }

    #[cfg(all(kani, feature = "kani-trace-ring"))]
    pub(crate) fn drain_for_run_kani_limit_4_summary(&mut self, target: RunId) -> KaniDrainSummary {
        let mut summary = KaniDrainSummary::new();
        for _ in 0..4 {
            let Some(event) = self.pop_pending_record() else {
                return summary;
            };
            if event.run() == target {
                summary.record(event.kind());
            }
        }
        summary
    }

    #[cfg(not(kani))]
    fn remember(&mut self, event: TraceEvent) {
        if self.history.len() >= self.capacity {
            if self.history.pop_front().is_none() {
                return;
            }
        }
        self.history.push_back(event);
    }

    #[cfg(kani)]
    fn remember_record(&mut self, record: KaniTraceRecord) {
        if self.history.len() >= self.capacity {
            if self.history.pop_front_record().is_none() {
                return;
            }
        }
        let _stored = self.history.push_back(record, self.capacity);
    }

    #[cfg(not(kani))]
    fn history_events(&self) -> impl Iterator<Item = &TraceEvent> {
        self.history.iter()
    }

    fn bounded_limit(&self, limit: usize) -> usize {
        if limit <= self.capacity {
            return limit;
        }
        self.capacity
    }

    #[cfg(not(kani))]
    fn push_pending(&mut self, event: TraceEvent) -> bool {
        self.producer.push(event).is_ok()
    }

    #[cfg(kani)]
    fn push_pending_record(&mut self, record: KaniTraceRecord) -> bool {
        self.pending.push_back(record, self.capacity)
    }

    #[cfg(not(kani))]
    fn pop_pending(&mut self) -> Option<TraceEvent> {
        self.consumer.pop().ok()
    }

    #[cfg(kani)]
    fn pop_pending(&mut self) -> Option<TraceEvent> {
        self.pending
            .pop_front_record()
            .map(KaniTraceRecord::to_event)
    }

    #[cfg(all(kani, feature = "kani-trace-ring"))]
    fn pop_pending_record(&mut self) -> Option<KaniTraceRecord> {
        self.pending.pop_front_record()
    }

    /// Returns the number of dropped events due to ring overflow.
    #[must_use]
    pub const fn dropped(&self) -> u64 {
        self.dropped
    }
}
