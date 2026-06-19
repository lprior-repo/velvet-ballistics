#![forbid(unsafe_code)]

use crate::events::event_to_json;

pub(super) fn replay_checkpoint_report(events: &[vb_storage::JournalEvent]) -> serde_json::Value {
    match last_replay_checkpoint(events) {
        Some(event) => serde_json::json!({
            "available": true,
            "seq": event.seq().get(),
            "kind": crate::commands_diff::event_name(event),
            "event": event_to_json(event)
        }),
        None => serde_json::json!({"available": false}),
    }
}

fn last_replay_checkpoint(
    events: &[vb_storage::JournalEvent],
) -> Option<&vb_storage::JournalEvent> {
    events
        .iter()
        .rev()
        .find(|event| is_replay_checkpoint(event))
}

fn is_replay_checkpoint(event: &vb_storage::JournalEvent) -> bool {
    matches!(
        event,
        vb_storage::JournalEvent::RunAdmission { .. }
            | vb_storage::JournalEvent::StepSucceeded { .. }
            | vb_storage::JournalEvent::SlotWrittenEvent { .. }
            | vb_storage::JournalEvent::ActionScheduledTicket { .. }
            | vb_storage::JournalEvent::ActionCompletedEnvelope { .. }
            | vb_storage::JournalEvent::RunFinished { .. }
            | vb_storage::JournalEvent::RunCancelled { .. }
            | vb_storage::JournalEvent::RunKilled { .. }
            | vb_storage::JournalEvent::RunFailedEvent { .. }
    )
}

pub(super) fn replay_safety_report(
    events: &[vb_storage::JournalEvent],
    replay_completed: bool,
) -> serde_json::Value {
    serde_json::json!({
        "replay_completed": replay_completed,
        "admission_evidence": has_run_admission(events),
        "terminal_observed": vb_storage::recovery::extract_terminal(events).is_some(),
        "last_checkpoint_available": last_replay_checkpoint(events).is_some()
    })
}

fn has_run_admission(events: &[vb_storage::JournalEvent]) -> bool {
    events
        .iter()
        .any(|event| matches!(event, vb_storage::JournalEvent::RunAdmission { .. }))
}

pub(super) fn write_replay_text_checkpoint(events: &[vb_storage::JournalEvent]) {
    match last_replay_checkpoint(events) {
        Some(event) => crate::outln!(
            "last_checkpoint: seq={} kind={}",
            event.seq().get(),
            crate::commands_diff::event_name(event)
        ),
        None => crate::outln!("last_checkpoint: none"),
    }
}
