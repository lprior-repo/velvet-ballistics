#![forbid(unsafe_code)]

//! Type definitions for the runtime engine.
//!
//! Exports evidence collection, error types, retry policy, and signals.

use vb_core::action::{
    ActionContract, ActionError, ActionFailureCode, ActionOutcome, ActionTicket, Idempotency,
    propagate_action_taint,
};
use vb_core::errors::EngineError;
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};

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
