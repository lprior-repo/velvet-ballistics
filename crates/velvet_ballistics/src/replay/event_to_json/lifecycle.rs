#![forbid(unsafe_code)]

use serde_json::{Value, json};
use vb_storage::JournalEvent;

pub(super) fn fields(event: &JournalEvent) -> Option<Value> {
    terminal(event).or_else(|| resumption(event))
}

fn terminal(event: &JournalEvent) -> Option<Value> {
    match event {
        JournalEvent::RunCancelled { reason, .. } => {
            Some(json!({"type":"RunCancelled","reason":reason}))
        }
        JournalEvent::RunKilled { .. } => Some(json!({"type":"RunKilled"})),
        JournalEvent::RunFailedEvent { .. } => Some(json!({"type":"RunFailed"})),
        JournalEvent::RunFinished { result, .. } => {
            Some(json!({"type":"RunFinished","result":result.get()}))
        }
        _ => None,
    }
}

fn resumption(event: &JournalEvent) -> Option<Value> {
    match event {
        JournalEvent::RunResumed { timestamp, .. } => {
            Some(json!({"type":"RunResumed","timestamp":timestamp.to_rfc3339()}))
        }
        JournalEvent::RunRetried { timestamp, .. } => {
            Some(json!({"type":"RunRetried","timestamp":timestamp.to_rfc3339()}))
        }
        JournalEvent::RunAnswered {
            slot_idx,
            answer,
            timestamp,
            ..
        } => Some(
            json!({"type":"RunAnswered","slot_idx":slot_idx.get(),"answer":format!("{answer:?}"),"timestamp":timestamp.to_rfc3339()}),
        ),
        _ => None,
    }
}
