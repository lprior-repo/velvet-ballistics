#![cfg(test)]

use super::test_support::*;
use super::{IncidentCheckpoint, SideEffectDisposition, analyze_incident_events};
use crate::{EventSeq, RecordKind};

#[test]
fn t_009_last_sequence_and_checkpoint() {
    let events = vec![
        step_event_at(1, 3),
        slot_written_at(2, 7),
        step_succeeded_at(3, 3, 7),
        run_finished_at(4, 7),
    ];
    let analysis = analyze_incident_events(&events);
    assert_eq!(analysis.last_sequence, Some(EventSeq::new(4)));
    assert_eq!(analysis.counts.total, 4);
    assert_eq!(analysis.counts.steps_started, 1);
    assert_eq!(analysis.counts.slot_writes, 1);
    assert_eq!(analysis.counts.steps_succeeded, 1);
    assert_eq!(analysis.counts.run_finished, 1);
    assert!(matches!(
        analysis.last_checkpoint,
        Some(IncidentCheckpoint {
            seq,
            kind: RecordKind::RunFinished,
            slot: Some(7),
            ..
        }) if seq == EventSeq::new(4)
    ));
}

#[test]
fn t_010_pending_scheduled_actions() {
    let events = vec![
        action_scheduled_at(1, 1, 10, 1),
        action_scheduled_at(2, 1, 20, 1),
        action_completed_at(3, 1, 10, 1),
    ];
    let analysis = analyze_incident_events(&events);
    assert_eq!(analysis.counts.actions_scheduled, 2);
    assert_eq!(analysis.counts.actions_completed, 1);
    assert_eq!(analysis.pending_scheduled_actions.len(), 1);
    assert_eq!(
        analysis
            .pending_scheduled_actions
            .first()
            .map(|evidence| (evidence.action, evidence.disposition)),
        Some((20, SideEffectDisposition::Scheduled))
    );
}

#[test]
fn t_011_failed_action_evidence() {
    let events = vec![
        action_scheduled_at(1, 2, 30, 3),
        action_failed_at(9, 2, 30, 3),
    ];
    let analysis = analyze_incident_events(&events);
    assert_eq!(analysis.pending_scheduled_actions.len(), 0);
    assert_eq!(analysis.failed_action_evidence.len(), 1);
    assert_eq!(
        analysis.failed_action_evidence.first().map(|evidence| (
            evidence.seq,
            evidence.step,
            evidence.action,
            evidence.attempt
        )),
        Some((EventSeq::new(9), 2, 30, 3))
    );
}

#[test]
fn t_012_ticket_action_events() {
    let events = vec![
        action_scheduled_ticket_at(1, 4, 70),
        action_completed_envelope_at(2, 4, 70),
    ];
    let analysis = analyze_incident_events(&events);
    assert_eq!(analysis.counts.actions_scheduled, 1);
    assert_eq!(analysis.counts.actions_completed, 1);
    assert_eq!(analysis.pending_scheduled_actions.len(), 0);
    assert_eq!(analysis.side_effect_evidence.len(), 2);
    assert_eq!(
        analysis
            .side_effect_evidence
            .first()
            .map(|evidence| (evidence.step, evidence.action)),
        Some((4, 70))
    );
}
