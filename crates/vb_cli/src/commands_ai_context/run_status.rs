//! Run status determination and CLI-command suggestions for AI context output.

#![forbid(unsafe_code)]

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunStatus {
    Running,
    Finished,
    Failed,
    Cancelled,
}

pub(super) fn run_status_from_events(events: &[vb_storage::JournalEvent]) -> RunStatus {
    match events.last() {
        Some(vb_storage::JournalEvent::RunFinished { .. }) => RunStatus::Finished,
        Some(vb_storage::JournalEvent::RunFailedEvent { .. }) => RunStatus::Failed,
        Some(vb_storage::JournalEvent::RunCancelled { .. }) => RunStatus::Cancelled,
        _ => RunStatus::Running,
    }
}

pub(super) fn trace_ring_snapshot() -> Value {
    serde_json::json!({
        "available": false,
        "reason": "TraceRing is volatile in-memory runtime state; this packet does not fabricate a durable trace snapshot",
        "fabricated": false,
        "events": []
    })
}

pub(crate) fn suggested_ai_commands(
    run_id: &str,
    db: &std::path::Path,
    status: RunStatus,
) -> Vec<String> {
    let db_arg = db.display();
    let base = vec![
        format!("velvet-ballistics inspect {run_id} --db {db_arg} --emit yaml"),
        format!("velvet-ballistics events {run_id} --db {db_arg} --emit yaml"),
    ];
    match status {
        RunStatus::Failed | RunStatus::Cancelled => base
            .into_iter()
            .chain([
                format!("velvet-ballistics incident {run_id} --db {db_arg} --emit yaml"),
                format!("velvet-ballistics retry {run_id} --db {db_arg} --emit yaml"),
            ])
            .collect(),
        RunStatus::Running => base
            .into_iter()
            .chain([
                format!("velvet-ballistics trace {run_id} --db {db_arg} --emit yaml"),
                format!("velvet-ballistics resume {run_id} --db {db_arg} --emit yaml"),
            ])
            .collect(),
        RunStatus::Finished => base
            .into_iter()
            .chain([format!(
                "velvet-ballistics replay {run_id} --db {db_arg} --emit yaml"
            )])
            .collect(),
    }
}
