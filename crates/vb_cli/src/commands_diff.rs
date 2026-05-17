#![forbid(unsafe_code)]
#![allow(unreachable_pub)]
//! Pure diff computation logic, separated from I/O and formatting.

use std::collections::HashMap;

use vb_core::SlotValue;
use vb_storage::events::JournalEvent;

/// Result of comparing two event streams.
pub struct DiffResult {
    /// Number of events in stream A.
    pub events_a: usize,
    /// Number of events in stream B.
    pub events_b: usize,
    /// Ordered list of difference entries (as JSON values for downstream formatting).
    pub diffs: Vec<serde_json::Value>,
}

/// Compare two event streams and produce a structured diff.
pub fn compute_diff(events_a: &[JournalEvent], events_b: &[JournalEvent]) -> DiffResult {
    let len_a = events_a.len();
    let len_b = events_b.len();
    let max_len = len_a.max(len_b);
    let mut diffs: Vec<serde_json::Value> = Vec::new();

    for idx in 0..max_len {
        let ev_a = events_a.get(idx);
        let ev_b = events_b.get(idx);
        match (ev_a, ev_b) {
            (Some(a), None) => {
                diffs.push(serde_json::json!({
                    "index": idx,
                    "kind": "only_in_a",
                    "event_a": diff_event_summary(a)
                }));
            }
            (None, Some(b)) => {
                diffs.push(serde_json::json!({
                    "index": idx,
                    "kind": "only_in_b",
                    "event_b": diff_event_summary(b)
                }));
            }
            (Some(a), Some(b)) => {
                if events_differ(a, b) {
                    diffs.push(serde_json::json!({
                        "index": idx,
                        "kind": "changed",
                        "event_a": diff_event_summary(a),
                        "event_b": diff_event_summary(b)
                    }));
                }
            }
            (None, None) => {}
        }
    }

    let steps_a = collect_step_outcomes(events_a);
    let steps_b = collect_step_outcomes(events_b);
    for (step, outcome) in &steps_a {
        match steps_b.get(step) {
            None => {
                diffs.push(serde_json::json!({
                    "kind": "step_missing_in_b",
                    "step": step,
                    "outcome_a": outcome
                }));
            }
            Some(bo) => {
                if outcome != bo {
                    diffs.push(serde_json::json!({
                        "kind": "step_outcome_differs",
                        "step": step,
                        "outcome_a": outcome,
                        "outcome_b": bo
                    }));
                }
            }
        }
    }
    for (step, outcome) in &steps_b {
        if !steps_a.contains_key(step) {
            diffs.push(serde_json::json!({
                "kind": "step_missing_in_a",
                "step": step,
                "outcome_b": outcome
            }));
        }
    }

    let slots_a = collect_slot_values(events_a);
    let slots_b = collect_slot_values(events_b);
    for (slot, va) in &slots_a {
        match slots_b.get(slot) {
            None => {
                diffs.push(serde_json::json!({
                    "kind": "slot_missing_in_b",
                    "slot": slot,
                    "value_a": va
                }));
            }
            Some(vb) => {
                if va != vb {
                    diffs.push(serde_json::json!({
                        "kind": "slot_value_differs",
                        "slot": slot,
                        "value_a": va,
                        "value_b": vb
                    }));
                }
            }
        }
    }
    for (slot, vb) in &slots_b {
        if !slots_a.contains_key(slot) {
            diffs.push(serde_json::json!({
                "kind": "slot_missing_in_a",
                "slot": slot,
                "value_b": vb
            }));
        }
    }

    DiffResult {
        events_a: len_a,
        events_b: len_b,
        diffs,
    }
}

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
    }
}

/// Check whether two events differ in a semantically meaningful way.
pub fn events_differ(a: &JournalEvent, b: &JournalEvent) -> bool {
    match (a, b) {
        (
            JournalEvent::StepSucceeded {
                step: sa,
                output: oa,
                ..
            },
            JournalEvent::StepSucceeded {
                step: sb,
                output: ob,
                ..
            },
        ) => sa != sb || oa != ob,
        (
            JournalEvent::StepStarted { step: sa, .. },
            JournalEvent::StepStarted { step: sb, .. },
        ) => sa != sb,
        (
            JournalEvent::ActionScheduled {
                step: sa,
                action: aa,
                ..
            },
            JournalEvent::ActionScheduled {
                step: sb,
                action: ab,
                ..
            },
        ) => sa != sb || aa != ab,
        (
            JournalEvent::ActionCompletedEvent {
                step: sa,
                action: aa,
                ..
            },
            JournalEvent::ActionCompletedEvent {
                step: sb,
                action: ab,
                ..
            },
        ) => sa != sb || aa != ab,
        (
            JournalEvent::ActionFailedEvent {
                step: sa,
                action: aa,
                ..
            },
            JournalEvent::ActionFailedEvent {
                step: sb,
                action: ab,
                ..
            },
        ) => sa != sb || aa != ab,
        (
            JournalEvent::SlotWrittenEvent {
                slot: sa,
                value: va,
                ..
            },
            JournalEvent::SlotWrittenEvent {
                slot: sb,
                value: vb,
                ..
            },
        ) => sa != sb || va != vb,
        (
            JournalEvent::RunFinished { result: ra, .. },
            JournalEvent::RunFinished { result: rb, .. },
        ) => ra != rb,
        _ => event_name(a) != event_name(b),
    }
}

/// Collect the final outcome per step from an event stream.
pub fn collect_step_outcomes(events: &[JournalEvent]) -> HashMap<u16, String> {
    let mut outcomes = HashMap::new();
    for event in events {
        match event {
            JournalEvent::StepSucceeded { step, output, .. } => {
                outcomes.insert(step.get(), format!("succeeded(output={})", output.get()));
            }
            JournalEvent::ActionFailedEvent { step, action, .. } => {
                outcomes.insert(step.get(), format!("failed(action={})", action.get()));
            }
            JournalEvent::ActionCompletedEvent { step, action, .. } => {
                outcomes.insert(
                    step.get(),
                    format!("action_completed(action={})", action.get()),
                );
            }
            _ => {}
        }
    }
    outcomes
}

/// Collect the final display value per slot from an event stream.
pub fn collect_slot_values(events: &[JournalEvent]) -> HashMap<u16, String> {
    let mut slots = HashMap::new();
    for event in events {
        if let JournalEvent::SlotWrittenEvent { slot, value, .. } = event {
            let display = match value {
                Some(bytes) => {
                    let decoded: Option<SlotValue> = postcard::from_bytes(bytes).ok();
                    match decoded {
                        Some(v) => format!("{v}"),
                        None => format!("[{} bytes]", bytes.len()),
                    }
                }
                None => String::from("none"),
            };
            slots.insert(slot.get(), display);
        }
    }
    slots
}
