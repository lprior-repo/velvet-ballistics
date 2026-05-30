#![forbid(unsafe_code)]

//! Type definitions for the runtime engine.
//!
//! Exports evidence collection, error types, retry policy, and signals.

use vb_core::action::{ActionError, ActionTicket};
use vb_core::errors::EngineError;
use vb_core::ids::{ActionId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};

use crate::primitives::collect::CollectPaginationState;

const REQUIRED_COLLECT_SLOT_EXTRA: &str = "collect SlotWritten extra";

/// Evidence event emitted by the deterministic drive loop for each step.
///
/// These events are collected during `drive_deterministic_full` and drained
/// by the shard to emit to the journal and trace ring. This satisfies
/// the Phase 40/44 evidence chain requirement that every deterministic step
/// emits `StepStarted` before `SlotWritten`, followed by `StepSucceeded`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
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
    /// A slot was written during step execution.
    SlotWritten {
        /// Slot index.
        slot: SlotIdx,
        /// Value written to the slot.
        value: SlotValue,
        /// Taint written to the slot.
        taint: Taint,
        /// Optional frame extra data captured with the slot write.
        extra: Option<CollectPaginationState>,
    },
}

/// Default maximum number of evidence events before the collector drops
/// new events. Each step emits up to 3 events (Started + SlotWritten +
/// Succeeded), so 3 * step_budget provides a safe upper bound.
const DEFAULT_EVIDENCE_CAPACITY: usize = 3 * 1024;

/// Bounded collector for evidence events produced during a drive loop.
///
/// Collected and drained once per drive loop iteration by the shard
/// to emit StepStarted/StepSucceeded/SlotWritten events to the journal.
/// The collector enforces a capacity limit to prevent unbounded memory
/// growth from malicious or buggy workflows. When at capacity, new events
/// are silently dropped (the evidence chain becomes incomplete but the
/// system remains memory-safe).
#[derive(Debug, Clone)]
pub struct EvidenceCollector {
    events: Vec<EvidenceEvent>,
    capacity: usize,
    dropped: usize,
}

impl EvidenceCollector {
    /// Creates a new empty collector with a default capacity bound.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            capacity: DEFAULT_EVIDENCE_CAPACITY,
            dropped: 0,
        }
    }

    /// Creates a new collector with a specific capacity bound.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: Vec::new(),
            capacity,
            dropped: 0,
        }
    }

    /// Records a StepStarted event.
    /// Silently drops the event if the collector is at capacity.
    pub fn push_step_started(&mut self, step: StepIdx) {
        if self.events.len() < self.capacity {
            self.events.push(EvidenceEvent::StepStarted { step });
        } else {
            self.dropped = self.dropped.saturating_add(1);
        }
    }

    /// Records a StepSucceeded event.
    /// Silently drops the event if the collector is at capacity.
    pub fn push_step_succeeded(&mut self, step: StepIdx, output: Option<SlotIdx>) {
        if self.events.len() < self.capacity {
            self.events
                .push(EvidenceEvent::StepSucceeded { step, output });
        } else {
            self.dropped = self.dropped.saturating_add(1);
        }
    }

    /// Records a SlotWritten event.
    /// Silently drops the event if the collector is at capacity.
    pub fn push_slot_written(&mut self, slot: SlotIdx, value: SlotValue) {
        self.push_slot_written_with_taint(slot, value, Taint::Clean);
    }

    /// Records a SlotWritten event with explicit taint.
    /// Silently drops the event if the collector is at capacity.
    pub fn push_slot_written_with_taint(&mut self, slot: SlotIdx, value: SlotValue, taint: Taint) {
        if self.events.len() < self.capacity {
            self.events.push(EvidenceEvent::SlotWritten {
                slot,
                value,
                taint,
                extra: None,
            });
        } else {
            self.dropped = self.dropped.saturating_add(1);
        }
    }

    /// Records a SlotWritten event with frame extra data.
    pub fn push_slot_written_with_extra(
        &mut self,
        slot: SlotIdx,
        value: SlotValue,
        taint: Taint,
        extra: Option<CollectPaginationState>,
    ) -> Result<(), EngineError> {
        if let Some(state) = extra
            && self.events.len() >= self.capacity
        {
            return Err(EngineError::CollectEvidenceCapacityExceeded {
                run_id: state.run_id,
                slot,
                capacity: self.capacity,
                len: self.events.len(),
                required: REQUIRED_COLLECT_SLOT_EXTRA,
            });
        }
        if self.events.len() < self.capacity {
            self.events.push(EvidenceEvent::SlotWritten {
                slot,
                value,
                taint,
                extra,
            });
        } else {
            self.dropped = self.dropped.saturating_add(1);
        }
        Ok(())
    }

    /// Drains all collected events, returning them for processing.
    pub fn drain(&mut self) -> Vec<EvidenceEvent> {
        self.dropped = 0;
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

    /// Returns the number of events dropped due to capacity limits.
    #[must_use]
    pub fn dropped(&self) -> usize {
        self.dropped
    }

    /// Returns the configured capacity.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
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
#[non_exhaustive]
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
    /// TogetherStart branch count exceeds the u16 representation limit.
    #[error("branch count {requested} exceeds maximum {max}")]
    BranchLimitExceeded {
        /// Maximum representable branch count.
        max: usize,
        /// Requested branch count.
        requested: usize,
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
    pub const BRANCH_LIMIT_EXCEEDED_RUNTIME_CODE: &str = "BRANCH_LIMIT_EXCEEDED";

    /// Returns the stable section 17 runtime code when this error has a direct mapping.
    #[must_use]
    pub const fn runtime_code(&self) -> Option<&'static str> {
        match self {
            Self::Core(error) => error.runtime_code(),
            Self::Action(error) => error.runtime_code(),
            Self::RetryExhausted { .. } => Some(Self::RETRY_EXHAUSTED_RUNTIME_CODE),
            Self::TaintViolation { .. } => None,
            Self::BranchLimitExceeded { .. } => Some(Self::BRANCH_LIMIT_EXCEEDED_RUNTIME_CODE),
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
#[non_exhaustive]
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
        let drained = collector.drain();
        assert_eq!(drained.len(), 1);
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

        assert_eq!(
            events.as_slice(),
            &[
                EvidenceEvent::StepStarted {
                    step: StepIdx::new(0)
                },
                EvidenceEvent::StepSucceeded {
                    step: StepIdx::new(0),
                    output: Some(SlotIdx::new(0))
                },
                EvidenceEvent::StepStarted {
                    step: StepIdx::new(1)
                },
                EvidenceEvent::StepSucceeded {
                    step: StepIdx::new(1),
                    output: None
                }
            ]
        );
    }

    // =====================================================================
    // EvidenceCollector reuse after drain
    // =====================================================================

    #[test]
    fn evidence_collector_accepts_events_after_drain() {
        let mut collector = EvidenceCollector::new();
        collector.push_step_started(StepIdx::new(0));
        let drained = collector.drain();
        assert_eq!(drained.len(), 1);

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

    // =====================================================================
    // EvidenceCollector capacity bounding
    // =====================================================================

    #[test]
    fn evidence_collector_new_has_default_capacity() {
        let collector = EvidenceCollector::new();
        assert_eq!(collector.capacity(), 3 * 1024);
        assert_eq!(collector.dropped(), 0);
    }

    #[test]
    fn evidence_collector_with_capacity_sets_custom_capacity() {
        let collector = EvidenceCollector::with_capacity(10);
        assert_eq!(collector.capacity(), 10);
        assert_eq!(collector.dropped(), 0);
    }

    #[test]
    fn evidence_collector_drops_events_at_capacity() {
        let mut collector = EvidenceCollector::with_capacity(2);
        collector.push_step_started(StepIdx::new(0));
        collector.push_step_started(StepIdx::new(1));
        assert_eq!(collector.len(), 2);
        assert_eq!(collector.dropped(), 0);
        // Third event exceeds capacity.
        collector.push_step_started(StepIdx::new(2));
        assert_eq!(collector.len(), 2, "capacity should be respected");
        assert_eq!(collector.dropped(), 1, "dropped count should increment");
    }

    #[test]
    fn evidence_collector_drops_slot_written_at_capacity() {
        let mut collector = EvidenceCollector::with_capacity(1);
        collector.push_step_started(StepIdx::new(0));
        collector.push_slot_written(SlotIdx::new(0), SlotValue::I64(1));
        assert_eq!(collector.len(), 1);
        assert_eq!(collector.dropped(), 1);
    }

    #[test]
    fn evidence_collector_drops_step_succeeded_at_capacity() {
        let mut collector = EvidenceCollector::with_capacity(1);
        collector.push_step_started(StepIdx::new(0));
        collector.push_step_succeeded(StepIdx::new(0), None);
        assert_eq!(collector.len(), 1);
        assert_eq!(collector.dropped(), 1);
    }

    #[test]
    fn evidence_collector_drain_resets_dropped_counter() {
        let mut collector = EvidenceCollector::with_capacity(1);
        collector.push_step_started(StepIdx::new(0));
        collector.push_step_started(StepIdx::new(1)); // dropped
        assert_eq!(collector.dropped(), 1);
        let events = collector.drain();
        assert_eq!(events.len(), 1);
        assert_eq!(collector.dropped(), 0, "drain should reset dropped");
        assert!(collector.is_empty());
    }

    #[test]
    fn evidence_collector_dropped_saturates_on_many_drops() {
        let mut collector = EvidenceCollector::with_capacity(0);
        (0u16..10_000).for_each(|i| collector.push_step_started(StepIdx::new(i)));
        assert_eq!(collector.len(), 0, "zero capacity should hold nothing");
        assert_eq!(collector.dropped(), 10_000, "all events should be dropped");
    }

    // =====================================================================
    // RuntimeEngineError runtime codes
    // =====================================================================

    #[test]
    fn branch_limit_exceeded_has_runtime_code() {
        let error = RuntimeEngineError::BranchLimitExceeded {
            max: 65535,
            requested: 70000,
        };
        assert_eq!(
            error.runtime_code(),
            Some(RuntimeEngineError::BRANCH_LIMIT_EXCEEDED_RUNTIME_CODE)
        );
        assert_eq!(
            RuntimeEngineError::BRANCH_LIMIT_EXCEEDED_RUNTIME_CODE,
            "BRANCH_LIMIT_EXCEEDED"
        );
    }

    #[test]
    fn retry_exhausted_has_runtime_code() {
        let error = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(7),
            attempts: 3,
        };
        assert_eq!(
            error.runtime_code(),
            Some(RuntimeEngineError::RETRY_EXHAUSTED_RUNTIME_CODE)
        );
        assert_eq!(
            RuntimeEngineError::RETRY_EXHAUSTED_RUNTIME_CODE,
            "RETRY_EXHAUSTED"
        );
    }

    #[test]
    fn taint_violation_has_no_runtime_code() {
        let error = RuntimeEngineError::TaintViolation {
            step: StepIdx::new(5),
        };
        assert_eq!(error.runtime_code(), None);
    }

    #[test]
    fn retry_exhausted_display_contains_action_and_attempts() {
        let error = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(42),
            attempts: 5,
        };
        let msg = format!("{error}");
        assert!(
            msg.contains("42"),
            "display should mention action id: '{msg}'"
        );
        assert!(
            msg.contains("5"),
            "display should mention attempts: '{msg}'"
        );
    }

    #[test]
    fn taint_violation_display_contains_step() {
        let error = RuntimeEngineError::TaintViolation {
            step: StepIdx::new(9),
        };
        let msg = format!("{error}");
        assert!(
            msg.contains("taint violation"),
            "display should mention taint violation: '{msg}'"
        );
    }

    #[test]
    fn branch_limit_exceeded_display_contains_counts() {
        let error = RuntimeEngineError::BranchLimitExceeded {
            max: 100,
            requested: 200,
        };
        let msg = format!("{error}");
        assert!(msg.contains("100"), "display should contain max: '{msg}'");
        assert!(
            msg.contains("200"),
            "display should contain requested: '{msg}'"
        );
    }

    #[test]
    fn retry_exhausted_equality_same_fields() {
        let a = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(1),
            attempts: 3,
        };
        let b = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(1),
            attempts: 3,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn retry_exhausted_inequality_different_attempts() {
        let a = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(1),
            attempts: 3,
        };
        let b = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(1),
            attempts: 5,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn taint_violation_equality_same_step() {
        let a = RuntimeEngineError::TaintViolation {
            step: StepIdx::new(3),
        };
        let b = RuntimeEngineError::TaintViolation {
            step: StepIdx::new(3),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn taint_violation_inequality_different_step() {
        let a = RuntimeEngineError::TaintViolation {
            step: StepIdx::new(1),
        };
        let b = RuntimeEngineError::TaintViolation {
            step: StepIdx::new(2),
        };
        assert_ne!(a, b);
    }

    #[test]
    fn runtime_engine_error_variants_are_not_equal() {
        let core = RuntimeEngineError::Core(EngineError::DivisionByZero);
        let retry = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(0),
            attempts: 1,
        };
        let taint = RuntimeEngineError::TaintViolation {
            step: StepIdx::new(0),
        };
        let branch = RuntimeEngineError::BranchLimitExceeded {
            max: 1,
            requested: 2,
        };
        assert_ne!(core, retry);
        assert_ne!(core, taint);
        assert_ne!(core, branch);
        assert_ne!(retry, taint);
        assert_ne!(retry, branch);
        assert_ne!(taint, branch);
    }

    #[test]
    fn runtime_engine_error_clone_preserves_variant() {
        let original = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(10),
            attempts: 4,
        };
        let cloned = original.clone();
        assert_eq!(cloned, original);
    }

    #[test]
    fn runtime_engine_error_debug_contains_variant_name() {
        let retry = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(1),
            attempts: 2,
        };
        let debug = format!("{retry:?}");
        assert!(
            debug.contains("RetryExhausted"),
            "expected 'RetryExhausted' in '{debug}'"
        );
        let taint = RuntimeEngineError::TaintViolation {
            step: StepIdx::new(5),
        };
        let debug = format!("{taint:?}");
        assert!(
            debug.contains("TaintViolation"),
            "expected 'TaintViolation' in '{debug}'"
        );
    }

    // =====================================================================
    // EvidenceCollector: SlotWritten content and mixed events
    // =====================================================================

    #[test]
    fn evidence_collector_slot_written_preserves_slot_and_value() {
        let mut collector = EvidenceCollector::new();
        collector.push_slot_written(SlotIdx::new(3), SlotValue::I64(99));
        let events = collector.drain();
        match events.first() {
            Some(EvidenceEvent::SlotWritten { slot, value, .. }) => {
                assert_eq!(*slot, SlotIdx::new(3));
                assert_eq!(*value, SlotValue::I64(99));
            }
            other => {
                let msg = format!("expected SlotWritten, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn evidence_collector_slot_written_bool_value() {
        let mut collector = EvidenceCollector::new();
        collector.push_slot_written(SlotIdx::new(0), SlotValue::Bool(true));
        let events = collector.drain();
        match events.first() {
            Some(EvidenceEvent::SlotWritten { value, .. }) => {
                assert_eq!(*value, SlotValue::Bool(true))
            }
            other => {
                let msg = format!("expected SlotWritten, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn evidence_collector_mixed_events_in_order() {
        let mut collector = EvidenceCollector::new();
        collector.push_step_started(StepIdx::new(0));
        collector.push_slot_written(SlotIdx::new(0), SlotValue::I64(1));
        collector.push_step_succeeded(StepIdx::new(0), Some(SlotIdx::new(0)));
        collector.push_step_started(StepIdx::new(1));
        collector.push_step_succeeded(StepIdx::new(1), None);
        let events = collector.drain();
        assert_eq!(events.len(), 5);
        assert!(matches!(
            events.as_slice(),
            [
                EvidenceEvent::StepStarted { .. },
                EvidenceEvent::SlotWritten { .. },
                EvidenceEvent::StepSucceeded { .. },
                EvidenceEvent::StepStarted { .. },
                EvidenceEvent::StepSucceeded { .. }
            ]
        ));
    }

    #[test]
    fn evidence_collector_zero_capacity_drops_all() {
        let mut collector = EvidenceCollector::with_capacity(0);
        assert_eq!(collector.capacity(), 0);
        collector.push_step_started(StepIdx::new(0));
        assert_eq!(collector.len(), 0);
        assert_eq!(collector.dropped(), 1);
        collector.push_slot_written(SlotIdx::new(0), SlotValue::I64(1));
        assert_eq!(collector.dropped(), 2);
        collector.push_step_succeeded(StepIdx::new(0), None);
        assert_eq!(collector.dropped(), 3);
    }

    // =====================================================================
    // RetryPolicy constants and equality
    // =====================================================================

    #[test]
    fn retry_policy_never_has_one_attempt() {
        assert_eq!(RetryPolicy::NEVER.max_attempts, 1);
        assert_eq!(RetryPolicy::NEVER.base_delay_ms, 0);
        assert!(!RetryPolicy::NEVER.exponential_backoff);
    }

    #[test]
    fn retry_policy_default_has_three_attempts() {
        assert_eq!(RetryPolicy::DEFAULT.max_attempts, 3);
        assert_eq!(RetryPolicy::DEFAULT.base_delay_ms, 100);
        assert!(!RetryPolicy::DEFAULT.exponential_backoff);
    }

    #[test]
    fn retry_policy_equality() {
        let a = RetryPolicy::NEVER;
        let b = RetryPolicy {
            max_attempts: 1,
            base_delay_ms: 0,
            exponential_backoff: false,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn retry_policy_inequality_different_attempts() {
        let a = RetryPolicy::NEVER;
        let b = RetryPolicy {
            max_attempts: 2,
            base_delay_ms: 0,
            exponential_backoff: false,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn retry_policy_inequality_different_backoff() {
        let a = RetryPolicy::DEFAULT;
        let b = RetryPolicy {
            max_attempts: 3,
            base_delay_ms: 100,
            exponential_backoff: true,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn retry_policy_copy_semantics() {
        let a = RetryPolicy::DEFAULT;
        let b = a;
        assert_eq!(a, b);
    }

    // =====================================================================
    // RuntimeSignal coverage
    // =====================================================================

    #[test]
    fn runtime_signal_continue_is_not_finished() {
        assert_ne!(
            RuntimeSignal::Continue,
            RuntimeSignal::Finished(SlotValue::I64(0))
        );
    }

    #[test]
    fn runtime_signal_finished_equality_same_value() {
        assert_eq!(
            RuntimeSignal::Finished(SlotValue::I64(42)),
            RuntimeSignal::Finished(SlotValue::I64(42))
        );
    }

    #[test]
    fn runtime_signal_finished_inequality_different_value() {
        assert_ne!(
            RuntimeSignal::Finished(SlotValue::I64(1)),
            RuntimeSignal::Finished(SlotValue::I64(2))
        );
    }

    #[test]
    fn runtime_signal_all_variants_are_distinct() {
        let signals = [
            RuntimeSignal::Continue,
            RuntimeSignal::Finished(SlotValue::Null),
            RuntimeSignal::StepBudgetExhausted,
            RuntimeSignal::AwaitingWait,
            RuntimeSignal::AwaitingAsk,
        ];
        for (i, si) in signals.iter().enumerate() {
            for (j, sj) in signals.iter().enumerate() {
                if i != j {
                    assert_ne!(si, sj);
                }
            }
        }
    }

    #[test]
    fn runtime_signal_clone_preserves_value() {
        let original = RuntimeSignal::Finished(SlotValue::Bool(true));
        let cloned = original.clone();
        assert_eq!(cloned, original);
    }

    // vb-qi37.3 STATE 5 RED PHASE: required collect SlotWritten extras must
    // not be silently dropped when evidence capacity is full. The approved
    // plan requires a typed error surface; the current API returns unit, so
    // compile failure is intentional red evidence for State 6.
    #[test]
    fn collect_slot_extra_capacity_full_returns_capacity_error_not_silent_drop() {
        let run_id = vb_core::ids::RunId::new(4101);
        let collector = SlotIdx::new(1);
        let page = vb_core::ids::ListId::new(7);
        let expected_state = crate::primitives::collect::CollectPaginationState {
            run_id,
            collector_slot: collector,
            source: vb_core::ids::ListId::new(3),
            current_page: page,
            cursor: 1,
            page_size: 1,
            item_count: 2,
            limit: 2,
            time_limit_ms: None,
            start_millis: 10,
        };
        let mut evidence = EvidenceCollector::with_capacity(2);
        evidence.push_step_started(StepIdx::ZERO);
        evidence.push_step_succeeded(StepIdx::ZERO, None);

        let result = evidence.push_slot_written_with_extra(
            collector,
            SlotValue::List(page),
            Taint::Clean,
            Some(expected_state),
        );

        assert_eq!(
            result,
            Err(EngineError::CollectEvidenceCapacityExceeded {
                run_id,
                slot: collector,
                capacity: 2,
                len: 2,
                required: "collect SlotWritten extra",
            })
        );
        assert_eq!(
            evidence.drain(),
            vec![
                EvidenceEvent::StepStarted {
                    step: StepIdx::ZERO,
                },
                EvidenceEvent::StepSucceeded {
                    step: StepIdx::ZERO,
                    output: None,
                },
            ]
        );
    }

    #[test]
    fn collect_slot_extra_capacity_zero_returns_capacity_error_before_success() {
        let run_id = vb_core::ids::RunId::new(4102);
        let collector = SlotIdx::new(1);
        let page = vb_core::ids::ListId::new(8);
        let expected_state = crate::primitives::collect::CollectPaginationState {
            run_id,
            collector_slot: collector,
            source: vb_core::ids::ListId::new(3),
            current_page: page,
            cursor: 1,
            page_size: 1,
            item_count: 2,
            limit: 2,
            time_limit_ms: None,
            start_millis: 10,
        };
        let mut evidence = EvidenceCollector::with_capacity(0);

        let result = evidence.push_slot_written_with_extra(
            collector,
            SlotValue::List(page),
            Taint::Clean,
            Some(expected_state),
        );

        assert_eq!(
            result,
            Err(EngineError::CollectEvidenceCapacityExceeded {
                run_id,
                slot: collector,
                capacity: 0,
                len: 0,
                required: "collect SlotWritten extra",
            })
        );
        assert_eq!(evidence.drain(), vec![]);
    }

    #[test]
    fn collect_slot_extra_capacity_one_returns_capacity_error_and_preserves_existing_evidence() {
        let run_id = vb_core::ids::RunId::new(4103);
        let collector = SlotIdx::new(1);
        let page = vb_core::ids::ListId::new(9);
        let expected_state = crate::primitives::collect::CollectPaginationState {
            run_id,
            collector_slot: collector,
            source: vb_core::ids::ListId::new(3),
            current_page: page,
            cursor: 1,
            page_size: 1,
            item_count: 2,
            limit: 2,
            time_limit_ms: None,
            start_millis: 10,
        };
        let mut evidence = EvidenceCollector::with_capacity(1);
        evidence.push_step_started(StepIdx::ZERO);

        let result = evidence.push_slot_written_with_extra(
            collector,
            SlotValue::List(page),
            Taint::Clean,
            Some(expected_state),
        );

        assert_eq!(
            result,
            Err(EngineError::CollectEvidenceCapacityExceeded {
                run_id,
                slot: collector,
                capacity: 1,
                len: 1,
                required: "collect SlotWritten extra",
            })
        );
        assert_eq!(
            evidence.drain(),
            vec![EvidenceEvent::StepStarted {
                step: StepIdx::ZERO,
            }]
        );
    }
}
