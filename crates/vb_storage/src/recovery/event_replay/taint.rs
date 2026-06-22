#![forbid(unsafe_code)]
//! Slot taint resolution for fail-closed replay.
//!
//! These helpers allow the recovery layer to read existing slot taints
//! without erasing read failures into implicit defaults. A failed read
//! must surface as [`RecoveryError::SlotTaintReadFailed`] so callers
//! cannot silently launder taint inconsistencies into a Clean default
//! (vb-7ol6y / Bug 3).
//!
//! Types and helpers are also `#[cfg(kani)]` accessible for the
//! bounded-model harnesses.

/// Copy-only observation of `RunFrame::read_taint` for fail-closed replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlotTaintReadObservation {
    /// Existing taint was read successfully.
    Existing(vb_core::Taint),
    /// The slot is not initialized, so Clean is the only allowed default.
    Uninitialized,
    /// The taint read failed for any other reason and must fail closed.
    Failed,
}

/// Copy-only taint resolution decision for slot replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlotTaintResolution {
    /// Continue with the selected taint value.
    Use(vb_core::Taint),
    /// Abort replay instead of downgrading taint to Clean.
    FailClosed,
}

/// Resolves a taint read without allocating or mutating the frame.
#[must_use]
pub(crate) const fn resolve_slot_taint_read(
    observation: SlotTaintReadObservation,
) -> SlotTaintResolution {
    match observation {
        SlotTaintReadObservation::Existing(taint) => SlotTaintResolution::Use(taint),
        SlotTaintReadObservation::Uninitialized => SlotTaintResolution::Use(vb_core::Taint::Clean),
        SlotTaintReadObservation::Failed => SlotTaintResolution::FailClosed,
    }
}

pub(crate) fn observe_slot_taint_read(
    result: Result<vb_core::Taint, vb_core::CoreError>,
) -> SlotTaintReadObservation {
    match result {
        Ok(taint) => SlotTaintReadObservation::Existing(taint),
        Err(vb_core::CoreError::SlotUninitialized { .. }) => {
            SlotTaintReadObservation::Uninitialized
        }
        Err(_) => SlotTaintReadObservation::Failed,
    }
}
