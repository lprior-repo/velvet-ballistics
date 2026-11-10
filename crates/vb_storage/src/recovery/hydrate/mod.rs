#![forbid(unsafe_code)]
//! RunFrame hydration from snapshot and journal events.
//!
//! Provides:
//! - `hydrate_run_frame`: Reconstruct live RunFrame from snapshot + tail events
//! - `hydrate_run_frame_from_events`: Reconstruct live RunFrame from events only

mod accounting;
mod classify;
mod envelope;
mod from_events;
mod preconditions;
mod validate;

pub use classify::classify_snapshot_tail_cannot_resume;
pub use from_events::hydrate_run_frame_from_events;
#[cfg(test)]
pub(crate) use preconditions::{
    SnapshotRecoveryInputViolation, TailEventMetadata, validate_recovery_data_present,
    validate_snapshot_metadata, validate_tail_run_metadata, validate_tail_seq_after_snapshot,
};
pub use preconditions::{
    hydrate_dimensions_positive, hydrate_events_preconditions, hydrate_snapshot_tail_has_evidence,
    hydrate_snapshot_tail_preconditions, hydrate_snapshot_tail_run_matches,
    hydrate_snapshot_tail_seq_after_snapshot,
};

use classify::reject_if_cannot_resume;
use validate::validate_snapshot_recovery_inputs;

use crate::JournalEvent;
use crate::recovery::hydrate_support::{
    apply_tail_events, decode_snapshot_slots, derive_dimensions_from_snapshot_and_tail,
};
use crate::recovery::types::{
    ActionReplayTracker, RecoveredSlotEntry, RecoveryError, RecoveryResult, RunSnapshot,
};
use vb_core::RunId;

/// Hydrates a live RunFrame from a snapshot plus ordered tail journal events.
///
/// Reconstructs the full runtime frame by decoding the snapshot's compact
/// slot/taint data and applying tail events on top.
///
/// # Errors
///
/// Returns `RecoveryError` when:
/// - Snapshot run_id does not match requested run_id
/// - Tail events contain a different run_id
/// - Tail event seq is not strictly after snapshot seq
/// - Snapshot bytes are corrupt or undecodable
/// - No snapshot and no tail events are provided
/// - Derived dimensions are zero or overflow `u16`
///
/// Hydrates a live `RunFrame` from a snapshot plus ordered tail events.
///
/// The snapshot+tail path is the **lower-level** durable hydration
/// mechanism: it consumes the snapshot bytes and reapplies tail events
/// to derive a `RunFrame` shape with hydrated slot/taint data and a
/// monotonically advanced program counter. The corresponding cannot-
/// resume witness lives at the typed boundary that consumes this
/// frame (the `RuntimeRecoveryBoundary`
/// implementation at the runtime layer; see
/// `DurableFrameRecoveryBoundary::hydrate_run_frame` and
/// `summary::recover_runtime_frame_seed_from_events`). The events-only
/// [`hydrate_run_frame_from_events`] entry point is where the typed
/// `UnsupportedFrameSeed { run, reason }` rejection is enforced, so
/// any caller passing a frame seed alone through the typed boundary
/// still fails closed with the precise reason string. The
/// snapshot+tail entry point ALSO enforces the typed cannot-resume
/// gate up-front via [`classify_snapshot_tail_cannot_resume`] so a
/// tail that would leave pending timers/asks/actions or carry a
/// `SlotWrittenEvent { value: None, .. }` is rejected with
/// `UnsupportedFrameSeed { reason }` BEFORE any frame is allocated.
/// This closes BLOCKER 4 and matches the typed-boundary contract the
/// events-only path already satisfies. The per-event-typed lower-
/// level gate is also enforced inside the tail application helpers
/// (see `hydrate_support::apply_slot_written`) so any
/// `SlotWrittenEvent { value: None }` that slips past the up-front
/// check (e.g. through a programmatic call path) is rejected by the
/// helper as defense-in-depth. A valid (snapshot slot bytes + tail
/// step lifecycle) path continues to assemble the lower-level frame
/// successfully. This keeps the unit-level durability, taint
/// preservation, and replay-shape tests green: they construct valid
/// durable evidence.
pub fn hydrate_run_frame(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
    run_id: RunId,
) -> RecoveryResult<vb_core::RunFrame> {
    reject_owned_continuation_events(tail_events)?;
    validate_snapshot_recovery_inputs(snapshot, tail_events, run_id)?;
    reject_if_cannot_resume(snapshot, tail_events, run_id)?;
    let snapshot_slots =
        decode_snapshot_slots(&snapshot.slots, &snapshot.taint, run_id, snapshot.seq)?;
    let (step_count, slot_count, first_step) =
        derive_dimensions_from_snapshot_and_tail(snapshot, tail_events, run_id, &snapshot_slots)?;
    ensure_nonzero_step_count(step_count)?;

    let mut frame = vb_core::RunFrame::new(run_id, first_step, step_count, slot_count)
        .map_err(|_| RecoveryError::FrameDimensionOverflow { run: run_id })?;
    apply_snapshot_slots(&mut frame, &snapshot_slots)?;

    let mut tracker = ActionReplayTracker::new();
    let executed = apply_tail_events(&mut frame, tail_events, &mut tracker, &[]);
    increment_executed(&mut frame, run_id, executed?)?;
    Ok(frame)
}

fn ensure_nonzero_step_count(step_count: u16) -> RecoveryResult<()> {
    if step_count == 0 {
        return Err(RecoveryError::ReplayDivergence {
            step: vb_core::StepIdx::ZERO,
            detail: "derived step_count is zero".to_owned(),
        });
    }
    Ok(())
}

fn apply_snapshot_slots(
    frame: &mut vb_core::RunFrame,
    snapshot_slots: &[RecoveredSlotEntry],
) -> RecoveryResult<()> {
    for entry in snapshot_slots {
        frame
            .write_slot_with_taint(entry.slot, entry.value, entry.taint)
            .map_err(|_| RecoveryError::ReplayDivergence {
                step: vb_core::StepIdx::ZERO,
                detail: "snapshot slot write out of bounds".to_owned(),
            })?;
    }
    Ok(())
}

fn increment_executed(
    frame: &mut vb_core::RunFrame,
    run_id: RunId,
    executed: u64,
) -> RecoveryResult<()> {
    for _ in 0..executed {
        frame
            .increment_executed()
            .map_err(|_| RecoveryError::FrameDimensionOverflow { run: run_id })?;
    }
    Ok(())
}

// A cold carrier cannot authorize either legacy hydration route. Check metadata
// only, before either route can inspect/mutate a frame or reinterpret its prefix.
fn reject_owned_continuation_events(events: &[JournalEvent]) -> RecoveryResult<()> {
    for event in events {
        if matches!(event, JournalEvent::RunContinuation { .. }) {
            return Err(RecoveryError::UnsupportedFrameSeed {
                run: event.run_id(),
                reason: String::from("owned-continuation-needs-authoritative-recovery"),
            });
        }
    }
    Ok(())
}
