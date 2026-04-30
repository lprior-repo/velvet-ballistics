//! Bounded trace ring using rtrb SPSC ring buffer.

use rtrb::RingBuffer;
use vb_core::ids::{RunId, SlotIdx, StepIdx};

/// Bounded trace event ring for one shard.
#[derive(Debug)]
pub struct TraceRing {
    producer: rtrb::Producer<TraceEvent>,
    consumer: rtrb::Consumer<TraceEvent>,
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
        match self.producer.push(event) {
            Ok(()) => true,
            Err(_) => {
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
            | Self::RunSubmitted { run }
            | Self::RunFinished { run }
            | Self::RunFailed { run } => *run,
        }
    }
}
