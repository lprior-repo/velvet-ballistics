#![forbid(unsafe_code)]
//! Bead `vb-s25h2` — recovery entry points must read pre-snapshot events.
//!
//! Master §18 (Fjall Persistence Behavior), persistence invariant 3:
//! "Recovery replays snapshots plus tail journal or full journal
//! deterministically." Each of the 6 public recovery entry points in
//! `recovery::recover` is asserted here to surface pre-snapshot events
//! (RunAccepted, RunAdmission, step / action lifecycle prior to the
//! durable snapshot) so that recovery is complete even when a snapshot
//! is present. Without `events_for_run_full`, the snapshot+tail reader
//! skips these events and every entry point below would either return
//! `NoRecoveryData` or refuse to produce a hydration product.
//!
//! Each test writes a `RunAccepted` (and any required sibling) event at
//! seq 0, appends lifecycle events, places a durable snapshot at seq >= 1,
//! and asserts the recovery entry point can still observe the pre-snapshot
//! events. The companion test `events_for_run_full_returns_pre_snapshot_events`
//! in `vb_s25h2_full_journal_reader.rs` proves the underlying reader.

use std::collections::BTreeSet;

use vb_core::{CapabilitySet, RunId, RuntimePolicy, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};
use vb_storage::recovery::recover::{
    check_workflow_source_digest, recover_all_incomplete_runs, recover_run_admission,
    recover_runtime_frame_seed, recover_runtime_summary,
    recover_runtime_summary_with_expected,
};
use vb_storage::recovery::types::{RecoveryHydration, RecoveryTerminalState, RunSnapshot};
use vb_storage::records::{RunHeaderRecord, RunHeaderStatus};
use vb_storage::{EventSeq, FjallJournal, JournalEvent, constants::DIGEST_BYTES};

fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
    (temp, journal)
}

fn digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; DIGEST_BYTES])
}

fn run_accepted(run: RunId, seq: u64, wf: WorkflowDigest) -> JournalEvent {
    JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(seq),
        workflow: wf,
    }
}

fn run_admission(run: RunId, seq: u64, artifact: WorkflowDigest) -> JournalEvent {
    JournalEvent::RunAdmission {
        run,
        seq: EventSeq::new(seq),
        artifact_digest: artifact,
        granted_capabilities: CapabilitySet::empty(),
        policy: RuntimePolicy::Relaxed,
    }
}

fn step_started(run: RunId, seq: u64, step: u16) -> JournalEvent {
    JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        attempt: 1,
    }
}

fn step_succeeded(run: RunId, seq: u64, step: u16, output: u16) -> JournalEvent {
    JournalEvent::StepSucceeded {
        run,
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        output: SlotIdx::new(output),
    }
}

fn make_snapshot(run: RunId, seq: u64, wf: WorkflowDigest) -> RunSnapshot {
    RunSnapshot {
        run,
        seq: EventSeq::new(seq),
        workflow: wf,
        slots: vec![0, 1, 2],
        taint: vec![0, 0, 0],
    }
}

fn make_header(run: RunId, workflow_id: u32, compiled_digest: WorkflowDigest) -> RunHeaderRecord {
    RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(workflow_id),
        compiled_digest,
        status: RunHeaderStatus::ACCEPTED.as_byte(),
        accepted_at_ms: 0,
    }
}

#[test]
fn check_workflow_source_digest_finds_pre_snapshot_run_accepted(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(30_001);
    let wf = digest(0xA1);
    journal.append_journaled(&run_accepted(run, 0, wf))?;
    journal.append_journaled(&step_started(run, 1, 0))?;
    journal.append_journaled(&step_succeeded(run, 2, 0, 0))?;
    journal.put_snapshot(&make_snapshot(run, 2, wf))?;
    check_workflow_source_digest(&journal, run, wf)?;
    Ok(())
}

#[test]
fn recover_runtime_summary_includes_pre_snapshot_events(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(30_002);
    let wf = digest(0xA2);
    journal.append_journaled(&run_accepted(run, 0, wf))?;
    journal.append_journaled(&step_started(run, 1, 0))?;
    journal.append_journaled(&step_succeeded(run, 2, 0, 0))?;
    journal.put_snapshot(&make_snapshot(run, 2, wf))?;

    let hydration = recover_runtime_summary(&journal, run)?;
    match hydration {
        RecoveryHydration::Summary(summary) => {
            assert_eq!(summary.run, run);
            assert_eq!(summary.workflow, Some(wf));
            assert_eq!(summary.steps_started, 1);
            assert_eq!(summary.steps_succeeded, 1);
            assert_eq!(summary.first_seq, EventSeq::new(0));
            assert_eq!(summary.last_seq, EventSeq::new(2));
        }
        other => return Err(format!("expected Summary hydration, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn recover_runtime_summary_with_expected_sees_pre_snapshot_finish(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(30_003);
    let wf = digest(0xA3);
    journal.append_journaled(&run_accepted(run, 0, wf))?;
    journal.append_journaled(&step_started(run, 1, 0))?;
    journal.append_journaled(&step_succeeded(run, 2, 0, 0))?;
    journal.append_journaled(&JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(3),
        result: SlotIdx::new(0),
        attempt: 1,
    })?;
    journal.put_snapshot(&make_snapshot(run, 2, wf))?;

    let expected = RecoveryTerminalState::Finished {
        result: SlotIdx::new(0),
    };
    let hydration =
        recover_runtime_summary_with_expected(&journal, run, expected)?;
    assert_eq!(hydration.summary().terminal, Some(expected));
    Ok(())
}

#[test]
fn recover_runtime_frame_seed_includes_pre_snapshot_events(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(30_004);
    let wf = digest(0xA4);
    journal.append_journaled(&run_accepted(run, 0, wf))?;
    journal.append_journaled(&step_started(run, 1, 0))?;
    journal.append_journaled(&step_succeeded(run, 2, 0, 0))?;
    journal.put_snapshot(&make_snapshot(run, 2, wf))?;

    let seed = recover_runtime_frame_seed(&journal, run)?;
    assert_eq!(seed.summary.workflow, Some(wf));
    assert_eq!(seed.summary.steps_started, 1);
    assert_eq!(seed.summary.steps_succeeded, 1);
    assert_eq!(seed.summary.first_seq, EventSeq::new(0));
    Ok(())
}

#[test]
fn recover_run_admission_finds_pre_snapshot_admission_event(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(30_005);
    let wf = digest(0xA5);
    let artifact = digest(0xA6);
    journal.append_journaled(&run_accepted(run, 0, wf))?;
    journal.append_journaled(&run_admission(run, 1, artifact))?;
    journal.append_journaled(&step_started(run, 2, 0))?;
    journal.put_snapshot(&make_snapshot(run, 2, wf))?;

    let admission = recover_run_admission(&journal, run)?
        .ok_or_else(|| "expected Some(RecoveredRunAdmission), got None")?;
    assert_eq!(admission.run_id, run);
    assert_eq!(admission.artifact_digest, artifact);
    Ok(())
}

#[test]
fn recover_all_incomplete_runs_finds_pre_snapshot_events_per_run(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, journal) = temp_journal();
    let run_a = RunId::new(30_006);
    let run_b = RunId::new(30_007);
    let wf_a = digest(0xA7);
    let wf_b = digest(0xA8);
    for (run, wf, workflow_id) in [
        (run_a, wf_a, 60_u32),
        (run_b, wf_b, 61_u32),
    ] {
        journal.append_journaled(&run_accepted(run, 0, wf))?;
        journal.append_journaled(&step_started(run, 1, 0))?;
        journal.append_journaled(&step_succeeded(run, 2, 0, 0))?;
        journal.put_run_header(&make_header(run, workflow_id, wf))?;
        journal.put_snapshot(&make_snapshot(run, 2, wf))?;
    }

    let incomplete = recover_all_incomplete_runs(&journal)?;
    assert_eq!(incomplete.len(), 2);
    let runs: BTreeSet<RunId> = incomplete.iter().map(|h| h.summary().run).collect();
    assert!(runs.contains(&run_a));
    assert!(runs.contains(&run_b));
    Ok(())
}
