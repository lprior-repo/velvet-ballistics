#![forbid(unsafe_code)]
//! Engine signal types and step budget.

use crate::errors::EngineError;
use crate::limits::MAX_STEP_BUDGET;
use crate::value::{SlotValue, Taint};

/// Bounded number of steps a caller may execute in one engine slice.
///
/// The hard ceiling is [`MAX_STEP_BUDGET`]. Any value provided to [`StepBudget::new`]
/// that exceeds this ceiling is clamped, and [`StepBudget::try_take`] returns an
/// error if the internal counter somehow exceeds the ceiling.
#[derive(Debug)]
pub struct StepBudget {
    remaining: u64,
}

impl StepBudget {
    /// Largest bounded execution slice representable by the runtime.
    pub const MAX: Self = Self {
        remaining: MAX_STEP_BUDGET,
    };

    /// Creates a budget clamped to the hard ceiling. Zero is valid and
    /// executes no transitions.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self {
            remaining: if value > MAX_STEP_BUDGET {
                MAX_STEP_BUDGET
            } else {
                value
            },
        }
    }

    /// Attempts to consume one transition from the budget.
    ///
    /// Returns `Ok(true)` when a transition was consumed, `Ok(false)` when the
    /// budget is exhausted, and an error if the remaining counter somehow
    /// exceeds the hard ceiling (a runtime invariant violation).
    ///
    /// # Invariant guarantee
    ///
    /// The `remaining > MAX_STEP_BUDGET` overflow check is kept as a defense-in-depth
    /// guard. The field is private and can only be set by [`Self::new`] (which clamps)
    /// or [`Self::MAX`] (which uses `MAX_STEP_BUDGET` directly), and `try_take` only
    /// ever decreases via `saturating_sub`. The check therefore cannot fire through
    /// normal API use, but guards against future modifications or memory corruption.
    pub fn try_take(&mut self) -> Result<bool, EngineError> {
        if self.remaining > MAX_STEP_BUDGET {
            return Err(EngineError::StepCounterOverflow);
        }
        if self.remaining == 0 {
            Ok(false)
        } else {
            self.remaining = self.remaining.saturating_sub(1);
            Ok(true)
        }
    }

    /// Remaining transitions.
    #[must_use]
    pub const fn remaining(&self) -> u64 {
        self.remaining
    }

    /// Environment variable name for bench latency budget.
    const BENCH_LATENCY_BUDGET_US: &'static str = "VB_BENCH_LATENCY_BUDGET_US";

    /// Default budget when env var is absent.
    const DEFAULT_BUDGET: u64 = MAX_STEP_BUDGET;

    /// Creates a budget from the VB_BENCH_LATENCY_BUDGET_US environment variable.
    ///
    /// - Returns `Ok(budget)` when env var is set and parses as u64.
    ///   Value is clamped to [`MAX_STEP_BUDGET`].
    /// - Returns `Err(EngineError::BudgetParse { reason })` when env var is set
    ///   but contains non-numeric content.
    /// - Returns `Ok(Self::new(Self::DEFAULT_BUDGET))` when env var is absent.
    pub fn from_env() -> Result<Self, EngineError> {
        match std::env::var(Self::BENCH_LATENCY_BUDGET_US) {
            Ok(raw) => {
                let parsed = raw.parse::<u64>().map_err(|_| EngineError::BudgetParse {
                    reason: "invalid u64 value",
                })?;
                Ok(Self::new(parsed))
            }
            Err(std::env::VarError::NotPresent) => Ok(Self::new(Self::DEFAULT_BUDGET)),
            Err(_) => Err(EngineError::BudgetParse {
                reason: "env var access error",
            }),
        }
    }
}

/// Outcome of one or more engine transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineSignal {
    /// The run made progress and can continue immediately.
    Continue,
    /// The run finished with a result value and its taint level.
    Finished(SlotValue, Taint),
    /// The caller's execution slice ended before completion.
    StepBudgetExhausted,
    /// The run suspended on an action.
    AwaitingAction,
    /// The run suspended on wait.
    AwaitingWait,
    /// The run suspended on ask.
    AwaitingAsk,
}

#[cfg(test)]
mod tests {
    use super::{EngineSignal, StepBudget};
    use crate::limits::MAX_STEP_BUDGET;
    use crate::value::{SlotValue, Taint};

    fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
    where
        T: core::fmt::Debug + PartialEq,
    {
        if actual == expected {
            Ok(())
        } else {
            Err(format!("expected {expected:?}, found {actual:?}"))
        }
    }

    // ===== StepBudget tests =====

    #[test]
    fn budget_new_clamps_to_max_step_budget() -> Result<(), String> {
        let over = StepBudget::new(MAX_STEP_BUDGET + 100);
        ensure_equal(over.remaining(), MAX_STEP_BUDGET)
    }

    #[test]
    fn budget_new_zero_is_valid() -> Result<(), String> {
        let zero = StepBudget::new(0);
        ensure_equal(zero.remaining(), 0)
    }

    #[test]
    fn budget_max_equals_max_step_budget_constant() -> Result<(), String> {
        ensure_equal(StepBudget::MAX.remaining(), MAX_STEP_BUDGET)
    }

    #[test]
    fn budget_try_take_decrements_remaining() -> Result<(), String> {
        let mut b = StepBudget::new(3);
        ensure_equal(b.remaining(), 3)?;
        b.try_take().map_err(|e| e.to_string())?;
        ensure_equal(b.remaining(), 2)?;
        b.try_take().map_err(|e| e.to_string())?;
        ensure_equal(b.remaining(), 1)?;
        b.try_take().map_err(|e| e.to_string())?;
        ensure_equal(b.remaining(), 0)?;
        let taken = b.try_take().map_err(|e| e.to_string())?;
        ensure_equal(taken, false)?;
        ensure_equal(b.remaining(), 0)
    }

    #[test]
    fn budget_try_take_returns_true_while_available() -> Result<(), String> {
        let mut b = StepBudget::new(1);
        ensure_equal(b.try_take().map_err(|e| e.to_string())?, true)?;
        ensure_equal(b.try_take().map_err(|e| e.to_string())?, false)
    }

    #[test]
    fn budget_new_accepts_exact_max() -> Result<(), String> {
        let b = StepBudget::new(MAX_STEP_BUDGET);
        ensure_equal(b.remaining(), MAX_STEP_BUDGET)
    }

    // ===== EngineSignal variant tests =====

    #[test]
    fn engine_signal_continue_is_not_terminal() -> Result<(), String> {
        let signal = EngineSignal::Continue;
        ensure_equal(matches!(signal, EngineSignal::Continue), true)
    }

    #[test]
    fn engine_signal_finished_carries_value_and_taint() -> Result<(), String> {
        let signal = EngineSignal::Finished(SlotValue::I64(42), Taint::Clean);
        if let EngineSignal::Finished(value, taint) = signal {
            ensure_equal(value, SlotValue::I64(42))?;
            ensure_equal(taint, Taint::Clean)
        } else {
            Err("expected Finished variant".into())
        }
    }

    #[test]
    fn engine_signal_finished_with_secret_taint() -> Result<(), String> {
        let signal = EngineSignal::Finished(SlotValue::Bool(true), Taint::Secret);
        if let EngineSignal::Finished(_, taint) = signal {
            ensure_equal(taint, Taint::Secret)
        } else {
            Err("expected Finished variant".into())
        }
    }

    #[test]
    fn engine_signal_variants_are_distinct() -> Result<(), String> {
        let signals: Vec<EngineSignal> = vec![
            EngineSignal::Continue,
            EngineSignal::Finished(SlotValue::Null, Taint::Clean),
            EngineSignal::StepBudgetExhausted,
            EngineSignal::AwaitingAction,
            EngineSignal::AwaitingWait,
            EngineSignal::AwaitingAsk,
        ];
        for (i, sig) in signals.iter().enumerate() {
            for (j, other) in signals.iter().enumerate() {
                if i == j {
                    ensure_equal(sig == other, true)?;
                } else {
                    ensure_equal(sig == other, false)?;
                }
            }
        }
        Ok(())
    }

    #[test]
    fn engine_signal_debug_format_contains_variant_name() -> Result<(), String> {
        let debug_str = format!("{:?}", EngineSignal::AwaitingAction);
        ensure_equal(debug_str.contains("AwaitingAction"), true)?;
        let debug_str = format!("{:?}", EngineSignal::StepBudgetExhausted);
        ensure_equal(debug_str.contains("StepBudgetExhausted"), true)
    }

    // ===== Additional StepBudget coverage =====

    #[test]
    fn budget_new_one_consumes_single_transition() -> Result<(), String> {
        let mut b = StepBudget::new(1);
        ensure_equal(b.remaining(), 1)?;
        let taken = b.try_take().map_err(|e| e.to_string())?;
        ensure_equal(taken, true)?;
        ensure_equal(b.remaining(), 0)?;
        let taken_again = b.try_take().map_err(|e| e.to_string())?;
        ensure_equal(taken_again, false)?;
        ensure_equal(b.remaining(), 0)
    }

    #[test]
    fn budget_zero_never_returns_true() -> Result<(), String> {
        let mut b = StepBudget::new(0);
        ensure_equal(b.remaining(), 0)?;
        let taken = b.try_take().map_err(|e| e.to_string())?;
        ensure_equal(taken, false)?;
        // Second take on zero budget is still false, not an error
        let taken2 = b.try_take().map_err(|e| e.to_string())?;
        ensure_equal(taken2, false)?;
        ensure_equal(b.remaining(), 0)
    }

    #[test]
    fn budget_max_decrements_by_one() -> Result<(), String> {
        let mut b = StepBudget::MAX;
        let initial = b.remaining();
        ensure_equal(initial, MAX_STEP_BUDGET)?;
        b.try_take().map_err(|e| e.to_string())?;
        ensure_equal(b.remaining(), MAX_STEP_BUDGET.saturating_sub(1))
    }

    #[test]
    fn budget_clamps_u64_max_to_max_step_budget() -> Result<(), String> {
        let b = StepBudget::new(u64::MAX);
        ensure_equal(b.remaining(), MAX_STEP_BUDGET)
    }

    #[test]
    fn budget_remaining_never_increases() -> Result<(), String> {
        let mut b = StepBudget::new(5);
        let mut prev = b.remaining();
        for _ in 0..5 {
            b.try_take().map_err(|e| e.to_string())?;
            let cur = b.remaining();
            if cur > prev {
                return Err(format!("remaining increased from {prev} to {cur}"));
            }
            prev = cur;
        }
        Ok(())
    }

    #[test]
    fn budget_exhaustion_is_stable_after_zero() -> Result<(), String> {
        let mut b = StepBudget::new(2);
        b.try_take().map_err(|e| e.to_string())?;
        b.try_take().map_err(|e| e.to_string())?;
        ensure_equal(b.remaining(), 0)?;
        // Repeated takes on exhausted budget remain false
        for _ in 0..10 {
            let taken = b.try_take().map_err(|e| e.to_string())?;
            ensure_equal(taken, false)?;
            ensure_equal(b.remaining(), 0)?;
        }
        Ok(())
    }

    // ===== Additional EngineSignal coverage =====

    #[test]
    fn engine_signal_finished_with_null_value() -> Result<(), String> {
        let signal = EngineSignal::Finished(SlotValue::Null, Taint::Clean);
        if let EngineSignal::Finished(value, taint) = signal {
            ensure_equal(value, SlotValue::Null)?;
            ensure_equal(taint, Taint::Clean)
        } else {
            Err("expected Finished variant".into())
        }
    }

    #[test]
    fn engine_signal_finished_with_derived_from_secret_taint() -> Result<(), String> {
        let signal = EngineSignal::Finished(SlotValue::I64(0), Taint::DerivedFromSecret);
        if let EngineSignal::Finished(_, taint) = signal {
            ensure_equal(taint, Taint::DerivedFromSecret)
        } else {
            Err("expected Finished variant".into())
        }
    }

    #[test]
    fn engine_signal_clone_preserves_variant_and_data() -> Result<(), String> {
        let original = EngineSignal::Finished(SlotValue::Bool(false), Taint::Secret);
        let cloned = original.clone();
        ensure_equal(cloned, original)?;

        let continue_orig = EngineSignal::Continue;
        let continue_clone = continue_orig.clone();
        ensure_equal(continue_clone, EngineSignal::Continue)?;

        let exhausted_orig = EngineSignal::StepBudgetExhausted;
        let exhausted_clone = exhausted_orig.clone();
        ensure_equal(exhausted_clone, EngineSignal::StepBudgetExhausted)
    }

    #[test]
    fn engine_signal_all_suspension_variants_are_distinct() -> Result<(), String> {
        let action = EngineSignal::AwaitingAction;
        let wait = EngineSignal::AwaitingWait;
        let ask = EngineSignal::AwaitingAsk;

        ensure_equal(action != wait, true)?;
        ensure_equal(action != ask, true)?;
        ensure_equal(wait != ask, true)?;

        ensure_equal(action.clone(), EngineSignal::AwaitingAction)?;
        ensure_equal(wait.clone(), EngineSignal::AwaitingWait)?;
        ensure_equal(ask.clone(), EngineSignal::AwaitingAsk)
    }

    #[test]
    fn engine_signal_debug_format_all_variants() -> Result<(), String> {
        ensure_equal(
            format!("{:?}", EngineSignal::Continue).contains("Continue"),
            true,
        )?;
        ensure_equal(
            format!("{:?}", EngineSignal::StepBudgetExhausted).contains("StepBudgetExhausted"),
            true,
        )?;
        ensure_equal(
            format!("{:?}", EngineSignal::AwaitingWait).contains("AwaitingWait"),
            true,
        )?;
        ensure_equal(
            format!("{:?}", EngineSignal::AwaitingAsk).contains("AwaitingAsk"),
            true,
        )?;
        let finished_debug = format!(
            "{:?}",
            EngineSignal::Finished(SlotValue::I64(1), Taint::Clean)
        );
        ensure_equal(finished_debug.contains("Finished"), true)
    }

    #[test]
    fn budget_try_take_on_exactly_max_step_budget_succeeds() -> Result<(), String> {
        let mut b = StepBudget::new(MAX_STEP_BUDGET);
        // First take from a full budget must succeed
        let taken = b.try_take().map_err(|e| e.to_string())?;
        ensure_equal(taken, true)?;
        ensure_equal(b.remaining(), MAX_STEP_BUDGET.saturating_sub(1))
    }
}
