use crate::{RuntimeError, RuntimeResult};
use vb_core::{ActionId, ActionTicket, CompiledNodeKind, CompiledWorkflow, RunId, StepIdx};
use vb_storage::JournalEvent;

struct FailedAction<'a> {
    run: RunId,
    step: StepIdx,
    action: ActionId,
    action_attempt: u16,
    prefix: &'a [JournalEvent],
    tail: &'a [JournalEvent],
}

pub(super) fn validate(events: &[JournalEvent], workflow: &CompiledWorkflow) -> RuntimeResult<()> {
    let Some(failed) = latest_failed_action(events) else {
        return Ok(());
    };
    let Some(ticket) = remaining_immediate_retry(&failed, workflow) else {
        return Ok(());
    };
    if selected_failure_outcome(&failed, workflow, ticket) {
        return Ok(());
    }
    Err(RuntimeError::RecoveryCannotResume {
        reason: String::from("immediate_retry_decision_missing"),
    })
}

fn latest_failed_action(events: &[JournalEvent]) -> Option<FailedAction<'_>> {
    events.iter().enumerate().rev().find_map(|(index, event)| {
        let JournalEvent::ActionFailedEvent {
            run,
            step,
            action,
            attempt,
            ..
        } = event
        else {
            return None;
        };
        Some(FailedAction {
            run: *run,
            step: *step,
            action: *action,
            action_attempt: *attempt,
            prefix: events.get(..index)?,
            tail: events.get(index.checked_add(1)?..)?,
        })
    })
}

fn remaining_immediate_retry(
    failed: &FailedAction<'_>,
    workflow: &CompiledWorkflow,
) -> Option<ActionTicket> {
    let node = workflow.node(failed.step)?;
    let policy = node.retry?;
    if policy.delay_ms != 0
        || !matches!(node.kind, CompiledNodeKind::Do { action, .. } if action == failed.action)
    {
        return None;
    }
    let ticket = matching_schedule(failed)?;
    (ticket.attempt > 0
        && ticket.attempt < policy.max_attempts
        && ticket.capacity == policy.max_attempts)
        .then_some(ticket)
}

fn matching_schedule(failed: &FailedAction<'_>) -> Option<ActionTicket> {
    failed.prefix.iter().rev().find_map(|event| match event {
        JournalEvent::ActionScheduledTicket { run, ticket, .. }
            if *run == failed.run
                && ticket.run == failed.run
                && ticket.step == failed.step
                && ticket.action == failed.action
                && ticket.attempt == failed.action_attempt =>
        {
            Some(*ticket)
        }
        _ => None,
    })
}

fn selected_failure_outcome(
    failed: &FailedAction<'_>,
    workflow: &CompiledWorkflow,
    ticket: ActionTicket,
) -> bool {
    let next = failed
        .tail
        .iter()
        .find(|event| !matches!(event, JournalEvent::RunResumed { .. }));
    match next {
        Some(JournalEvent::RunFailedEvent { run, attempt, .. }) => {
            *run == failed.run && *attempt == ticket.run_attempt
        }
        Some(JournalEvent::StepStarted {
            run, step, attempt, ..
        }) => {
            *run == failed.run
                && *attempt == ticket.run_attempt
                && crate::recovery::handler_route_for_failure(workflow, failed.step)
                    .is_some_and(|route| route.handler == *step)
        }
        _ => false,
    }
}
