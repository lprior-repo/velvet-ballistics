#![forbid(unsafe_code)]
//! Bounded trace ring using rtrb SPSC ring buffer.

#[cfg(not(kani))]
use std::collections::VecDeque;

#[cfg(not(kani))]
use rtrb::RingBuffer;
use vb_core::action::ActionFailureCode;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::limits::MAX_TRACE_RING_CAPACITY;

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
        let bounded_capacity = capacity.clamp(1, KANI_TRACE_MODEL_CAPACITY);
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
        let remembered = event.clone();
        if self.push_pending(event) {
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

    /// Returns true when remembered trace evidence shows the run reached a terminal state.
    #[must_use]
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

    #[cfg(kani)]
    pub(crate) fn drain_pending_kani_limit_64(&mut self) {
        for _ in 0..KANI_TRACE_MODEL_CAPACITY {
            if self.pop_pending().is_none() {
                return;
            }
        }
    }

    #[cfg(kani)]
    pub(crate) fn drain_for_run_kani_limit_4_summary(
        &mut self,
        target: RunId,
    ) -> KaniDrainSummary {
        let mut summary = KaniDrainSummary::new();
        for _ in 0..4 {
            let Some(event) = self.pop_pending() else {
                return summary;
            };
            if event.run_id() == target {
                summary.record(event.kind());
            }
        }
        summary
    }

    fn remember(&mut self, event: TraceEvent) {
        #[cfg(not(kani))]
        {
            if self.history.len() >= self.capacity {
                if self.history.pop_front().is_none() {
                    return;
                }
            }
            self.history.push_back(event);
        }
        #[cfg(kani)]
        {
            if self.history.len() >= self.capacity {
                if self.history.pop_front().is_none() {
                    return;
                }
            }
            let _stored = self.history.push_back(event, self.capacity);
        }
    }

    #[cfg(not(kani))]
    fn history_events(&self) -> impl Iterator<Item = &TraceEvent> {
        self.history.iter()
    }

    #[cfg(kani)]
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
    fn push_pending(&mut self, event: TraceEvent) -> bool {
        self.pending.push_back(event, self.capacity)
    }

    #[cfg(not(kani))]
    fn pop_pending(&mut self) -> Option<TraceEvent> {
        self.consumer.pop().ok()
    }

    #[cfg(kani)]
    fn pop_pending(&mut self) -> Option<TraceEvent> {
        self.pending.pop_front()
    }

    /// Returns the number of dropped events due to ring overflow.
    #[must_use]
    pub const fn dropped(&self) -> u64 {
        self.dropped
    }
}

#[cfg(kani)]
const KANI_TRACE_MODEL_CAPACITY: usize = 64;

#[cfg(kani)]
#[derive(Debug)]
struct KaniTraceQueue {
    events: [Option<TraceEvent>; KANI_TRACE_MODEL_CAPACITY],
    len: usize,
}

#[cfg(kani)]
impl KaniTraceQueue {
    fn new() -> Self {
        Self {
            events: std::array::from_fn(|_| None),
            len: 0,
        }
    }

    const fn len(&self) -> usize {
        self.len
    }

    const fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn iter(&self) -> impl Iterator<Item = &TraceEvent> {
        self.events
            .iter()
            .take(self.len)
            .filter_map(core::option::Option::as_ref)
    }

    fn push_back(&mut self, event: TraceEvent, capacity: usize) -> bool {
        if self.len >= capacity || self.len >= KANI_TRACE_MODEL_CAPACITY {
            return false;
        }
        let Some(next_len) = self.len.checked_add(1) else {
            return false;
        };
        let Some(slot) = self.events.get_mut(self.len) else {
            return false;
        };
        *slot = Some(event);
        self.len = next_len;
        true
    }

    fn pop_front(&mut self) -> Option<TraceEvent> {
        if self.len == 0 {
            return None;
        }
        let first = self.take_at(0);
        for index in 1..KANI_TRACE_MODEL_CAPACITY {
            if index >= self.len {
                break;
            }
            let Some(previous) = index.checked_sub(1) else {
                return first;
            };
            let moved = self.take_at(index);
            if !self.put_at(previous, moved) {
                return first;
            }
        }
        self.len = match self.len.checked_sub(1) {
            Some(next_len) => next_len,
            None => 0,
        };
        first
    }

    fn take_at(&mut self, index: usize) -> Option<TraceEvent> {
        self.events.get_mut(index).and_then(core::option::Option::take)
    }

    fn put_at(&mut self, index: usize, event: Option<TraceEvent>) -> bool {
        let Some(slot) = self.events.get_mut(index) else {
            return false;
        };
        *slot = event;
        true
    }
}

#[cfg(kani)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum KaniTraceEventKind {
    StepStarted,
    StepEnded,
    SlotWritten,
    ActionScheduled,
    ActionCompleted,
    ActionFailed,
    AskAnswered,
    RunSubmitted,
    RunFinished,
    RunFailed,
    RunCancelled,
    RunKilled,
}

#[cfg(kani)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct KaniDrainSummary {
    matched: u8,
    first: Option<KaniTraceEventKind>,
    second: Option<KaniTraceEventKind>,
    overflow: bool,
}

#[cfg(kani)]
impl KaniDrainSummary {
    const fn new() -> Self {
        Self {
            matched: 0,
            first: None,
            second: None,
            overflow: false,
        }
    }

    pub(crate) const fn matched(&self) -> u8 {
        self.matched
    }

    pub(crate) const fn first(&self) -> Option<KaniTraceEventKind> {
        self.first
    }

    pub(crate) const fn second(&self) -> Option<KaniTraceEventKind> {
        self.second
    }

    pub(crate) const fn overflow(&self) -> bool {
        self.overflow
    }

    fn record(&mut self, kind: KaniTraceEventKind) {
        match self.matched {
            0 => self.first = Some(kind),
            1 => self.second = Some(kind),
            _ => self.overflow = true,
        }
        self.matched = match self.matched.checked_add(1) {
            Some(next) => next,
            None => self.matched,
        };
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
    #[cfg(kani)]
    const fn kind(&self) -> KaniTraceEventKind {
        match self {
            Self::StepStarted { .. } => KaniTraceEventKind::StepStarted,
            Self::StepEnded { .. } => KaniTraceEventKind::StepEnded,
            Self::SlotWritten { .. } => KaniTraceEventKind::SlotWritten,
            Self::ActionScheduled { .. } => KaniTraceEventKind::ActionScheduled,
            Self::ActionCompleted { .. } => KaniTraceEventKind::ActionCompleted,
            Self::ActionFailed { .. } => KaniTraceEventKind::ActionFailed,
            Self::AskAnswered { .. } => KaniTraceEventKind::AskAnswered,
            Self::RunSubmitted { .. } => KaniTraceEventKind::RunSubmitted,
            Self::RunFinished { .. } => KaniTraceEventKind::RunFinished,
            Self::RunFailed { .. } => KaniTraceEventKind::RunFailed,
            Self::RunCancelled { .. } => KaniTraceEventKind::RunCancelled,
            Self::RunKilled { .. } => KaniTraceEventKind::RunKilled,
        }
    }

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
