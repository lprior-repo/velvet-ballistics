use std::collections::HashMap;

use serde_json::{Value, json};
use vb_core::ActionTicket;
use vb_storage::{JournalEvent, SideEffect, SideEffectCertainty};

pub(super) fn side_effect_rows(events: &[JournalEvent], effects: &[SideEffect]) -> Vec<Value> {
    let evidence = ticket_phase_evidence(events);
    effects
        .iter()
        .map(|effect| side_effect_row(&evidence, effect))
        .collect()
}

fn side_effect_row(evidence: &TicketPhaseIndex, effect: &SideEffect) -> Value {
    let (scheduled, completed) = durable_phases(evidence, effect);
    json!({
        "run_id": effect.run.get(),
        "seq": effect.seq.get(),
        "attempt": effect.attempt,
        "step": effect.step,
        "action": effect.action,
        "certainty": certainty_name(&effect.certainty),
        "ticket": effect.ticket.map_or_else(|| Value::Null, ticket_value),
        "action_abi_digest": effect.action_abi_digest.map_or_else(|| Value::Null, digest_value),
        "scheduled_durable": scheduled,
        "completion_durable": completed,
        "external_effect": "uncertain",
        "safe_to_retry": Value::Null,
        "reason": "retry_contract_not_loaded",
    })
}

fn certainty_name(certainty: &SideEffectCertainty) -> &'static str {
    match certainty {
        SideEffectCertainty::Scheduled => "scheduled",
        SideEffectCertainty::Confirmed => "confirmed",
        SideEffectCertainty::Failed => "failed",
        SideEffectCertainty::Abandoned => "abandoned",
    }
}

fn ticket_value(ticket: ActionTicket) -> Value {
    json!({
        "run": ticket.run.get(),
        "step": ticket.step.get(),
        "seq": ticket.seq.get(),
        "action": ticket.action.get(),
        "run_attempt": ticket.run_attempt,
        "attempt": ticket.attempt,
        "idempotency_key": ticket.idempotency_key.to_string(),
        "capacity": ticket.capacity,
        "fencing_token": ticket.fencing_token,
    })
}

fn digest_value(digest: vb_core::WorkflowDigest) -> Value {
    let encoded = digest
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Value::String(encoded)
}

fn durable_phases(evidence: &TicketPhaseIndex, effect: &SideEffect) -> (Value, Value) {
    match effect.ticket {
        Some(ticket) => ticket_phases(evidence.get(&ticket), effect),
        None => legacy_phases(effect),
    }
}

fn ticket_phases(evidence: Option<&TicketPhaseEvidence>, effect: &SideEffect) -> (Value, Value) {
    let scheduled_before = evidence
        .and_then(|value| value.scheduled.first)
        .is_some_and(|seq| seq < effect.seq.get());
    let completed_after = evidence
        .and_then(|value| value.completed.last)
        .is_some_and(|seq| seq > effect.seq.get());
    match effect.certainty {
        SideEffectCertainty::Scheduled => (Value::Bool(true), phase_value(completed_after)),
        SideEffectCertainty::Confirmed => (phase_value(scheduled_before), Value::Bool(true)),
        SideEffectCertainty::Abandoned => (phase_value(scheduled_before), Value::Null),
        SideEffectCertainty::Failed => (Value::Null, Value::Null),
    }
}

fn legacy_phases(effect: &SideEffect) -> (Value, Value) {
    match effect.certainty {
        SideEffectCertainty::Scheduled => (Value::Bool(true), Value::Null),
        SideEffectCertainty::Confirmed => (Value::Null, Value::Bool(true)),
        SideEffectCertainty::Failed | SideEffectCertainty::Abandoned => (Value::Null, Value::Null),
    }
}

fn phase_value(present: bool) -> Value {
    if present {
        Value::Bool(true)
    } else {
        Value::Null
    }
}

type TicketPhaseIndex = HashMap<ActionTicket, TicketPhaseEvidence>;

#[derive(Default)]
struct TicketPhaseEvidence {
    scheduled: SequenceBounds,
    completed: SequenceBounds,
}

impl TicketPhaseEvidence {
    fn record(&mut self, phase: TicketPhase, seq: u64) {
        match phase {
            TicketPhase::Scheduled => self.scheduled.include(seq),
            TicketPhase::Completed => self.completed.include(seq),
        }
    }
}

#[derive(Default)]
struct SequenceBounds {
    first: Option<u64>,
    last: Option<u64>,
}

impl SequenceBounds {
    fn include(&mut self, seq: u64) {
        self.first = Some(self.first.map_or(seq, |seen| seen.min(seq)));
        self.last = Some(self.last.map_or(seq, |seen| seen.max(seq)));
    }
}

#[derive(Clone, Copy)]
enum TicketPhase {
    Scheduled,
    Completed,
}

fn ticket_phase_evidence(events: &[JournalEvent]) -> TicketPhaseIndex {
    events.iter().filter_map(ticket_phase).fold(
        HashMap::new(),
        |mut index, (ticket, seq, phase)| {
            index.entry(ticket).or_default().record(phase, seq);
            index
        },
    )
}

fn ticket_phase(event: &JournalEvent) -> Option<(ActionTicket, u64, TicketPhase)> {
    match event {
        JournalEvent::ActionScheduledTicket { ticket, .. } => {
            Some((*ticket, event.seq().get(), TicketPhase::Scheduled))
        }
        JournalEvent::ActionCompletedEnvelope { ticket, .. } => {
            Some((*ticket, event.seq().get(), TicketPhase::Completed))
        }
        _ => None,
    }
}

pub(super) fn journal_tail(events: &[JournalEvent]) -> Vec<Value> {
    events
        .iter()
        .map(|event| {
            json!({
                "seq": event.seq().get(),
                "kind": format!("{:?}", event.record_kind()),
                "run_id": event.run_id().get(),
                "attempt": event.attempt(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
