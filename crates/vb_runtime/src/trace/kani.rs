#![forbid(unsafe_code)]
//! Kani verification models for the trace ring.
//!
//! Lightweight bounded queues and event-kind enums used only under `cfg(kani)`.

use vb_core::action::ActionFailureCode;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use crate::trace::TraceEvent;

pub(super) const KANI_TRACE_MODEL_CAPACITY: usize = 64;

/// Lightweight record used by Kani verification harnesses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct KaniTraceRecord {
    run: RunId,
    kind: KaniTraceEventKind,
}

impl KaniTraceRecord {
    pub(super) const fn from_event(event: &TraceEvent) -> Self {
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

    pub(super) const fn unknown() -> Self {
        Self {
            run: RunId::new(0),
            kind: KaniTraceEventKind::RunSubmitted,
        }
    }

    #[cfg(feature = "kani-trace-ring")]
    pub(super) const fn run(&self) -> RunId {
        self.run
    }

    #[cfg(feature = "kani-trace-ring")]
    pub(super) const fn kind(&self) -> KaniTraceEventKind {
        self.kind
    }

    pub(super) fn to_event(self) -> TraceEvent {
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

    pub(super) const fn is_terminal_for_run(self, target: RunId) -> bool {
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

/// Lightweight event kind used by Kani verification models.
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

/// Fixed-capacity bounded queue for Kani model checking.
#[derive(Debug)]
pub(super) struct KaniTraceQueue {
    first: Option<KaniTraceRecord>,
    second: Option<KaniTraceRecord>,
    third: Option<KaniTraceRecord>,
    fourth: Option<KaniTraceRecord>,
    len: usize,
}

impl KaniTraceQueue {
    pub(super) const fn new() -> Self {
        Self {
            first: None,
            second: None,
            third: None,
            fourth: None,
            len: 0,
        }
    }

    pub(super) const fn len(&self) -> usize {
        self.len
    }

    pub(super) const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub(super) const fn capacity() -> usize {
        KANI_TRACE_MODEL_CAPACITY
    }

    pub(super) fn push_back(&mut self, record: KaniTraceRecord, capacity: usize) -> bool {
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

    pub(super) fn pop_front_record(&mut self) -> Option<KaniTraceRecord> {
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

    pub(super) fn has_terminal_for_run(&self, target: RunId, capacity: usize) -> bool {
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

/// Summary of a bounded drain operation used by Kani verification harnesses.
#[cfg(feature = "kani-trace-ring")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct KaniDrainSummary {
    matched: u8,
    first: Option<KaniTraceEventKind>,
    second: Option<KaniTraceEventKind>,
    overflow: bool,
}

#[cfg(feature = "kani-trace-ring")]
impl KaniDrainSummary {
    pub(super) const fn new() -> Self {
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

    pub(super) fn record(&mut self, kind: KaniTraceEventKind) {
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
