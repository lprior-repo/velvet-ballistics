#![forbid(unsafe_code)]

//! Type definitions for the runtime engine.
//!
//! Exports evidence collection, error types, retry policy, and signals.

use vb_core::action::{ActionError, ActionTicket};
use vb_core::errors::EngineError;
use vb_core::ids::{ActionId, SlotIdx, StepIdx};
use vb_core::value::SlotValue;

/// Evidence event emitted by the deterministic drive loop for each step.
///
/// These events are collected during `drive_deterministic_full` and drained
/// by the shard to emit to the journal and trace ring. This satisfies
/// the Phase 40/44 evidence chain requirement that every deterministic step
/// emits `StepStarted` before `SlotWritten`, followed by `StepSucceeded`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceEvent {
    /// Step began execution.
    StepStarted {
        /// Step index.
        step: StepIdx,
    },
    /// Step completed and optionally wrote an output slot.
    StepSucceeded {
        /// Step index.
        step: StepIdx,
        /// Output slot written, if any (Nop/Jump have no output).
        output: Option<SlotIdx>,
    },
}

/// Bounded collector for evidence events produced during a drive loop.
///
/// Collected and drained once per drive loop iteration by the shard
/// to emit StepStarted/StepSucceeded/SlotWritten events to the journal.
#[derive(Debug)]
pub struct EvidenceCollector {
    events: Vec<EvidenceEvent>,
}

impl EvidenceCollector {
    /// Creates a new empty collector.
    #[must_use]
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Records a StepStarted event.
    pub fn push_step_started(&mut self, step: StepIdx) {
        self.events.push(EvidenceEvent::StepStarted { step });
    }

    /// Records a StepSucceeded event.
    pub fn push_step_succeeded(&mut self, step: StepIdx, output: Option<SlotIdx>) {
        self.events
            .push(EvidenceEvent::StepSucceeded { step, output });
    }

    /// Drains all collected events, returning them for processing.
    pub fn drain(&mut self) -> Vec<EvidenceEvent> {
        core::mem::take(&mut self.events)
    }

    /// Returns the number of collected events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Returns true if no events have been collected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Default for EvidenceCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Result type for runtime engine operations.
pub type RuntimeEngineResult<T> = Result<T, RuntimeEngineError>;

/// Errors from the runtime engine's action-aware execution.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RuntimeEngineError {
    /// Core engine error.
    #[error("{0}")]
    Core(EngineError),
    /// Action subsystem error.
    #[error("{0}")]
    Action(ActionError),
    /// Retry policy exhausted all attempts.
    #[error("retry exhausted for action {action:?} after {attempts} attempts")]
    RetryExhausted {
        /// Action that exhausted retries.
        action: ActionId,
        /// Number of attempts made.
        attempts: u16,
    },
    /// Taint propagation rejected a clean result from tainted input.
    #[error("taint violation at step {step:?}")]
    TaintViolation {
        /// Step where the violation occurred.
        step: StepIdx,
    },
}

impl From<EngineError> for RuntimeEngineError {
    fn from(error: EngineError) -> Self {
        Self::Core(error)
    }
}

impl From<ActionError> for RuntimeEngineError {
    fn from(error: ActionError) -> Self {
        Self::Action(error)
    }
}

impl RuntimeEngineError {
    /// Runtime code for exhausted retry policies.
    pub const RETRY_EXHAUSTED_RUNTIME_CODE: &str = "RETRY_EXHAUSTED";

    /// Returns the stable section 17 runtime code when this error has a direct mapping.
    #[must_use]
    pub const fn runtime_code(&self) -> Option<&'static str> {
        match self {
            Self::Core(error) => error.runtime_code(),
            Self::Action(error) => error.runtime_code(),
            Self::RetryExhausted { .. } => Some(Self::RETRY_EXHAUSTED_RUNTIME_CODE),
            Self::TaintViolation { .. } => None,
        }
    }
}

/// Retry policy for action invocations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Maximum number of attempts before giving up.
    pub max_attempts: u16,
    /// Base delay between attempts in milliseconds.
    pub base_delay_ms: u64,
    /// Whether to use exponential backoff.
    pub exponential_backoff: bool,
}

impl RetryPolicy {
    /// Policy that never retries.
    pub const NEVER: Self = Self {
        max_attempts: 1,
        base_delay_ms: 0,
        exponential_backoff: false,
    };

    /// Policy with up to 3 attempts and no backoff.
    pub const DEFAULT: Self = Self {
        max_attempts: 3,
        base_delay_ms: 100,
        exponential_backoff: false,
    };
}

/// Extended engine signal returned by the action-aware execution loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSignal {
    /// Deterministic execution can continue.
    Continue,
    /// Run finished with a result value.
    Finished(SlotValue),
    /// Step budget was exhausted before completion.
    StepBudgetExhausted,
    /// Run is awaiting action completion with the given ticket.
    AwaitingAction(ActionTicket),
    /// Run is awaiting a wait condition.
    AwaitingWait,
    /// Run is awaiting external input (ask).
    AwaitingAsk,
}

#[cfg(test)]
mod tests {
    use super::*;

    // =====================================================================
    // EvidenceCollector construction and basic operations
    // =====================================================================

    #[test]
    fn evidence_collector_new_is_empty() {
        let collector = EvidenceCollector::new();
        assert!(collector.is_empty());
        assert_eq!(collector.len(), 0);
    }

    #[test]
    fn evidence_collector_default_is_empty() {
        let collector = EvidenceCollector::default();
        assert!(collector.is_empty());
        assert_eq!(collector.len(), 0);
    }

    #[test]
    fn evidence_collector_push_step_started_increments_len() {
        let mut collector = EvidenceCollector::new();
        collector.push_step_started(StepIdx::new(0));
        assert!(!collector.is_empty());
        assert_eq!(collector.len(), 1);
    }

    #[test]
    fn evidence_collector_push_step_succeeded_increments_len() {
        let mut collector = EvidenceCollector::new();
        collector.push_step_succeeded(StepIdx::new(0), Some(SlotIdx::new(1)));
        assert_eq!(collector.len(), 1);
    }

    #[test]
    fn evidence_collector_push_step_succeeded_with_no_output() {
        let mut collector = EvidenceCollector::new();
        collector.push_step_succeeded(StepIdx::new(5), None);
        assert_eq!(collector.len(), 1);
    }

    // =====================================================================
    // EvidenceCollector drain
    // =====================================================================

    #[test]
    fn evidence_collector_drain_returns_all_events() {
        let mut collector = EvidenceCollector::new();
        collector.push_step_started(StepIdx::new(0));
        collector.push_step_succeeded(StepIdx::new(0), Some(SlotIdx::new(1)));
        collector.push_step_started(StepIdx::new(1));
        assert_eq!(collector.len(), 3);

        let events = collector.drain();
        assert_eq!(events.len(), 3);
        assert!(collector.is_empty());
    }

    #[test]
    fn evidence_collector_drain_leaves_empty() {
        let mut collector = EvidenceCollector::new();
        collector.push_step_started(StepIdx::new(0));
        let _ = collector.drain();
        assert_eq!(collector.len(), 0);
        assert!(collector.is_empty());
    }

    #[test]
    fn evidence_collector_double_drain_second_returns_nothing() {
        let mut collector = EvidenceCollector::new();
        collector.push_step_started(StepIdx::new(0));
        let first = collector.drain();
        assert_eq!(first.len(), 1);
        let second = collector.drain();
        assert!(second.is_empty());
    }

    #[test]
    fn evidence_collector_drain_empty_returns_nothing() {
        let mut collector = EvidenceCollector::new();
        let events = collector.drain();
        assert!(events.is_empty());
    }

    // =====================================================================
    // EvidenceCollector event content
    // =====================================================================

    #[test]
    fn evidence_collector_events_preserve_step_index() {
        let mut collector = EvidenceCollector::new();
        collector.push_step_started(StepIdx::new(42));
        let events = collector.drain();
        match events.first() {
            Some(EvidenceEvent::StepStarted { step }) => assert_eq!(*step, StepIdx::new(42)),
            other => {
                let msg = format!("expected StepStarted, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn evidence_collector_step_succeeded_preserves_output_slot() {
        let mut collector = EvidenceCollector::new();
        collector.push_step_succeeded(StepIdx::new(3), Some(SlotIdx::new(7)));
        let events = collector.drain();
        match events.first() {
            Some(EvidenceEvent::StepSucceeded { step, output }) => {
                assert_eq!(*step, StepIdx::new(3));
                assert_eq!(*output, Some(SlotIdx::new(7)));
            }
            other => {
                let msg = format!("expected StepSucceeded, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn evidence_collector_step_succeeded_none_output_for_boundary_nodes() {
        let mut collector = EvidenceCollector::new();
        collector.push_step_succeeded(StepIdx::new(1), None);
        let events = collector.drain();
        match events.first() {
            Some(EvidenceEvent::StepSucceeded { step, output }) => {
                assert_eq!(*step, StepIdx::new(1));
                assert_eq!(*output, None);
            }
            other => {
                let msg = format!("expected StepSucceeded, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn evidence_collector_events_maintain_insertion_order() {
        let mut collector = EvidenceCollector::new();
        collector.push_step_started(StepIdx::new(0));
        collector.push_step_succeeded(StepIdx::new(0), Some(SlotIdx::new(0)));
        collector.push_step_started(StepIdx::new(1));
        collector.push_step_succeeded(StepIdx::new(1), None);

        let events = collector.drain();
        assert_eq!(events.len(), 4);

        let step_0_start = &events[0];
        let step_0_succ = &events[1];
        let step_1_start = &events[2];
        let step_1_succ = &events[3];

        assert_eq!(
            *step_0_start,
            EvidenceEvent::StepStarted {
                step: StepIdx::new(0)
            }
        );
        assert_eq!(
            *step_0_succ,
            EvidenceEvent::StepSucceeded {
                step: StepIdx::new(0),
                output: Some(SlotIdx::new(0))
            }
        );
        assert_eq!(
            *step_1_start,
            EvidenceEvent::StepStarted {
                step: StepIdx::new(1)
            }
        );
        assert_eq!(
            *step_1_succ,
            EvidenceEvent::StepSucceeded {
                step: StepIdx::new(1),
                output: None
            }
        );
    }

    // =====================================================================
    // EvidenceCollector reuse after drain
    // =====================================================================

    #[test]
    fn evidence_collector_accepts_events_after_drain() {
        let mut collector = EvidenceCollector::new();
        collector.push_step_started(StepIdx::new(0));
        let _ = collector.drain();

        collector.push_step_started(StepIdx::new(1));
        collector.push_step_succeeded(StepIdx::new(1), Some(SlotIdx::new(2)));
        assert_eq!(collector.len(), 2);
    }

    // =====================================================================
    // EvidenceEvent equality and debug
    // =====================================================================

    #[test]
    fn evidence_event_step_started_equality() {
        let a = EvidenceEvent::StepStarted {
            step: StepIdx::new(5),
        };
        let b = EvidenceEvent::StepStarted {
            step: StepIdx::new(5),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn evidence_event_step_started_inequality_different_step() {
        let a = EvidenceEvent::StepStarted {
            step: StepIdx::new(1),
        };
        let b = EvidenceEvent::StepStarted {
            step: StepIdx::new(2),
        };
        assert_ne!(a, b);
    }

    #[test]
    fn evidence_event_step_succeeded_equality() {
        let a = EvidenceEvent::StepSucceeded {
            step: StepIdx::new(3),
            output: Some(SlotIdx::new(1)),
        };
        let b = EvidenceEvent::StepSucceeded {
            step: StepIdx::new(3),
            output: Some(SlotIdx::new(1)),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn evidence_event_step_succeeded_inequality_different_output() {
        let a = EvidenceEvent::StepSucceeded {
            step: StepIdx::new(3),
            output: Some(SlotIdx::new(1)),
        };
        let b = EvidenceEvent::StepSucceeded {
            step: StepIdx::new(3),
            output: None,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn evidence_event_different_variants_are_not_equal() {
        let started = EvidenceEvent::StepStarted {
            step: StepIdx::new(0),
        };
        let succeeded = EvidenceEvent::StepSucceeded {
            step: StepIdx::new(0),
            output: None,
        };
        assert_ne!(started, succeeded);
    }

    #[test]
    fn evidence_event_debug_format_contains_variant_name() {
        let started = EvidenceEvent::StepStarted {
            step: StepIdx::new(5),
        };
        let debug = format!("{started:?}");
        assert!(
            debug.contains("StepStarted"),
            "expected 'StepStarted' in '{debug}'"
        );

        let succeeded = EvidenceEvent::StepSucceeded {
            step: StepIdx::new(5),
            output: Some(SlotIdx::new(3)),
        };
        let debug = format!("{succeeded:?}");
        assert!(
            debug.contains("StepSucceeded"),
            "expected 'StepSucceeded' in '{debug}'"
        );
    }

    // =====================================================================
    // EvidenceEvent copy semantics
    // =====================================================================

    #[test]
    fn evidence_event_is_copy() {
        let event = EvidenceEvent::StepStarted {
            step: StepIdx::new(10),
        };
        let copy = event;
        assert_eq!(event, copy);
    }
}
