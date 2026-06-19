//! Incident event analysis pipeline.

use super::checkpoint::checkpoint_from_event;
use super::types::{
    IncidentAnalysis, IncidentEventCounts, IncidentFailureKind, SideEffect, SideEffectCertainty,
    SideEffectDisposition, SideEffectEvidence,
};
use crate::{EventSeq, events::JournalEvent};

/// Build incident analysis from a run's event stream.
pub fn analyze_incident_events(events: &[JournalEvent]) -> IncidentAnalysis {
    let mut result = IncidentAnalysis::default();
    let mut scan = IncidentScanState::default();

    for event in events {
        record_incident_event(&mut result, &mut scan, event);
    }

    result
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct IncidentScanState {
    last_step_started: Option<u16>,
}

fn record_incident_event(
    result: &mut IncidentAnalysis,
    scan: &mut IncidentScanState,
    event: &JournalEvent,
) {
    result.counts.record(event);
    result.last_sequence = Some(event.seq());
    result.last_checkpoint = Some(checkpoint_from_event(event));
    match event {
        JournalEvent::StepStarted { step, .. } => scan.last_step_started = Some(step.get()),
        JournalEvent::ActionScheduled { .. } => record_scheduled_event(result, event),
        JournalEvent::ActionScheduledTicket { .. } => record_scheduled_ticket(result, event),
        JournalEvent::ActionCompletedEvent { .. } => record_completed_event(result, event),
        JournalEvent::ActionCompletedEnvelope { .. } => record_completed_envelope(result, event),
        JournalEvent::ActionFailedEvent { .. } => record_failed_event(result, event),
        JournalEvent::RunFailedEvent { .. } => record_run_failure(
            result,
            IncidentFailureKind::RunFailed,
            scan.last_step_started,
        ),
        JournalEvent::RunCancelled { .. } => record_run_failure(
            result,
            IncidentFailureKind::RunCancelled,
            scan.last_step_started,
        ),
        JournalEvent::RunKilled { .. } => record_run_failure(
            result,
            IncidentFailureKind::RunKilled,
            scan.last_step_started,
        ),
        _ => {}
    }
}

impl IncidentEventCounts {
    fn record(&mut self, event: &JournalEvent) {
        increment(&mut self.total);
        match event {
            JournalEvent::RunAccepted { .. } => increment(&mut self.run_accepted),
            JournalEvent::RunAdmission { .. } => increment(&mut self.run_admission),
            JournalEvent::StepStarted { .. } => increment(&mut self.steps_started),
            JournalEvent::StepSucceeded { .. } => increment(&mut self.steps_succeeded),
            JournalEvent::ActionScheduled { .. } | JournalEvent::ActionScheduledTicket { .. } => {
                increment(&mut self.actions_scheduled);
            }
            JournalEvent::ActionCompletedEvent { .. }
            | JournalEvent::ActionCompletedEnvelope { .. } => {
                increment(&mut self.actions_completed);
            }
            JournalEvent::ActionFailedEvent { .. } => increment(&mut self.actions_failed),
            JournalEvent::SlotWrittenEvent { .. } => increment(&mut self.slot_writes),
            JournalEvent::WaitScheduledEvent { .. } => increment(&mut self.waits_scheduled),
            JournalEvent::AskScheduledEvent { .. } => increment(&mut self.asks_scheduled),
            JournalEvent::AskAnsweredEvent { .. } => increment(&mut self.asks_answered),
            JournalEvent::RetryScheduledEvent { .. } => increment(&mut self.retries_scheduled),
            JournalEvent::RunCancelled { .. } => increment(&mut self.run_cancelled),
            JournalEvent::RunKilled { .. } => increment(&mut self.run_killed),
            JournalEvent::RunFinished { .. } => increment(&mut self.run_finished),
            JournalEvent::RunFailedEvent { .. } => increment(&mut self.run_failed),
            JournalEvent::RunResumed { .. } => increment(&mut self.run_resumed),
            JournalEvent::RunRetried { .. } => increment(&mut self.run_retried),
            JournalEvent::RunAnswered { .. } => increment(&mut self.run_answered),
        }
    }
}

fn increment(count: &mut usize) {
    *count = count.saturating_add(1);
}

fn record_scheduled_event(result: &mut IncidentAnalysis, event: &JournalEvent) {
    if let JournalEvent::ActionScheduled {
        seq,
        step,
        action,
        attempt,
        ..
    } = event
    {
        record_scheduled_action(result, *seq, step.get(), action.get(), *attempt);
    }
}

fn record_scheduled_ticket(result: &mut IncidentAnalysis, event: &JournalEvent) {
    if let JournalEvent::ActionScheduledTicket { seq, ticket, .. } = event {
        record_scheduled_action(
            result,
            *seq,
            ticket.step.get(),
            ticket.action.get(),
            ticket.attempt,
        );
    }
}

fn record_completed_event(result: &mut IncidentAnalysis, event: &JournalEvent) {
    if let JournalEvent::ActionCompletedEvent {
        seq,
        step,
        action,
        attempt,
        ..
    } = event
    {
        let evidence = action_evidence(
            *seq,
            step.get(),
            action.get(),
            *attempt,
            SideEffectDisposition::Completed,
        );
        record_completed_action(result, evidence);
        record_legacy_side_effect(
            result,
            step.get(),
            action.get(),
            SideEffectCertainty::Confirmed,
        );
    }
}

fn record_completed_envelope(result: &mut IncidentAnalysis, event: &JournalEvent) {
    if let JournalEvent::ActionCompletedEnvelope { seq, ticket, .. } = event {
        let evidence = action_evidence(
            *seq,
            ticket.step.get(),
            ticket.action.get(),
            ticket.attempt,
            SideEffectDisposition::Completed,
        );
        record_completed_action(result, evidence);
        record_legacy_side_effect(
            result,
            ticket.step.get(),
            ticket.action.get(),
            SideEffectCertainty::Confirmed,
        );
    }
}

fn record_failed_event(result: &mut IncidentAnalysis, event: &JournalEvent) {
    if let JournalEvent::ActionFailedEvent {
        seq,
        step,
        action,
        attempt,
        ..
    } = event
    {
        let evidence = action_evidence(
            *seq,
            step.get(),
            action.get(),
            *attempt,
            SideEffectDisposition::Failed,
        );
        record_failed_action(result, evidence);
        record_legacy_side_effect(
            result,
            step.get(),
            action.get(),
            SideEffectCertainty::Failed,
        );
    }
}

fn record_run_failure(
    result: &mut IncidentAnalysis,
    kind: IncidentFailureKind,
    failed_at_step: Option<u16>,
) {
    result.failure_found = true;
    result.failure_kind = Some(kind);
    result.failure_code = kind.code().to_string();
    result.failed_at_step = failed_at_step;
}

fn record_scheduled_action(
    result: &mut IncidentAnalysis,
    seq: EventSeq,
    step: u16,
    action: u16,
    attempt: u16,
) {
    let evidence = action_evidence(seq, step, action, attempt, SideEffectDisposition::Scheduled);
    result.side_effect_evidence.push(evidence);
    result.pending_scheduled_actions.push(evidence);
}

fn record_completed_action(result: &mut IncidentAnalysis, evidence: SideEffectEvidence) {
    result.side_effect_evidence.push(evidence);
    resolve_pending_action(&mut result.pending_scheduled_actions, evidence);
}

fn record_failed_action(result: &mut IncidentAnalysis, evidence: SideEffectEvidence) {
    result.side_effect_evidence.push(evidence);
    result.failed_action_evidence.push(evidence);
    resolve_pending_action(&mut result.pending_scheduled_actions, evidence);
}

fn record_legacy_side_effect(
    result: &mut IncidentAnalysis,
    step: u16,
    action: u16,
    certainty: SideEffectCertainty,
) {
    result.side_effects.push(SideEffect {
        step,
        action,
        certainty,
    });
}

fn resolve_pending_action(pending: &mut Vec<SideEffectEvidence>, resolved: SideEffectEvidence) {
    let key = ActionEvidenceKey::from_evidence(resolved);
    pending.retain(|candidate| !key.matches(*candidate));
}

fn action_evidence(
    seq: EventSeq,
    step: u16,
    action: u16,
    attempt: u16,
    disposition: SideEffectDisposition,
) -> SideEffectEvidence {
    SideEffectEvidence {
        seq,
        step,
        action,
        attempt,
        disposition,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActionEvidenceKey {
    step: u16,
    action: u16,
    attempt: u16,
}

impl ActionEvidenceKey {
    fn from_evidence(evidence: SideEffectEvidence) -> Self {
        Self {
            step: evidence.step,
            action: evidence.action,
            attempt: evidence.attempt,
        }
    }

    fn matches(self, evidence: SideEffectEvidence) -> bool {
        self.step == evidence.step
            && self.action == evidence.action
            && self.attempt == evidence.attempt
    }
}
