#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_core::{RunId, SlotIdx, SlotValue, StepIdx, Taint, WorkflowDigest};
use vb_storage::recovery::RecoveryError;
use vb_storage::recovery::hydrate::{hydrate_dimensions_positive, hydrate_run_frame};
use vb_storage::{EventSeq, JournalEvent};

const MODE_OK: u8 = 0;
const MODE_RUN_MISMATCH: u8 = 1;
const MODE_CORRUPT_SNAPSHOT: u8 = 2;
const MODE_TAIL_GAP: u8 = 3;
const MODE_NO_DATA: u8 = 4;

fuzz_target!(|data: &[u8]| {
    let mode = data.first().copied().unwrap_or(MODE_OK) % 5;
    let step = data.get(1).copied().map_or(1, u16::from).saturating_add(1);
    let slot = data.get(2).copied().map_or(1, u16::from).saturating_add(1);
    assert!(hydrate_dimensions_positive(step, slot));

    let run = RunId::new(u64::from(data.get(3).copied().unwrap_or(0)).saturating_add(1));
    let slot = SlotIdx::new(slot);
    let snapshot = snapshot_for_mode(mode, run, slot);
    let tail = tail_for_mode(mode, run, step);
    let requested_run = if mode == MODE_RUN_MISMATCH {
        next_run(run)
    } else {
        run
    };
    observe_snapshot_tail_result(
        hydrate_run_frame(&snapshot, &tail, requested_run),
        mode,
        run,
    );
});

fn snapshot_for_mode(mode: u8, run: RunId, slot: SlotIdx) -> vb_storage::RunSnapshot {
    let snapshot_seq = EventSeq::new(0);
    let (slots, taint) = match mode {
        MODE_CORRUPT_SNAPSHOT => (vec![0xFF], vec![0xFF]),
        MODE_NO_DATA => (Vec::new(), Vec::new()),
        MODE_OK | MODE_RUN_MISMATCH | MODE_TAIL_GAP => encode_snapshot_slot(slot)
            .map(|encoded| (encoded.clone(), encoded))
            .unwrap_or_else(|| (Vec::new(), Vec::new())),
        _ => (Vec::new(), Vec::new()),
    };
    vb_storage::RunSnapshot {
        run,
        seq: snapshot_seq,
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
        slots,
        taint,
    }
}

fn encode_snapshot_slot(slot: SlotIdx) -> Option<Vec<u8>> {
    let entries = vec![(slot, SlotValue::I64(i64::from(slot.get())), Taint::Clean)];
    postcard::to_allocvec(&entries).ok()
}

fn tail_for_mode(mode: u8, run: RunId, step: u16) -> Vec<JournalEvent> {
    if mode == MODE_NO_DATA {
        return Vec::new();
    }
    let seq = if mode == MODE_TAIL_GAP { 2 } else { 1 };
    vec![JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        attempt: 1,
    }]
}

fn next_run(run: RunId) -> RunId {
    match run.get().checked_add(1) {
        Some(value) => RunId::new(value),
        None => RunId::new(1),
    }
}

fn observe_snapshot_tail_result(
    result: Result<vb_core::RunFrame, RecoveryError>,
    mode: u8,
    run: RunId,
) {
    match mode {
        MODE_OK => assert_hydrated(result, run),
        MODE_RUN_MISMATCH | MODE_CORRUPT_SNAPSHOT => {
            assert!(
                matches!(result, Err(RecoveryError::CorruptSnapshot { .. })),
                "run mismatch or corrupt snapshot bytes must fail as CorruptSnapshot"
            );
        }
        MODE_TAIL_GAP => {
            assert!(
                matches!(result, Err(RecoveryError::ReplayDivergence { .. })),
                "tail sequence gap must fail as ReplayDivergence"
            );
        }
        MODE_NO_DATA => {
            assert!(
                matches!(result, Err(RecoveryError::NoRecoveryData { .. })),
                "empty snapshot plus empty tail must fail as NoRecoveryData"
            );
        }
        _ => {
            assert!(
                matches!(
                    mode,
                    MODE_OK
                        | MODE_RUN_MISMATCH
                        | MODE_CORRUPT_SNAPSHOT
                        | MODE_TAIL_GAP
                        | MODE_NO_DATA
                ),
                "snapshot-tail mode must be normalized before observation"
            );
        }
    }
}

fn assert_hydrated(result: Result<vb_core::RunFrame, RecoveryError>, run: RunId) {
    assert!(
        result.is_ok(),
        "valid snapshot plus tail evidence must hydrate"
    );
    let Ok(frame) = result else {
        return;
    };
    assert_eq!(
        frame.run_id(),
        run,
        "hydrated frame must keep snapshot run id"
    );
}
