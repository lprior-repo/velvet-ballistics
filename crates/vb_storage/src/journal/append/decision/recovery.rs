#![forbid(unsafe_code)]
//! Recovery outcome logic.

use crate::journal::append::intent::{ActionIndexIntent, Mrwe6EventClass, mrwe6_action_index_intent, mrwe6_event_class};
use crate::journal::append::mrwe6_kernel::Mrwe6RecoveryOutcome;
use crate::error::JournalError;
use crate::events::JournalEvent;
use crate::keys::index_action_key;

#[cfg(kani)]
#[allow(dead_code)]
pub(crate) type VerificationRecoveryOutcome = Mrwe6RecoveryOutcome;

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
        ActionIndexIntent::Put { .. } | ActionIndexIntent::None => {
            Ok(false)
        }
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
    crate::journal::append::mrwe6_kernel::recovery_outcome_from_facts(
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
) -> Result<Mrwe6RecoveryOutcome, crate::journal::append::intent::Mrwe6SeamError> {
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
        Err(crate::journal::append::intent::Mrwe6SeamError::RecoveryOutcomeNotPendingInventory)
    }
}
