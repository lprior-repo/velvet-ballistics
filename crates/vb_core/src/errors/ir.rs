#![forbid(unsafe_code)]

//! IR validation failures — diagnostic codes 0x1001–0x1104.
//!
//! These errors arise when compiled workflow data is structurally unsound:
//! out-of-bounds references, type mismatches, and arithmetic anomalies.

use crate::diagnostic::DiagnosticCode;

// ── Diagnostic-code constants ──────────────────────────────────────────

/// Invalid program counter diagnostic code.
pub(super) const INVALID_PROGRAM_COUNTER_CODE: DiagnosticCode = DiagnosticCode::new(0x1001);
/// Missing next step diagnostic code.
pub(super) const MISSING_NEXT_STEP_CODE: DiagnosticCode = DiagnosticCode::new(0x1002);
/// Slot out-of-bounds diagnostic code.
pub(super) const SLOT_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x1011);
/// Slot uninitialized diagnostic code.
pub(super) const SLOT_UNINITIALIZED_CODE: DiagnosticCode = DiagnosticCode::new(0x1012);
/// Constant out-of-bounds diagnostic code.
pub(super) const CONST_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x1013);
/// Expression out-of-bounds diagnostic code.
pub(super) const EXPR_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x1015);
/// Type mismatch diagnostic code.
pub(super) const TYPE_MISMATCH_CODE: DiagnosticCode = DiagnosticCode::new(0x1101);
/// Non-boolean condition diagnostic code.
pub(super) const NON_BOOL_CONDITION_CODE: DiagnosticCode = DiagnosticCode::new(0x1104);
/// Non-finite number diagnostic code.
pub(super) const NON_FINITE_NUMBER_CODE: DiagnosticCode = DiagnosticCode::new(0x1102);
/// Division by zero diagnostic code.
pub(super) const DIVISION_BY_ZERO_CODE: DiagnosticCode = DiagnosticCode::new(0x1103);

// ── diagnostic_code match arm ──────────────────────────────────────────

/// Returns the diagnostic code for IR-validation variants.
#[must_use]
pub(super) const fn diagnostic_code(error: &crate::errors::CoreError) -> Option<DiagnosticCode> {
    match error {
        crate::errors::CoreError::InvalidProgramCounter { .. } => {
            Some(INVALID_PROGRAM_COUNTER_CODE)
        }
        crate::errors::CoreError::MissingNextStep { .. } => Some(MISSING_NEXT_STEP_CODE),
        crate::errors::CoreError::SlotOutOfBounds { .. } => Some(SLOT_OUT_OF_BOUNDS_CODE),
        crate::errors::CoreError::SlotUninitialized { .. } => Some(SLOT_UNINITIALIZED_CODE),
        crate::errors::CoreError::ExprOutOfBounds { .. } => Some(EXPR_OUT_OF_BOUNDS_CODE),
        crate::errors::CoreError::ConstOutOfBounds { .. } => Some(CONST_OUT_OF_BOUNDS_CODE),
        crate::errors::CoreError::TypeMismatch { .. } => Some(TYPE_MISMATCH_CODE),
        crate::errors::CoreError::NonBoolCondition { .. } => Some(NON_BOOL_CONDITION_CODE),
        crate::errors::CoreError::NonFiniteNumber => Some(NON_FINITE_NUMBER_CODE),
        crate::errors::CoreError::DivisionByZero => Some(DIVISION_BY_ZERO_CODE),
        _ => None,
    }
}

// ── runtime_code match arm ─────────────────────────────────────────────

/// Returns the section-17 runtime code for IR-validation variants, if any.
#[must_use]
pub(super) const fn runtime_code(error: &crate::errors::CoreError) -> Option<&'static str> {
    match error {
        crate::errors::CoreError::ConstOutOfBounds { .. } => Some(CONST_OUT_OF_BOUNDS_RUNTIME_CODE),
        crate::errors::CoreError::TypeMismatch { .. }
        | crate::errors::CoreError::NonBoolCondition { .. } => {
            Some(INPUT_TYPE_MISMATCH_RUNTIME_CODE)
        }
        _ => None,
    }
}

/// Runtime code for constant-pool bounds failures.
pub(super) const CONST_OUT_OF_BOUNDS_RUNTIME_CODE: &str = "CONST_OUT_OF_BOUNDS";
/// Runtime code for runtime input type mismatches.
pub(super) const INPUT_TYPE_MISMATCH_RUNTIME_CODE: &str = "INPUT_TYPE_MISMATCH";
