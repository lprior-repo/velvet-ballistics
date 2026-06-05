use crate::{error::JournalError, events::JournalEvent, keys::index_action_key};

use super::intent::{
    ActionIndexIntent, Mrwe6EventClass, Mrwe6IntentKind, mrwe6_event_class,
    mrwe6_required_intent_kind_for_class,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mrwe6DuplicateRetryDecision {
    IdempotentEqualRetry,
    DivergentDuplicateConflict,
    MissingExpectedIndexState,
}

#[cfg(kani)]
pub(crate) type VerificationDuplicateRetryDecision = Mrwe6DuplicateRetryDecision;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mrwe6ResolutionCommitDecision {
    CommittedAndMarkerRemoved,
    CommitFailedMarkerRetained,
    MismatchedResolutionRejected,
    NonResolutionRejected,
}

#[cfg(kani)]
pub(crate) type VerificationResolutionCommitDecision = Mrwe6ResolutionCommitDecision;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mrwe6RecoveryOutcome {
    PendingInventory,
    ResolvedNoPending,
    ParityDefect,
    LegacyFallback,
}

#[cfg(kani)]
pub(crate) type VerificationRecoveryOutcome = Mrwe6RecoveryOutcome;

#[cfg(kani)]
pub(crate) fn verification_duplicate_retry_decision(
    existing: &JournalEvent,
    retry: &JournalEvent,
    index_marker_present: bool,
) -> VerificationDuplicateRetryDecision {
    mrwe6_duplicate_retry_decision(existing, retry, index_marker_present)
}

#[must_use]
pub fn mrwe6_duplicate_retry_decision(
    existing: &JournalEvent,
    retry: &JournalEvent,
    index_marker_present: bool,
) -> Mrwe6DuplicateRetryDecision {
    mrwe6_duplicate_retry_decision_from_facts(
        existing == retry,
        mrwe6_event_class(retry),
        index_marker_present,
    )
}

#[must_use]
pub fn mrwe6_duplicate_retry_decision_from_facts(
    equal_payload: bool,
    retry_class: Mrwe6EventClass,
    index_marker_present: bool,
) -> Mrwe6DuplicateRetryDecision {
    if !equal_payload {
        return Mrwe6DuplicateRetryDecision::DivergentDuplicateConflict;
    }
    match mrwe6_required_intent_kind_for_class(retry_class) {
        Mrwe6IntentKind::PutPending if index_marker_present => {
            Mrwe6DuplicateRetryDecision::IdempotentEqualRetry
        }
        Mrwe6IntentKind::RemovePending if !index_marker_present => {
            Mrwe6DuplicateRetryDecision::IdempotentEqualRetry
        }
        Mrwe6IntentKind::None => Mrwe6DuplicateRetryDecision::IdempotentEqualRetry,
        Mrwe6IntentKind::PutPending | Mrwe6IntentKind::RemovePending => {
            Mrwe6DuplicateRetryDecision::MissingExpectedIndexState
        }
    }
}

#[cfg(kani)]
pub(crate) fn verification_resolution_marker_present_after_commit(
    event: &JournalEvent,
    existing_marker_matches_resolution: bool,
    commit_success: bool,
) -> Result<bool, JournalError> {
    match ActionIndexIntent::for_event(event) {
        ActionIndexIntent::Delete { .. } => {
            Ok(!(commit_success && existing_marker_matches_resolution))
        }
        ActionIndexIntent::Put { .. } | ActionIndexIntent::None => {
            Ok(existing_marker_matches_resolution)
        }
    }
}

#[cfg(kani)]
pub(crate) fn verification_resolution_commit_decision(
    event: &JournalEvent,
    pending_action: vb_core::ActionId,
    pending_run: vb_core::RunId,
    pending_step: vb_core::StepIdx,
    commit_success: bool,
) -> Result<VerificationResolutionCommitDecision, JournalError> {
    mrwe6_resolution_commit_decision(
        event,
        pending_action,
        pending_run,
        pending_step,
        commit_success,
    )
}

pub fn mrwe6_resolution_commit_decision(
    event: &JournalEvent,
    pending_action: vb_core::ActionId,
    pending_run: vb_core::RunId,
    pending_step: vb_core::StepIdx,
    commit_success: bool,
) -> Result<Mrwe6ResolutionCommitDecision, JournalError> {
    let _pending_key = index_action_key(pending_action, pending_run, pending_step)?;
    match ActionIndexIntent::for_event(event) {
        ActionIndexIntent::Delete { action, run, step } => {
            let _resolution_key = index_action_key(action, run, step)?;
            Ok(mrwe6_resolution_commit_decision_from_facts(
                true,
                action == pending_action && run == pending_run && step == pending_step,
                commit_success,
            ))
        }
        ActionIndexIntent::Put { .. } | ActionIndexIntent::None => Ok(
            mrwe6_resolution_commit_decision_from_facts(false, false, commit_success),
        ),
    }
}

#[must_use]
pub fn mrwe6_resolution_commit_decision_from_facts(
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

#[cfg(kani)]
pub(crate) fn verification_recovery_outcome(
    scheduled: &JournalEvent,
    resolution: Option<&JournalEvent>,
    marker_present: bool,
    legacy_profile: bool,
) -> Result<VerificationRecoveryOutcome, JournalError> {
    mrwe6_recovery_outcome(scheduled, resolution, marker_present, legacy_profile)
}

pub fn mrwe6_recovery_outcome(
    scheduled: &JournalEvent,
    resolution: Option<&JournalEvent>,
    marker_present: bool,
    legacy_profile: bool,
) -> Result<Mrwe6RecoveryOutcome, JournalError> {
    let ActionIndexIntent::Put { action, run, step } = ActionIndexIntent::for_event(scheduled)
    else {
        return Ok(Mrwe6RecoveryOutcome::ParityDefect);
    };
    let _scheduled_key = index_action_key(action, run, step)?;
    if let Some(resolution_event) = resolution {
        match ActionIndexIntent::for_event(resolution_event) {
            ActionIndexIntent::Delete {
                action: resolved_action,
                run: resolved_run,
                step: resolved_step,
            } => {
                let _resolution_key =
                    index_action_key(resolved_action, resolved_run, resolved_step)?;
                Ok(mrwe6_recovery_outcome_from_facts(
                    true,
                    true,
                    resolved_action == action && resolved_run == run && resolved_step == step,
                    marker_present,
                    legacy_profile,
                ))
            }
            ActionIndexIntent::Put { .. } | ActionIndexIntent::None => {
                Ok(mrwe6_recovery_outcome_from_facts(
                    true,
                    true,
                    false,
                    marker_present,
                    legacy_profile,
                ))
            }
        }
    } else {
        Ok(mrwe6_recovery_outcome_from_facts(
            true,
            false,
            false,
            marker_present,
            legacy_profile,
        ))
    }
}

#[must_use]
pub fn mrwe6_recovery_outcome_from_facts(
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
