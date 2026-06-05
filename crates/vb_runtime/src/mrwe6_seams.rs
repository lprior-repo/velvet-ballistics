#![forbid(unsafe_code)]
//! MRWE6 runtime-facing verification seams.
//!
//! Runtime proof and Flux artifacts can import this module to bind queued,
//! scheduled, and resolution event obligations to the production storage
//! side-index classifiers rather than to standalone support enums.

pub use vb_storage::mrwe6_seams::{
    Mrwe6ActionIndexIntent, Mrwe6AtomKind, Mrwe6DuplicateRetryDecision, Mrwe6EventClass,
    Mrwe6IntentKind, Mrwe6RecoveryOutcome, Mrwe6ResolutionCommitDecision, Mrwe6SeamError,
    Mrwe6ValidatedAtom, mrwe6_action_index_intent, mrwe6_action_index_key_for_intent,
    mrwe6_committed_resolution_from_facts, mrwe6_duplicate_retry_decision,
    mrwe6_duplicate_retry_decision_from_facts, mrwe6_event_class, mrwe6_event_intent_matches_class,
    mrwe6_idempotent_duplicate_retry_from_facts, mrwe6_intent_kind,
    mrwe6_intent_kind_matches_event_class, mrwe6_kernel_atom_kind_for_intent_kind,
    mrwe6_kernel_checked_atom_kind, mrwe6_kernel_checked_queued_relevant_atom_kind,
    mrwe6_kernel_checked_scheduled_atom_kind, mrwe6_kernel_duplicate_retry_decision_from_facts,
    mrwe6_kernel_intent_kind_matches_event_class, mrwe6_kernel_recovery_outcome_from_facts,
    mrwe6_kernel_required_intent_kind_for_class,
    mrwe6_kernel_resolution_commit_decision_from_facts, mrwe6_pending_inventory_from_facts,
    mrwe6_recovery_outcome, mrwe6_recovery_outcome_from_facts,
    mrwe6_required_intent_kind_for_class, mrwe6_resolution_commit_decision,
    mrwe6_resolution_commit_decision_from_facts, mrwe6_valid_queued_relevant_intent,
    mrwe6_valid_scheduled_atom, mrwe6_validated_atom, mrwe6_validated_atom_for_event,
};
