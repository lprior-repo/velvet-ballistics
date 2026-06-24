#![forbid(unsafe_code)]
//! Bounded trace ring using rtrb SPSC ring buffer.

use std::collections::VecDeque;

use rtrb::RingBuffer;
use vb_core::action::ActionFailureCode;
use vb_core::ids::{RunId, SlotIdx, StepIdx};

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
    /// Creates a trace ring with the given bounded capacity.
    ///
    /// # Invariants
    ///
    /// `capacity` must be ≥ 1. A value of 0 is normalized to 1.
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

    /// Returns the number of events currently in the ring.
    #[must_use]
    pub fn len(&self) -> usize {
        self.consumer.slots()
    }

    /// Returns true if the ring contains no events.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.consumer.is_empty()
    }

    /// Attempts to push a trace event. Returns false if the ring is full (drops oldest policy
    /// is not used here; the caller may choose to count the drop).
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
        let mut inspected = 0usize;
        while inspected < bounded_limit {
            let Ok(event) = self.consumer.pop() else {
                return events;
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
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
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
        /// Encoded slot value bytes (postcard-encoded `SlotValue`).
        value: Vec<u8>,
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
    /// An action failed.
    ActionFailed {
        /// Run identifier.
        run: RunId,
        /// Step that received the failure.
        step: StepIdx,
        /// Failure code.
        code: ActionFailureCode,
    },
    /// An ask was answered.
    AskAnswered {
        /// Run identifier.
        run: RunId,
        /// Step that scheduled the ask.
        step: StepIdx,
        /// Slot that received the answer.
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
    /// A run was cancelled.
    RunCancelled {
        /// Run identifier.
        run: RunId,
    },
    /// A run was killed.
    RunKilled {
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
            | Self::RunFailed { run }
            | Self::RunCancelled { run }
            | Self::RunKilled { run } => *run,
        }
    }

    /// Returns true when this event is terminal evidence for the given run.
    #[must_use]
    pub fn is_terminal_for_run(&self, target: RunId) -> bool {
        match self {
            Self::RunFinished { run }
            | Self::RunFailed { run }
            | Self::RunCancelled { run }
            | Self::RunKilled { run } => *run == target,
            Self::StepStarted { .. }
            | Self::StepEnded { .. }
            | Self::SlotWritten { .. }
            | Self::ActionScheduled { .. }
            | Self::ActionCompleted { .. }
            | Self::ActionFailed { .. }
            | Self::AskAnswered { .. }
            | Self::RunSubmitted { .. } => false,
        }
    }
}

#[cfg(test)]
mod tests;
