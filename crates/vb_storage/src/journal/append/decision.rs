use super::intent::{
    ActionIndexIntent, Mrwe6EventClass, Mrwe6SeamError, mrwe6_action_index_intent,
    mrwe6_event_class,
};
pub use super::mrwe6_kernel::{
    Mrwe6DuplicateRetryDecision, Mrwe6RecoveryOutcome, Mrwe6ResolutionCommitDecision,
};
use crate::{error::JournalError, events::JournalEvent, keys::index_action_key};
#[cfg(kani)]
#[allow(dead_code)]
pub(crate) type VerificationDuplicateRetryDecision = Mrwe6DuplicateRetryDecision;
#[cfg(kani)]
#[allow(dead_code)]
pub(crate) type VerificationResolutionCommitDecision = Mrwe6ResolutionCommitDecision;
#[cfg(kani)]
#[allow(dead_code)]
pub(crate) type VerificationRecoveryOutcome = Mrwe6RecoveryOutcome;
#[cfg(kani)]
#[allow(dead_code)]
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
    super::mrwe6_kernel::duplicate_retry_decision_from_facts(
        equal_payload,
        retry_class,
        index_marker_present,
    )
}

pub fn mrwe6_idempotent_duplicate_retry_from_facts(
    equal_payload: bool,
    retry_class: Mrwe6EventClass,
    index_marker_present: bool,
) -> Result<Mrwe6DuplicateRetryDecision, Mrwe6SeamError> {
    let decision =
        mrwe6_duplicate_retry_decision_from_facts(equal_payload, retry_class, index_marker_present);
    if decision == Mrwe6DuplicateRetryDecision::IdempotentEqualRetry {
        Ok(decision)
    } else {
        Err(Mrwe6SeamError::DuplicateRetryNotIdempotent)
    }
}

#[cfg(kani)]
#[allow(dead_code)]
pub(crate) fn verification_resolution_marker_present_after_commit(
    event: &JournalEvent,
    existing_marker_matches_resolution: bool,
    commit_success: bool,
) -> Result<bool, JournalError> {
    match mrwe6_action_index_intent(event) {
        ActionIndexIntent::Delete { .. } => {
            Ok(!(commit_success && existing_marker_matches_resolution))
        }
        ActionIndexIntent::Put { .. } | ActionIndexIntent::None => {
            Ok(existing_marker_matches_resolution)
        }
    }
}

#[cfg(kani)]
#[allow(dead_code)]
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
    super::mrwe6_kernel::resolution_commit_decision_from_facts(
        is_resolution_event,
        key_matches_pending,
        commit_success,
    )
}

pub fn mrwe6_committed_resolution_from_facts(
    is_resolution_event: bool,
    key_matches_pending: bool,
    commit_success: bool,
) -> Result<Mrwe6ResolutionCommitDecision, Mrwe6SeamError> {
    let decision = mrwe6_resolution_commit_decision_from_facts(
        is_resolution_event,
        key_matches_pending,
        commit_success,
    );
    if decision == Mrwe6ResolutionCommitDecision::CommittedAndMarkerRemoved {
        Ok(decision)
    } else {
        Err(Mrwe6SeamError::ResolutionDidNotRemovePending)
    }
}

#[cfg(kani)]
#[allow(dead_code)]
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
    let ActionIndexIntent::Put { action, run, step } = mrwe6_action_index_intent(scheduled) else {
        return Ok(Mrwe6RecoveryOutcome::ParityDefect);
    };
    let _scheduled_key = index_action_key(action, run, step)?;
    recovery_outcome_for_scheduled(
        action,
        run,
        step,
        resolution,
        marker_present,
        legacy_profile,
    )
}

fn recovery_outcome_for_scheduled(
    action: vb_core::ActionId,
    run: vb_core::RunId,
    step: vb_core::StepIdx,
    resolution: Option<&JournalEvent>,
    marker_present: bool,
    legacy_profile: bool,
) -> Result<Mrwe6RecoveryOutcome, JournalError> {
    match resolution {
        Some(resolution_event) => recovery_outcome_for_resolution(
            action,
            run,
            step,
            resolution_event,
            marker_present,
            legacy_profile,
        ),
        None => Ok(recovery_outcome_without_resolution(
            marker_present,
            legacy_profile,
        )),
    }
}

fn recovery_outcome_without_resolution(
    marker_present: bool,
    legacy_profile: bool,
) -> Mrwe6RecoveryOutcome {
    mrwe6_recovery_outcome_from_facts(true, false, false, marker_present, legacy_profile)
}

fn recovery_outcome_for_resolution(
    action: vb_core::ActionId,
    run: vb_core::RunId,
    step: vb_core::StepIdx,
    resolution: &JournalEvent,
    marker_present: bool,
    legacy_profile: bool,
) -> Result<Mrwe6RecoveryOutcome, JournalError> {
    let matches_scheduled = resolution_matches_scheduled(action, run, step, resolution)?;
    Ok(mrwe6_recovery_outcome_from_facts(
        true,
        true,
        matches_scheduled,
        marker_present,
        legacy_profile,
    ))
}

fn resolution_matches_scheduled(
    action: vb_core::ActionId,
    run: vb_core::RunId,
    step: vb_core::StepIdx,
    resolution: &JournalEvent,
) -> Result<bool, JournalError> {
    match mrwe6_action_index_intent(resolution) {
        ActionIndexIntent::Delete {
            action: resolved_action,
            run: resolved_run,
            step: resolved_step,
        } => same_resolution_key(
            action,
            run,
            step,
            resolved_action,
            resolved_run,
            resolved_step,
        ),
        ActionIndexIntent::Put { .. } | ActionIndexIntent::None => Ok(false),
    }
}

fn same_resolution_key(
    action: vb_core::ActionId,
    run: vb_core::RunId,
    step: vb_core::StepIdx,
    resolved_action: vb_core::ActionId,
    resolved_run: vb_core::RunId,
    resolved_step: vb_core::StepIdx,
) -> Result<bool, JournalError> {
    let _resolution_key = index_action_key(resolved_action, resolved_run, resolved_step)?;
    Ok(resolved_action == action && resolved_run == run && resolved_step == step)
}

#[must_use]
pub fn mrwe6_recovery_outcome_from_facts(
    scheduled_has_pending_intent: bool,
    resolution_present: bool,
    resolution_matches_scheduled: bool,
    marker_present: bool,
    legacy_profile: bool,
) -> Mrwe6RecoveryOutcome {
    super::mrwe6_kernel::recovery_outcome_from_facts(
        scheduled_has_pending_intent,
        resolution_present,
        resolution_matches_scheduled,
        marker_present,
        legacy_profile,
    )
}

pub fn mrwe6_pending_inventory_from_facts(
    scheduled_has_pending_intent: bool,
    resolution_present: bool,
    resolution_matches_scheduled: bool,
    marker_present: bool,
    legacy_profile: bool,
) -> Result<Mrwe6RecoveryOutcome, Mrwe6SeamError> {
    let outcome = mrwe6_recovery_outcome_from_facts(
        scheduled_has_pending_intent,
        resolution_present,
        resolution_matches_scheduled,
        marker_present,
        legacy_profile,
    );
    if outcome == Mrwe6RecoveryOutcome::PendingInventory {
        Ok(outcome)
    } else {
        Err(Mrwe6SeamError::RecoveryOutcomeNotPendingInventory)
    }
}
