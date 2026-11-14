//! Durable run resource accounting reconstructed from journal evidence.

use crate::{RuntimeError, RuntimeResult};

#[path = "accounting/origin.rs"]
mod origin;

#[cfg(test)]
pub(crate) use origin::hydrate_run_budget_origin;
pub(crate) use origin::{recover_run_budget_origin, resolve_run_started_millis};

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

/// Returns the durable tenant attributed to the latest admission, if any.
#[must_use]
pub(crate) fn hydrate_run_tenant(
    events: &[vb_storage::JournalEvent],
) -> Option<vb_core::ids::TenantId> {
    events.iter().rev().find_map(|event| match event {
        vb_storage::JournalEvent::TenantAdmissionEvent { tenant, .. } => Some(*tenant),
        _ => None,
    })
}

/// Reconstructs cumulative journal bytes from durable event wire sizes.
pub(crate) fn recover_journal_bytes_written(
    events: &[vb_storage::JournalEvent],
) -> RuntimeResult<u64> {
    let mut total = 0u64;
    for event in events {
        let len = crate::journal::storage_event_bytes(event)?;
        total = total.checked_add(len).ok_or(RuntimeError::EncodeFailed)?;
    }
    Ok(total)
}

/// Requires replayed counters whenever the historical admission enforces them.
pub(crate) fn recover_admission_accounting(
    admission: Option<&crate::admission::RunAdmission>,
    action_attempts: Option<Box<[u16]>>,
    journal_bytes_written: Option<u64>,
    step_count: u16,
) -> RuntimeResult<(Box<[u16]>, u64)> {
    let budgeted = admission.is_some_and(|value| value.budget().is_some());
    let action_attempts = match (action_attempts, budgeted) {
        (Some(attempts), _) => attempts,
        (None, true) => return Err(missing_accounting("action_attempts_missing")),
        (None, false) => crate::shard::helpers::new_action_attempts(step_count),
    };
    let journal_bytes_written = match (journal_bytes_written, budgeted) {
        (Some(bytes), _) => bytes,
        (None, true) => return Err(missing_accounting("journal_bytes_missing")),
        (None, false) => 0,
    };
    Ok((action_attempts, journal_bytes_written))
}

fn missing_accounting(reason: &'static str) -> RuntimeError {
    RuntimeError::RecoveryCannotResume {
        reason: String::from(reason),
    }
}

/// Rejects a durable origin that cannot be trusted as a past or current time.
pub(crate) fn validate_budget_origin(started: u64, now: u64) -> RuntimeResult<()> {
    if started > now {
        return Err(RuntimeError::RecoveryCannotResume {
            reason: String::from("budget_origin_future"),
        });
    }
    Ok(())
}

/// Refuses a future or expired origin against its admitted lifetime cap.
pub(crate) fn validate_lifetime_budget(
    budget: Option<&vb_core::AggregateResourceBudget>,
    started: u64,
    now: u64,
) -> RuntimeResult<()> {
    validate_budget_origin(started, now)?;
    let Some(budget) = budget else {
        return Ok(());
    };
    match vb_core::budget::RunLifetimeBudget::from_budget(started, budget)
        .and_then(|lifetime| lifetime.check(now).map(|()| lifetime))
    {
        Ok(_) => Ok(()),
        Err(vb_core::budget::LifetimeBudgetError::Expired { .. }) => {
            Err(RuntimeError::RecoveryCannotResume {
                reason: String::from("run_lifetime_expired"),
            })
        }
        Err(_) => Err(RuntimeError::RecoveryCannotResume {
            reason: String::from("budget_deadline_invalid"),
        }),
    }
}

/// Refuses expired durable budgets before recovery can publish resumable state.
#[cfg(test)]
pub(crate) fn validate_recovered_lifetime(
    admission: Option<&crate::admission::RunAdmission>,
    recovered: Option<u64>,
    now: u64,
) -> RuntimeResult<Option<u64>> {
    let Some(started) = resolve_run_started_millis(admission, recovered)? else {
        return Ok(None);
    };
    let budget = admission.and_then(crate::admission::RunAdmission::budget);
    validate_lifetime_budget(budget.as_ref(), started, now)?;
    Ok(Some(started))
}
/// Validates durable lifetime evidence before recovery mutates shard state.
pub(crate) fn validate_recovered_lifetime_at_wall_clock(
    admission: Option<&crate::admission::RunAdmission>,
    recovered: Option<u64>,
) -> RuntimeResult<Option<u64>> {
    let Some(started) = resolve_run_started_millis(admission, recovered)? else {
        return Ok(None);
    };
    let now = crate::shard::helpers::wall_clock_millis_since_epoch().ok_or(
        RuntimeError::RecoveryCannotResume {
            reason: String::from("wall_clock_unavailable"),
        },
    )?;
    let budget = admission.and_then(crate::admission::RunAdmission::budget);
    validate_lifetime_budget(budget.as_ref(), started, now)?;
    Ok(Some(started))
}
