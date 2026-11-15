#![forbid(unsafe_code)]

use serde_json::{Value, json};
use vb_storage::JournalEvent;

pub(super) fn fields(event: &JournalEvent) -> Option<Value> {
    admission(event)
        .or_else(|| budget(event))
        .or_else(|| retry(event))
        .or_else(|| decision(event))
        .or_else(|| gate_opened(event))
        .or_else(|| gate_opened_bound(event))
        .or_else(|| run_record(event))
}

fn admission(event: &JournalEvent) -> Option<Value> {
    match event {
        JournalEvent::RunAdmission {
            artifact_digest,
            input_digest,
            policy_pin,
            granted_capabilities,
            policy,
            ..
        } => Some(
            json!({"type":"RunAdmission","artifact_digest":format!("{artifact_digest:?}"),"input_digest":format!("{input_digest:?}"),"policy_pin":format!("{policy_pin:?}"),"granted_capabilities":format!("{granted_capabilities:?}"),"policy":format!("{policy:?}")}),
        ),
        _ => None,
    }
}

fn budget(event: &JournalEvent) -> Option<Value> {
    match event {
        JournalEvent::RunAdmissionBudget {
            artifact_digest,
            input_digest,
            policy_pin,
            granted_capabilities,
            policy,
            budget,
            ..
        } => Some(
            json!({"type":"RunAdmissionBudget","artifact_digest":format!("{artifact_digest:?}"),"input_digest":format!("{input_digest:?}"),"policy_pin":format!("{policy_pin:?}"),"granted_capabilities":format!("{granted_capabilities:?}"),"policy":format!("{policy:?}"),"budget":format!("{budget:?}")}),
        ),
        _ => None,
    }
}

fn retry(event: &JournalEvent) -> Option<Value> {
    match event {
        JournalEvent::RetryCheckpoint {
            step,
            payload_digest,
            payload,
            ..
        } => Some(
            json!({"type":"RetryCheckpoint","step":step.get(),"payload_digest":format!("{payload_digest:?}"),"payload":payload}),
        ),
        JournalEvent::RetryTimerScheduled { step, timer, .. } => Some(
            json!({"type":"RetryTimerScheduled","step":step.get(),"timer":format!("{timer:?}")}),
        ),
        _ => None,
    }
}

fn decision(event: &JournalEvent) -> Option<Value> {
    match event {
        JournalEvent::PolicyDecisionEvent { decision, .. } => {
            Some(json!({"type":"PolicyDecisionEvent","decision":format!("{decision:?}")}))
        }
        JournalEvent::GateDecisionEvent { record, .. } => {
            Some(json!({"type":"GateDecisionEvent","record":format!("{record:?}")}))
        }
        JournalEvent::CompatibilityDecisionEvent { decision, .. } => {
            Some(json!({"type":"CompatibilityDecisionEvent","decision":format!("{decision:?}")}))
        }
        JournalEvent::GateDecisionBoundEvent { record, .. } => {
            Some(json!({"type":"GateDecisionBoundEvent","record":format!("{record:?}")}))
        }
        _ => None,
    }
}

fn gate_opened(event: &JournalEvent) -> Option<Value> {
    match event {
        JournalEvent::GateOpenedEvent {
            gate,
            opened_at_ms,
            requester,
            ..
        } => Some(
            json!({"type":"GateOpenedEvent","gate":format!("{gate:?}"),"opened_at_ms":opened_at_ms,"requester":format!("{requester:?}")}),
        ),
        _ => None,
    }
}

fn gate_opened_bound(event: &JournalEvent) -> Option<Value> {
    match event {
        JournalEvent::GateOpenedBoundEvent {
            gate,
            opened_at_ms,
            requester,
            ..
        } => Some(
            json!({"type":"GateOpenedBoundEvent","gate":format!("{gate:?}"),"opened_at_ms":opened_at_ms,"requester":format!("{requester:?}")}),
        ),
        _ => None,
    }
}

fn run_record(event: &JournalEvent) -> Option<Value> {
    match event {
        JournalEvent::RunAccepted { workflow, .. } => {
            Some(json!({"type":"RunAccepted", "workflow":format!("{workflow:?}")}))
        }
        JournalEvent::TenantAdmissionEvent {
            tenant, cost_units, ..
        } => Some(
            json!({"type":"TenantAdmissionEvent","tenant":tenant.get(),"cost_units":cost_units}),
        ),
        JournalEvent::RunBudgetOriginEvent { started_millis, .. } => {
            Some(json!({"type":"RunBudgetOriginEvent","started_millis":started_millis}))
        }
        JournalEvent::RunContinuation { carrier } => {
            Some(json!({"type":"RunContinuation","carrier":format!("{carrier:?}")}))
        }
        _ => None,
    }
}
