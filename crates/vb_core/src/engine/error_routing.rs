#![forbid(unsafe_code)]
//! On-error/then handler routing for step failures.
//!
//! When a step fails, the engine checks if the compiled node declares an
//! `on_error` handler. If present, the PC is routed to the handler step and
//! the failed step index is written to the designated error slot as an `I64`.
//! The `ErrorSlotData` struct captures full diagnostic details (code, message,
//! step) for logging and auditing, but only the step index is written to the
//! slot because the `SlotValue` type system does not support arbitrary strings.
//! If no handler exists, the run fails.

use crate::HasSymbolicCode;
use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::{SlotIdx, StepIdx};
use crate::value::SlotValue;
use crate::workflow::CompiledWorkflow;

#[cfg(test)]
use crate::workflow::CompiledNode;

/// Outcome of an error handler routing decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorHandlerOutcome {
    /// The error was routed to a handler step. PC has been updated.
    Routed,
    /// No handler was configured; the run must fail.
    NoHandler,
}

/// Diagnostic error information captured when routing to an error handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorSlotData {
    pub code: Box<str>,
    pub message: Box<str>,
    pub failed_step: StepIdx,
}

impl ErrorSlotData {
    #[must_use]
    pub fn from_engine_error(error: &EngineError, failed_step: StepIdx) -> Self {
        let code = error_code_string(error);
        let message = format!("{error}");
        Self {
            code,
            message: message.into_boxed_str(),
            failed_step,
        }
    }
}

/// Returns the symbolic code stored in an engine error slot.
fn engine_error_static_code(error: &EngineError) -> &'static str {
    match error {
        EngineError::LifecycleStorageUnavailable { .. } => "LIFECYCLE_STORAGE_UNAVAILABLE",
        EngineError::LifecycleDuplicateRequest { .. } => "LIFECYCLE_DUPLICATE_REQUEST",
        EngineError::LifecycleInvalidTransition { .. } => "LIFECYCLE_INVALID_TRANSITION",
        _ => error.symbolic_code().as_str(),
    }
}

fn error_code_string(error: &EngineError) -> Box<str> {
    error
        .runtime_code()
        .map(|code| code.into())
        .unwrap_or_else(|| engine_error_static_code(error).into())
}

pub fn route_error_handler(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    failed_step: StepIdx,
    error: &EngineError,
) -> Result<ErrorHandlerOutcome, EngineError> {
    let node = plan
        .node(failed_step)
        .ok_or(EngineError::InvalidProgramCounter { step: failed_step })?;
    let crate::workflow::ErrorRouting::Handler {
        handler: handler_step,
        ..
    } = node.routing.error
    else {
        return Ok(ErrorHandlerOutcome::NoHandler);
    };

    let error_slot = plan.resolved_error_slot(node);

    if let Some(error_slot) = error_slot {
        write_error_slot(run, error_slot, error, failed_step)?;
    }

    abandon_together_scopes(plan, run, failed_step, handler_step)?;
    advance_to_handler(run, handler_step)?;
    Ok(ErrorHandlerOutcome::Routed)
}

/// Retires reservations belonging to Together scopes exited by an error route.
pub fn abandon_together_scopes(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    failed: StepIdx,
    handler: StepIdx,
) -> Result<(), EngineError> {
    let count = abandoned_together_count(plan, run, failed, handler)?;
    run.sub_parallel_in_flight(count)
}

/// Read-only preflight for the durable action-failure path.
pub fn abandoned_together_count(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    failed: StepIdx,
    handler: StepIdx,
) -> Result<u16, EngineError> {
    let mut count = 0u16;
    for raw in 0..plan.node_count() {
        let step = StepIdx::new(raw);
        let node = plan
            .node(step)
            .ok_or(EngineError::InvalidProgramCounter { step })?;
        if let Some(branch_count) = abandoned_group_branch_count(node, run, failed, handler)? {
            count =
                count
                    .checked_add(branch_count)
                    .ok_or(EngineError::InternalInvariantViolation {
                        reason: "parallel_in_flight overflow".into(),
                    })?;
        }
    }
    if count > run.parallel_in_flight() {
        return Err(EngineError::InternalInvariantViolation {
            reason: "parallel_in_flight underflow".into(),
        });
    }
    Ok(count)
}

fn abandoned_group_branch_count(
    node: &crate::workflow::CompiledNode,
    run: &RunFrame,
    failed: StepIdx,
    handler: StepIdx,
) -> Result<Option<u16>, EngineError> {
    if let crate::workflow::CompiledNodeKind::TogetherStart { ref branches, join } = node.kind
        && node.id < failed
        && failed <= join
        && !(node.id < handler && handler <= join)
        && run.step_state(node.id)? == crate::frame::StepState::Succeeded
        && run.step_state(join)? != crate::frame::StepState::Succeeded
    {
        let branches = u16::try_from(branches.len())
            .map_err(|_| EngineError::TogetherBranchLimitExceeded { max: u16::MAX })?;
        return Ok(Some(branches));
    }
    Ok(None)
}

#[inline]
fn advance_to_handler(run: &mut RunFrame, handler_step: StepIdx) -> Result<(), EngineError> {
    run.set_pc(handler_step)?;
    run.increment_executed()?;
    Ok(())
}

fn write_error_slot(
    run: &mut RunFrame,
    error_slot: SlotIdx,
    _error: &EngineError,
    failed_step: StepIdx,
) -> Result<(), EngineError> {
    run.write_slot(error_slot, SlotValue::I64(i64::from(failed_step.get())))?;
    Ok(())
}

#[cfg(test)]
#[must_use]
fn has_error_handler(node: &CompiledNode) -> bool {
    node.routing.error.handler().is_some()
}

#[cfg(test)]
#[path = "error_routing/tests.rs"]
mod tests;
