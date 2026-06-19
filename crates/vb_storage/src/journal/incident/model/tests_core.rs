#![cfg(test)]

use super::test_support::*;
use super::{
    IncidentFailureKind, SideEffectCertainty, SideEffectDisposition, analyze_incident_events,
};

#[test]
fn t_001_empty_events() {
    let analysis = analyze_incident_events(&[]);
    assert!(!analysis.failure_found);
    assert!(analysis.failure_kind.is_none());
    assert_eq!(analysis.failure_code, "");
    assert!(analysis.failed_at_step.is_none());
    assert!(analysis.last_sequence.is_none());
    assert!(analysis.last_checkpoint.is_none());
    assert_eq!(analysis.counts.total, 0);
    assert!(analysis.side_effects.is_empty());
    assert!(analysis.side_effect_evidence.is_empty());
    assert!(analysis.failed_action_evidence.is_empty());
    assert!(analysis.pending_scheduled_actions.is_empty());
}

#[test]
fn t_002_run_failed_event() {
    let events = vec![step_event(1), run_failed()];
    let analysis = analyze_incident_events(&events);
    assert!(analysis.failure_found);
    assert_eq!(analysis.failure_kind, Some(IncidentFailureKind::RunFailed));
    assert_eq!(analysis.failure_code, "RunFailed");
    assert_eq!(analysis.failed_at_step, Some(1));
}

#[test]
fn t_003_run_cancelled() {
    let events = vec![step_event(1), step_event(2), run_cancelled()];
    let analysis = analyze_incident_events(&events);
    assert!(analysis.failure_found);
    assert_eq!(
        analysis.failure_kind,
        Some(IncidentFailureKind::RunCancelled)
    );
    assert_eq!(analysis.failure_code, "RunCancelled");
    assert_eq!(analysis.failed_at_step, Some(2));
}

#[test]
fn t_004_action_completed_side_effects() {
    let events = vec![action_completed(1, 100)];
    let analysis = analyze_incident_events(&events);
    assert!(!analysis.failure_found);
    assert_eq!(analysis.side_effects.len(), 1);
    assert_eq!(
        analysis.side_effects.first().map(|effect| effect.step),
        Some(1)
    );
    assert!(matches!(
        analysis.side_effects.first(),
        Some(effect) if effect.action == 100 && effect.certainty == SideEffectCertainty::Confirmed
    ));
    assert_eq!(analysis.side_effect_evidence.len(), 1);
    assert_eq!(
        analysis
            .side_effect_evidence
            .first()
            .map(|evidence| evidence.disposition),
        Some(SideEffectDisposition::Completed)
    );
}

#[test]
fn t_005_action_failed_side_effects() {
    let events = vec![action_failed(2, 200)];
    let analysis = analyze_incident_events(&events);
    assert!(!analysis.failure_found);
    assert_eq!(analysis.side_effects.len(), 1);
    assert!(matches!(
        analysis.side_effects.first(),
        Some(effect) if effect.certainty == SideEffectCertainty::Failed
    ));
    assert_eq!(analysis.failed_action_evidence.len(), 1);
    assert_eq!(
        analysis
            .failed_action_evidence
            .first()
            .map(|evidence| evidence.disposition),
        Some(SideEffectDisposition::Failed)
    );
}

#[test]
fn t_006_multiple_events() {
    let events = vec![
        step_event(1),
        action_completed(1, 10),
        action_failed(1, 20),
        step_event(2),
        action_completed(2, 30),
        run_failed(),
    ];
    let analysis = analyze_incident_events(&events);
    assert!(analysis.failure_found);
    assert_eq!(analysis.failure_code, "RunFailed");
    assert_eq!(analysis.failed_at_step, Some(2));
    assert_eq!(analysis.side_effects.len(), 3);
}

#[test]
fn t_007_multiple_step_started_tracking() {
    let events = vec![
        step_event(1),
        step_event(3),
        step_event(5),
        step_event(7),
        run_failed(),
    ];
    let analysis = analyze_incident_events(&events);
    assert!(analysis.failure_found);
    assert_eq!(analysis.failed_at_step, Some(7));
}

#[test]
fn t_008_mixed_events() {
    let events = vec![
        step_event(1),
        action_completed(1, 10),
        step_event(2),
        action_failed(2, 20),
        step_event(3),
        action_completed(3, 30),
        run_failed(),
    ];
    let analysis = analyze_incident_events(&events);
    assert!(analysis.failure_found);
    assert_eq!(analysis.failure_code, "RunFailed");
    assert_eq!(analysis.failed_at_step, Some(3));
    assert_eq!(analysis.side_effects.len(), 3);
}

#[test]
fn t_013_run_killed_failure() {
    let events = vec![step_event(9), run_killed()];
    let analysis = analyze_incident_events(&events);
    assert!(analysis.failure_found);
    assert_eq!(analysis.failure_kind, Some(IncidentFailureKind::RunKilled));
    assert_eq!(analysis.failure_code, "RunKilled");
    assert_eq!(analysis.failed_at_step, Some(9));
    assert_eq!(analysis.counts.run_killed, 1);
}
