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
    // SR-019: project slots and taint into separate encodings so the on-disk
    // format carries distinct information per field. The decoder merges the two
    // vectors by `SlotIdx`, so a divergent taint entry actually changes the
    // snapshot bytes (instead of being a duplicate of the slots payload).
    let slot_value_entries = snapshot_slot_value_entries(seed);
    let slot_taint_entries = snapshot_slot_taint_entries(seed);
    let slots = encode_snapshot_slots(seed, &slot_value_entries)?;
    let taint = encode_snapshot_taint(seed, &slot_taint_entries)?;
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

fn snapshot_slot_value_entries(seed: &RecoveryFrameSeed) -> Vec<(SlotIdx, SlotValue)> {
    seed.slots
        .iter()
        .map(|entry| (entry.slot, entry.value))
        .collect()
}

fn snapshot_slot_taint_entries(seed: &RecoveryFrameSeed) -> Vec<(SlotIdx, Taint)> {
    seed.slots
        .iter()
        .map(|entry| (entry.slot, entry.taint))
        .collect()
}

fn encode_snapshot_slots(
    seed: &RecoveryFrameSeed,
    slot_value_entries: &[(SlotIdx, SlotValue)],
) -> RecoveryResult<Vec<u8>> {
    postcard::to_allocvec(slot_value_entries).map_err(|error| RecoveryError::ReplayDivergence {
        step: seed.pc,
        detail: format!("snapshot slot encode failed: {error}"),
    })
}

fn encode_snapshot_taint(
    seed: &RecoveryFrameSeed,
    slot_taint_entries: &[(SlotIdx, Taint)],
) -> RecoveryResult<Vec<u8>> {
    postcard::to_allocvec(slot_taint_entries).map_err(|error| RecoveryError::ReplayDivergence {
        step: seed.pc,
        detail: format!("snapshot taint encode failed: {error}"),
    })
}

#[cfg(test)]
mod snapshot_write_sr_019_tests {
    use super::*;
    use crate::recovery::RecoveredSlotEntry;
    use crate::recovery::types::state::{RecoveryRuntimeSummary, UnsupportedRecoveryState};
    use vb_core::{RunId, StepIdx, Taint as CoreTaint, WorkflowDigest};

    /// SR-019: divergent taint must change the on-disk bytes.
    ///
    /// Before the fix, `encode_snapshot_slots` and `encode_snapshot_taint`
    /// both encoded the full `(SlotIdx, SlotValue, Taint)` triple, so the two
    /// payloads were byte-identical for any input — the taint field never
    /// actually diverged from the slots payload. After the fix, slots are
    /// projected to `(SlotIdx, SlotValue)` and taint to `(SlotIdx, Taint)`,
    /// so flipping one slot's taint MUST change the taint bytes while leaving
    /// the slots bytes untouched.
    #[test]
    fn sr_019_divergent_taint_changes_taint_bytes_but_not_slot_bytes() {
        let run = RunId::new(9001);
        let workflow = WorkflowDigest::from_bytes([1_u8; 32]);
        let summary = RecoveryRuntimeSummary {
            run,
            first_seq: crate::EventSeq::new(0),
            last_seq: crate::EventSeq::new(0),
            workflow: Some(workflow),
            steps_started: 0,
            steps_succeeded: 0,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 0,
            terminal: None,
        };
        let slot_entries = vec![
            (SlotIdx::new(0), SlotValue::I64(42), Taint::Clean),
            (SlotIdx::new(1), SlotValue::I64(7), Taint::Clean),
        ];
        let seed = RecoveryFrameSeed {
            summary,
            first_step: StepIdx::new(0),
            step_count: 0,
            slot_count: 0,
            pc: StepIdx::new(0),
            steps: Vec::new(),
            slots: slot_entries
                .iter()
                .map(|(s, v, t)| RecoveredSlotEntry {
                    slot: *s,
                    value: *v,
                    taint: *t,
                })
                .collect(),
            unsupported: UnsupportedRecoveryState::SUPPORTED,
        };

        let slot_value_entries = snapshot_slot_value_entries(&seed);
        let slot_taint_entries = snapshot_slot_taint_entries(&seed);
        let slots_bytes_clean =
            encode_snapshot_slots(&seed, &slot_value_entries).expect("slots encode succeeds");
        let taint_bytes_clean = encode_snapshot_taint(&seed, &slot_taint_entries)
            .expect("taint encode succeeds (clean)");
        // Before the fix these were byte-identical; after the fix they must
        // differ in shape because they carry different projections.
        assert_ne!(
            slots_bytes_clean, taint_bytes_clean,
            "SR-019: slots and taint payloads must carry distinct projections, not duplicate bytes"
        );

        // Now flip one slot's taint and re-encode. The slots bytes must
        // remain identical (the slot-value projection is untouched), but the
        // taint bytes MUST change.
        let mut dirty_seed = seed.clone();
        if let Some(entry) = dirty_seed.slots.first_mut() {
            entry.taint = CoreTaint::Secret;
        }
        let slots_bytes_dirty =
            encode_snapshot_slots(&dirty_seed, &snapshot_slot_value_entries(&dirty_seed))
                .expect("slots encode succeeds (dirty)");
        let taint_bytes_dirty =
            encode_snapshot_taint(&dirty_seed, &snapshot_slot_taint_entries(&dirty_seed))
                .expect("taint encode succeeds (dirty)");
        assert_eq!(
            slots_bytes_clean, slots_bytes_dirty,
            "SR-019: flipping taint must NOT change slots bytes"
        );
        assert_ne!(
            taint_bytes_clean, taint_bytes_dirty,
            "SR-019: flipping taint MUST change taint bytes (the bug was that they were duplicates)"
        );
    }

    /// SR-019 end-to-end: `write_recovered_snapshot` must produce on-disk
    /// bytes whose `slots` and `taint` fields carry distinct information,
    /// and `hydrate_run_frame` must propagate a divergent taint through the
    /// real journal round-trip.
    ///
    /// The focused test above exercises the encoder helpers in isolation.
    /// This test pins the public-API wiring: a regression that re-merged the
    /// two projections in the writer's outer call would still pass the helper
    /// test but fail here.
    #[test]
    fn sr_019_write_recovered_snapshot_round_trip_propagates_divergent_taint() {
        use crate::recovery::hydrate::hydrate_run_frame;

        let dir = tempfile::tempdir().expect("temp dir");
        let journal = crate::FjallJournal::open(dir.path(), None).expect("journal opens");
        let run = RunId::new(9100);
        let workflow = WorkflowDigest::from_bytes([2_u8; 32]);
        let summary = RecoveryRuntimeSummary {
            run,
            first_seq: crate::EventSeq::new(0),
            last_seq: crate::EventSeq::new(0),
            workflow: Some(workflow),
            steps_started: 0,
            steps_succeeded: 0,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 0,
            terminal: None,
        };
        // Build a seed with two slots: slot 0 carries `Secret` taint, slot 1
        // stays `Clean`. Pre-fix the on-disk taint payload was a duplicate of
        // the slots payload, so the divergent taint could not influence
        // hydration. Post-fix the decoder's taint vector carries the
        // divergence all the way through `hydrate_run_frame` to
        // `frame.read_taint(...)`.
        let seed = RecoveryFrameSeed {
            summary,
            first_step: StepIdx::new(0),
            step_count: 1,
            slot_count: 2,
            pc: StepIdx::new(0),
            steps: Vec::new(),
            slots: vec![
                RecoveredSlotEntry {
                    slot: SlotIdx::new(0),
                    value: SlotValue::I64(100),
                    taint: CoreTaint::Secret,
                },
                RecoveredSlotEntry {
                    slot: SlotIdx::new(1),
                    value: SlotValue::I64(200),
                    taint: CoreTaint::Clean,
                },
            ],
            unsupported: UnsupportedRecoveryState::SUPPORTED,
        };

        write_recovered_snapshot(&journal, &seed).expect("write_recovered_snapshot succeeds");

        // Read the snapshot back through the public journal API. The wire
        // bytes MUST show distinct slots/taint projections — this is the
        // SR-019 invariant on the on-disk format.
        let stored = journal
            .snapshot(run, crate::EventSeq::new(0))
            .expect("snapshot read succeeds")
            .expect("snapshot present");
        assert_ne!(
            stored.slots, stored.taint,
            "SR-019: on-disk slots and taint payloads must carry distinct projections"
        );

        // Round-trip the snapshot through `hydrate_run_frame`. Provide a
        // minimal tail event so `derive_dimensions_from_snapshot_and_tail`
        // produces a non-zero step count (the hydration boundary rejects
        // step_count == 0).
        let tail = vec![crate::JournalEvent::StepStarted {
            run,
            seq: crate::EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        }];
        let frame = hydrate_run_frame(&stored, &tail, run).expect("hydrate_run_frame succeeds");
        let taint_0 = frame
            .read_taint(SlotIdx::new(0))
            .expect("slot 0 taint readable");
        let taint_1 = frame
            .read_taint(SlotIdx::new(1))
            .expect("slot 1 taint readable");
        assert_eq!(
            taint_0,
            CoreTaint::Secret,
            "SR-019: divergent taint from the taint vector must override the slots default"
        );
        assert_eq!(
            taint_1,
            CoreTaint::Clean,
            "SR-019: clean taint must round-trip as Clean when no override is supplied"
        );
    }
}
