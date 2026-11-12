#![forbid(unsafe_code)]
//! RunFrame hydration support functions.
//!
//! Internal helpers for slot decoding, dimension derivation, event application,
//! and parallel in-flight tracking. Not part of the public API.

mod apply;
mod apply_action;
mod snapshot;
mod verify;

pub(crate) use crate::recovery::replay::core::check_action_abi_digest_against_expected;
pub(super) use apply::apply_tail_events;
pub(super) use snapshot::{decode_snapshot_slots, derive_dimensions_from_snapshot_and_tail};
pub(crate) use verify::{verified_action_envelope_digest, verify_action_ticket_event};

use crate::recovery::types::{RecoveryError, RecoveryResult};

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

fn observe_slot_taint_read(
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

pub(crate) fn resolve_slot_taint_or_fail(
    frame: &vb_core::RunFrame,
    slot: vb_core::SlotIdx,
) -> RecoveryResult<vb_core::Taint> {
    match resolve_slot_taint_read(observe_slot_taint_read(frame.read_taint(slot))) {
        SlotTaintResolution::Use(taint) => Ok(taint),
        SlotTaintResolution::FailClosed => Err(RecoveryError::SlotTaintReadFailed { slot }),
    }
}
