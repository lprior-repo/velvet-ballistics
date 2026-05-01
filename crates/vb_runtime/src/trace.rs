//! Bounded trace ring using rtrb SPSC ring buffer.

use rtrb::RingBuffer;
use std::collections::VecDeque;
use vb_core::action::ActionFailureCode;
use vb_core::ids::{RunId, SlotIdx, StepIdx};

/// Bounded trace event ring for one shard.
#[derive(Debug)]
pub struct TraceRing {
    producer: rtrb::Producer<TraceEvent>,
    consumer: rtrb::Consumer<TraceEvent>,
    mirror: VecDeque<TraceEvent>,
    capacity: usize,
    dropped: u64,
}

impl TraceRing {
    /// Creates a trace ring with the given bounded capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (producer, consumer) = RingBuffer::new(capacity);
        Self {
            producer,
            consumer,
            mirror: VecDeque::with_capacity(capacity),
            capacity,
            dropped: 0,
        }
    }

    /// Returns the ring capacity.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Attempts to push a trace event. Returns false if the ring is full (drops oldest policy
    /// is not used here; the caller may choose to count the drop).
    pub fn push(&mut self, event: TraceEvent) -> bool {
        self.remember(event.clone());
        match self.producer.push(event) {
            Ok(()) => true,
            Err(_) => {
                self.dropped = self.dropped.saturating_add(1);
                false
            }
        }
    }

    /// Returns a non-destructive bounded snapshot of events for one run.
    pub fn snapshot_for_run(&self, target: RunId, limit: usize) -> Vec<TraceEvent> {
        let bounded_limit = limit.min(self.capacity);
        let mut events = Vec::with_capacity(bounded_limit);
        for event in &self.mirror {
            if events.len() >= bounded_limit {
                return events;
            }
            if event.run_id() == target {
                events.push(event.clone());
            }
        }
        events
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
            let event = match self.consumer.pop() {
                Ok(event) => event,
                Err(_) => return,
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
        let mut inspected = 0usize;
        while inspected < bounded_limit {
            let event = match self.consumer.pop() {
                Ok(event) => event,
                Err(_) => return events,
            };
            if event.run_id() == target {
                events.push(event);
            }
            inspected = match inspected.checked_add(1) {
                Some(next) => next,
                None => return events,
            };
        }
        events
    }

    /// Returns the number of dropped events due to ring overflow.
    #[must_use]
    pub const fn dropped(&self) -> u64 {
        self.dropped
    }

    fn remember(&mut self, event: TraceEvent) {
        if self.capacity == 0 {
            return;
        }
        if self.mirror.len() >= self.capacity {
            let _ = self.mirror.pop_front();
        }
        self.mirror.push_back(event);
    }
}

/// Binary trace event recorded by the shard execution loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceEvent {
    /// A step began execution.
    StepStarted {
        /// Run identifier.
        run: RunId,
        /// Step index.
        step: StepIdx,
    },
    /// A step completed execution.
    StepEnded {
        /// Run identifier.
        run: RunId,
        /// Step index.
        step: StepIdx,
    },
    /// A slot was written.
    SlotWritten {
        /// Run identifier.
        run: RunId,
        /// Slot index.
        slot: SlotIdx,
    },
    /// An action was scheduled.
    ActionScheduled {
        /// Run identifier.
        run: RunId,
        /// Step that scheduled the action.
        step: StepIdx,
    },
    /// An action completed.
    ActionCompleted {
        /// Run identifier.
        run: RunId,
        /// Step that received the completion.
        step: StepIdx,
    },
    /// An action failed with a typed failure code.
    ActionFailed {
        /// Run identifier.
        run: RunId,
        /// Step that received the failure.
        step: StepIdx,
        /// Machine-readable failure code.
        code: ActionFailureCode,
    },
    /// An ask was answered and the run was resumed.
    AskAnswered {
        /// Run identifier.
        run: RunId,
        /// Step that issued the ask.
        step: StepIdx,
        /// Slot that received the answer payload.
        slot: SlotIdx,
    },
    /// A run was submitted.
    RunSubmitted {
        /// Run identifier.
        run: RunId,
    },
    /// A run finished.
    RunFinished {
        /// Run identifier.
        run: RunId,
    },
    /// A run failed.
    RunFailed {
        /// Run identifier.
        run: RunId,
    },
}

impl TraceEvent {
    /// Returns the run associated with this trace event.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        match self {
            Self::StepStarted { run, .. }
            | Self::StepEnded { run, .. }
            | Self::SlotWritten { run, .. }
            | Self::ActionScheduled { run, .. }
            | Self::ActionCompleted { run, .. }
            | Self::ActionFailed { run, .. }
            | Self::AskAnswered { run, .. }
            | Self::RunSubmitted { run }
            | Self::RunFinished { run }
            | Self::RunFailed { run } => *run,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_for_run_does_not_drain_events() {
        let run = RunId::new(3);
        let mut ring = TraceRing::new(4);
        assert!(ring.push(TraceEvent::RunSubmitted { run }));

        let first = ring.snapshot_for_run(run, 4);
        let second = ring.snapshot_for_run(run, 4);

        assert_eq!(first, vec![TraceEvent::RunSubmitted { run }]);
        assert_eq!(second, vec![TraceEvent::RunSubmitted { run }]);
        assert_eq!(ring.drain(), vec![TraceEvent::RunSubmitted { run }]);
    }

    #[test]
    fn snapshot_for_run_respects_capacity_window() {
        let first_run = RunId::new(1);
        let second_run = RunId::new(2);
        let mut ring = TraceRing::new(2);
        assert!(ring.push(TraceEvent::RunSubmitted { run: first_run }));
        assert!(ring.push(TraceEvent::RunSubmitted { run: second_run }));
        assert!(!ring.push(TraceEvent::RunFinished { run: first_run }));

        let first_events = ring.snapshot_for_run(first_run, 2);
        let second_events = ring.snapshot_for_run(second_run, 2);

        assert_eq!(
            first_events,
            vec![TraceEvent::RunFinished { run: first_run }]
        );
        assert_eq!(
            second_events,
            vec![TraceEvent::RunSubmitted { run: second_run }]
        );
    }
}
