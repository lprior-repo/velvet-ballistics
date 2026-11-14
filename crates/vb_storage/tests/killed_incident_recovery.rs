use vb_core::{ActionId, RunId, SlotIdx, StepIdx, workflow::LifecycleState};
use vb_storage::{
    EventSeq, JournalEvent, SideEffectCertainty, analyze_incident_events,
    derive_lifecycle_state_from_events,
};

fn killed() -> JournalEvent {
    JournalEvent::RunKilled {
        run: RunId::new(1),
        seq: EventSeq::new(9),
        attempt: 1,
    }
}

fn started(seq: u64, step: u16) -> JournalEvent {
    JournalEvent::StepStarted {
        run: RunId::new(1),
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        attempt: 1,
    }
}

#[test]
fn killed_history_is_an_incident_without_invented_step_or_effects() {
    let analysis = analyze_incident_events(&[killed()]);
    assert!(analysis.failure_found);
    assert_eq!(analysis.failure_code, "RunKilled");
    assert_eq!(analysis.failed_at_step, None);
    assert!(analysis.side_effects.is_empty());
}

#[test]
fn kill_preserves_last_started_step_and_exact_observed_effects() {
    let events = [
        started(1, 3),
        JournalEvent::ActionCompletedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(2),
            step: StepIdx::new(3),
            action: ActionId::new(7),
            attempt: 1,
        },
        JournalEvent::ActionFailedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(3),
            step: StepIdx::new(3),
            action: ActionId::new(8),
            attempt: 1,
        },
        started(4, 5),
        killed(),
    ];
    let original = events.clone();
    let analysis = analyze_incident_events(&events);
    assert!(analysis.failure_found);
    assert_eq!(analysis.failure_code, "RunKilled");
    assert_eq!(analysis.failed_at_step, Some(5));
    let effects: Vec<_> = analysis
        .side_effects
        .iter()
        .map(|effect| {
            (
                effect.step,
                effect.action,
                matches!(effect.certainty, SideEffectCertainty::Confirmed),
            )
        })
        .collect();
    assert_eq!(effects, [(3, 7, true), (3, 8, false)]);
    assert_eq!(
        derive_lifecycle_state_from_events(&events),
        LifecycleState::Cancelled
    );
    assert_eq!(events, original);
}

#[test]
fn cancellation_keeps_its_distinct_existing_incident_code() {
    let analysis = analyze_incident_events(&[
        started(1, 2),
        JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(2),
            attempt: 1,
            reason: None,
        },
    ]);
    assert!(analysis.failure_found);
    assert_eq!(analysis.failure_code, "RunCancelled");
    assert_eq!(analysis.failed_at_step, Some(2));
    assert!(analysis.side_effects.is_empty());
}

#[test]
fn successful_and_wait_or_timeout_progression_remain_nonincidents() {
    for event in [
        JournalEvent::RunFinished {
            run: RunId::new(1),
            seq: EventSeq::new(2),
            result: SlotIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::WaitResolvedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(2),
            step: StepIdx::new(4),
            attempt: 1,
        },
        JournalEvent::AskTimedOutEvent {
            run: RunId::new(1),
            seq: EventSeq::new(2),
            step: StepIdx::new(4),
            attempt: 1,
        },
    ] {
        let analysis = analyze_incident_events(&[started(1, 4), event]);
        assert!(!analysis.failure_found);
        assert_eq!(analysis.failure_code, "");
        assert_eq!(analysis.failed_at_step, None);
        assert!(analysis.side_effects.is_empty());
    }
}
