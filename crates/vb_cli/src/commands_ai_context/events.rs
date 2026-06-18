//! Journal event serialization and slot redaction for AI context output.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

use super::node_rendering::push_unique_u32;

pub(super) fn ai_journal_events(
    events: &[vb_storage::JournalEvent],
    snapshot: Option<&vb_storage::RunSnapshot>,
) -> Vec<Value> {
    events
        .iter()
        .map(|event| ai_event_to_json(event, snapshot))
        .collect()
}

fn ai_event_to_json(
    event: &vb_storage::JournalEvent,
    snapshot: Option<&vb_storage::RunSnapshot>,
) -> Value {
    let value = event_to_json(event);
    match (event, value) {
        (
            vb_storage::JournalEvent::SlotWrittenEvent {
                slot, value: bytes, ..
            },
            Value::Object(object),
        ) => Value::Object(Map::from_iter(object.into_iter().chain([
            ("slot".to_string(), Value::from(slot.get())),
            (
                "value".to_string(),
                redacted_slot_value(*slot, bytes.as_ref(), snapshot),
            ),
        ]))),
        (_, value) => value,
    }
}

pub(crate) fn redacted_slot_value(
    slot: vb_core::SlotIdx,
    value: Option<&Vec<u8>>,
    snapshot: Option<&vb_storage::RunSnapshot>,
) -> Value {
    if slot_is_secret_or_derived(slot, snapshot) {
        return Value::String("[REDACTED]".to_string());
    }
    value.map_or(Value::Null, |bytes| {
        postcard::from_bytes::<vb_core::SlotValue>(bytes)
            .map_or(Value::String("[UNDECODED]".to_string()), |slot_value| {
                Value::String(slot_value.to_string())
            })
    })
}

fn slot_is_secret_or_derived(
    slot: vb_core::SlotIdx,
    snapshot: Option<&vb_storage::RunSnapshot>,
) -> bool {
    snapshot
        .and_then(|snapshot| snapshot.taint.get(slot.as_usize()))
        .is_some_and(|raw| matches!(*raw, 1 | 2))
}

fn event_to_json(event: &vb_storage::JournalEvent) -> Value {
    match event {
        vb_storage::JournalEvent::RunAccepted { seq, run, workflow } => {
            serde_json::json!({"seq": seq.get(), "type": "RunAccepted", "run": run.get(), "workflow": format!("{:?}", workflow)})
        }
        vb_storage::JournalEvent::RunAdmission {
            seq,
            artifact_digest,
            granted_capabilities,
            policy,
            ..
        } => serde_json::json!({
            "seq": seq.get(),
            "type": "RunAdmission",
            "artifact_digest": format!("{artifact_digest:?}"),
            "granted_capabilities": format!("{granted_capabilities:?}"),
            "policy": format!("{policy:?}")
        }),
        vb_storage::JournalEvent::StepStarted { seq, step, .. } => {
            serde_json::json!({"seq": seq.get(), "type": "StepStarted", "step": step.get()})
        }
        vb_storage::JournalEvent::StepSucceeded {
            seq, step, output, ..
        } => {
            serde_json::json!({"seq": seq.get(), "type": "StepSucceeded", "step": step.get(), "output": output.get()})
        }
        vb_storage::JournalEvent::ActionScheduled {
            seq, step, action, ..
        } => {
            serde_json::json!({"seq": seq.get(), "type": "ActionScheduled", "step": step.get(), "action": action.get()})
        }
        vb_storage::JournalEvent::ActionCompletedEvent {
            seq, step, action, ..
        } => {
            serde_json::json!({"seq": seq.get(), "type": "ActionCompleted", "step": step.get(), "action": action.get()})
        }
        vb_storage::JournalEvent::ActionFailedEvent {
            seq, step, action, ..
        } => {
            serde_json::json!({"seq": seq.get(), "type": "ActionFailed", "step": step.get(), "action": action.get()})
        }
        vb_storage::JournalEvent::SlotWrittenEvent { seq, slot, .. } => {
            serde_json::json!({"seq": seq.get(), "type": "SlotWritten", "slot": slot.get()})
        }
        vb_storage::JournalEvent::WaitScheduledEvent { seq, step, .. } => {
            serde_json::json!({"seq": seq.get(), "type": "WaitScheduled", "step": step.get()})
        }
        vb_storage::JournalEvent::AskScheduledEvent { seq, step, .. } => {
            serde_json::json!({"seq": seq.get(), "type": "AskScheduled", "step": step.get()})
        }
        vb_storage::JournalEvent::AskAnsweredEvent { seq, step, .. } => {
            serde_json::json!({"seq": seq.get(), "type": "AskAnswered", "step": step.get()})
        }
        vb_storage::JournalEvent::RetryScheduledEvent { seq, step, .. } => {
            serde_json::json!({"seq": seq.get(), "type": "RetryScheduled", "step": step.get()})
        }
        vb_storage::JournalEvent::RunCancelled { seq, .. } => {
            serde_json::json!({"seq": seq.get(), "type": "RunCancelled"})
        }
        vb_storage::JournalEvent::RunFinished { seq, result, .. } => {
            serde_json::json!({"seq": seq.get(), "type": "RunFinished", "result": result.get()})
        }
        vb_storage::JournalEvent::RunFailedEvent { seq, .. } => {
            serde_json::json!({"seq": seq.get(), "type": "RunFailed"})
        }
        vb_storage::JournalEvent::RunResumed {
            run,
            seq: _,
            timestamp,
        } => {
            serde_json::json!({"type": "RunResumed", "run": run.get(), "timestamp": timestamp.to_rfc3339()})
        }
        vb_storage::JournalEvent::RunRetried {
            run,
            seq: _,
            timestamp,
        } => {
            serde_json::json!({"type": "RunRetried", "run": run.get(), "timestamp": timestamp.to_rfc3339()})
        }
        vb_storage::JournalEvent::RunAnswered {
            run,
            seq: _,
            slot_idx,
            answer,
            timestamp,
        } => {
            serde_json::json!({"type": "RunAnswered", "run": run.get(), "slot_idx": slot_idx.get(), "answer": format!("{:?}", answer), "timestamp": timestamp.to_rfc3339()})
        }
        _ => serde_json::json!({"type": "Unknown"}),
    }
}
