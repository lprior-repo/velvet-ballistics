#![forbid(unsafe_code)]

//! Step-budget timers: consume one step from the budget and surface
//! typed errors instead of panicking on overflow.

use vb_core::engine::StepBudget;

use crate::engine::types::{RuntimeEngineError, RuntimeEngineResult};

/// Attempts to take one step from the budget. Returns `Ok(true)` when
/// a step was consumed and `Ok(false)` when the budget is exhausted.
/// On internal arithmetic overflow a typed `InternalInvariantViolation`
/// error is returned rather than a panic or silent saturation.
pub(super) fn try_consume_step_budget(budget: &mut StepBudget) -> RuntimeEngineResult<bool> {
    budget.try_take().map_err(RuntimeEngineError::Core)
}