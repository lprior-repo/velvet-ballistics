use vstd::prelude::*;

verus! {

// Verus-checked generated equivalent for
// crates/vb_storage/src/journal/append/mrwe6_kernel.rs.
// The State 11 evidence command records the production SHA-256 and this file's
// SHA-256; no trusted shortcut or external body is used here.

pub enum Mrwe6EventClassView { Scheduled, Resolution, Unrelated }
pub enum Mrwe6IntentKindView { None, PutPending, RemovePending }
pub enum Mrwe6AtomKindView { EventOnly, EventAndPutPending, EventAndRemovePending }
pub enum Mrwe6DuplicateRetryDecisionView { IdempotentEqualRetry, DivergentDuplicateConflict, MissingExpectedIndexState, UnsupportedDuplicateClassRejected }
pub enum Mrwe6ResolutionCommitDecisionView { CommittedAndMarkerRemoved, CommitFailedMarkerRetained, MismatchedResolutionRejected, NonResolutionRejected }
pub enum Mrwe6RecoveryOutcomeView { PendingInventory, ResolvedNoPending, ParityDefect, LegacyFallback }

pub open spec fn spec_required_intent_kind_for_class(class: Mrwe6EventClassView) -> Mrwe6IntentKindView {
    match class {
        Mrwe6EventClassView::Scheduled => Mrwe6IntentKindView::PutPending,
        Mrwe6EventClassView::Resolution => Mrwe6IntentKindView::RemovePending,
        Mrwe6EventClassView::Unrelated => Mrwe6IntentKindView::None,
    }
}

pub fn required_intent_kind_for_class_exec(class: Mrwe6EventClassView) -> (intent: Mrwe6IntentKindView)
    ensures intent == spec_required_intent_kind_for_class(class),
{
    match class {
        Mrwe6EventClassView::Scheduled => Mrwe6IntentKindView::PutPending,
        Mrwe6EventClassView::Resolution => Mrwe6IntentKindView::RemovePending,
        Mrwe6EventClassView::Unrelated => Mrwe6IntentKindView::None,
    }
}

pub open spec fn spec_intent_kind_matches_event_class(class: Mrwe6EventClassView, intent: Mrwe6IntentKindView) -> bool {
    intent == spec_required_intent_kind_for_class(class)
}

pub fn intent_kind_matches_event_class_exec(class: Mrwe6EventClassView, intent: Mrwe6IntentKindView) -> (matches: bool)
    ensures matches == spec_intent_kind_matches_event_class(class, intent),
{
    match (class, intent) {
        (Mrwe6EventClassView::Scheduled, Mrwe6IntentKindView::PutPending) => true,
        (Mrwe6EventClassView::Resolution, Mrwe6IntentKindView::RemovePending) => true,
        (Mrwe6EventClassView::Unrelated, Mrwe6IntentKindView::None) => true,
        _ => false,
    }
}

pub open spec fn spec_atom_kind_for_intent_kind(intent: Mrwe6IntentKindView) -> Mrwe6AtomKindView {
    match intent {
        Mrwe6IntentKindView::None => Mrwe6AtomKindView::EventOnly,
        Mrwe6IntentKindView::PutPending => Mrwe6AtomKindView::EventAndPutPending,
        Mrwe6IntentKindView::RemovePending => Mrwe6AtomKindView::EventAndRemovePending,
    }
}

pub fn atom_kind_for_intent_kind_exec(intent: Mrwe6IntentKindView) -> (atom: Mrwe6AtomKindView)
    ensures atom == spec_atom_kind_for_intent_kind(intent),
{
    match intent {
        Mrwe6IntentKindView::None => Mrwe6AtomKindView::EventOnly,
        Mrwe6IntentKindView::PutPending => Mrwe6AtomKindView::EventAndPutPending,
        Mrwe6IntentKindView::RemovePending => Mrwe6AtomKindView::EventAndRemovePending,
    }
}

pub open spec fn spec_duplicate_retry_decision_from_facts(equal_payload: bool, retry_class: Mrwe6EventClassView, index_marker_present: bool) -> Mrwe6DuplicateRetryDecisionView {
    if !equal_payload {
        Mrwe6DuplicateRetryDecisionView::DivergentDuplicateConflict
    } else {
        match retry_class {
            Mrwe6EventClassView::Scheduled => if index_marker_present {
                Mrwe6DuplicateRetryDecisionView::IdempotentEqualRetry
            } else {
                Mrwe6DuplicateRetryDecisionView::MissingExpectedIndexState
            },
            Mrwe6EventClassView::Resolution | Mrwe6EventClassView::Unrelated => {
                Mrwe6DuplicateRetryDecisionView::UnsupportedDuplicateClassRejected
            },
        }
    }
}

pub fn duplicate_retry_decision_from_facts_exec(equal_payload: bool, retry_class: Mrwe6EventClassView, index_marker_present: bool) -> (decision: Mrwe6DuplicateRetryDecisionView)
    ensures decision == spec_duplicate_retry_decision_from_facts(equal_payload, retry_class, index_marker_present),
{
    if !equal_payload {
        return Mrwe6DuplicateRetryDecisionView::DivergentDuplicateConflict;
    }
    match retry_class {
        Mrwe6EventClassView::Scheduled => if index_marker_present { Mrwe6DuplicateRetryDecisionView::IdempotentEqualRetry } else { Mrwe6DuplicateRetryDecisionView::MissingExpectedIndexState },
        Mrwe6EventClassView::Resolution | Mrwe6EventClassView::Unrelated => Mrwe6DuplicateRetryDecisionView::UnsupportedDuplicateClassRejected,
    }
}

pub open spec fn spec_resolution_commit_decision_from_facts(is_resolution_event: bool, key_matches_pending: bool, commit_success: bool) -> Mrwe6ResolutionCommitDecisionView {
    if !is_resolution_event { Mrwe6ResolutionCommitDecisionView::NonResolutionRejected }
    else if !key_matches_pending { Mrwe6ResolutionCommitDecisionView::MismatchedResolutionRejected }
    else if commit_success { Mrwe6ResolutionCommitDecisionView::CommittedAndMarkerRemoved }
    else { Mrwe6ResolutionCommitDecisionView::CommitFailedMarkerRetained }
}

pub fn resolution_commit_decision_from_facts_exec(is_resolution_event: bool, key_matches_pending: bool, commit_success: bool) -> (decision: Mrwe6ResolutionCommitDecisionView)
    ensures decision == spec_resolution_commit_decision_from_facts(is_resolution_event, key_matches_pending, commit_success),
{
    if !is_resolution_event { Mrwe6ResolutionCommitDecisionView::NonResolutionRejected }
    else if !key_matches_pending { Mrwe6ResolutionCommitDecisionView::MismatchedResolutionRejected }
    else if commit_success { Mrwe6ResolutionCommitDecisionView::CommittedAndMarkerRemoved }
    else { Mrwe6ResolutionCommitDecisionView::CommitFailedMarkerRetained }
}

pub open spec fn spec_recovery_outcome_from_facts(scheduled_has_pending_intent: bool, resolution_present: bool, resolution_matches_scheduled: bool, marker_present: bool, legacy_profile: bool) -> Mrwe6RecoveryOutcomeView {
    if !scheduled_has_pending_intent { Mrwe6RecoveryOutcomeView::ParityDefect }
    else if resolution_present {
        if resolution_matches_scheduled { Mrwe6RecoveryOutcomeView::ResolvedNoPending } else { Mrwe6RecoveryOutcomeView::ParityDefect }
    } else if marker_present { Mrwe6RecoveryOutcomeView::PendingInventory }
    else if legacy_profile { Mrwe6RecoveryOutcomeView::LegacyFallback }
    else { Mrwe6RecoveryOutcomeView::ParityDefect }
}

pub fn recovery_outcome_from_facts_exec(scheduled_has_pending_intent: bool, resolution_present: bool, resolution_matches_scheduled: bool, marker_present: bool, legacy_profile: bool) -> (outcome: Mrwe6RecoveryOutcomeView)
    ensures outcome == spec_recovery_outcome_from_facts(scheduled_has_pending_intent, resolution_present, resolution_matches_scheduled, marker_present, legacy_profile),
{
    if !scheduled_has_pending_intent { Mrwe6RecoveryOutcomeView::ParityDefect }
    else if resolution_present {
        if resolution_matches_scheduled { Mrwe6RecoveryOutcomeView::ResolvedNoPending } else { Mrwe6RecoveryOutcomeView::ParityDefect }
    } else if marker_present { Mrwe6RecoveryOutcomeView::PendingInventory }
    else if legacy_profile { Mrwe6RecoveryOutcomeView::LegacyFallback }
    else { Mrwe6RecoveryOutcomeView::ParityDefect }
}

} // verus!
