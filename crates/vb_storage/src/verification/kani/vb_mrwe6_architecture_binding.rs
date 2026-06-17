#![cfg(kani)]

//! Kani harness for obl-vb-in8ib-architecture-kani.
//!
//! This artifact is included from `vb_mrwe6_atomic_index.rs` so it is compiled
//! by the existing `kani-vb-mrwe6` crate wiring without editing production
//! behavior. It calls the public production seam exports and the State 11
//! production-called MRWE6 semantic kernel re-exports from `crate::mrwe6_seams`
//! for all five MRWE6 domains under bounded generated inputs.

use crate::mrwe6_seams::{
    Mrwe6AtomKind, Mrwe6DuplicateRetryDecision, Mrwe6EventClass, Mrwe6IntentKind,
    Mrwe6RecoveryOutcome, Mrwe6ResolutionCommitDecision, mrwe6_committed_resolution_from_facts,
    mrwe6_duplicate_retry_decision_from_facts, mrwe6_intent_kind_matches_event_class,
    mrwe6_kernel_checked_queued_relevant_atom_kind, mrwe6_kernel_checked_scheduled_atom_kind,
    mrwe6_kernel_duplicate_retry_decision_from_facts, mrwe6_kernel_intent_kind_matches_event_class,
    mrwe6_kernel_recovery_outcome_from_facts, mrwe6_kernel_required_intent_kind_for_class,
    mrwe6_kernel_resolution_commit_decision_from_facts, mrwe6_pending_inventory_from_facts,
    mrwe6_recovery_outcome_from_facts, mrwe6_required_intent_kind_for_class,
    mrwe6_resolution_commit_decision_from_facts, mrwe6_valid_queued_relevant_intent,
    mrwe6_valid_scheduled_atom, mrwe6_validated_atom,
};

fn generated_class() -> Mrwe6EventClass {
    match kani::any::<u8>() % 3 {
        0 => Mrwe6EventClass::Scheduled,
        1 => Mrwe6EventClass::Resolution,
        _ => Mrwe6EventClass::Unrelated,
    }
}

fn generated_intent_kind() -> Mrwe6IntentKind {
    match kani::any::<u8>() % 3 {
        0 => Mrwe6IntentKind::PutPending,
        1 => Mrwe6IntentKind::RemovePending,
        _ => Mrwe6IntentKind::None,
    }
}

#[kani::proof]
fn vb_mrwe6_architecture_binding_all_domains() {
    let class = generated_class();
    let intent = generated_intent_kind();
    let required = mrwe6_required_intent_kind_for_class(class);
    let kernel_required = mrwe6_kernel_required_intent_kind_for_class(class);
    kani::assert(required == kernel_required, "assertion failed");
    let valid_match = mrwe6_intent_kind_matches_event_class(class, intent);
    let kernel_valid_match = mrwe6_kernel_intent_kind_matches_event_class(class, intent);
    kani::assert(valid_match == kernel_valid_match, "assertion failed");
    kani::assert(valid_match == required == intent, "assertion failed");

    let atom = mrwe6_validated_atom(class, intent);
    if valid_match {
        kani::assert(atom.is_ok(), "kani harness assertion");
    } else {
        kani::assert(atom.is_err(), "kani harness assertion");
    }

    let scheduled_atom = mrwe6_valid_scheduled_atom(class, intent);
    let kernel_scheduled_atom = mrwe6_kernel_checked_scheduled_atom_kind(class, intent);
    if matches!(
        (class, intent),
        (Mrwe6EventClass::Scheduled, Mrwe6IntentKind::PutPending)
    ) {
        kani::assert(matches!(
            scheduled_atom.map(|validated| validated.atom_kind()),
            Ok(Mrwe6AtomKind::EventAndPutPending)
        ));
        kani::assert(kernel_scheduled_atom == Ok(Mrwe6AtomKind::EventAndPutPending), "assertion failed");
    } else {
        kani::assert(scheduled_atom.is_err(), "kani harness assertion");
        kani::assert(kernel_scheduled_atom.is_err(), "kani harness assertion");
    }

    let queued_relevant = mrwe6_valid_queued_relevant_intent(class, intent);
    let kernel_queued_relevant = mrwe6_kernel_checked_queued_relevant_atom_kind(class, intent);
    if valid_match && !matches!(intent, Mrwe6IntentKind::None) {
        kani::assert(queued_relevant.is_ok(), "kani harness assertion");
        kani::assert(kernel_queued_relevant.is_ok(), "kani harness assertion");
    } else {
        kani::assert(queued_relevant.is_err(), "kani harness assertion");
        kani::assert(kernel_queued_relevant.is_err(), "kani harness assertion");
    }

    let equal_payload = kani::any::<bool>();
    let marker_present = kani::any::<bool>();
    let duplicate = mrwe6_duplicate_retry_decision_from_facts(equal_payload, class, marker_present);
    let kernel_duplicate =
        mrwe6_kernel_duplicate_retry_decision_from_facts(equal_payload, class, marker_present);
    kani::assert(duplicate == kernel_duplicate, "assertion failed");
    if !equal_payload {
        kani::assert(duplicate == Mrwe6DuplicateRetryDecision::DivergentDuplicateConflict, "assertion failed");
    }
    if equal_payload && !matches!(class, Mrwe6EventClass::Scheduled) {
        kani::assert(duplicate == Mrwe6DuplicateRetryDecision::UnsupportedDuplicateClassRejected, "assertion failed");
    }

    let is_resolution_event = kani::any::<bool>();
    let key_matches_pending = kani::any::<bool>();
    let commit_success = kani::any::<bool>();
    let resolution = mrwe6_resolution_commit_decision_from_facts(
        is_resolution_event,
        key_matches_pending,
        commit_success,
    );
    let kernel_resolution = mrwe6_kernel_resolution_commit_decision_from_facts(
        is_resolution_event,
        key_matches_pending,
        commit_success,
    );
    kani::assert(resolution == kernel_resolution, "assertion failed");
    if !is_resolution_event {
        kani::assert(resolution == Mrwe6ResolutionCommitDecision::NonResolutionRejected, "assertion failed");
    }
    let committed = mrwe6_committed_resolution_from_facts(
        is_resolution_event,
        key_matches_pending,
        commit_success,
    );
    if matches!(
        resolution,
        Mrwe6ResolutionCommitDecision::CommittedAndMarkerRemoved
    ) {
        kani::assert(committed.is_ok(), "kani harness assertion");
    } else {
        kani::assert(committed.is_err(), "kani harness assertion");
    }

    let resolution_present = kani::any::<bool>();
    let resolution_matches_scheduled = kani::any::<bool>();
    let recovery = mrwe6_recovery_outcome_from_facts(
        intent == Mrwe6IntentKind::PutPending,
        resolution_present,
        resolution_matches_scheduled,
        marker_present,
        kani::any::<bool>(),
    );
    let kernel_recovery = mrwe6_kernel_recovery_outcome_from_facts(
        intent == Mrwe6IntentKind::PutPending,
        resolution_present,
        resolution_matches_scheduled,
        marker_present,
        false,
    );
    if matches!(kernel_recovery, Mrwe6RecoveryOutcome::PendingInventory) {
        kani::assert(marker_present, "kani harness assertion");
    }
    if matches!(recovery, Mrwe6RecoveryOutcome::PendingInventory) {
        let pending = mrwe6_pending_inventory_from_facts(
            intent == Mrwe6IntentKind::PutPending,
            resolution_present,
            resolution_matches_scheduled,
            marker_present,
            false,
        );
        kani::assert(pending.is_ok(), "kani harness assertion");
    } else {
        kani::assert(!matches!(recovery, Mrwe6RecoveryOutcome::PendingInventory));
    }
}
