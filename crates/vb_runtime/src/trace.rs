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
        let mut events = Vec::new();
        while let Ok(event) = self.consumer.pop() {
            events.push(event);
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
