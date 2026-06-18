#![cfg(kani)]

//! Kani harness for obl-vb-in8ib-architecture-kani.
//!
//! The harness binds public MRWE6 seam exports to the production-called semantic
//! kernel re-exports under finite generated facts.

use crate::mrwe6_seams::{
    Mrwe6AtomKind, Mrwe6DuplicateRetryDecision, Mrwe6EventClass, Mrwe6IntentKind,
    Mrwe6RecoveryOutcome, Mrwe6ResolutionCommitDecision, mrwe6_committed_resolution_from_facts,
    mrwe6_duplicate_retry_decision_from_facts, mrwe6_intent_kind_matches_event_class,
    mrwe6_kernel_checked_atom_kind, mrwe6_kernel_checked_queued_relevant_atom_kind,
    mrwe6_kernel_checked_scheduled_atom_kind, mrwe6_kernel_duplicate_retry_decision_from_facts,
    mrwe6_kernel_intent_kind_matches_event_class, mrwe6_kernel_recovery_outcome_from_facts,
    mrwe6_kernel_required_intent_kind_for_class,
    mrwe6_kernel_resolution_commit_decision_from_facts, mrwe6_pending_inventory_from_facts,
    mrwe6_recovery_outcome_from_facts, mrwe6_required_intent_kind_for_class,
    mrwe6_resolution_commit_decision_from_facts, mrwe6_valid_queued_relevant_intent,
    mrwe6_valid_scheduled_atom, mrwe6_validated_atom,
};

fn generated_class() -> Mrwe6EventClass {
    match kani::any::<u8>() {
        0 => Mrwe6EventClass::Scheduled,
        1 => Mrwe6EventClass::Resolution,
        _ => Mrwe6EventClass::Unrelated,
    }
}

fn generated_intent_kind() -> Mrwe6IntentKind {
    match kani::any::<u8>() {
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
    kani::assert(
        required == kernel_required,
        "required intent matches kernel",
    );

    let valid_match = mrwe6_intent_kind_matches_event_class(class, intent);
    let kernel_valid_match = mrwe6_kernel_intent_kind_matches_event_class(class, intent);
    kani::assert(
        valid_match == kernel_valid_match,
        "intent/class match mirrors kernel",
    );
    kani::assert(
        valid_match == (required == intent),
        "required intent defines valid match",
    );

    let atom_kind = mrwe6_validated_atom(class, intent).map(|atom| atom.atom_kind());
    let kernel_atom_kind = mrwe6_kernel_checked_atom_kind(class, intent);
    kani::assert(
        atom_kind == kernel_atom_kind,
        "validated atom mirrors kernel",
    );
    kani::assert(
        atom_kind.is_ok() == valid_match,
        "atom result follows match predicate",
    );

    let scheduled_atom = mrwe6_valid_scheduled_atom(class, intent).map(|atom| atom.atom_kind());
    let kernel_scheduled_atom = mrwe6_kernel_checked_scheduled_atom_kind(class, intent);
    kani::assert(
        scheduled_atom == kernel_scheduled_atom,
        "scheduled atom mirrors kernel",
    );
    if matches!(
        (class, intent),
        (Mrwe6EventClass::Scheduled, Mrwe6IntentKind::PutPending)
    ) {
        kani::assert(
            scheduled_atom == Ok(Mrwe6AtomKind::EventAndPutPending),
            "scheduled put maps to put-pending atom",
        );
    } else {
        kani::assert(scheduled_atom.is_err(), "non-scheduled-put is rejected");
    }

    let queued_relevant =
        mrwe6_valid_queued_relevant_intent(class, intent).map(|atom| atom.atom_kind());
    let kernel_queued_relevant = mrwe6_kernel_checked_queued_relevant_atom_kind(class, intent);
    kani::assert(
        queued_relevant == kernel_queued_relevant,
        "queued relevant atom mirrors kernel",
    );
    if valid_match && !matches!(intent, Mrwe6IntentKind::None) {
        kani::assert(
            queued_relevant.is_ok(),
            "non-none valid intent is queue relevant",
        );
    } else {
        kani::assert(
            queued_relevant.is_err(),
            "invalid or none intent is not queue relevant",
        );
    }

    let equal_payload = kani::any::<bool>();
    let marker_present = kani::any::<bool>();
    let duplicate = mrwe6_duplicate_retry_decision_from_facts(equal_payload, class, marker_present);
    let kernel_duplicate =
        mrwe6_kernel_duplicate_retry_decision_from_facts(equal_payload, class, marker_present);
    kani::assert(
        duplicate == kernel_duplicate,
        "duplicate decision mirrors kernel",
    );
    if !equal_payload {
        kani::assert(
            duplicate == Mrwe6DuplicateRetryDecision::DivergentDuplicateConflict,
            "unequal payload is divergent duplicate",
        );
    }
    if equal_payload && !matches!(class, Mrwe6EventClass::Scheduled) {
        kani::assert(
            duplicate == Mrwe6DuplicateRetryDecision::UnsupportedDuplicateClassRejected,
            "unsupported equal duplicate class is rejected",
        );
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
    kani::assert(
        resolution == kernel_resolution,
        "resolution decision mirrors kernel",
    );
    if !is_resolution_event {
        kani::assert(
            resolution == Mrwe6ResolutionCommitDecision::NonResolutionRejected,
            "non-resolution event is rejected",
        );
    }
    let committed = mrwe6_committed_resolution_from_facts(
        is_resolution_event,
        key_matches_pending,
        commit_success,
    );
    kani::assert(
        committed.is_ok()
            == matches!(
                resolution,
                Mrwe6ResolutionCommitDecision::CommittedAndMarkerRemoved
            ),
        "committed helper follows resolution decision",
    );

    let scheduled_has_pending_intent = intent == Mrwe6IntentKind::PutPending;
    let resolution_present = kani::any::<bool>();
    let resolution_matches_scheduled = kani::any::<bool>();
    let legacy_profile = kani::any::<bool>();
    let recovery = mrwe6_recovery_outcome_from_facts(
        scheduled_has_pending_intent,
        resolution_present,
        resolution_matches_scheduled,
        marker_present,
        legacy_profile,
    );
    let kernel_recovery = mrwe6_kernel_recovery_outcome_from_facts(
        scheduled_has_pending_intent,
        resolution_present,
        resolution_matches_scheduled,
        marker_present,
        legacy_profile,
    );
    kani::assert(
        recovery == kernel_recovery,
        "recovery outcome mirrors kernel",
    );
    let pending = mrwe6_pending_inventory_from_facts(
        scheduled_has_pending_intent,
        resolution_present,
        resolution_matches_scheduled,
        marker_present,
        legacy_profile,
    );
    kani::assert(
        pending.is_ok() == matches!(recovery, Mrwe6RecoveryOutcome::PendingInventory),
        "pending helper follows recovery outcome",
    );
}
