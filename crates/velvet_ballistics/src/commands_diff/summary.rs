use vb_storage::events::JournalEvent;

/// Produce a short JSON summary of a single event for diff display.
pub fn diff_event_summary(event: &JournalEvent) -> serde_json::Value {
    match event {
        JournalEvent::RunAccepted { seq, .. } => {
            serde_json::json!({"type": "RunAccepted", "seq": seq.get()})
        }
        JournalEvent::RunAdmission { seq, policy, .. } => {
            serde_json::json!({"type": "RunAdmission", "seq": seq.get(), "policy": format!("{policy:?}")})
        }
        JournalEvent::StepStarted { seq, step, .. } => {
            serde_json::json!({"type": "StepStarted", "seq": seq.get(), "step": step.get()})
        }
        JournalEvent::StepSucceeded {
            seq, step, output, ..
        } => serde_json::json!({
            "type": "StepSucceeded",
            "seq": seq.get(),
            "step": step.get(),
            "output": output.get()
        }),
        JournalEvent::ActionScheduled {
            seq, step, action, ..
        } => serde_json::json!({
            "type": "ActionScheduled",
            "seq": seq.get(),
            "step": step.get(),
            "action": action.get()
        }),
        JournalEvent::ActionCompletedEvent {
            seq, step, action, ..
        } => serde_json::json!({
            "type": "ActionCompleted",
            "seq": seq.get(),
            "step": step.get(),
            "action": action.get()
        }),
        JournalEvent::ActionFailedEvent {
            seq, step, action, ..
        } => serde_json::json!({
            "type": "ActionFailed",
            "seq": seq.get(),
            "step": step.get(),
            "action": action.get()
        }),
        JournalEvent::SlotWrittenEvent {
            seq, slot, value, ..
        } => serde_json::json!({
            "type": "SlotWritten",
            "seq": seq.get(),
            "slot": slot.get(),
            "has_value": value.is_some()
        }),
        JournalEvent::WaitScheduledEvent { seq, step, .. } => {
            serde_json::json!({"type": "WaitScheduled", "seq": seq.get(), "step": step.get()})
        }
        JournalEvent::AskScheduledEvent { seq, step, .. } => {
            serde_json::json!({"type": "AskScheduled", "seq": seq.get(), "step": step.get()})
        }
        JournalEvent::AskAnsweredEvent { seq, step, .. } => {
            serde_json::json!({"type": "AskAnswered", "seq": seq.get(), "step": step.get()})
        }
        JournalEvent::RetryScheduledEvent { seq, step, .. } => {
            serde_json::json!({"type": "RetryScheduled", "seq": seq.get(), "step": step.get()})
        }
        JournalEvent::RunCancelled { seq, .. } => {
            serde_json::json!({"type": "RunCancelled", "seq": seq.get()})
        }
        JournalEvent::RunFinished { seq, result, .. } => {
            serde_json::json!({"type": "RunFinished", "seq": seq.get(), "result": result.get()})
        }
        JournalEvent::RunFailedEvent { seq, .. } => {
            serde_json::json!({"type": "RunFailed", "seq": seq.get()})
        }
        JournalEvent::RunResumed { run, .. } => {
            serde_json::json!({"type": "RunResumed", "run": run.get()})
        }
        JournalEvent::RunRetried { run, .. } => {
            serde_json::json!({"type": "RunRetried", "run": run.get()})
        }
        JournalEvent::RunAnswered { run, slot_idx, .. } => {
            serde_json::json!({"type": "RunAnswered", "run": run.get(), "slot_idx": slot_idx.get()})
        }
        _ => serde_json::json!({"type": "Unknown"}),
    }
}

/// Return the static name string for an event variant.
pub fn event_name(event: &JournalEvent) -> &'static str {
    match event {
        JournalEvent::RunAccepted { .. } => "RunAccepted",
        JournalEvent::RunAdmission { .. } => "RunAdmission",
        JournalEvent::StepStarted { .. } => "StepStarted",
        JournalEvent::StepSucceeded { .. } => "StepSucceeded",
        JournalEvent::ActionScheduled { .. } => "ActionScheduled",
        JournalEvent::ActionCompletedEvent { .. } => "ActionCompleted",
        JournalEvent::ActionFailedEvent { .. } => "ActionFailed",
        JournalEvent::SlotWrittenEvent { .. } => "SlotWritten",
        JournalEvent::WaitScheduledEvent { .. } => "WaitScheduled",
        JournalEvent::AskScheduledEvent { .. } => "AskScheduled",
        JournalEvent::AskAnsweredEvent { .. } => "AskAnswered",
        JournalEvent::RetryScheduledEvent { .. } => "RetryScheduled",
        JournalEvent::RunCancelled { .. } => "RunCancelled",
        JournalEvent::RunFinished { .. } => "RunFinished",
        JournalEvent::RunFailedEvent { .. } => "RunFailed",
        JournalEvent::RunResumed { .. } => "RunResumed",
        JournalEvent::RunRetried { .. } => "RunRetried",
        JournalEvent::RunAnswered { .. } => "RunAnswered",
        _ => "Unknown",
    }
}
