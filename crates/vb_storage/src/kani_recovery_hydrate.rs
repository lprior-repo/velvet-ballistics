//! Kani harnesses for vb-jpq7.3 recovery/replay seams.
//!
//! These harnesses deliberately target allocation-free production seams that are
//! called by the storage/recovery implementation. Full Fjall and live
//! `RunFrame` hydration remain covered by behavior tests because they allocate
//! and cross storage/tooling boundaries that Kani 0.67 cannot model cleanly.

#![forbid(unsafe_code)]

use crate::journal::EventReplayLimit;
use crate::journal::replay::{ReplayPushLimitDecision, classify_replay_push_len};
use crate::recovery::hydrate::{
    SnapshotRecoveryInputViolation, TailEventMetadata, validate_recovery_data_present,
    validate_snapshot_metadata, validate_tail_run_metadata, validate_tail_seq_after_snapshot,
};
use crate::recovery::hydrate_support::{
    SlotTaintReadObservation, SlotTaintResolution, resolve_slot_taint_read,
};
use crate::{EventSeq, JournalError};
use vb_core::{RunId, Taint};

const MAX_TAIL_EVENTS: u8 = 4;
const MAX_TAIL_EVENTS_USIZE: usize = 4;

#[derive(Clone, Copy)]
struct TailMetadataBatch {
    len: u8,
    events: [TailEventMetadata; MAX_TAIL_EVENTS_USIZE],
}

impl kani::Arbitrary for TailEventMetadata {
    fn any() -> Self {
        Self::new(arbitrary_run_id(), EventSeq::new(kani::any()))
    }
}

impl kani::Arbitrary for TailMetadataBatch {
    fn any() -> Self {
        Self {
            len: kani::any::<u8>() % (MAX_TAIL_EVENTS + 1),
            events: [kani::any(), kani::any(), kani::any(), kani::any()],
        }
    }
}

fn arbitrary_run_id() -> RunId {
    RunId::new(kani::any())
}

fn arbitrary_taint() -> Taint {
    match kani::any::<u8>() % 5 {
        0 => Taint::Clean,
        1 => Taint::DerivedFromSecret,
        2 => Taint::Secret,
        3 => Taint::Secret,
        _ => Taint::Secret,
    }
}

fn tail_run_scan(
    batch: TailMetadataBatch,
    run_id: RunId,
) -> Result<(), SnapshotRecoveryInputViolation> {
    let mut seen = 0u8;
    for event in batch.events {
        if seen < batch.len {
            validate_tail_run_metadata(event, run_id)?;
        }
        seen = seen.saturating_add(1);
    }
    Ok(())
}

fn tail_seq_scan(
    batch: TailMetadataBatch,
    snapshot_seq: EventSeq,
) -> Result<(), SnapshotRecoveryInputViolation> {
    let mut seen = 0u8;
    for event in batch.events {
        if seen < batch.len {
            validate_tail_seq_after_snapshot(event, snapshot_seq)?;
        }
        seen = seen.saturating_add(1);
    }
    Ok(())
}

fn batch_has_run_mismatch(batch: TailMetadataBatch, run_id: RunId) -> bool {
    let mut seen = 0u8;
    let mut mismatch = false;
    for event in batch.events {
        if seen < batch.len && event.run != run_id {
            mismatch = true;
        }
        seen = seen.saturating_add(1);
    }
    mismatch
}

fn batch_has_seq_not_after(batch: TailMetadataBatch, snapshot_seq: EventSeq) -> bool {
    let mut seen = 0u8;
    let mut invalid = false;
    for event in batch.events {
        if seen < batch.len && event.seq.get() <= snapshot_seq.get() {
            invalid = true;
        }
        seen = seen.saturating_add(1);
    }
    invalid
}

#[kani::proof]
#[kani::unwind(8)]
fn replay_next_seq_overflow_boundary() {
    let raw: u64 = kani::any();
    let result = crate::codec::next_seq(EventSeq::new(raw));

    if raw == u64::MAX {
        kani::assert(matches!(&result, Err(JournalError::SequenceOverflow), "assertion failed"),
            "next_seq returns SequenceOverflow at u64::MAX",
        );
    } else {
        let expected = raw.checked_add(1);
        let ok = match (&result, expected) {
            (Ok(seq), Some(expected_raw)) => seq.get() == expected_raw,
            _ => false,
        };
        ,
            "next_seq returns SequenceOverflow at u64::MAX",
        );
    } else {
        let expected = raw.checked_add(1);
        let ok = match (&result, expected) {
            (Ok(seq), Some(expected_raw)) => seq.get() == expected_raw,
            _ => false,
        };
        kani::assert(ok, "next_seq increments every non-max EventSeq by one");
    }

    core::mem::forget(result);
}

#[kani::proof]
#[kani::unwind(8)]
fn replay_push_limit_decision_matches_checked_count() {
    let raw_limit: usize = kani::any();
    let current_len: usize = kani::any();
    kani::assume(raw_limit > 0);
    kani::cover!(
        raw_limit == 1,
        "limit domain includes the minimum non-zero limit"
    );

    let Some(limit) = EventReplayLimit::new(raw_limit) else {
        kani::assert(
            false,
            "positive raw_limit always constructs EventReplayLimit",
        );
        return;
    };

    let decision = classify_replay_push_len(current_len, limit);
    match current_len.checked_add(1) {
        None => match decision {
            ReplayPushLimitDecision::TooMany { observed, limit } => {
                kani::assert(
                    observed == usize::MAX,
                    "overflow reports usize::MAX observed",
                );
                kani::assert(limit == raw_limit, "overflow preserves configured limit");
            }
            ReplayPushLimitDecision::Accept { .. } => {
                kani::assert(false, "overflow cannot be accepted");
            }
        },
        Some(observed) if observed > raw_limit => match decision {
            ReplayPushLimitDecision::TooMany {
                observed: actual,
                limit,
            } => {
                kani::assert(
                    actual == observed,
                    "over-limit decision reports observed count",
                );
                kani::assert(limit == raw_limit, "over-limit decision preserves limit");
            }
            ReplayPushLimitDecision::Accept { .. } => {
                kani::assert(false, "over-limit count cannot be accepted");
            }
        },
        Some(observed) => match decision {
            ReplayPushLimitDecision::Accept { observed: actual } => {
                kani::assert(
                    actual == observed,
                    "accepted decision reports next observed count",
                );
            }
            ReplayPushLimitDecision::TooMany { .. } => {
                kani::assert(false, "within-limit count cannot be rejected");
            }
        },
    }
}

#[kani::proof]
#[kani::unwind(8)]
fn snapshot_metadata_rejects_run_mismatch() {
    let snapshot_run = arbitrary_run_id();
    let run_id = arbitrary_run_id();
    let snapshot_seq: EventSeq = kani::any();
    kani::assume(snapshot_run != run_id);
    kani::cover!(snapshot_seq.get() == 0, "snapshot seq domain includes zero");

    let result = validate_snapshot_metadata(snapshot_run, snapshot_seq, run_id);

    match result {
        Err(SnapshotRecoveryInputViolation::SnapshotRunMismatch {
            snapshot_run: actual_run,
            snapshot_seq: actual_seq,
        }) => {
             == 0, "snapshot seq domain includes zero");

    let result = validate_snapshot_metadata(snapshot_run, snapshot_seq, run_id);

    match result {
        Err(SnapshotRecoveryInputViolation::SnapshotRunMismatch {
            snapshot_run: actual_run,
            snapshot_seq: actual_seq,
        }) => {
            kani::assert(
                actual_run == snapshot_run,
                "snapshot mismatch preserves run",
            );
            kani::assert(
                actual_seq == snapshot_seq,
                "snapshot mismatch preserves seq",
            );
        }
        _ => kani::assert(false, "snapshot run mismatch must be rejected"),
    }
}

#[kani::proof]
#[kani::unwind(12)]
fn tail_run_scan_matches_any_metadata_batch_len_le_4() {
    let batch: TailMetadataBatch = kani::any();
    let run_id = arbitrary_run_id();
    let has_mismatch = batch_has_run_mismatch(batch, run_id);
    kani::cover!(batch.len == 0, "tail run scan covers empty batch");
    kani::cover!(
        batch.len == MAX_TAIL_EVENTS,
        "tail run scan covers max batch"
    );

    let result = tail_run_scan(batch, run_id);

    match (has_mismatch, result) {
        (true, Err(SnapshotRecoveryInputViolation::TailRunMismatch { .. })) => {}
        (false, Ok(())) => {}
        _ => ) => {}
        (false, Ok(())) => {}
        _ => kani::assert(
            false,
            "tail run scan result matches metadata mismatch predicate",
        ),
    }
}

#[kani::proof]
#[kani::unwind(12)]
fn tail_seq_scan_matches_any_metadata_batch_len_le_4() {
    let batch: TailMetadataBatch = kani::any();
    let snapshot_seq: EventSeq = kani::any();
    let has_invalid_seq = batch_has_seq_not_after(batch, snapshot_seq);
    kani::cover!(batch.len == 0, "tail seq scan covers empty batch");
    kani::cover!(
        batch.len == MAX_TAIL_EVENTS,
        "tail seq scan covers max batch"
    );

    let result = tail_seq_scan(batch, snapshot_seq);

    match (has_invalid_seq, result) {
        (true, Err(SnapshotRecoveryInputViolation::TailSeqNotAfterSnapshot { .. })) => {}
        (false, Ok(())) => {}
        _ => ) => {}
        (false, Ok(())) => {}
        _ => kani::assert(false, "tail seq scan result matches sequence predicate"),
    }
}

#[kani::proof]
#[kani::unwind(8)]
fn recovery_data_presence_rejects_only_all_empty() {
    let tail_empty: bool = kani::any();
    let slots_empty: bool = kani::any();
    let taint_empty: bool = kani::any();
    let run_id = arbitrary_run_id();
    let all_empty = tail_empty && slots_empty && taint_empty;

    let result = validate_recovery_data_present(tail_empty, slots_empty, taint_empty, run_id);

    match (all_empty, result) {
        (true, Err(SnapshotRecoveryInputViolation::NoRecoveryData { run })) => {
            ) => {
            kani::assert(run == run_id, "NoRecoveryData preserves requested run");
        }
        (false, Ok(())) => {}
        _ => ) => {}
        _ => kani::assert(false, "recovery data presence rejects only all-empty input"),
    }
}

#[kani::proof]
#[kani::unwind(8)]
fn slot_taint_resolution_fails_closed_on_read_failure() {
    let decision = resolve_slot_taint_read(SlotTaintReadObservation::Failed);

    kani::assert(
        matches!(decision, SlotTaintResolution::FailClosed),
        "failed taint reads are never downgraded to Clean",
    );
}

#[kani::proof]
#[kani::unwind(8)]
fn slot_taint_resolution_defaults_clean_only_for_uninitialized() {
    let decision = resolve_slot_taint_read(SlotTaintReadObservation::Uninitialized);

    kani::assert(matches!(decision, SlotTaintResolution::Use(Taint::Clean), "assertion failed"),
        "uninitialized slots are the only Clean default path",
    );
}

#[kani::proof]
#[kani::unwind(8)]
fn slot_taint_resolution_preserves_existing_taint() {
    let taint = arbitrary_taint();
    let decision = resolve_slot_taint_read(SlotTaintReadObservation::Existing(taint));

    match decision {
        SlotTaintResolution::Use(actual) => {
            ,
        "uninitialized slots are the only Clean default path",
    );
}

#[kani::proof]
#[kani::unwind(8)]
fn slot_taint_resolution_preserves_existing_taint() {
    let taint = arbitrary_taint();
    let decision = resolve_slot_taint_read(SlotTaintReadObservation::Existing(taint));

    match decision {
        SlotTaintResolution::Use(actual) => {
            kani::assert(actual == taint, "existing taint is preserved exactly");
        }
        SlotTaintResolution::FailClosed => {
            actual == taint, "existing taint is preserved exactly");
        }
        SlotTaintResolution::FailClosed => {
            kani::assert(false, "successful taint reads must not fail closed");
        }
    }
}
