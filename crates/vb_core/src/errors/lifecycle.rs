#![forbid(unsafe_code)]

//! Lifecycle, journal, and replay failures — diagnostic codes 0x15xx.
//!
//! These errors cover storage unavailability, duplicate/stale requests,
//! invalid state transitions, journal write failures, and replay
//! corruption.

use crate::diagnostic::DiagnosticCode;

// ── Diagnostic-code constants ──────────────────────────────────────────

/// Lifecycle storage unavailable diagnostic code.
pub(super) const LIFECYCLE_STORAGE_UNAVAILABLE_CODE: DiagnosticCode = DiagnosticCode::new(0x1501);
/// Lifecycle duplicate request diagnostic code.
pub(super) const LIFECYCLE_DUPLICATE_REQUEST_CODE: DiagnosticCode = DiagnosticCode::new(0x1502);
/// Lifecycle stale request diagnostic code.
pub(super) const LIFECYCLE_STALE_REQUEST_CODE: DiagnosticCode = DiagnosticCode::new(0x1503);
/// Lifecycle invalid transition diagnostic code.
pub(super) const LIFECYCLE_INVALID_TRANSITION_CODE: DiagnosticCode = DiagnosticCode::new(0x1504);
/// Journal write failure diagnostic code.
pub(super) const JOURNAL_WRITE_FAILURE_CODE: DiagnosticCode = DiagnosticCode::new(0x1505);
/// Replay corruption diagnostic code.
pub(super) const REPLAY_CORRUPTION_CODE: DiagnosticCode = DiagnosticCode::new(0x1506);

// ── diagnostic_code match arm ──────────────────────────────────────────

/// Returns the diagnostic code for lifecycle variants.
#[must_use]
pub(super) const fn diagnostic_code(error: &crate::errors::core::CoreError) -> Option<DiagnosticCode> {
    match error {
        crate::errors::core::CoreError::LifecycleStorageUnavailable { .. } => {
            Some(LIFECYCLE_STORAGE_UNAVAILABLE_CODE)
        }
        crate::errors::core::CoreError::LifecycleDuplicateRequest { .. } => {
            Some(LIFECYCLE_DUPLICATE_REQUEST_CODE)
        }
        crate::errors::core::CoreError::LifecycleStaleRequest { .. } => {
            Some(LIFECYCLE_STALE_REQUEST_CODE)
        }
        crate::errors::core::CoreError::LifecycleInvalidTransition { .. } => {
            Some(LIFECYCLE_INVALID_TRANSITION_CODE)
        }
        crate::errors::core::CoreError::JournalWriteFailure { .. } => Some(JOURNAL_WRITE_FAILURE_CODE),
        crate::errors::core::CoreError::ReplayCorruption { .. } => Some(REPLAY_CORRUPTION_CODE),
        _ => None,
    }
}
