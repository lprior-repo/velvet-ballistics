#![forbid(unsafe_code)]
//! Bounded trace ring using rtrb SPSC ring buffer.

#[cfg(not(kani))]
use std::collections::VecDeque;

#[cfg(not(kani))]
use rtrb::RingBuffer;
use vb_core::action::ActionFailureCode;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
#[cfg(not(kani))]
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

    #[cfg(kani)]
    pub(crate) fn drain_pending_kani_limit_64(&mut self) {
        for _ in 0..KANI_TRACE_MODEL_CAPACITY {
            if self.pop_pending_record().is_none() {
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
            let Some(event) = self.pop_pending_record() else {
                return summary;
            };
            if event.run == target {
                summary.record(event.kind);
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
        self.pending.pop_front_record().map(KaniTraceRecord::to_event)
    }

    #[cfg(kani)]
    fn pop_pending_record(&mut self) -> Option<KaniTraceRecord> {
        self.pending.pop_front_record()
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
    first: Option<KaniTraceRecord>,
    second: Option<KaniTraceRecord>,
    third: Option<KaniTraceRecord>,
    fourth: Option<KaniTraceRecord>,
    len: usize,
}

#[cfg(kani)]
impl KaniTraceQueue {
    const fn new() -> Self {
        Self {
            first: None,
            second: None,
            third: None,
            fourth: None,
            len: 0,
        }
    }

    const fn len(&self) -> usize {
        self.len
    }

    const fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn push_back(&mut self, record: KaniTraceRecord, capacity: usize) -> bool {
        if self.len >= capacity || self.len >= KANI_TRACE_MODEL_CAPACITY {
            return false;
        }
        let Some(next_len) = self.len.checked_add(1) else {
            return false;
        };
        match self.len {
            0 => self.first = Some(record),
            1 => self.second = Some(record),
            2 => self.third = Some(record),
            3 => self.fourth = Some(record),
            _ => {}
        }
        self.len = next_len;
        true
    }

    fn pop_front_record(&mut self) -> Option<KaniTraceRecord> {
        let old_len = self.len;
        if old_len == 0 {
            return None;
        }
        let first = self.first.or(Some(KaniTraceRecord::unknown()));
        self.first = self.second;
        self.second = self.third;
        self.third = self.fourth;
        self.fourth = if old_len > 4 {
            Some(KaniTraceRecord::unknown())
        } else {
            None
        };
        self.len = match self.len.checked_sub(1) {
            Some(next_len) => next_len,
            None => 0,
        };
        first
    }

    fn has_terminal_for_run(&self, target: RunId, capacity: usize) -> bool {
        self.slot_is_terminal(self.first, target, capacity, 0)
            || self.slot_is_terminal(self.second, target, capacity, 1)
            || self.slot_is_terminal(self.third, target, capacity, 2)
            || self.slot_is_terminal(self.fourth, target, capacity, 3)
    }

    fn slot_is_terminal(
        &self,
        record: Option<KaniTraceRecord>,
        target: RunId,
        capacity: usize,
        position: usize,
    ) -> bool {
        if position >= capacity || position >= self.len {
            return false;
        }
        match record {
            Some(value) => value.is_terminal_for_run(target),
            None => false,
        }
    }
}

#[cfg(kani)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct KaniTraceRecord {
    run: RunId,
    kind: KaniTraceEventKind,
}

#[cfg(kani)]
impl KaniTraceRecord {
    const fn from_event(event: &TraceEvent) -> Self {
        match event {
            TraceEvent::StepStarted { run, .. } => Self {
                run: *run,
                kind: KaniTraceEventKind::StepStarted,
            },
            TraceEvent::StepEnded { run, .. } => Self {
                run: *run,
                kind: KaniTraceEventKind::StepEnded,
            },
            TraceEvent::SlotWritten { run, .. } => Self {
                run: *run,
                kind: KaniTraceEventKind::SlotWritten,
            },
            TraceEvent::ActionScheduled { run, .. } => Self {
                run: *run,
                kind: KaniTraceEventKind::ActionScheduled,
            },
            TraceEvent::ActionCompleted { run, .. } => Self {
                run: *run,
                kind: KaniTraceEventKind::ActionCompleted,
            },
            TraceEvent::ActionFailed { run, .. } => Self {
                run: *run,
                kind: KaniTraceEventKind::ActionFailed,
            },
            TraceEvent::AskAnswered { run, .. } => Self {
                run: *run,
                kind: KaniTraceEventKind::AskAnswered,
            },
            TraceEvent::RunSubmitted { run } => Self {
                run: *run,
                kind: KaniTraceEventKind::RunSubmitted,
            },
            TraceEvent::RunFinished { run } => Self {
                run: *run,
                kind: KaniTraceEventKind::RunFinished,
            },
            TraceEvent::RunFailed { run } => Self {
                run: *run,
                kind: KaniTraceEventKind::RunFailed,
            },
            TraceEvent::RunCancelled { run } => Self {
                run: *run,
                kind: KaniTraceEventKind::RunCancelled,
            },
            TraceEvent::RunKilled { run } => Self {
                run: *run,
                kind: KaniTraceEventKind::RunKilled,
            },
        }
    }

    const fn unknown() -> Self {
        Self {
            run: RunId::new(0),
            kind: KaniTraceEventKind::RunSubmitted,
        }
    }

    fn to_event(self) -> TraceEvent {
        match self.kind {
            KaniTraceEventKind::StepStarted => TraceEvent::StepStarted {
                run: self.run,
                step: StepIdx::new(0),
            },
            KaniTraceEventKind::StepEnded => TraceEvent::StepEnded {
                run: self.run,
                step: StepIdx::new(0),
            },
            KaniTraceEventKind::SlotWritten => TraceEvent::SlotWritten {
                run: self.run,
                slot: SlotIdx::new(0),
                value: Vec::new(),
            },
            KaniTraceEventKind::ActionScheduled => TraceEvent::ActionScheduled {
                run: self.run,
                step: StepIdx::new(0),
            },
            KaniTraceEventKind::ActionCompleted => TraceEvent::ActionCompleted {
                run: self.run,
                step: StepIdx::new(0),
            },
            KaniTraceEventKind::ActionFailed => TraceEvent::ActionFailed {
                run: self.run,
                step: StepIdx::new(0),
                code: ActionFailureCode::Timeout,
            },
            KaniTraceEventKind::AskAnswered => TraceEvent::AskAnswered {
                run: self.run,
                step: StepIdx::new(0),
                slot: SlotIdx::new(0),
            },
            KaniTraceEventKind::RunSubmitted => TraceEvent::RunSubmitted { run: self.run },
            KaniTraceEventKind::RunFinished => TraceEvent::RunFinished { run: self.run },
            KaniTraceEventKind::RunFailed => TraceEvent::RunFailed { run: self.run },
            KaniTraceEventKind::RunCancelled => TraceEvent::RunCancelled { run: self.run },
            KaniTraceEventKind::RunKilled => TraceEvent::RunKilled { run: self.run },
        }
    }

    const fn is_terminal_for_run(self, target: RunId) -> bool {
        match self.kind {
            KaniTraceEventKind::RunFinished
            | KaniTraceEventKind::RunFailed
            | KaniTraceEventKind::RunCancelled
            | KaniTraceEventKind::RunKilled => self.run.get() == target.get(),
            KaniTraceEventKind::StepStarted
            | KaniTraceEventKind::StepEnded
            | KaniTraceEventKind::SlotWritten
            | KaniTraceEventKind::ActionScheduled
            | KaniTraceEventKind::ActionCompleted
            | KaniTraceEventKind::ActionFailed
            | KaniTraceEventKind::AskAnswered
            | KaniTraceEventKind::RunSubmitted => false,
        }
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
