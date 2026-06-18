#![forbid(unsafe_code)]
//! Snapshot writing for recovered runtime frame seeds.

use crate::FjallJournal;
use crate::recovery::{RecoveryError, RecoveryFrameSeed, RecoveryResult, RunSnapshot};
use vb_core::{SlotIdx, SlotValue, Taint};

/// Persists a compact [`RunSnapshot`] derived from a fully-supported
/// [`RecoveryFrameSeed`].
///
/// The seed must be fully supported (`unsupported.is_fully_supported()`) and
/// must not contain unresolved pending actions. Pending actions are valid live
/// recovery seed state, but the compact snapshot format cannot faithfully store
/// them, so this function rejects such seeds with `ReplayDivergence`.
pub fn write_recovered_snapshot(
    journal: &FjallJournal,
    seed: &RecoveryFrameSeed,
) -> RecoveryResult<()> {
    reject_unsnapshotable_seed(seed)?;
    let workflow = seed.summary.workflow.ok_or(RecoveryError::NoRecoveryData {
        run: seed.summary.run,
    })?;
    let slot_entries = snapshot_slot_entries(seed);
    let slots = encode_snapshot_slots(seed, &slot_entries)?;
    let taint = encode_snapshot_taint(seed, &slot_entries)?;
    journal.put_snapshot(&RunSnapshot {
        run: seed.summary.run,
        seq: seed.summary.last_seq,
        workflow,
        slots,
        taint,
    })?;
    Ok(())
}

fn reject_unsnapshotable_seed(seed: &RecoveryFrameSeed) -> RecoveryResult<()> {
    if !seed.unsupported.is_fully_supported() {
        return Err(RecoveryError::ReplayDivergence {
            step: seed.pc,
            detail: "snapshot write rejected: seed has unsupported recovery state".to_owned(),
        });
    }
    Ok(())
}

fn snapshot_slot_entries(seed: &RecoveryFrameSeed) -> Vec<(SlotIdx, SlotValue, Taint)> {
    seed.slots
        .iter()
        .map(|entry| (entry.slot, entry.value, entry.taint))
        .collect()
}

fn encode_snapshot_slots(
    seed: &RecoveryFrameSeed,
    slot_entries: &[(SlotIdx, SlotValue, Taint)],
) -> RecoveryResult<Vec<u8>> {
    postcard::to_allocvec(slot_entries).map_err(|error| RecoveryError::ReplayDivergence {
        step: seed.pc,
        detail: format!("snapshot slot encode failed: {error}"),
    })
}

fn encode_snapshot_taint(
    seed: &RecoveryFrameSeed,
    slot_entries: &[(SlotIdx, SlotValue, Taint)],
) -> RecoveryResult<Vec<u8>> {
    postcard::to_allocvec(slot_entries).map_err(|error| RecoveryError::ReplayDivergence {
        step: seed.pc,
        detail: format!("snapshot taint encode failed: {error}"),
    })
}
