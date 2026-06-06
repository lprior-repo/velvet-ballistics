#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mrwe6EventClass {
    Scheduled,
    Resolution,
    Unrelated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mrwe6IntentKind {
    None,
    PutPending,
    RemovePending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mrwe6AtomKind {
    EventOnly,
    EventAndPutPending,
    EventAndRemovePending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mrwe6SeamError {
    ClassIntentMismatch,
    ScheduledAtomMissingPutPending,
    QueuedRelevantEventMissingIntent,
    DuplicateRetryNotIdempotent,
    ResolutionDidNotRemovePending,
    RecoveryOutcomeNotPendingInventory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mrwe6DuplicateRetryDecision {
    IdempotentEqualRetry,
    DivergentDuplicateConflict,
    MissingExpectedIndexState,
    UnsupportedDuplicateClassRejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mrwe6ResolutionCommitDecision {
    CommittedAndMarkerRemoved,
    CommitFailedMarkerRetained,
    MismatchedResolutionRejected,
    NonResolutionRejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mrwe6RecoveryOutcome {
    PendingInventory,
    ResolvedNoPending,
    ParityDefect,
    LegacyFallback,
}

#[must_use]
pub const fn required_intent_kind_for_class(class: Mrwe6EventClass) -> Mrwe6IntentKind {
    match class {
        Mrwe6EventClass::Scheduled => Mrwe6IntentKind::PutPending,
        Mrwe6EventClass::Resolution => Mrwe6IntentKind::RemovePending,
        Mrwe6EventClass::Unrelated => Mrwe6IntentKind::None,
    }
}

#[must_use]
pub const fn intent_kind_matches_event_class(
    class: Mrwe6EventClass,
    intent_kind: Mrwe6IntentKind,
) -> bool {
    match (class, intent_kind) {
        (Mrwe6EventClass::Scheduled, Mrwe6IntentKind::PutPending)
        | (Mrwe6EventClass::Resolution, Mrwe6IntentKind::RemovePending)
        | (Mrwe6EventClass::Unrelated, Mrwe6IntentKind::None) => true,
        (Mrwe6EventClass::Scheduled, Mrwe6IntentKind::None)
        | (Mrwe6EventClass::Scheduled, Mrwe6IntentKind::RemovePending)
        | (Mrwe6EventClass::Resolution, Mrwe6IntentKind::None)
        | (Mrwe6EventClass::Resolution, Mrwe6IntentKind::PutPending)
        | (Mrwe6EventClass::Unrelated, Mrwe6IntentKind::PutPending)
        | (Mrwe6EventClass::Unrelated, Mrwe6IntentKind::RemovePending) => false,
    }
}

#[must_use]
pub const fn atom_kind_for_intent_kind(intent_kind: Mrwe6IntentKind) -> Mrwe6AtomKind {
    match intent_kind {
        Mrwe6IntentKind::None => Mrwe6AtomKind::EventOnly,
        Mrwe6IntentKind::PutPending => Mrwe6AtomKind::EventAndPutPending,
        Mrwe6IntentKind::RemovePending => Mrwe6AtomKind::EventAndRemovePending,
    }
}

pub const fn checked_atom_kind(
    class: Mrwe6EventClass,
    intent_kind: Mrwe6IntentKind,
) -> Result<Mrwe6AtomKind, Mrwe6SeamError> {
    if intent_kind_matches_event_class(class, intent_kind) {
        Ok(atom_kind_for_intent_kind(intent_kind))
    } else {
        Err(Mrwe6SeamError::ClassIntentMismatch)
    }
}

pub const fn checked_scheduled_atom_kind(
    class: Mrwe6EventClass,
    intent_kind: Mrwe6IntentKind,
) -> Result<Mrwe6AtomKind, Mrwe6SeamError> {
    match checked_atom_kind(class, intent_kind) {
        Ok(Mrwe6AtomKind::EventAndPutPending) => Ok(Mrwe6AtomKind::EventAndPutPending),
        Ok(Mrwe6AtomKind::EventOnly | Mrwe6AtomKind::EventAndRemovePending) => {
            Err(Mrwe6SeamError::ScheduledAtomMissingPutPending)
        }
        Err(error) => Err(error),
    }
}

pub const fn checked_queued_relevant_atom_kind(
    class: Mrwe6EventClass,
    intent_kind: Mrwe6IntentKind,
) -> Result<Mrwe6AtomKind, Mrwe6SeamError> {
    match checked_atom_kind(class, intent_kind) {
        Ok(Mrwe6AtomKind::EventAndPutPending) => Ok(Mrwe6AtomKind::EventAndPutPending),
        Ok(Mrwe6AtomKind::EventAndRemovePending) => Ok(Mrwe6AtomKind::EventAndRemovePending),
        Ok(Mrwe6AtomKind::EventOnly) => Err(Mrwe6SeamError::QueuedRelevantEventMissingIntent),
        Err(error) => Err(error),
    }
}

#[must_use]
pub const fn duplicate_retry_decision_from_facts(
    equal_payload: bool,
    retry_class: Mrwe6EventClass,
    index_marker_present: bool,
) -> Mrwe6DuplicateRetryDecision {
    if !equal_payload {
        return Mrwe6DuplicateRetryDecision::DivergentDuplicateConflict;
    }
    match retry_class {
        Mrwe6EventClass::Scheduled if index_marker_present => {
            Mrwe6DuplicateRetryDecision::IdempotentEqualRetry
        }
        Mrwe6EventClass::Scheduled => Mrwe6DuplicateRetryDecision::MissingExpectedIndexState,
        Mrwe6EventClass::Resolution | Mrwe6EventClass::Unrelated => {
            Mrwe6DuplicateRetryDecision::UnsupportedDuplicateClassRejected
        }
    }
}

#[must_use]
pub const fn resolution_commit_decision_from_facts(
    is_resolution_event: bool,
    key_matches_pending: bool,
    commit_success: bool,
) -> Mrwe6ResolutionCommitDecision {
    if !is_resolution_event {
        return Mrwe6ResolutionCommitDecision::NonResolutionRejected;
    }
    if !key_matches_pending {
        return Mrwe6ResolutionCommitDecision::MismatchedResolutionRejected;
    }
    if commit_success {
        Mrwe6ResolutionCommitDecision::CommittedAndMarkerRemoved
    } else {
        Mrwe6ResolutionCommitDecision::CommitFailedMarkerRetained
    }
}

#[must_use]
pub const fn recovery_outcome_from_facts(
    scheduled_has_pending_intent: bool,
    resolution_present: bool,
    resolution_matches_scheduled: bool,
    marker_present: bool,
    legacy_profile: bool,
) -> Mrwe6RecoveryOutcome {
    if !scheduled_has_pending_intent {
        return Mrwe6RecoveryOutcome::ParityDefect;
    }
    if resolution_present {
        if resolution_matches_scheduled {
            Mrwe6RecoveryOutcome::ResolvedNoPending
        } else {
            Mrwe6RecoveryOutcome::ParityDefect
        }
    } else if marker_present {
        Mrwe6RecoveryOutcome::PendingInventory
    } else if legacy_profile {
        Mrwe6RecoveryOutcome::LegacyFallback
    } else {
        Mrwe6RecoveryOutcome::ParityDefect
    }
}
