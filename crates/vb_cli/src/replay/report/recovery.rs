#![forbid(unsafe_code)]

pub(super) fn recovery_summary_report(events: &[vb_storage::JournalEvent]) -> serde_json::Value {
    match vb_storage::recovery::summarize_recovery_events(events) {
        Ok(hydration) => recovery_summary_value(hydration.summary()),
        Err(error) => serde_json::json!({
            "available": false,
            "error": error.to_string()
        }),
    }
}

fn recovery_summary_value(
    summary: vb_storage::recovery::RecoveryRuntimeSummary,
) -> serde_json::Value {
    serde_json::json!({
        "available": true,
        "run": summary.run.get(),
        "first_seq": summary.first_seq.get(),
        "last_seq": summary.last_seq.get(),
        "workflow": summary.workflow.map(|digest| format!("{digest:?}")),
        "steps_started": summary.steps_started,
        "steps_succeeded": summary.steps_succeeded,
        "actions_scheduled": summary.actions_scheduled,
        "actions_resolved": summary.actions_resolved,
        "suspensions": summary.suspensions,
        "slots_written": summary.slots_written,
        "terminal": recovery_terminal_value(summary.terminal)
    })
}

fn recovery_terminal_value(
    terminal: Option<vb_storage::recovery::RecoveryTerminalState>,
) -> serde_json::Value {
    match terminal {
        Some(vb_storage::recovery::RecoveryTerminalState::Cancelled) => {
            serde_json::json!({"status": "cancelled"})
        }
        Some(vb_storage::recovery::RecoveryTerminalState::Killed) => {
            serde_json::json!({"status": "killed"})
        }
        Some(vb_storage::recovery::RecoveryTerminalState::Finished { result }) => {
            serde_json::json!({"status": "finished", "result_slot": result.get()})
        }
        Some(vb_storage::recovery::RecoveryTerminalState::Failed) => {
            serde_json::json!({"status": "failed"})
        }
        Some(_) => serde_json::json!({"status": "unknown"}),
        None => serde_json::Value::Null,
    }
}

pub(super) fn write_replay_text_recovery_summary(events: &[vb_storage::JournalEvent]) {
    match vb_storage::recovery::summarize_recovery_events(events) {
        Ok(hydration) => write_recovery_summary_line(hydration.summary()),
        Err(error) => crate::outln!("recovery_summary: unavailable ({error})"),
    }
}

fn write_recovery_summary_line(summary: vb_storage::recovery::RecoveryRuntimeSummary) {
    crate::outln!(
        "recovery_summary: seq={}..{} steps={}/{} actions={}/{} slots={} suspensions={}",
        summary.first_seq.get(),
        summary.last_seq.get(),
        summary.steps_succeeded,
        summary.steps_started,
        summary.actions_resolved,
        summary.actions_scheduled,
        summary.slots_written,
        summary.suspensions
    );
}
