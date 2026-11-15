#![forbid(unsafe_code)]

use serde_json::{Value, json};
use vb_storage::JournalEvent;

pub(super) fn fields(event: &JournalEvent) -> Option<Value> {
    steps(event).or_else(|| waits(event))
}

fn steps(event: &JournalEvent) -> Option<Value> {
    match event {
        JournalEvent::StepStarted { step, .. } => {
            Some(json!({"type":"StepStarted","step":step.get()}))
        }
        JournalEvent::StepSucceeded { step, output, .. } => {
            Some(json!({"type":"StepSucceeded","step":step.get(),"output":output.get()}))
        }
        JournalEvent::StepFailed { step, .. } => {
            Some(json!({"type":"StepFailed","step":step.get()}))
        }
        JournalEvent::SlotWrittenEvent {
            slot, value, extra, ..
        } => Some(json!({"type":"SlotWritten","slot":slot.get(),"value":value,"extra":extra})),
        _ => None,
    }
}

fn waits(event: &JournalEvent) -> Option<Value> {
    match event {
        JournalEvent::WaitScheduledEvent { step, .. } => {
            Some(json!({"type":"WaitScheduled","step":step.get()}))
        }
        JournalEvent::AskScheduledEvent { step, .. } => {
            Some(json!({"type":"AskScheduled","step":step.get()}))
        }
        JournalEvent::AskAnsweredEvent { step, .. } => {
            Some(json!({"type":"AskAnswered","step":step.get()}))
        }
        JournalEvent::WaitResolvedEvent { step, .. } => {
            Some(json!({"type":"WaitResolved","step":step.get()}))
        }
        JournalEvent::RetryScheduledEvent { step, .. } => {
            Some(json!({"type":"RetryScheduled","step":step.get()}))
        }
        JournalEvent::AskTimedOutEvent { step, .. } => {
            Some(json!({"type":"AskTimedOut","step":step.get()}))
        }
        _ => None,
    }
}
