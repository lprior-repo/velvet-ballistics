//! Durable run resource accounting reconstructed from journal evidence.

use crate::{RuntimeError, RuntimeResult};

/// Returns the durable wall-clock origin for the latest budgeted admission.
#[must_use]
pub(crate) fn hydrate_run_budget_origin(events: &[vb_storage::JournalEvent]) -> Option<u64> {
    events.iter().rev().find_map(|event| match event {
        vb_storage::JournalEvent::RunBudgetOriginEvent { started_millis, .. } => {
            Some(*started_millis)
        }
        _ => None,
    })
}

/// Reconstructs per-step action attempts from durable action evidence.
#[must_use]
pub(crate) fn recover_action_attempts(
    events: &[vb_storage::JournalEvent],
    step_count: u16,
) -> Box<[u16]> {
    let mut attempts = crate::shard::helpers::new_action_attempts(step_count);
    for event in events {
        let (step, attempt) = match event {
            vb_storage::JournalEvent::ActionScheduled { step, attempt, .. }
            | vb_storage::JournalEvent::ActionFailedEvent { step, attempt, .. } => {
                (*step, *attempt)
            }
            vb_storage::JournalEvent::ActionScheduledTicket { ticket, .. }
            | vb_storage::JournalEvent::ActionCompletedEnvelope { ticket, .. } => {
                (ticket.step, ticket.attempt)
            }
            _ => continue,
        };
        if let Some(slot) = attempts.get_mut(step.as_usize())
            && attempt > *slot
        {
            *slot = attempt;
        }
    }
    attempts
}

/// Reconstructs cumulative journal bytes from durable event wire sizes.
pub(crate) fn recover_journal_bytes_written(
    events: &[vb_storage::JournalEvent],
) -> RuntimeResult<u64> {
    let mut total = 0u64;
    for event in events {
        let encoded = postcard::to_allocvec(event).map_err(|_| RuntimeError::EncodeFailed)?;
        let len = u64::try_from(encoded.len()).map_err(|_| RuntimeError::EncodeFailed)?;
        total = total.checked_add(len).ok_or(RuntimeError::EncodeFailed)?;
    }
    Ok(total)
}

/// Resolves the single durable wall-clock origin for a recovered admission.
///
/// A budgeted admission without any durable origin is refused with a typed
/// missing-authority recovery error; it is never restarted at `now` and never
/// silently expired at epoch zero.
pub(crate) fn resolve_run_started_millis(
    admission: Option<&crate::admission::RunAdmission>,
    recovered: Option<u64>,
) -> RuntimeResult<Option<u64>> {
    let budgeted = admission.is_some_and(|admission| admission.budget().is_some());
    match (recovered, budgeted) {
        (Some(started), _) => Ok(Some(started)),
        (None, true) => Err(RuntimeError::RecoveryCannotResume {
            reason: String::from("budget_origin_missing"),
        }),
        (None, false) => Ok(None),
    }
}
