use super::support::*;
use vb_core::{RunId, WorkflowDigest};
use vb_storage::recovery::{RecoveryError, hydrate_run_frame, hydrate_run_frame_from_events};
use vb_storage::{EventSeq, JournalEvent, RunSnapshot};

const REASON: &str = "owned-continuation-needs-authoritative-recovery";

#[test]
fn both_public_hydration_paths_refuse_first_carrier_with_its_actual_run() -> TestResult {
    let (state, _, _, _) = fixture(false, 3)?;
    let snapshot = empty_snapshot(state.workflow.digest());
    let original_snapshot = snapshot.clone();
    let events = carrier_tail(state.workflow.digest())?;
    let original_events = events.clone();
    // Mismatched caller and earlier bad ordinary prefix cannot replace actual
    // carrier identity or cause old snapshot/seed errors to hide the refusal.
    for caller in [RunId::new(17), RunId::new(555)] {
        assert_refusal(hydrate_run_frame(&snapshot, &events, caller))?;
        assert_refusal(hydrate_run_frame_from_events(&events, caller))?;
    }
    assert_eq!(snapshot, original_snapshot);
    assert_eq!(events, original_events);
    Ok(())
}

fn assert_refusal(result: Result<vb_core::RunFrame, RecoveryError>) -> TestResult {
    match result {
        Err(RecoveryError::UnsupportedFrameSeed { run, reason }) => {
            assert_eq!(run, RunId::new(17));
            assert_eq!(reason, REASON);
        }
        other => return Err(format!("expected exact carrier refusal, got {other:?}").into()),
    }
    Ok(())
}

fn carrier_tail(workflow: WorkflowDigest) -> TestResult<Vec<JournalEvent>> {
    let first = event(2)?;
    let JournalEvent::RunContinuation { carrier } = &first else {
        return Err("missing carrier".into());
    };
    // This second wire-valid scalar identity is NOT a component/runtime
    // authority claim. It detects last-carrier/caller-run substitution.
    let header = vb_storage::ContinuationCarrierHeader::new(
        RunId::new(18),
        EventSeq::new(6),
        2,
        EventSeq::new(5),
        carrier.header().component_digest(),
    )?;
    let record =
        vb_storage::encode_owned_continuation(header, carrier.component(), &mut budget()?)?;
    let mut operation = budget()?;
    let second = vb_storage::decode_owned_continuation(
        &key(header)?,
        header.run(),
        record.as_bytes(),
        &mut operation,
    )?
    .into_event(&mut operation)?;
    Ok(vec![
        JournalEvent::RunAccepted {
            run: RunId::new(999),
            seq: EventSeq::new(0),
            workflow,
        },
        first,
        second,
    ])
}

fn empty_snapshot(workflow: WorkflowDigest) -> RunSnapshot {
    RunSnapshot {
        run: RunId::new(17),
        seq: EventSeq::new(0),
        workflow,
        slots: Vec::new(),
        taint: Vec::new(),
    }
}

#[test]
fn no_carrier_keeps_old_empty_and_snapshot_identity_errors_literal() -> TestResult {
    let (state, _, _, _) = fixture(false, 3)?;
    let snapshot = empty_snapshot(state.workflow.digest());
    let run = RunId::new(17);
    let mismatch = RunId::new(555);
    assert!(matches!(hydrate_run_frame_from_events(&[], mismatch),
        Err(RecoveryError::NoRecoveryData { run: actual }) if actual == mismatch));
    assert!(matches!(hydrate_run_frame(&snapshot, &[], run),
        Err(RecoveryError::NoRecoveryData { run: actual }) if actual == run));
    assert!(matches!(hydrate_run_frame(&snapshot, &[], mismatch),
        Err(RecoveryError::CorruptSnapshot { run: actual, seq })
            if actual == mismatch && seq == EventSeq::new(0)));
    Ok(())
}
