#![forbid(unsafe_code)]

use serde_json::{Value, json};
use vb_storage::JournalEvent;

pub(super) fn fields(event: &JournalEvent) -> Option<Value> {
    simple(event)
        .or_else(|| ticket(event))
        .or_else(|| completion(event))
}

fn simple(event: &JournalEvent) -> Option<Value> {
    match event {
        JournalEvent::ActionScheduled { step, action, .. } => {
            Some(json!({"type":"ActionScheduled","step":step.get(),"action":action.get()}))
        }
        JournalEvent::ActionCompletedEvent { step, action, .. } => {
            Some(json!({"type":"ActionCompleted","step":step.get(),"action":action.get()}))
        }
        JournalEvent::ActionFailedEvent { step, action, .. } => {
            Some(json!({"type":"ActionFailed","step":step.get(),"action":action.get()}))
        }
        JournalEvent::ActionAbandoned { ticket, .. } => {
            Some(json!({"type":"ActionAbandoned","ticket":format!("{ticket:?}")}))
        }
        _ => None,
    }
}

fn ticket(event: &JournalEvent) -> Option<Value> {
    match event {
        JournalEvent::ActionScheduledTicket {
            ticket,
            input,
            output,
            action_abi_digest,
            ..
        } => Some(
            json!({"type":"ActionScheduledTicket","ticket":format!("{ticket:?}"),"input":input.get(),"output":output.get(),"action_abi_digest":format!("{action_abi_digest:?}")}),
        ),
        _ => None,
    }
}

fn completion(event: &JournalEvent) -> Option<Value> {
    match event {
        JournalEvent::ActionCompletedEnvelope {
            ticket,
            output,
            outcome,
            value,
            encoded_len,
            taint,
            value_digest,
            action_abi_digest,
            ..
        } => Some(
            json!({"type":"ActionCompletedEnvelope","ticket":format!("{ticket:?}"),"output":output.get(),"outcome":format!("{outcome:?}"),"value":value,"encoded_len":encoded_len,"taint":format!("{taint:?}"),"value_digest":format!("{value_digest:?}"),"action_abi_digest":format!("{action_abi_digest:?}")}),
        ),
        _ => None,
    }
}
