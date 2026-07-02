//! Convert a journal event to a JSON value for structured output.
#![forbid(unsafe_code)]

/// Convert a journal event to a JSON value for structured output.
pub(crate) fn event_to_json(event: &vb_storage::JournalEvent) -> serde_json::Value {
    match event {
        vb_storage::JournalEvent::RunAccepted { seq, run, workflow } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "RunAccepted",
                "run": run.get(),
                "workflow": format!("{:?}", workflow)
            })
        }
        vb_storage::JournalEvent::RunAdmission {
            seq,
            run,
            artifact_digest,
            granted_capabilities,
            policy,
        } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "RunAdmission",
                "run": run.get(),
                "artifact_digest": format!("{artifact_digest:?}"),
                "granted_capabilities": format!("{granted_capabilities:?}"),
                "policy": format!("{policy:?}")
            })
        }
        vb_storage::JournalEvent::StepStarted { seq, step, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "StepStarted",
                "step": step.get()
            })
        }
        vb_storage::JournalEvent::StepSucceeded {
            seq,
            step: _,
            output,
            ..
        } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "StepSucceeded",
                "output": output.get()
            })
        }
        vb_storage::JournalEvent::ActionScheduled {
            seq,
            step: _,
            action,
            ..
        } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "ActionScheduled",
                "action": action.get()
            })
        }
        vb_storage::JournalEvent::ActionCompletedEvent {
            seq,
            step: _,
            action,
            ..
        } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "ActionCompleted",
                "action": action.get()
            })
        }
        vb_storage::JournalEvent::ActionFailedEvent {
            seq,
            step: _,
            action,
            ..
        } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "ActionFailed",
                "action": action.get()
            })
        }
        vb_storage::JournalEvent::SlotWrittenEvent { seq, slot, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "SlotWritten",
                "slot": slot.get()
            })
        }
        vb_storage::JournalEvent::WaitScheduledEvent { seq, step, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "WaitScheduled",
                "step": step.get()
            })
        }
        vb_storage::JournalEvent::AskScheduledEvent { seq, step, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "AskScheduled",
                "step": step.get()
            })
        }
        vb_storage::JournalEvent::AskAnsweredEvent { seq, step, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "AskAnswered",
                "step": step.get()
            })
        }
        vb_storage::JournalEvent::RetryScheduledEvent { seq, step, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "RetryScheduled",
                "step": step.get()
            })
        }
        vb_storage::JournalEvent::RunCancelled { seq, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "RunCancelled"
            })
        }
        vb_storage::JournalEvent::RunFinished { seq, result, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "RunFinished",
                "result": result.get()
            })
        }
        vb_storage::JournalEvent::RunFailedEvent { seq, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "RunFailed"
            })
        }
        vb_storage::JournalEvent::RunResumed {
            run,
            seq: _,
            timestamp,
        } => {
            serde_json::json!({
                "type": "RunResumed",
                "run": run.get(),
                "timestamp": timestamp.to_rfc3339()
            })
        }
        vb_storage::JournalEvent::RunRetried {
            run,
            seq: _,
            timestamp,
        } => {
            serde_json::json!({
                "type": "RunRetried",
                "run": run.get(),
                "timestamp": timestamp.to_rfc3339()
            })
        }
        vb_storage::JournalEvent::RunAnswered {
            run,
            seq: _,
            slot_idx,
            answer,
            timestamp,
        } => {
            serde_json::json!({
                "type": "RunAnswered",
                "run": run.get(),
                "slot_idx": slot_idx.get(),
                "answer": format!("{:?}", answer),
                "timestamp": timestamp.to_rfc3339()
            })
        }
        vb_storage::JournalEvent::ActionScheduledTicket {
            seq,
            run,
            ticket,
            input,
            output,
            action_abi_digest,
        } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "ActionScheduledTicket",
                "run": run.get(),
                "ticket": format!("{ticket:?}"),
                "input": input.get(),
                "output": output.get(),
                "action_abi_digest": format!("{action_abi_digest:?}")
            })
        }
        vb_storage::JournalEvent::ActionCompletedEnvelope {
            seq,
            run,
            ticket,
            output,
            outcome,
            value: _,
            encoded_len,
            taint,
            value_digest,
            action_abi_digest,
            ..
        } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "ActionCompletedEnvelope",
                "run": run.get(),
                "ticket": format!("{ticket:?}"),
                "output": output.get(),
                "outcome": format!("{outcome:?}"),
                "encoded_len": encoded_len,
                "taint": format!("{taint:?}"),
                "value_digest": format!("{value_digest:?}"),
                "action_abi_digest": format!("{action_abi_digest:?}")
            })
        }
        vb_storage::JournalEvent::ActionAbandoned { seq, run, ticket } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "ActionAbandoned",
                "run": run.get(),
                "ticket": format!("{ticket:?}")
            })
        }
        vb_storage::JournalEvent::WaitResolvedEvent { seq, step, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "WaitResolved",
                "step": step.get()
            })
        }
        vb_storage::JournalEvent::RunKilled { seq, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "RunKilled"
            })
        }
        vb_storage::JournalEvent::AskTimedOutEvent { seq, step, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "AskTimedOut",
                "step": step.get()
            })
        }
        _ => serde_json::json!({
            "type": "Unknown",
            "note": "future non-exhaustive variant — add explicit match arm"
        }),
    }
}
