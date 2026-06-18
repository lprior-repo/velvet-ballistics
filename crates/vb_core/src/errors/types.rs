#![forbid(unsafe_code)]

//! Auxiliary types used by the [`CoreError`](super::CoreError) variants.
//!
//! These types are not part of the `CoreError` enum itself but are referenced
//! as field types in several variants. They live here so that any module
//! holding a `CoreError` variant can inspect the auxiliary payload without
//! depending on the internal structure of the parent module.

use crate::ids::{ListId, RunId, SlotIdx};

/// Kind of page-order violation during evidence collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CollectPageOrderViolationKind {
    /// A page was collected out of sequential order.
    OutOfOrder,
    /// A duplicate page was observed.
    Duplicate,
    /// A stale page was observed.
    Stale,
}

/// Kind of extra-hydration failure.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CollectExtraHydrationFailureKind {
    /// Extra data was empty.
    EmptyExtra,
    /// Extra data decoding failed.
    DecodeFailed,
    /// Run ID mismatch.
    RunMismatch {
        /// Expected run ID.
        expected: RunId,
        /// Actual run ID.
        actual: RunId,
    },
    /// Slot mismatch.
    SlotMismatch {
        /// Expected slot.
        expected: SlotIdx,
        /// Actual slot.
        actual: SlotIdx,
    },
    /// Current page mismatch.
    CurrentPageMismatch {
        /// Expected page.
        expected: ListId,
        /// Actual page.
        actual: ListId,
    },
}

/// Evidence collection failed because capacity was exceeded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectEvidenceCapacityExceeded {
    /// Run identifier.
    pub run_id: crate::ids::RunId,
    /// Slot that caused the overflow.
    pub slot: crate::ids::SlotIdx,
    /// Configured capacity.
    pub capacity: usize,
    /// Actual length of data.
    pub len: usize,
    /// Required extra slots.
    pub required: usize,
}

/// Lifecycle error: storage is unavailable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleStorageUnavailable {
    /// Diagnostic code.
    pub code: crate::diagnostic::DiagnosticCode,
    /// Human-readable context.
    pub context: String,
    /// Timestamp of the error.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Associated run ID if available.
    pub bead_id: Option<crate::ids::RunId>,
}

/// Lifecycle error: duplicate request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleDuplicateRequest {
    /// Diagnostic code.
    pub code: crate::diagnostic::DiagnosticCode,
    /// Human-readable context.
    pub context: String,
    /// Timestamp of the error.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Associated run ID if available.
    pub bead_id: Option<crate::ids::RunId>,
    /// Command that triggered the duplicate request.
    pub command: Option<&'static str>,
}

/// Lifecycle error: stale request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleStaleRequest {
    /// Diagnostic code.
    pub code: crate::diagnostic::DiagnosticCode,
    /// Human-readable context.
    pub context: String,
    /// Timestamp of the error.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Associated run ID if available.
    pub bead_id: Option<crate::ids::RunId>,
    /// Command that triggered the stale request.
    pub command: Option<&'static str>,
}

/// Lifecycle error: invalid state transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleInvalidTransition {
    /// Diagnostic code.
    pub code: crate::diagnostic::DiagnosticCode,
    /// Human-readable context.
    pub context: String,
    /// Timestamp of the error.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Associated run ID if available.
    pub bead_id: Option<crate::ids::RunId>,
    /// Command that triggered the invalid transition.
    pub command: Option<&'static str>,
}

/// Journal write failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalWriteFailure {
    /// Diagnostic code.
    pub code: crate::diagnostic::DiagnosticCode,
    /// Human-readable context.
    pub context: String,
    /// Timestamp of the error.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Associated run ID if available.
    pub bead_id: Option<crate::ids::RunId>,
}

/// Replay detected corruption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayCorruption {
    /// Diagnostic code.
    pub code: crate::diagnostic::DiagnosticCode,
    /// Human-readable context.
    pub context: String,
    /// Timestamp of the error.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Associated run ID if available.
    pub bead_id: Option<crate::ids::RunId>,
}
