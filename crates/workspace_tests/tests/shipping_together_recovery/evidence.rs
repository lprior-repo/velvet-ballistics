#![forbid(unsafe_code)]
//! Typed rejection of missing, contradictory or corrupt Together evidence.

use super::support::*;
use vb_core::{SlotValue, WorkflowDigest};
use vb_storage::recovery::{
    ActionReplayTracker, RecoveryError, RecoveryTerminalState, hydrate_run_frame_from_events,
    recover_raw_runtime_frame_seed_from_events_with_workflow,
    recover_runtime_summary_with_expected, replay_events, summarize_recovery_events,
};
use vb_storage::{EventSeq, JournalEvent};

#[test]
fn missing_or_contradictory_together_evidence_is_rejected() -> TestResult {
    let live = live(FANOUT, Box::new([]))?;
    let plan = live.plan.clone();
    let events = events(&live)?;

    let without_accept = events
        .iter()
        .filter(|event| !matches!(event, JournalEvent::RunAccepted { .. }))
        .cloned()
        .collect::<Vec<_>>();
    assert!(matches!(
        recover_raw_runtime_frame_seed_from_events_with_workflow(&without_accept, &plan),
        Err(RecoveryError::ReplayDivergence { .. })
    ));

    let mut wrong_digest = events.clone();
    for event in &mut wrong_digest {
        if let JournalEvent::RunAccepted { workflow, .. } = event {
            *workflow = WorkflowDigest::from_bytes([0xAB; 32]);
        }
    }
    assert!(matches!(
        recover_raw_runtime_frame_seed_from_events_with_workflow(&wrong_digest, &plan),
        Err(RecoveryError::CompiledIrDigestMismatch { .. })
    ));

    let mut gap = events.clone();
    gap.remove(1);
    let mut tracker = ActionReplayTracker::new();
    assert!(matches!(
        replay_events(&gap, &mut tracker, &[]),
        Err(RecoveryError::ReplayDivergence { .. })
    ));

    let without_terminal = events
        .iter()
        .filter(|event| !matches!(event, JournalEvent::RunFinished { .. }))
        .cloned()
        .collect::<Vec<_>>();
    let join_slot = finish_result(&plan)?;
    let (_directory, truncated) = journal_with(&without_terminal)?;
    assert!(matches!(
        recover_runtime_summary_with_expected(
            &truncated,
            RUN,
            RecoveryTerminalState::Finished { result: join_slot }
        ),
        Err(RecoveryError::TerminalStateMismatch { .. })
    ));

    let mut contradictory = events.clone();
    contradictory.push(JournalEvent::RunFailedEvent {
        run: RUN,
        seq: EventSeq::new(contradictory.len() as u64),
        attempt: 1,
    });
    assert!(matches!(
        summarize_recovery_events(&contradictory).map(|hydration| hydration.summary().terminal),
        Ok(Some(RecoveryTerminalState::Failed))
    ));
    Ok(())
}

#[test]
fn corrupt_join_slot_capture_is_rejected() -> TestResult {
    let live = live(FANOUT, Box::new([]))?;
    let plan = live.plan.clone();
    let mut events = events(&live)?;
    let join_slot = finish_result(&plan)?;
    let mut corrupted = false;
    for event in &mut events {
        if let JournalEvent::SlotWrittenEvent { slot, value, .. } = event
            && *slot == join_slot
        {
            *value = Some(vec![0xFF, 0xFF, 0xFF, 0xFF]);
            corrupted = true;
        }
    }
    assert!(corrupted, "no join SlotWrittenEvent to corrupt");
    let seed = recover_raw_runtime_frame_seed_from_events_with_workflow(&events, &plan)?;
    assert!(
        seed.unsupported.slot_values,
        "corrupt composite capture must fail closed as unsupported slot values"
    );
    assert!(
        !seed
            .slots
            .iter()
            .any(|entry| entry.slot == join_slot && matches!(entry.value, SlotValue::List(_))),
        "corrupt capture must not fabricate a reusable join list"
    );
    assert!(matches!(
        hydrate_run_frame_from_events(&events, RUN),
        Err(RecoveryError::UnsupportedFrameSeed { .. })
    ));
    Ok(())
}

#[test]
fn nested_together_rejection_is_preserved() -> TestResult {
    let nested = r#"version: velvet-ballistics/v1
name: nested_together_rejection
when: {manual: {}}
steps:
  - id: outer
    together:
      branches:
        - label: first
          steps:
            - id: inner
              together:
                branches:
                  - label: leaf
                    steps:
                      - id: leaf_value
                        set: {output: leaf, value: '7'}
  - id: done
    finish: {result: '$step.outer'}
"#;
    let errors = match vb_compile::compile_workflow(nested.as_bytes()) {
        Ok(_) => return Err("nested Together source was accepted".into()),
        Err(errors) => errors,
    };
    assert!(
        errors.iter().any(|error| matches!(
            error,
            vb_compile::CompileError::Workflow(vb_core::WorkflowError::NestedTogether { .. })
        )),
        "nested Together must keep the typed rejection: {errors:?}"
    );
    Ok(())
}
