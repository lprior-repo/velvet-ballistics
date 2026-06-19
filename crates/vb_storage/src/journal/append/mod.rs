#![forbid(unsafe_code)]
//! Journal append intent classification and decision logic.

pub(super) mod decision;
mod intent;
pub(crate) mod mrwe6_kernel;
mod journal_impl;

pub use self::decision::{
    Mrwe6DuplicateRetryDecision, Mrwe6RecoveryOutcome, Mrwe6ResolutionCommitDecision,
    mrwe6_committed_resolution_from_facts, mrwe6_duplicate_retry_decision,
    mrwe6_duplicate_retry_decision_from_facts, mrwe6_idempotent_duplicate_retry_from_facts,
    mrwe6_pending_inventory_from_facts, mrwe6_recovery_outcome, mrwe6_recovery_outcome_from_facts,
    mrwe6_resolution_commit_decision, mrwe6_resolution_commit_decision_from_facts,
};
pub use self::intent::{
    Mrwe6ActionIndexIntent, Mrwe6AtomKind, Mrwe6EventClass, Mrwe6IntentKind, Mrwe6SeamError,
    Mrwe6ValidatedAtom, mrwe6_action_index_intent, mrwe6_action_index_key_for_intent,
    mrwe6_event_class, mrwe6_event_intent_matches_class, mrwe6_intent_kind,
    mrwe6_intent_kind_matches_event_class, mrwe6_required_intent_kind_for_class,
    mrwe6_valid_queued_relevant_intent, mrwe6_valid_scheduled_atom, mrwe6_validated_atom,
    mrwe6_validated_atom_for_event,
};
pub use self::mrwe6_kernel::{
    atom_kind_for_intent_kind as mrwe6_kernel_atom_kind_for_intent_kind,
    checked_atom_kind as mrwe6_kernel_checked_atom_kind,
    checked_queued_relevant_atom_kind as mrwe6_kernel_checked_queued_relevant_atom_kind,
    checked_scheduled_atom_kind as mrwe6_kernel_checked_scheduled_atom_kind,
    duplicate_retry_decision_from_facts as mrwe6_kernel_duplicate_retry_decision_from_facts,
    intent_kind_matches_event_class as mrwe6_kernel_intent_kind_matches_event_class,
    recovery_outcome_from_facts as mrwe6_kernel_recovery_outcome_from_facts,
    required_intent_kind_for_class as mrwe6_kernel_required_intent_kind_for_class,
    resolution_commit_decision_from_facts as mrwe6_kernel_resolution_commit_decision_from_facts,
};

#[cfg(kani)]
#[allow(unused_imports)]
pub(crate) use self::decision::{
    VerificationDuplicateRetryDecision, VerificationRecoveryOutcome,
    VerificationResolutionCommitDecision, verification_duplicate_retry_decision,
    verification_recovery_outcome, verification_resolution_commit_decision,
    verification_resolution_marker_present_after_commit,
};
#[cfg(kani)]
#[allow(unused_imports)]
pub(crate) use self::intent::{
    VerificationActionIndexIntent, verification_action_index_intent,
    verification_event_and_index_keys_exist,
};
