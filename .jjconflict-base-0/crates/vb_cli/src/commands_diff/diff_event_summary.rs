#![forbid(unsafe_code)]
//! Per-event JSON projection for diff display.
//!
//! Owns [`diff_event_summary`], which produces a `serde_json::Value`
//! summary of a single [`JournalEvent`] for the diff renderer. Split out
//! of `diff.rs` so the comparison and projection responsibilities stay
//! under the Holzman source-length limit.

use vb_storage::events::JournalEvent;

use super::event_name::event_name;

/// Produce a short JSON summary of a single event for diff display.
///
/// The outer match is over `JournalEvent` and therefore requires the
/// `#[non_exhaustive]` wildcard; each per-variant arm resolves the
/// `type` field through [`super::event_name::event_name`] (which in turn
/// uses [`super::schema::KnownVariant::name`]) so the schema guard
/// remains tied to the closed enum. New `KnownVariant` arms added to
/// [`KnownVariant::name`] must be matched here in lockstep; the runtime
/// test `every_known_variant_maps_to_a_non_unknown_name` enforces the
/// invariant from the other direction.
#[allow(clippy::too_many_lines)]
pub fn diff_event_summary(event: &JournalEvent) -> serde_json::Value {
    let type_name = event_name(event);
    match event {
        JournalEvent::RunAccepted { seq, .. } => {
            serde_json::json!({"type": type_name, "seq": seq.get()})
        }
        JournalEvent::RunAdmission { seq, policy, .. } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "policy": format!("{policy:?}")
        }),
        JournalEvent::StepStarted { seq, step, .. } => {
            serde_json::json!({
                "type": type_name,
                "seq": seq.get(),
                "step": step.get()
            })
        }
        JournalEvent::StepSucceeded {
            seq, step, output, ..
        } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "step": step.get(),
            "output": output.get()
        }),
        JournalEvent::ActionScheduled {
            seq, step, action, ..
        } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "step": step.get(),
            "action": action.get()
        }),
        JournalEvent::ActionCompletedEvent {
            seq, step, action, ..
        } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "step": step.get(),
            "action": action.get()
        }),
        JournalEvent::ActionScheduledTicket {
            seq,
            run,
            ticket,
            input,
            output,
            action_abi_digest,
        } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "run": run.get(),
            "ticket": format!("{ticket:?}"),
            "input": input.get(),
            "output": output.get(),
            "action_abi_digest": format!("{action_abi_digest:?}")
        }),
        JournalEvent::ActionCompletedEnvelope {
            seq,
            run,
            ticket,
            output,
            outcome,
            encoded_len,
            taint,
            value_digest,
            action_abi_digest,
            ..
        } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "run": run.get(),
            "ticket": format!("{ticket:?}"),
            "output": output.get(),
            "outcome": format!("{outcome:?}"),
            "encoded_len": encoded_len,
            "taint": format!("{taint:?}"),
            "value_digest": format!("{value_digest:?}"),
            "action_abi_digest": format!("{action_abi_digest:?}")
        }),
        JournalEvent::ActionFailedEvent {
            seq, step, action, ..
        } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "step": step.get(),
            "action": action.get()
        }),
        JournalEvent::ActionAbandoned { seq, run, ticket } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "run": run.get(),
            "ticket": format!("{ticket:?}")
        }),
        JournalEvent::SlotWrittenEvent {
            seq, slot, value, ..
        } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "slot": slot.get(),
            "has_value": value.is_some()
        }),
        JournalEvent::WaitScheduledEvent { seq, step, .. } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "step": step.get()
        }),
        JournalEvent::AskScheduledEvent { seq, step, .. } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "step": step.get()
        }),
        JournalEvent::AskAnsweredEvent { seq, step, .. } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "step": step.get()
        }),
        JournalEvent::WaitResolvedEvent { seq, step, .. } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "step": step.get()
        }),
        JournalEvent::RetryScheduledEvent { seq, step, .. } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "step": step.get()
        }),
        JournalEvent::RunCancelled { seq, .. } => {
            serde_json::json!({"type": type_name, "seq": seq.get()})
        }
        JournalEvent::RunKilled { seq, .. } => {
            serde_json::json!({"type": type_name, "seq": seq.get()})
        }
        JournalEvent::RunFinished { seq, result, .. } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "result": result.get()
        }),
        JournalEvent::RunFailedEvent { seq, .. } => {
            serde_json::json!({"type": type_name, "seq": seq.get()})
        }
        JournalEvent::RunResumed { run, .. } => {
            serde_json::json!({"type": type_name, "run": run.get()})
        }
        JournalEvent::RunRetried { run, .. } => {
            serde_json::json!({"type": type_name, "run": run.get()})
        }
        JournalEvent::RunAnswered { run, slot_idx, .. } => serde_json::json!({
            "type": type_name,
            "run": run.get(),
            "slot_idx": slot_idx.get()
        }),
        JournalEvent::AskTimedOutEvent { seq, step, .. } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "step": step.get()
        }),
        _ => serde_json::json!({"type": type_name}),
    }
}
