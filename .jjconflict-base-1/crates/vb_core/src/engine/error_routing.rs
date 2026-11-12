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

use crate::CoreError;
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

/// Returns the static string code for engine errors that don't have a runtime code.
fn engine_error_static_code(error: &EngineError) -> &'static str {
    use CoreError::*;
    match error {
        InvalidProgramCounter { .. } => "INVALID_PROGRAM_COUNTER",
        MissingNextStep { .. } => "MISSING_NEXT_STEP",
        MissingOutputSlot { .. } => "MISSING_OUTPUT_SLOT",
        InvalidCompiledWorkflow { .. } => "INVALID_COMPILED_WORKFLOW",
        SlotOutOfBounds { .. } => "SLOT_OUT_OF_BOUNDS",
        ExprOutOfBounds { .. } => "EXPR_OUT_OF_BOUNDS",
        ConstOutOfBounds { .. } => "CONST_OUT_OF_BOUNDS",
        StepStateOutOfBounds { .. } => "STEP_STATE_OUT_OF_BOUNDS",
        ListIndexOutOfBounds { .. } => "LIST_INDEX_OUT_OF_BOUNDS",
        SymbolOutOfBounds { .. } => "SYMBOL_OUT_OF_BOUNDS",
        ListOutOfBounds { .. } => "LIST_OUT_OF_BOUNDS",
        ObjectOutOfBounds { .. } => "OBJECT_OUT_OF_BOUNDS",
        BlobOutOfBounds { .. } => "BLOB_OUT_OF_BOUNDS",
        TypeMismatch { .. } => "TYPE_MISMATCH",
        NonBoolCondition { .. } => "NON_BOOL_CONDITION",
        DivisionByZero => "DIVISION_BY_ZERO",
        NonFiniteNumber => "NON_FINITE_NUMBER",
        StepBudgetExhausted => "STEP_BUDGET_EXHAUSTED",
        StepCounterOverflow => "STEP_COUNTER_OVERFLOW",
        QueueFull => "QUEUE_FULL",
        AllocationFailed => "ALLOCATION_FAILED",
        ExpressionStackOverflow { .. } => "EXPRESSION_STACK_OVERFLOW",
        ExpressionStackUnderflow => "EXPRESSION_STACK_UNDERFLOW",
        UnsupportedPrimitive { .. } => "UNSUPPORTED_PRIMITIVE",
        UnsupportedAccessorTraversal { .. } => "UNSUPPORTED_ACCESSOR_TRAVERSAL",
        ObjectFieldNotFound { .. } => "OBJECT_FIELD_NOT_FOUND",
        InternalInvariantViolation { .. } => "INTERNAL_INVARIANT_VIOLATION",
        IterationLimitExceeded { .. } => "ITERATION_LIMIT_EXCEEDED",
        RepeatExhausted { .. } => "REPEAT_EXHAUSTED",
        InvalidRepeatState { .. } => "INVALID_REPEAT_STATE",
        CollectPageLimitExceeded => "COLLECT_PAGE_LIMIT_EXCEEDED",
        CollectItemLimitExceeded => "COLLECT_ITEM_LIMIT_EXCEEDED",
        CollectTimeLimitExceeded => "COLLECT_TIME_LIMIT_EXCEEDED",
        TogetherBranchLimitExceeded { .. } => "TOGETHER_BRANCH_LIMIT_EXCEEDED",
        ParallelLimitExceeded { .. } => "PARALLEL_LIMIT_EXCEEDED",
        BudgetExceeded { .. } => "BUDGET_EXCEEDED",
        BudgetParse { .. } => "BUDGET_PARSE",
        ResourceLimitExceeded { .. } => "RESOURCE_LIMIT_EXCEEDED",
        &CoreError::SlotUninitialized { .. } => "SLOT_UNINITIALIZED",
        CapabilityDenied { .. } => "CAPABILITY_DENIED",
        ActionFailed { .. } => "ACTION_FAILED",
        ActionResumeRejected { .. } => "ACTION_RESUME_REJECTED",
        CollectPageOrderViolation { .. } => "COLLECT_PAGE_ORDER_VIOLATION",
        CollectExtraHydrationFailed { .. } => "COLLECT_EXTRA_HYDRATION_FAILED",
        CollectEvidenceCapacityExceeded { .. } => "COLLECT_EVIDENCE_CAPACITY_EXCEEDED",
        EvidenceCapacityExceeded { .. } => "EVIDENCE_CAPACITY_EXCEEDED",
        LifecycleStorageUnavailable { .. } => "LIFECYCLE_STORAGE_UNAVAILABLE",
        LifecycleDuplicateRequest { .. } => "LIFECYCLE_DUPLICATE_REQUEST",
        LifecycleStaleRequest { .. } => "LIFECYCLE_STALE_REQUEST",
        LifecycleInvalidTransition { .. } => "LIFECYCLE_INVALID_TRANSITION",
        JournalWriteFailure { .. } => "JOURNAL_WRITE_FAILURE",
        ReplayCorruption { .. } => "REPLAY_CORRUPTION",
        InvalidSpan { .. } => "INVALID_SPAN",
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
        let node = plan
            .node(StepIdx::new(raw))
            .ok_or(EngineError::InvalidProgramCounter {
                step: StepIdx::new(raw),
            })?;
        if let crate::workflow::CompiledNodeKind::TogetherStart { ref branches, join } = node.kind
            && node.id < failed
            && failed <= join
            && !(node.id < handler && handler <= join)
            && run.step_state(node.id)? == crate::frame::StepState::Succeeded
            && run.step_state(join)? != crate::frame::StepState::Succeeded
        {
            let branches = u16::try_from(branches.len())
                .map_err(|_| EngineError::TogetherBranchLimitExceeded { max: u16::MAX })?;
            count = count
                .checked_add(branches)
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
