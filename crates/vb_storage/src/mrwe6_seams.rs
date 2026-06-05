#![forbid(unsafe_code)]
//! MRWE6 production-bound verification seams.
//!
//! These helpers expose the journal side-index semantics used by production
//! append, duplicate, completion, and recovery paths.  They do not mutate
//! runtime state; proof artifacts can bind to this module instead of copying a
//! support-only finite model.

pub use crate::journal::append::{
    Mrwe6ActionIndexIntent, Mrwe6AtomKind, Mrwe6DuplicateRetryDecision, Mrwe6EventClass,
    Mrwe6IntentKind, Mrwe6RecoveryOutcome, Mrwe6ResolutionCommitDecision, Mrwe6SeamError,
    Mrwe6ValidatedAtom, mrwe6_action_index_intent, mrwe6_action_index_key_for_intent,
    mrwe6_duplicate_retry_decision, mrwe6_duplicate_retry_decision_from_facts, mrwe6_event_class,
    mrwe6_event_intent_matches_class, mrwe6_intent_kind, mrwe6_intent_kind_matches_event_class,
    mrwe6_recovery_outcome, mrwe6_recovery_outcome_from_facts,
    mrwe6_required_intent_kind_for_class, mrwe6_resolution_commit_decision,
    mrwe6_resolution_commit_decision_from_facts, mrwe6_validated_atom,
    mrwe6_validated_atom_for_event,
};
