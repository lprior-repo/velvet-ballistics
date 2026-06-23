#![forbid(unsafe_code)]

//! Collect, budget, and capability failures — diagnostic codes 0x14xx.
//!
//! These errors cover iteration/repeat limits, collection pagination and
//! capacity limits, budget violations, capability denials, and parse
//! failures.

use crate::diagnostic::DiagnosticCode;

// ── Diagnostic-code constants ──────────────────────────────────────────

/// Iteration limit exceeded diagnostic code.
pub(super) const ITERATION_LIMIT_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x1401);
/// Repeat exhausted diagnostic code.
pub(super) const REPEAT_EXHAUSTED_CODE: DiagnosticCode = DiagnosticCode::new(0x1402);
/// Invalid repeat state diagnostic code.
pub(super) const INVALID_REPEAT_STATE_CODE: DiagnosticCode = DiagnosticCode::new(0x0309);
/// Collect page limit exceeded diagnostic code.
pub(super) const COLLECT_PAGE_LIMIT_CODE: DiagnosticCode = DiagnosticCode::new(0x1403);
/// Collect item limit exceeded diagnostic code.
pub(super) const COLLECT_ITEM_LIMIT_CODE: DiagnosticCode = DiagnosticCode::new(0x1404);
/// Together branch limit exceeded diagnostic code.
pub(super) const TOGETHER_BRANCH_LIMIT_CODE: DiagnosticCode = DiagnosticCode::new(0x1405);
/// Budget exceeded diagnostic code.
pub(super) const BUDGET_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x1406);
/// Collect time limit exceeded diagnostic code.
pub(super) const COLLECT_TIME_LIMIT_CODE: DiagnosticCode = DiagnosticCode::new(0x1407);
/// Parallel limit exceeded diagnostic code.
pub(super) const PARALLEL_LIMIT_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x1408);
/// Capability denied diagnostic code.
pub(super) const CAPABILITY_DENIED_CODE: DiagnosticCode = DiagnosticCode::new(0x1409);
/// Budget parse diagnostic code.
pub(super) const BUDGET_PARSE_CODE: DiagnosticCode = DiagnosticCode::new(0x140A);
/// Collect page order violation diagnostic code.
pub(super) const COLLECT_PAGE_ORDER_VIOLATION_CODE: DiagnosticCode = DiagnosticCode::new(0x140B);
/// Collect extra hydration failed diagnostic code.
pub(super) const COLLECT_EXTRA_HYDRATION_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x140C);
/// Collect evidence capacity exceeded diagnostic code.
pub(super) const COLLECT_EVIDENCE_CAPACITY_EXCEEDED_CODE: DiagnosticCode =
    DiagnosticCode::new(0x140D);

// ── diagnostic_code match arm ──────────────────────────────────────────

/// Returns the diagnostic code for collect/budget/capability variants.
#[must_use]
pub(super) const fn diagnostic_code(
    error: &crate::errors::core::CoreError,
) -> Option<DiagnosticCode> {
    match error {
        crate::errors::core::CoreError::IterationLimitExceeded { .. } => {
            Some(ITERATION_LIMIT_EXCEEDED_CODE)
        }
        crate::errors::core::CoreError::RepeatExhausted { .. } => Some(REPEAT_EXHAUSTED_CODE),
        crate::errors::core::CoreError::InvalidRepeatState => Some(INVALID_REPEAT_STATE_CODE),
        crate::errors::core::CoreError::CollectPageLimitExceeded => Some(COLLECT_PAGE_LIMIT_CODE),
        crate::errors::core::CoreError::CollectItemLimitExceeded => Some(COLLECT_ITEM_LIMIT_CODE),
        crate::errors::core::CoreError::CollectTimeLimitExceeded => Some(COLLECT_TIME_LIMIT_CODE),
        crate::errors::core::CoreError::TogetherBranchLimitExceeded { .. } => {
            Some(TOGETHER_BRANCH_LIMIT_CODE)
        }
        crate::errors::core::CoreError::ParallelLimitExceeded { .. } => {
            Some(PARALLEL_LIMIT_EXCEEDED_CODE)
        }
        crate::errors::core::CoreError::BudgetExceeded { .. } => Some(BUDGET_EXCEEDED_CODE),
        crate::errors::core::CoreError::BudgetParse { .. } => Some(BUDGET_PARSE_CODE),
        crate::errors::core::CoreError::CapabilityDenied { .. } => Some(CAPABILITY_DENIED_CODE),
        crate::errors::core::CoreError::CollectPageOrderViolation { .. } => {
            Some(COLLECT_PAGE_ORDER_VIOLATION_CODE)
        }
        crate::errors::core::CoreError::CollectExtraHydrationFailed { .. } => {
            Some(COLLECT_EXTRA_HYDRATION_FAILED_CODE)
        }
        crate::errors::core::CoreError::CollectEvidenceCapacityExceeded { .. } => {
            Some(COLLECT_EVIDENCE_CAPACITY_EXCEEDED_CODE)
        }
        _ => None,
    }
}

// ── runtime_code match arm ─────────────────────────────────────────────

/// Returns the section-17 runtime code for collect/budget/capability variants, if any.
#[must_use]
pub(super) const fn runtime_code(error: &crate::errors::core::CoreError) -> Option<&'static str> {
    match error {
        crate::errors::core::CoreError::RepeatExhausted { .. } => {
            Some(REPEAT_LIMIT_REACHED_RUNTIME_CODE)
        }
        crate::errors::core::CoreError::InvalidRepeatState => {
            Some(INVALID_REPEAT_STATE_RUNTIME_CODE)
        }
        crate::errors::core::CoreError::CollectPageLimitExceeded
        | crate::errors::core::CoreError::CollectItemLimitExceeded
        | crate::errors::core::CoreError::CollectTimeLimitExceeded => {
            Some(COLLECT_LIMIT_REACHED_RUNTIME_CODE)
        }
        crate::errors::core::CoreError::BudgetExceeded { .. } => Some(BUDGET_EXCEEDED_RUNTIME_CODE),
        crate::errors::core::CoreError::CapabilityDenied { .. } => {
            Some(CAPABILITY_DENIED_RUNTIME_CODE)
        }
        _ => None,
    }
}

// ── Runtime-code constants ─────────────────────────────────────────────

/// Runtime code for repeat attempt-limit failures.
pub(super) const REPEAT_LIMIT_REACHED_RUNTIME_CODE: &str = "REPEAT_LIMIT_REACHED";
/// Runtime code for invalid repeat-state failures.
pub(super) const INVALID_REPEAT_STATE_RUNTIME_CODE: &str = "INVALID_REPEAT";
/// Runtime code for collect item/page limit failures.
pub(super) const COLLECT_LIMIT_REACHED_RUNTIME_CODE: &str = "COLLECT_LIMIT_REACHED";
/// Runtime code for budget exceeded failures.
pub(super) const BUDGET_EXCEEDED_RUNTIME_CODE: &str = "BUDGET_EXCEEDED";
/// Capability denied runtime code.
pub(super) const CAPABILITY_DENIED_RUNTIME_CODE: &str = "CAPABILITY_DENIED";
