#![forbid(unsafe_code)]

//! Signal and error types for the runtime engine.

use vb_core::action::{
    propagate_action_taint, ActionContract, ActionError, ActionFailure, ActionFailureCode,
    ActionOutcome, ActionTicket, Idempotency,
};
use vb_core::errors::EngineError;
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow};

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
    use vb_core::action::ActionTicket;
    use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
    use vb_core::value::SlotValue;

    // =====================================================================
    // RuntimeEngineError construction and equality
    // =====================================================================

    #[test]
    fn runtime_engine_error_core_wraps_engine_error() {
        let core_err = EngineError::DivisionByZero;
        let err = RuntimeEngineError::Core(core_err.clone());
        assert_eq!(err, RuntimeEngineError::Core(EngineError::DivisionByZero));
    }

    #[test]
    fn runtime_engine_error_action_wraps_action_error() {
        let action_err = ActionError::InvalidTicket;
        let err = RuntimeEngineError::Action(action_err.clone());
        assert_eq!(err, RuntimeEngineError::Action(ActionError::InvalidTicket));
    }

    #[test]
    fn runtime_engine_error_retry_exhausted_fields() {
        let err = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(42),
            attempts: 7,
        };
        match err {
            RuntimeEngineError::RetryExhausted { action, attempts } => {
                assert_eq!(action, ActionId::new(42));
                assert_eq!(attempts, 7);
            }
            other => {
                let msg = format!("expected RetryExhausted, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn runtime_engine_error_taint_violation_fields() {
        let err = RuntimeEngineError::TaintViolation {
            step: StepIdx::new(99),
        };
        match err {
            RuntimeEngineError::TaintViolation { step } => {
                assert_eq!(step, StepIdx::new(99));
            }
            other => {
                let msg = format!("expected TaintViolation, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    // =====================================================================
    // From<EngineError> conversion
    // =====================================================================

    #[test]
    fn from_engine_error_creates_core_variant() {
        let core = EngineError::StepBudgetExhausted;
        let runtime: RuntimeEngineError = core.into();
        assert_eq!(
            runtime,
            RuntimeEngineError::Core(EngineError::StepBudgetExhausted)
        );
    }

    #[test]
    fn from_engine_error_preserves_slot_out_of_bounds() {
        let core = EngineError::SlotOutOfBounds {
            slot: vb_core::ids::SlotIdx::new(10),
        };
        let runtime: RuntimeEngineError = core.into();
        match runtime {
            RuntimeEngineError::Core(EngineError::SlotOutOfBounds { slot }) => {
                assert_eq!(slot, vb_core::ids::SlotIdx::new(10));
            }
            other => {
                let msg = format!("expected Core(SlotOutOfBounds), got {other:?}");
                panic!("{msg}");
            }
        }
    }

    // =====================================================================
    // From<ActionError> conversion
    // =====================================================================

    #[test]
    fn from_action_error_creates_action_variant() {
        let action = ActionError::QueueFull;
        let runtime: RuntimeEngineError = action.into();
        assert_eq!(runtime, RuntimeEngineError::Action(ActionError::QueueFull));
    }

    #[test]
    fn from_action_error_preserves_unknown_action_id() {
        let action = ActionError::UnknownAction {
            action: ActionId::new(255),
        };
        let runtime: RuntimeEngineError = action.into();
        match runtime {
            RuntimeEngineError::Action(ActionError::UnknownAction { action }) => {
                assert_eq!(action, ActionId::new(255));
            }
            other => {
                let msg = format!("expected Action(UnknownAction), got {other:?}");
                panic!("{msg}");
            }
        }
    }

    // =====================================================================
    // runtime_code
    // =====================================================================

    #[test]
    fn runtime_code_core_error_delegates() {
        let err = RuntimeEngineError::Core(EngineError::QueueFull);
        assert_eq!(err.runtime_code(), Some("QUEUE_FULL"));
    }

    #[test]
    fn runtime_code_core_error_returns_none_for_unmapped() {
        let err = RuntimeEngineError::Core(EngineError::DivisionByZero);
        assert_eq!(err.runtime_code(), None);
    }

    #[test]
    fn runtime_code_action_error_delegates() {
        let err = RuntimeEngineError::Action(ActionError::UnknownAction {
            action: ActionId::new(1),
        });
        assert_eq!(err.runtime_code(), Some("REFERENCE_MISSING"));
    }

    #[test]
    fn runtime_code_action_error_returns_none_for_unmapped() {
        let err = RuntimeEngineError::Action(ActionError::InvalidTicket);
        assert_eq!(err.runtime_code(), None);
    }

    #[test]
    fn runtime_code_retry_exhausted_returns_constant() {
        let err = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(1),
            attempts: 3,
        };
        assert_eq!(
            err.runtime_code(),
            Some(RuntimeEngineError::RETRY_EXHAUSTED_RUNTIME_CODE)
        );
        assert_eq!(err.runtime_code(), Some("RETRY_EXHAUSTED"));
    }

    #[test]
    fn runtime_code_taint_violation_returns_none() {
        let err = RuntimeEngineError::TaintViolation {
            step: StepIdx::new(1),
        };
        assert_eq!(err.runtime_code(), None);
    }

    #[test]
    fn retry_exhausted_runtime_code_constant_value() {
        assert_eq!(
            RuntimeEngineError::RETRY_EXHAUSTED_RUNTIME_CODE,
            "RETRY_EXHAUSTED"
        );
    }

    // =====================================================================
    // RetryPolicy constants
    // =====================================================================

    #[test]
    fn retry_policy_never_constants() {
        assert_eq!(RetryPolicy::NEVER.max_attempts, 1);
        assert_eq!(RetryPolicy::NEVER.base_delay_ms, 0);
        assert_eq!(RetryPolicy::NEVER.exponential_backoff, false);
    }

    #[test]
    fn retry_policy_default_constants() {
        assert_eq!(RetryPolicy::DEFAULT.max_attempts, 3);
        assert_eq!(RetryPolicy::DEFAULT.base_delay_ms, 100);
        assert_eq!(RetryPolicy::DEFAULT.exponential_backoff, false);
    }

    #[test]
    fn retry_policy_custom_construction() {
        let policy = RetryPolicy {
            max_attempts: 10,
            base_delay_ms: 500,
            exponential_backoff: true,
        };
        assert_eq!(policy.max_attempts, 10);
        assert_eq!(policy.base_delay_ms, 500);
        assert_eq!(policy.exponential_backoff, true);
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
    fn retry_policy_inequality() {
        assert_ne!(RetryPolicy::NEVER, RetryPolicy::DEFAULT);
    }

    #[test]
    fn retry_policy_clone() {
        let policy = RetryPolicy::DEFAULT;
        let cloned = policy.clone();
        assert_eq!(policy, cloned);
    }

    #[test]
    fn retry_policy_copy_semantics() {
        let a = RetryPolicy::DEFAULT;
        let b = a;
        assert_eq!(a, b);
    }

    // =====================================================================
    // RuntimeSignal construction and equality
    // =====================================================================

    #[test]
    fn runtime_signal_continue_equality() {
        assert_eq!(RuntimeSignal::Continue, RuntimeSignal::Continue);
    }

    #[test]
    fn runtime_signal_step_budget_exhausted_equality() {
        assert_eq!(
            RuntimeSignal::StepBudgetExhausted,
            RuntimeSignal::StepBudgetExhausted
        );
    }

    #[test]
    fn runtime_signal_awaiting_wait_equality() {
        assert_eq!(RuntimeSignal::AwaitingWait, RuntimeSignal::AwaitingWait);
    }

    #[test]
    fn runtime_signal_awaiting_ask_equality() {
        assert_eq!(RuntimeSignal::AwaitingAsk, RuntimeSignal::AwaitingAsk);
    }

    #[test]
    fn runtime_signal_awaiting_wait_differs_from_awaiting_ask() {
        assert_ne!(RuntimeSignal::AwaitingWait, RuntimeSignal::AwaitingAsk);
    }

    #[test]
    fn runtime_signal_continue_differs_from_exhausted() {
        assert_ne!(RuntimeSignal::Continue, RuntimeSignal::StepBudgetExhausted);
    }

    #[test]
    fn runtime_signal_finished_with_null() {
        let signal = RuntimeSignal::Finished(SlotValue::Null);
        match signal {
            RuntimeSignal::Finished(SlotValue::Null) => {}
            other => {
                let msg = format!("expected Finished(Null), got {other:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn runtime_signal_finished_with_bool() {
        let signal = RuntimeSignal::Finished(SlotValue::Bool(true));
        match signal {
            RuntimeSignal::Finished(SlotValue::Bool(v)) => assert!(v),
            other => {
                let msg = format!("expected Finished(Bool(true)), got {other:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn runtime_signal_finished_with_i64() {
        let signal = RuntimeSignal::Finished(SlotValue::I64(42));
        match signal {
            RuntimeSignal::Finished(SlotValue::I64(v)) => assert_eq!(v, 42),
            other => {
                let msg = format!("expected Finished(I64(42)), got {other:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn runtime_signal_finished_equality_depends_on_value() {
        let a = RuntimeSignal::Finished(SlotValue::I64(1));
        let b = RuntimeSignal::Finished(SlotValue::I64(1));
        let c = RuntimeSignal::Finished(SlotValue::I64(2));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn runtime_signal_awaiting_action_equality_depends_on_ticket() {
        let ticket = ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(2),
            seq: SeqNo::new(3),
            action: ActionId::new(4),
            attempt: 1,
            idempotency_key: 99,
        };
        let a = RuntimeSignal::AwaitingAction(ticket);
        let b = RuntimeSignal::AwaitingAction(ticket);
        assert_eq!(a, b);
    }

    #[test]
    fn runtime_signal_awaiting_action_differs_for_different_run() {
        let a = RuntimeSignal::AwaitingAction(ActionTicket {
            run: RunId::new(1),
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(0),
            attempt: 1,
            idempotency_key: 0,
        });
        let b = RuntimeSignal::AwaitingAction(ActionTicket {
            run: RunId::new(2),
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(0),
            attempt: 1,
            idempotency_key: 0,
        });
        assert_ne!(a, b);
    }

    #[test]
    fn runtime_signal_awaiting_action_differs_for_different_attempt() {
        let a = RuntimeSignal::AwaitingAction(ActionTicket {
            run: RunId::new(1),
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(0),
            attempt: 1,
            idempotency_key: 0,
        });
        let b = RuntimeSignal::AwaitingAction(ActionTicket {
            run: RunId::new(1),
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(0),
            attempt: 2,
            idempotency_key: 0,
        });
        assert_ne!(a, b);
    }

    // =====================================================================
    // RuntimeEngineError display (thiserror)
    // =====================================================================

    #[test]
    fn runtime_engine_error_display_retry_exhausted() {
        let err = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(5),
            attempts: 3,
        };
        let msg = err.to_string();
        assert!(
            msg.contains("retry exhausted"),
            "expected 'retry exhausted' in '{msg}'"
        );
        assert!(msg.contains("3"), "expected attempt count in '{msg}'");
    }

    #[test]
    fn runtime_engine_error_display_taint_violation() {
        let err = RuntimeEngineError::TaintViolation {
            step: StepIdx::new(7),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("taint violation"),
            "expected 'taint violation' in '{msg}'"
        );
    }

    #[test]
    fn runtime_engine_error_display_core() {
        let err = RuntimeEngineError::Core(EngineError::DivisionByZero);
        let msg = err.to_string();
        assert!(
            msg.contains("division by zero"),
            "expected 'division by zero' in '{msg}'"
        );
    }

    #[test]
    fn runtime_engine_error_display_action() {
        let err = RuntimeEngineError::Action(ActionError::QueueFull);
        let msg = err.to_string();
        assert!(
            msg.contains("queue full"),
            "expected 'queue full' in '{msg}'"
        );
    }

    // =====================================================================
    // RuntimeEngineError variant distinctness
    // =====================================================================

    #[test]
    fn runtime_engine_error_variants_are_distinct() {
        let errors = [
            RuntimeEngineError::Core(EngineError::DivisionByZero),
            RuntimeEngineError::Action(ActionError::InvalidTicket),
            RuntimeEngineError::RetryExhausted {
                action: ActionId::new(1),
                attempts: 1,
            },
            RuntimeEngineError::TaintViolation {
                step: StepIdx::new(1),
            },
        ];
        for (i, a) in errors.iter().enumerate() {
            for (j, b) in errors.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    // =====================================================================
    // RuntimeEngineResult alias
    // =====================================================================

    #[test]
    fn runtime_engine_result_ok_value() {
        let result: RuntimeEngineResult<SlotValue> = Ok(SlotValue::I64(10));
        match result {
            Ok(v) => assert_eq!(v, SlotValue::I64(10)),
            Err(e) => {
                let msg = format!("expected Ok, got Err({e:?})");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn runtime_engine_result_err_value() {
        let result: RuntimeEngineResult<SlotValue> = Err(RuntimeEngineError::Core(
            EngineError::DivisionByZero,
        ));
        match result {
            Err(RuntimeEngineError::Core(EngineError::DivisionByZero)) => {}
            other => {
                let msg = format!("expected Err(Core(DivisionByZero)), got {other:?}");
                panic!("{msg}");
            }
        }
    }

    // =====================================================================
    // Debug format coverage
    // =====================================================================

    #[test]
    fn runtime_signal_debug_formats() {
        let debug = format!("{:?}", RuntimeSignal::Continue);
        assert!(debug.contains("Continue"), "expected 'Continue' in '{debug}'");

        let debug = format!("{:?}", RuntimeSignal::StepBudgetExhausted);
        assert!(
            debug.contains("StepBudgetExhausted"),
            "expected 'StepBudgetExhausted' in '{debug}'"
        );

        let debug = format!("{:?}", RuntimeSignal::AwaitingWait);
        assert!(
            debug.contains("AwaitingWait"),
            "expected 'AwaitingWait' in '{debug}'"
        );

        let debug = format!("{:?}", RuntimeSignal::AwaitingAsk);
        assert!(
            debug.contains("AwaitingAsk"),
            "expected 'AwaitingAsk' in '{debug}'"
        );
    }

    #[test]
    fn retry_policy_debug_formats() {
        let debug = format!("{:?}", RetryPolicy::NEVER);
        assert!(debug.contains("RetryPolicy"), "expected 'RetryPolicy' in '{debug}'");
        assert!(debug.contains("1"), "expected max_attempts in '{debug}'");
    }

    #[test]
    fn runtime_engine_error_debug_formats() {
        let debug = format!(
            "{:?}",
            RuntimeEngineError::RetryExhausted {
                action: ActionId::new(1),
                attempts: 2,
            }
        );
        assert!(
            debug.contains("RetryExhausted"),
            "expected 'RetryExhausted' in '{debug}'"
        );
    }

    // =====================================================================
    // Edge case: zero-value and boundary fields
    // =====================================================================

    #[test]
    fn retry_policy_zero_base_delay_is_valid() {
        let policy = RetryPolicy {
            max_attempts: 5,
            base_delay_ms: 0,
            exponential_backoff: false,
        };
        assert_eq!(policy.base_delay_ms, 0);
    }

    #[test]
    fn runtime_engine_error_retry_exhausted_with_zero_attempts() {
        let err = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(0),
            attempts: 0,
        };
        match err {
            RuntimeEngineError::RetryExhausted { attempts, .. } => {
                assert_eq!(attempts, 0);
            }
            other => {
                let msg = format!("expected RetryExhausted, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn runtime_engine_error_taint_violation_with_zero_step() {
        let err = RuntimeEngineError::TaintViolation {
            step: StepIdx::ZERO,
        };
        match err {
            RuntimeEngineError::TaintViolation { step } => {
                assert_eq!(step, StepIdx::ZERO);
            }
            other => {
                let msg = format!("expected TaintViolation, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn runtime_signal_finished_with_all_slot_value_kinds() {
        let s = RuntimeSignal::Finished(SlotValue::Null);
        assert_eq!(s, RuntimeSignal::Finished(SlotValue::Null));

        let s = RuntimeSignal::Finished(SlotValue::Bool(false));
        assert_eq!(s, RuntimeSignal::Finished(SlotValue::Bool(false)));

        let s = RuntimeSignal::Finished(SlotValue::I64(0));
        assert_eq!(s, RuntimeSignal::Finished(SlotValue::I64(0)));
    }

    #[test]
    fn runtime_engine_error_clone_preserves_equality() {
        let err = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(3),
            attempts: 2,
        };
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }
}
