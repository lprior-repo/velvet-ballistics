#![forbid(unsafe_code)]

use vb_core::{ActionId, RunId, StepIdx};
use vb_storage::mrwe6_seams::{
    Mrwe6ActionIndexIntent, Mrwe6AtomKind, Mrwe6DuplicateRetryDecision, Mrwe6EventClass,
    Mrwe6IntentKind, Mrwe6RecoveryOutcome, Mrwe6ResolutionCommitDecision, Mrwe6SeamError,
    mrwe6_action_index_intent, mrwe6_duplicate_retry_decision,
    mrwe6_duplicate_retry_decision_from_facts, mrwe6_event_class, mrwe6_event_intent_matches_class,
    mrwe6_recovery_outcome, mrwe6_recovery_outcome_from_facts,
    mrwe6_required_intent_kind_for_class, mrwe6_resolution_commit_decision,
    mrwe6_resolution_commit_decision_from_facts, mrwe6_validated_atom,
    mrwe6_validated_atom_for_event,
};
use vb_storage::{EventSeq, JournalEvent};

fn run() -> RunId {
    RunId::new(7)
}

fn step() -> StepIdx {
    StepIdx::new(3)
}

fn action() -> ActionId {
    ActionId::new(11)
}

fn scheduled(seq: u64) -> JournalEvent {
    JournalEvent::ActionScheduled {
        run: run(),
        seq: EventSeq::new(seq),
        step: step(),
        action: action(),
        attempt: 1,
    }
}

fn completed(seq: u64, action_id: ActionId) -> JournalEvent {
    JournalEvent::ActionCompletedEvent {
        run: run(),
        seq: EventSeq::new(seq),
        step: step(),
        action: action_id,
        attempt: 1,
    }
}

#[test]
fn vb_mrwe6_bridge_scheduled_event_maps_to_put_pending_intent() {
    let event = scheduled(1);

    assert!(matches!(
        mrwe6_event_class(&event),
        Mrwe6EventClass::Scheduled
    ));
    assert!(matches!(
        mrwe6_action_index_intent(&event),
        Mrwe6ActionIndexIntent::Put { action: a, run: r, step: s }
            if a == action() && r == run() && s == step()
    ));
    assert!(matches!(
        mrwe6_required_intent_kind_for_class(mrwe6_event_class(&event)),
        Mrwe6IntentKind::PutPending
    ));
    assert!(mrwe6_event_intent_matches_class(&event));
}

#[test]
fn vb_mrwe6_bridge_resolution_event_maps_to_remove_pending_intent() {
    let event = completed(2, action());

    assert!(matches!(
        mrwe6_event_class(&event),
        Mrwe6EventClass::Resolution
    ));
    assert!(matches!(
        mrwe6_action_index_intent(&event),
        Mrwe6ActionIndexIntent::Delete { action: a, run: r, step: s }
            if a == action() && r == run() && s == step()
    ));
    assert!(matches!(
        mrwe6_required_intent_kind_for_class(mrwe6_event_class(&event)),
        Mrwe6IntentKind::RemovePending
    ));
    assert!(mrwe6_event_intent_matches_class(&event));
}

#[test]
fn vb_mrwe6_bridge_duplicate_classifier_separates_equal_from_divergent() {
    let existing = scheduled(3);
    let equal_retry = scheduled(3);
    let divergent_retry = completed(3, action());

    assert!(matches!(
        mrwe6_duplicate_retry_decision(&existing, &equal_retry, true),
        Mrwe6DuplicateRetryDecision::IdempotentEqualRetry
    ));
    assert!(matches!(
        mrwe6_duplicate_retry_decision(&existing, &equal_retry, false),
        Mrwe6DuplicateRetryDecision::MissingExpectedIndexState
    ));
    assert!(matches!(
        mrwe6_duplicate_retry_decision(&existing, &divergent_retry, true),
        Mrwe6DuplicateRetryDecision::DivergentDuplicateConflict
    ));
}

#[test]
fn vb_mrwe6_bridge_completion_classifier_removes_only_same_key_on_success() {
    let resolution = completed(4, action());
    let other_action = ActionId::new(12);

    assert!(matches!(
        mrwe6_resolution_commit_decision(&resolution, action(), run(), step(), true),
        Ok(Mrwe6ResolutionCommitDecision::CommittedAndMarkerRemoved)
    ));
    assert!(matches!(
        mrwe6_resolution_commit_decision(&resolution, action(), run(), step(), false),
        Ok(Mrwe6ResolutionCommitDecision::CommitFailedMarkerRetained)
    ));
    assert!(matches!(
        mrwe6_resolution_commit_decision(&resolution, other_action, run(), step(), true),
        Ok(Mrwe6ResolutionCommitDecision::MismatchedResolutionRejected)
    ));
}

#[test]
fn vb_mrwe6_bridge_recovery_classifier_separates_inventory_defect_and_fallback() {
    let schedule = scheduled(5);
    let resolution = completed(6, action());
    let mismatched_resolution = completed(7, ActionId::new(12));

    assert!(matches!(
        mrwe6_recovery_outcome(&schedule, None, true, false),
        Ok(Mrwe6RecoveryOutcome::PendingInventory)
    ));
    assert!(matches!(
        mrwe6_recovery_outcome(&schedule, Some(&resolution), true, false),
        Ok(Mrwe6RecoveryOutcome::ResolvedNoPending)
    ));
    assert!(matches!(
        mrwe6_recovery_outcome(&schedule, Some(&mismatched_resolution), true, false),
        Ok(Mrwe6RecoveryOutcome::ParityDefect)
    ));
    assert!(matches!(
        mrwe6_recovery_outcome(&schedule, None, false, true),
        Ok(Mrwe6RecoveryOutcome::LegacyFallback)
    ));
    assert!(matches!(
        mrwe6_recovery_outcome(&schedule, None, false, false),
        Ok(Mrwe6RecoveryOutcome::ParityDefect)
    ));
}

#[test]
fn vb_mrwe6_primitive_atom_constructor_rejects_invalid_state() {
    let valid_schedule =
        mrwe6_validated_atom(Mrwe6EventClass::Scheduled, Mrwe6IntentKind::PutPending);
    let invalid_schedule = mrwe6_validated_atom(Mrwe6EventClass::Scheduled, Mrwe6IntentKind::None);

    assert!(matches!(
        valid_schedule.map(|atom| atom.atom_kind()),
        Ok(Mrwe6AtomKind::EventAndPutPending)
    ));
    assert!(matches!(
        invalid_schedule,
        Err(Mrwe6SeamError::ClassIntentMismatch)
    ));
    assert!(matches!(
        mrwe6_validated_atom_for_event(&completed(8, action())).map(|atom| atom.atom_kind()),
        Ok(Mrwe6AtomKind::EventAndRemovePending)
    ));
}

#[test]
fn vb_mrwe6_primitive_decision_functions_match_event_wrappers() {
    let event = scheduled(9);
    let completion = completed(10, action());

    assert!(matches!(
        mrwe6_duplicate_retry_decision_from_facts(true, Mrwe6EventClass::Scheduled, true),
        Mrwe6DuplicateRetryDecision::IdempotentEqualRetry
    ));
    assert!(matches!(
        mrwe6_resolution_commit_decision_from_facts(true, true, true),
        Mrwe6ResolutionCommitDecision::CommittedAndMarkerRemoved
    ));
    assert!(matches!(
        mrwe6_recovery_outcome_from_facts(true, false, false, true, false),
        Mrwe6RecoveryOutcome::PendingInventory
    ));
    assert_eq!(
        mrwe6_duplicate_retry_decision(&event, &event, true),
        mrwe6_duplicate_retry_decision_from_facts(true, mrwe6_event_class(&event), true)
    );
    assert!(matches!(
        mrwe6_resolution_commit_decision(&completion, action(), run(), step(), true),
        Ok(decision)
            if decision == mrwe6_resolution_commit_decision_from_facts(true, true, true)
    ));
}
