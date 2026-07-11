use vb_core::action::{ActionTicket, action_ticket_has_valid_key};
use vb_core::ids::{ActionId, RunId, SlotIdx, WorkflowDigest};
use vb_core::{CompiledNodeKind, CompiledWorkflow};
use vb_storage::recovery::RecoveredPendingAction;

use super::RecoveredPendingActionTicket;
use crate::{RuntimeError, RuntimeResult};

/// Returns the single durable pending-action ticket needed by live recovery.
pub(crate) fn pending_action_ticket_from_events(
    seed: &vb_storage::recovery::RecoveryFrameSeed,
    events: &[vb_storage::JournalEvent],
    workflow: &CompiledWorkflow,
    expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
) -> RuntimeResult<Option<RecoveredPendingActionTicket>> {
    let Some((entry, rest)) = seed.pending_actions.split_first() else {
        return Ok(None);
    };
    if !rest.is_empty() {
        return Err(cannot_resume_error("pending_actions"));
    }
    matching_pending_action_ticket(entry, events, workflow, expected_action_abi_digests).map(Some)
}

fn matching_pending_action_ticket(
    entry: &RecoveredPendingAction,
    events: &[vb_storage::JournalEvent],
    workflow: &CompiledWorkflow,
    expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
) -> RuntimeResult<RecoveredPendingActionTicket> {
    let mut recovered = None;
    for event in events {
        match pending_action_event_effect(entry, event, workflow, expected_action_abi_digests)? {
            PendingActionEventEffect::Scheduled(evidence) => {
                if let Some(existing) = recovered
                    && existing != evidence
                {
                    return Err(cannot_resume_error("pending_actions"));
                }
                recovered = Some(evidence);
            }
            PendingActionEventEffect::Resolved => {
                recovered = None;
            }
            PendingActionEventEffect::Irrelevant => {}
        }
    }
    recovered.ok_or_else(|| cannot_resume_error("pending_actions"))
}

enum PendingActionEventEffect {
    Scheduled(RecoveredPendingActionTicket),
    Resolved,
    Irrelevant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingActionTicketEvent {
    run: RunId,
    ticket: ActionTicket,
    input: SlotIdx,
    output: SlotIdx,
    action_abi_digest: WorkflowDigest,
}

fn pending_action_event_effect(
    entry: &RecoveredPendingAction,
    event: &vb_storage::JournalEvent,
    workflow: &CompiledWorkflow,
    expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
) -> RuntimeResult<PendingActionEventEffect> {
    if let Some(effect) =
        scheduled_pending_action_effect(entry, event, workflow, expected_action_abi_digests)?
    {
        return Ok(effect);
    }
    Ok(resolved_pending_action_effect(entry, event))
}

fn scheduled_pending_action_effect(
    entry: &RecoveredPendingAction,
    event: &vb_storage::JournalEvent,
    workflow: &CompiledWorkflow,
    expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
) -> RuntimeResult<Option<PendingActionEventEffect>> {
    let Some((seq, ticket_event)) = scheduled_ticket_event_for_entry(entry, event) else {
        return Ok(None);
    };
    validate_pending_action_ticket_event(
        entry,
        ticket_event,
        workflow,
        expected_action_abi_digests,
    )?;
    Ok(Some(scheduled_ticket_effect(seq, ticket_event)))
}

fn scheduled_ticket_event_for_entry(
    entry: &RecoveredPendingAction,
    event: &vb_storage::JournalEvent,
) -> Option<(vb_storage::EventSeq, PendingActionTicketEvent)> {
    let vb_storage::JournalEvent::ActionScheduledTicket {
        run,
        seq,
        ticket,
        input,
        output,
        action_abi_digest,
    } = event
    else {
        return None;
    };
    if ticket.step != entry.step || ticket.action != entry.action {
        return None;
    }
    Some((
        *seq,
        PendingActionTicketEvent {
            run: *run,
            ticket: *ticket,
            input: *input,
            output: *output,
            action_abi_digest: *action_abi_digest,
        },
    ))
}

fn scheduled_ticket_effect(
    seq: vb_storage::EventSeq,
    event: PendingActionTicketEvent,
) -> PendingActionEventEffect {
    PendingActionEventEffect::Scheduled(RecoveredPendingActionTicket::new(
        seq,
        event.ticket,
        event.input,
        event.output,
        event.action_abi_digest,
    ))
}

fn resolved_pending_action_effect(
    entry: &RecoveredPendingAction,
    event: &vb_storage::JournalEvent,
) -> PendingActionEventEffect {
    match event {
        vb_storage::JournalEvent::ActionCompletedEvent { action, step, .. }
        | vb_storage::JournalEvent::ActionFailedEvent { action, step, .. }
            if *action == entry.action && *step == entry.step =>
        {
            PendingActionEventEffect::Resolved
        }
        vb_storage::JournalEvent::ActionCompletedEnvelope { ticket, .. }
        | vb_storage::JournalEvent::ActionAbandoned { ticket, .. }
            if ticket.action == entry.action && ticket.step == entry.step =>
        {
            PendingActionEventEffect::Resolved
        }
        _ => PendingActionEventEffect::Irrelevant,
    }
}

fn validate_pending_action_ticket_event(
    entry: &RecoveredPendingAction,
    event: PendingActionTicketEvent,
    workflow: &CompiledWorkflow,
    expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
) -> RuntimeResult<()> {
    if event.ticket.run != event.run
        || event.ticket.step != entry.step
        || event.ticket.action != entry.action
        || event.ticket.attempt == 0
        || event.ticket.capacity == 0
        || !action_ticket_has_valid_key(event.ticket)
    {
        return Err(cannot_resume_error("pending_actions"));
    }
    validate_pending_action_slots(event.ticket, event.input, event.output, workflow)?;
    validate_pending_action_abi(
        event.ticket.action,
        event.action_abi_digest,
        expected_action_abi_digests,
    )
}

fn validate_pending_action_slots(
    ticket: ActionTicket,
    input: SlotIdx,
    output: SlotIdx,
    workflow: &CompiledWorkflow,
) -> RuntimeResult<()> {
    let Some(node) = workflow.node(ticket.step) else {
        return Err(cannot_resume_error("pending_actions"));
    };
    match node.kind {
        CompiledNodeKind::Do {
            action,
            input: expected_input,
        } if action == ticket.action && expected_input == input && node.output == Some(output) => {
            Ok(())
        }
        _ => Err(cannot_resume_error("pending_actions")),
    }
}

fn validate_pending_action_abi(
    action: ActionId,
    found: WorkflowDigest,
    expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
) -> RuntimeResult<()> {
    if found.as_bytes() == [0u8; 32] {
        return Err(cannot_resume_error("action_abi_digests_missing"));
    }
    let Some(expected) = expected_action_abi_digest(action, expected_action_abi_digests) else {
        return Err(cannot_resume_error("action_abi_digests_missing"));
    };
    if expected.as_bytes() == [0u8; 32] || expected != found {
        return Err(cannot_resume_error("action_abi_digest_mismatch"));
    }
    Ok(())
}

fn expected_action_abi_digest(
    action: ActionId,
    expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
) -> Option<WorkflowDigest> {
    expected_action_abi_digests.iter().find_map(
        |(id, digest)| {
            if *id == action { Some(*digest) } else { None }
        },
    )
}

fn cannot_resume_error(reason: &'static str) -> RuntimeError {
    RuntimeError::RecoveryCannotResume {
        reason: String::from(reason),
    }
}
