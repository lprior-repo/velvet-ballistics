#![forbid(unsafe_code)]

//! Type definitions for the runtime engine.
//!
//! Exports error types, retry policy, and signals.

use vb_core::action::{ActionError, ActionTicket};
use vb_core::errors::EngineError;
use vb_core::ids::{ActionId, SlotIdx, StepIdx};
use vb_core::value::SlotValue;

// Re-export evidence types so property_tests can import from this module
pub use super::evidence::{EvidenceCollector, EvidenceEvent};

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
    /// Collect evidence capacity exceeded during a slot write.
    #[error("collect evidence capacity exceeded for run {run_id:?} at slot {slot:?}")]
    CollectEvidenceCapacityExceeded {
        /// Run ID.
        run_id: vb_core::ids::RunId,
        /// Slot index.
        slot: SlotIdx,
        /// Capacity that was exceeded.
        capacity: usize,
        /// Current length when capacity was exceeded.
        len: usize,
        /// Required extra data description.
        required: &'static str,
    },
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
    /// Retry policy admitted zero attempts. The runtime drive refuses to
    /// dispatch because issuing an action ticket with `attempt = 1` and
    /// `capacity = 0` would violate the ticket invariant `attempt <= capacity`.
    /// See bead vb-tqn41 (RE-013).
    #[error("retry policy has zero max_attempts")]
    RetryZeroMaxAttempts,
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
    /// Runtime code for a rejected zero-attempt retry policy.
    pub const RETRY_ZERO_MAX_ATTEMPTS_RUNTIME_CODE: &str = "RETRY_ZERO_MAX_ATTEMPTS";

    /// Returns the stable section 17 runtime code when this error has a direct mapping.
    #[must_use]
    pub const fn runtime_code(&self) -> Option<&'static str> {
        match self {
            Self::Core(error) => error.runtime_code(),
            Self::Action(error) => error.runtime_code(),
            Self::CollectEvidenceCapacityExceeded { .. } => None,
            Self::RetryExhausted { .. } => Some(Self::RETRY_EXHAUSTED_RUNTIME_CODE),
            Self::TaintViolation { .. } => None,
            Self::BranchLimitExceeded { .. } => Some(Self::BRANCH_LIMIT_EXCEEDED_RUNTIME_CODE),
            Self::RetryZeroMaxAttempts => Some(Self::RETRY_ZERO_MAX_ATTEMPTS_RUNTIME_CODE),
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

    /// RE-013: returns `true` when the policy admits at least one attempt.
    /// The runtime drive refuses to dispatch a policy with
    /// `max_attempts == 0` because issuing an action ticket with
    /// `attempt = 1` and `capacity = 0` would violate the ticket
    /// invariant `attempt <= capacity`.
    #[must_use]
    pub const fn is_valid_for_dispatch(self) -> bool {
        self.max_attempts > 0
    }

    /// RE-013: fallible constructor that rejects policies admitting zero
    /// attempts. Use this when building a policy from external input.
    #[must_use]
    pub const fn try_new(
        max_attempts: u16,
        base_delay_ms: u64,
        exponential_backoff: bool,
    ) -> Option<Self> {
        if max_attempts == 0 {
            None
        } else {
            Some(Self {
                max_attempts,
                base_delay_ms,
                exponential_backoff,
            })
        }
    }
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
    /// Run is awaiting a wait condition. Carries the slot the wait primitive read its deadline from.
    AwaitingWait(vb_core::ids::SlotIdx),
    /// Run is awaiting an event. Carries the event slot the primitive
    /// validated and an optional timeout slot. The event slot MUST NOT
    /// be treated as a deadline: an event without a timeout has no
    /// deadline at all and only resumes on the event firing.
    AwaitingEvent {
        /// Slot the wait-event primitive read its event identifier from.
        event: vb_core::ids::SlotIdx,
        /// Optional timeout slot for the event wait.
        timeout_slot: Option<vb_core::ids::SlotIdx>,
    },
    /// Run is awaiting external input (ask). Carries the optional timeout slot.
    AwaitingAsk(Option<vb_core::ids::SlotIdx>),
}
