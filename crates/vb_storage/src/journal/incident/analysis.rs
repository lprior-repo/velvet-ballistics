use super::{IncidentAnalysis, SideEffect, SideEffectCertainty};
use crate::events::JournalEvent;

/// Build incident analysis from a run's event stream.
pub fn analyze_incident_events(events: &[JournalEvent]) -> IncidentAnalysis {
    let (failure_found, failure_code, failed_at_step) = incident_failure(events);
    let side_effects = events.iter().filter_map(side_effect_from_event).collect();
    IncidentAnalysis {
        failure_found,
        failure_code,
        failed_at_step,
        side_effects,
    }
}

fn incident_failure(events: &[JournalEvent]) -> (bool, String, Option<u16>) {
    let (_, failure) = events
        .iter()
        .fold((None, None), |(last, failure), event| match event {
            JournalEvent::StepStarted { step, .. } => (Some(step.get()), failure),
            JournalEvent::StepFailed { step, .. } => (
                last,
                Some((true, "StepFailed".to_string(), Some(step.get()))),
            ),
            JournalEvent::RunFailedEvent { .. } => {
                (last, Some((true, "RunFailed".to_string(), last)))
            }
            JournalEvent::RunCancelled { .. } => {
                (last, Some((true, "RunCancelled".to_string(), last)))
            }
            JournalEvent::RunKilled { .. } => (last, Some((true, "RunKilled".to_string(), last))),
            _ => (last, failure),
        });
    failure.map_or((false, String::new(), None), std::convert::identity)
}

fn side_effect_from_event(event: &JournalEvent) -> Option<SideEffect> {
    let fact = action_fact(event)?;
    Some(SideEffect {
        run: event.run_id(),
        seq: event.seq(),
        attempt: event.attempt()?,
        step: fact.step,
        action: fact.action,
        ticket: fact.ticket,
        action_abi_digest: fact.digest,
        certainty: fact.certainty,
    })
}

struct ActionFact {
    step: u16,
    action: u16,
    ticket: Option<vb_core::ActionTicket>,
    digest: Option<vb_core::WorkflowDigest>,
    certainty: SideEffectCertainty,
}

impl ActionFact {
    fn legacy(step: u16, action: u16, certainty: SideEffectCertainty) -> Self {
        Self {
            step,
            action,
            ticket: None,
            digest: None,
            certainty,
        }
    }

    fn ticketed(
        ticket: vb_core::ActionTicket,
        digest: Option<vb_core::WorkflowDigest>,
        certainty: SideEffectCertainty,
    ) -> Self {
        Self {
            step: ticket.step.get(),
            action: ticket.action.get(),
            ticket: Some(ticket),
            digest,
            certainty,
        }
    }
}

fn action_fact(event: &JournalEvent) -> Option<ActionFact> {
    match event {
        JournalEvent::ActionScheduled { .. }
        | JournalEvent::ActionCompletedEvent { .. }
        | JournalEvent::ActionFailedEvent { .. } => legacy_action_fact(event),
        JournalEvent::ActionScheduledTicket { .. }
        | JournalEvent::ActionCompletedEnvelope { .. }
        | JournalEvent::ActionAbandoned { .. } => ticket_action_fact(event),
        _ => None,
    }
}

fn legacy_action_fact(event: &JournalEvent) -> Option<ActionFact> {
    match event {
        JournalEvent::ActionScheduled { step, action, .. } => Some(ActionFact::legacy(
            step.get(),
            action.get(),
            SideEffectCertainty::Scheduled,
        )),
        JournalEvent::ActionCompletedEvent { step, action, .. } => Some(ActionFact::legacy(
            step.get(),
            action.get(),
            SideEffectCertainty::Confirmed,
        )),
        JournalEvent::ActionFailedEvent { step, action, .. } => Some(ActionFact::legacy(
            step.get(),
            action.get(),
            SideEffectCertainty::Failed,
        )),
        _ => None,
    }
}

fn ticket_action_fact(event: &JournalEvent) -> Option<ActionFact> {
    match event {
        JournalEvent::ActionScheduledTicket {
            ticket,
            action_abi_digest,
            ..
        } => Some(ActionFact::ticketed(
            *ticket,
            Some(*action_abi_digest),
            SideEffectCertainty::Scheduled,
        )),
        JournalEvent::ActionCompletedEnvelope {
            ticket,
            action_abi_digest,
            ..
        } => Some(ActionFact::ticketed(
            *ticket,
            Some(*action_abi_digest),
            SideEffectCertainty::Confirmed,
        )),
        JournalEvent::ActionAbandoned { ticket, .. } => Some(ActionFact::ticketed(
            *ticket,
            None,
            SideEffectCertainty::Abandoned,
        )),
        _ => None,
    }
}
