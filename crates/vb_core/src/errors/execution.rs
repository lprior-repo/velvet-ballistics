#![forbid(unsafe_code)]

//! Execution and resource failures — diagnostic codes 0x12xx, 0x13xx.
//!
//! These errors cover budget exhaustion, resource limits, allocation
//! failures, expression stack anomalies, and various internal consistency
//! violations.

use crate::diagnostic::DiagnosticCode;

// ── Diagnostic-code constants ──────────────────────────────────────────

/// Step budget exhausted diagnostic code.
pub(super) const STEP_BUDGET_EXHAUSTED_CODE: DiagnosticCode = DiagnosticCode::new(0x1201);
/// Step counter overflow diagnostic code.
pub(super) const STEP_COUNTER_OVERFLOW_CODE: DiagnosticCode = DiagnosticCode::new(0x1202);
/// Queue full diagnostic code.
pub(super) const QUEUE_FULL_CODE: DiagnosticCode = DiagnosticCode::new(0x1301);
/// Resource limit exceeded diagnostic code.
pub(super) const RESOURCE_LIMIT_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x1302);
/// Allocation failed diagnostic code.
pub(super) const ALLOCATION_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x1303);
/// Expression stack overflow diagnostic code.
pub(super) const EXPRESSION_STACK_OVERFLOW_CODE: DiagnosticCode = DiagnosticCode::new(0x1304);
/// Missing output slot diagnostic code.
pub(super) const MISSING_OUTPUT_SLOT_CODE: DiagnosticCode = DiagnosticCode::new(0x1305);
/// Step state out-of-bounds diagnostic code.
pub(super) const STEP_STATE_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x1306);
/// Invalid compiled workflow diagnostic code.
pub(super) const INVALID_COMPILED_WORKFLOW_CODE: DiagnosticCode = DiagnosticCode::new(0x1307);
/// Unsupported primitive diagnostic code.
pub(super) const UNSUPPORTED_PRIMITIVE_CODE: DiagnosticCode = DiagnosticCode::new(0x1308);
/// Internal invariant diagnostic code.
pub(super) const INTERNAL_INVARIANT_CODE: DiagnosticCode = DiagnosticCode::new(0x1309);
/// Unsupported accessor traversal diagnostic code.
pub(super) const UNSUPPORTED_ACCESSOR_TRAVERSAL_CODE: DiagnosticCode = DiagnosticCode::new(0x130A);
/// Expression stack underflow diagnostic code.
pub(super) const EXPRESSION_STACK_UNDERFLOW_CODE: DiagnosticCode = DiagnosticCode::new(0x130B);
/// Object accessor field not found diagnostic code.
pub(super) const OBJECT_FIELD_NOT_FOUND_CODE: DiagnosticCode = DiagnosticCode::new(0x130C);
/// List accessor index out-of-bounds diagnostic code.
pub(super) const LIST_INDEX_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x130D);
/// Symbol handle out-of-bounds diagnostic code.
pub(super) const SYMBOL_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x1311);
/// List handle out-of-bounds diagnostic code.
pub(super) const LIST_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x1312);
/// Object handle out-of-bounds diagnostic code.
pub(super) const OBJECT_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x1313);
/// Blob handle out-of-bounds diagnostic code.
pub(super) const BLOB_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x1314);

// ── diagnostic_code match arm ──────────────────────────────────────────

/// Returns the diagnostic code for execution variants.
#[must_use]
pub(super) const fn diagnostic_code(
    error: &crate::errors::CoreError,
) -> Option<DiagnosticCode> {
    match error {
        crate::errors::CoreError::StepBudgetExhausted => Some(STEP_BUDGET_EXHAUSTED_CODE),
        crate::errors::CoreError::StepCounterOverflow => Some(STEP_COUNTER_OVERFLOW_CODE),
        crate::errors::CoreError::QueueFull => Some(QUEUE_FULL_CODE),
        crate::errors::CoreError::ResourceLimitExceeded { .. } => {
            Some(RESOURCE_LIMIT_EXCEEDED_CODE)
        }
        crate::errors::CoreError::AllocationFailed => Some(ALLOCATION_FAILED_CODE),
        crate::errors::CoreError::ExpressionStackOverflow { .. } => {
            Some(EXPRESSION_STACK_OVERFLOW_CODE)
        }
        crate::errors::CoreError::MissingOutputSlot { .. } => Some(MISSING_OUTPUT_SLOT_CODE),
        crate::errors::CoreError::StepStateOutOfBounds { .. } => {
            Some(STEP_STATE_OUT_OF_BOUNDS_CODE)
        }
        crate::errors::CoreError::InvalidCompiledWorkflow { .. } => {
            Some(INVALID_COMPILED_WORKFLOW_CODE)
        }
        crate::errors::CoreError::UnsupportedPrimitive { .. } => {
            Some(UNSUPPORTED_PRIMITIVE_CODE)
        }
        crate::errors::CoreError::InternalInvariantViolation { .. } => {
            Some(INTERNAL_INVARIANT_CODE)
        }
        crate::errors::CoreError::UnsupportedAccessorTraversal { .. } => {
            Some(UNSUPPORTED_ACCESSOR_TRAVERSAL_CODE)
        }
        crate::errors::CoreError::ObjectFieldNotFound { .. } => Some(OBJECT_FIELD_NOT_FOUND_CODE),
        crate::errors::CoreError::ListIndexOutOfBounds { .. } => {
            Some(LIST_INDEX_OUT_OF_BOUNDS_CODE)
        }
        crate::errors::CoreError::ExpressionStackUnderflow => {
            Some(EXPRESSION_STACK_UNDERFLOW_CODE)
        }
        crate::errors::CoreError::SymbolOutOfBounds { .. } => Some(SYMBOL_OUT_OF_BOUNDS_CODE),
        crate::errors::CoreError::ListOutOfBounds { .. } => Some(LIST_OUT_OF_BOUNDS_CODE),
        crate::errors::CoreError::ObjectOutOfBounds { .. } => Some(OBJECT_OUT_OF_BOUNDS_CODE),
        crate::errors::CoreError::BlobOutOfBounds { .. } => Some(BLOB_OUT_OF_BOUNDS_CODE),
        _ => None,
    }
}

// ── runtime_code match arm ─────────────────────────────────────────────

/// Returns the section-17 runtime code for execution variants, if any.
#[must_use]
pub(super) const fn runtime_code(
    error: &crate::errors::CoreError,
) -> Option<&'static str> {
    match error {
        crate::errors::CoreError::MissingOutputSlot { .. } => {
            Some(MISSING_OUTPUT_SLOT_RUNTIME_CODE)
        }
        crate::errors::CoreError::StepStateOutOfBounds { .. } => {
            Some(STEP_STATE_OUT_OF_BOUNDS_RUNTIME_CODE)
        }
        crate::errors::CoreError::ExpressionStackOverflow { .. } => {
            Some(EXPRESSION_STACK_OVERFLOW_RUNTIME_CODE)
        }
        crate::errors::CoreError::ExpressionStackUnderflow => {
            Some(EXPRESSION_STACK_UNDERFLOW_RUNTIME_CODE)
        }
        crate::errors::CoreError::InvalidCompiledWorkflow { .. } => {
            Some(INVALID_COMPILED_WORKFLOW_RUNTIME_CODE)
        }
        crate::errors::CoreError::InternalInvariantViolation { .. } => {
            Some(INTERNAL_INVARIANT_VIOLATION_RUNTIME_CODE)
        }
        crate::errors::CoreError::UnsupportedPrimitive { .. } => {
            Some(UNSUPPORTED_PRIMITIVE_RUNTIME_CODE)
        }
        crate::errors::CoreError::QueueFull => Some(QUEUE_FULL_RUNTIME_CODE),
        _ => None,
    }
}

// ── Runtime-code constants ─────────────────────────────────────────────

/// Runtime code for missing output-slot failures.
pub(super) const MISSING_OUTPUT_SLOT_RUNTIME_CODE: &str = "MISSING_OUTPUT_SLOT";
/// Runtime code for step-state bounds failures.
pub(super) const STEP_STATE_OUT_OF_BOUNDS_RUNTIME_CODE: &str = "STEP_STATE_OUT_OF_BOUNDS";
/// Runtime code for expression stack overflow failures.
pub(super) const EXPRESSION_STACK_OVERFLOW_RUNTIME_CODE: &str = "EXPRESSION_STACK_OVERFLOW";
/// Runtime code for expression stack underflow failures.
pub(super) const EXPRESSION_STACK_UNDERFLOW_RUNTIME_CODE: &str = "EXPRESSION_STACK_UNDERFLOW";
/// Runtime code for invalid compiled workflow failures.
pub(super) const INVALID_COMPILED_WORKFLOW_RUNTIME_CODE: &str = "INVALID_COMPILED_WORKFLOW";
/// Runtime code for internal invariant failures.
pub(super) const INTERNAL_INVARIANT_VIOLATION_RUNTIME_CODE: &str = "INTERNAL_INVARIANT_VIOLATION";
/// Runtime code for unsupported primitive failures.
pub(super) const UNSUPPORTED_PRIMITIVE_RUNTIME_CODE: &str = "UNSUPPORTED_PRIMITIVE";
/// Runtime code for queue capacity failures.
pub(super) const QUEUE_FULL_RUNTIME_CODE: &str = "QUEUE_FULL";
