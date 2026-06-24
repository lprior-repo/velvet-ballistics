#![forbid(unsafe_code)]
//! Bounded trace ring using rtrb SPSC ring buffer.

mod event;

pub use event::TraceEvent;

use std::collections::VecDeque;

use rtrb::RingBuffer;
use vb_core::ids::RunId;

/// Bounded trace event ring for one shard.
#[derive(Debug)]
pub struct TraceRing {
    producer: rtrb::Producer<TraceEvent>,
    consumer: rtrb::Consumer<TraceEvent>,
    capacity: usize,
    dropped: u64,
    history: VecDeque<TraceEvent>,
}

impl TraceRing {
    /// Creates a trace ring with nonzero bounded capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (producer, consumer) = RingBuffer::new(capacity.max(1));
        Self {
            producer,
            consumer,
            capacity,
            dropped: 0,
            history: VecDeque::with_capacity(capacity),
        }
    }

    /// Returns the ring capacity.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the number of drainable events currently buffered in the ring.
    /// This counts pending events the next `drain` would remove, not the
    /// replayable history retained by `snapshot_for_run`.
    #[must_use]
    pub fn pending_len(&self) -> usize {
        self.consumer.slots()
    }

    /// Returns true when no pending events remain to be drained.
    /// This observes the drainable queue, not the replayable history.
    #[must_use]
    pub fn pending_is_empty(&self) -> bool {
        self.consumer.is_empty()
    }

    /// Pending drainable events (alias for [`pending_len`]).
    pub fn len(&self) -> usize {
        self.pending_len()
    }
    /// Pending drainable events empty (alias for [`pending_is_empty`]).
    pub fn is_empty(&self) -> bool {
        self.pending_is_empty()
    }

    /// Returns the number of events retained for replay snapshots.
    /// Replayable history survives `drain`; see `pending_len` for the
    /// drainable queue occupancy.
    #[must_use]
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Returns true when no replayable history events are retained.
    /// This observes the snapshot history, not the drainable queue.
    #[must_use]
    pub fn history_is_empty(&self) -> bool {
        self.history.is_empty()
    }

    /// Attempts to push a trace event. Returns false if the ring is full.
    pub fn push(&mut self, event: TraceEvent) -> bool {
        let remembered = event.clone();
        if let Ok(()) = self.producer.push(event) {
            self.remember(remembered);
            true
        } else {
            self.dropped = self.dropped.saturating_add(1);
            false
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
        let mut drained = 0usize;
        while drained < limit {
            let Ok(event) = self.consumer.pop() else {
                return;
            };
            events.push(event);
            drained = match drained.checked_add(1) {
                Some(next) => next,
                None => return,
            };
        }
    }

    /// Drains at most `limit` events for one run into a vector.
    pub fn drain_for_run(&mut self, target: RunId, limit: usize) -> Vec<TraceEvent> {
        let bounded_limit = limit.min(self.capacity);
        let mut events = Vec::with_capacity(bounded_limit);
        if bounded_limit == 0 {
            return events;
        }
        let mut preserved = VecDeque::with_capacity(self.capacity);
        let mut inspected = 0usize;
        while inspected < self.capacity {
            let Ok(event) = self.consumer.pop() else {
                break;
            };
            if event.run_id() == target && events.len() < bounded_limit {
                events.push(event);
            } else {
                preserved.push_back(event);
            }
            inspected = match inspected.checked_add(1) {
                Some(next) => next,
                None => break,
            };
        }
        while let Some(event) = preserved.pop_front() {
            if self.producer.push(event).is_err() {
                self.dropped = self.dropped.saturating_add(1);
                break;
            }
        }
        events
    }

    /// Returns at most `limit` remembered trace events for one run without draining the ring.
    pub fn snapshot_for_run(&self, target: RunId, limit: usize) -> Vec<TraceEvent> {
        let bounded_limit = limit.min(self.capacity);
        let mut events = Vec::with_capacity(bounded_limit);
        let mut inspected = 0usize;
        for event in &self.history {
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

    /// Returns true when remembered trace evidence shows the run reached a terminal state.
    #[must_use]
    pub fn has_terminal_event_for_run(&self, target: RunId) -> bool {
        let mut inspected = 0usize;
        for event in &self.history {
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

    fn remember(&mut self, event: TraceEvent) {
        while self.history.len() >= self.capacity {
            if self.history.pop_front().is_none() {
                return;
            }
        }
        self.history.push_back(event);
    }

    /// Returns the number of dropped events due to ring overflow.
    #[must_use]
    pub const fn dropped(&self) -> u64 {
        self.dropped
    }
}

/// Binary trace event recorded by the shard execution loop.
#[cfg(test)]
mod tests;
