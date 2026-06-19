#![forbid(unsafe_code)]
//! Resolution commit decision logic.

use crate::error::JournalError;
use crate::events::JournalEvent;
use crate::journal::append::intent::{ActionIndexIntent, mrwe6_action_index_intent};
use crate::journal::append::mrwe6_kernel::Mrwe6ResolutionCommitDecision;
use crate::keys::index_action_key;

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

/// Type alias for Kani verification.
#[cfg(kani)]
#[allow(dead_code)]
pub(crate) type VerificationResolutionCommitDecision = Mrwe6ResolutionCommitDecision;

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
    crate::journal::append::mrwe6_kernel::resolution_commit_decision_from_facts(
        is_resolution_event,
        key_matches_pending,
        commit_success,
    )
}

pub fn mrwe6_committed_resolution_from_facts(
    is_resolution_event: bool,
    key_matches_pending: bool,
    commit_success: bool,
) -> Result<Mrwe6ResolutionCommitDecision, crate::journal::append::intent::Mrwe6SeamError> {
    let decision = mrwe6_resolution_commit_decision_from_facts(
        is_resolution_event,
        key_matches_pending,
        commit_success,
    );
    if decision == Mrwe6ResolutionCommitDecision::CommittedAndMarkerRemoved {
        Ok(decision)
    } else {
        Err(crate::journal::append::intent::Mrwe6SeamError::ResolutionDidNotRemovePending)
    }
}
