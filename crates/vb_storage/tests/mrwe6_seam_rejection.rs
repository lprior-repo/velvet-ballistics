#![forbid(unsafe_code)]

use vb_storage::mrwe6_seams::{
    Mrwe6EventClass, Mrwe6IntentKind, Mrwe6SeamError, mrwe6_committed_resolution_from_facts,
    mrwe6_idempotent_duplicate_retry_from_facts, mrwe6_pending_inventory_from_facts,
    mrwe6_valid_queued_relevant_intent, mrwe6_valid_scheduled_atom,
};

#[test]
fn scheduled_atom_rejects_event_only_state() {
    assert!(matches!(
        mrwe6_valid_scheduled_atom(Mrwe6EventClass::Unrelated, Mrwe6IntentKind::None),
        Err(Mrwe6SeamError::ScheduledAtomMissingPutPending)
    ));
}

#[test]
fn queued_relevant_intent_rejects_none_state() {
    assert!(matches!(
        mrwe6_valid_queued_relevant_intent(Mrwe6EventClass::Unrelated, Mrwe6IntentKind::None),
        Err(Mrwe6SeamError::QueuedRelevantEventMissingIntent)
    ));
}

#[test]
fn duplicate_success_rejects_divergent_retry() {
    assert!(matches!(
        mrwe6_idempotent_duplicate_retry_from_facts(false, Mrwe6EventClass::Scheduled, true),
        Err(Mrwe6SeamError::DuplicateRetryNotIdempotent)
    ));
}

#[test]
fn completion_policy_rejects_failed_marker_removal() {
    assert!(matches!(
        mrwe6_committed_resolution_from_facts(true, true, false),
        Err(Mrwe6SeamError::ResolutionDidNotRemovePending)
    ));
}

#[test]
fn recovery_reliance_rejects_non_legacy_missing_marker() {
    assert!(matches!(
        mrwe6_pending_inventory_from_facts(true, false, false, false, false),
        Err(Mrwe6SeamError::RecoveryOutcomeNotPendingInventory)
    ));
}
