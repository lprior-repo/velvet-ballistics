#![forbid(unsafe_code)]

mod checkpoint;
mod recovery;
mod summary;

use self::summary::{ReplayEventSummary, optional_sequence_label, optional_static_label};
use crate::args::OutputFormat;
use crate::events::event_to_json;
use crate::output::json_out_exit;
use crate::storage::print_event;
use std::process::ExitCode;

pub(super) fn write_replay_success(
    run_id: &str,
    events: &[vb_storage::JournalEvent],
    output: OutputFormat,
) -> ExitCode {
    match output {
        OutputFormat::Yaml | OutputFormat::Postcard => {
            write_replay_structured_success(run_id, events, output)
        }
        OutputFormat::Text => {
            write_replay_text_success(run_id, events);
            ExitCode::SUCCESS
        }
    }
}

fn write_replay_structured_success(
    run_id: &str,
    events: &[vb_storage::JournalEvent],
    output: OutputFormat,
) -> ExitCode {
    let summary = ReplayEventSummary::from_events(events);
    let event_list: Vec<serde_json::Value> = events.iter().map(event_to_json).collect();
    json_out_exit(
        &serde_json::json!({
            "schema_version": crate::cli_envelope::SCHEMA_VERSION,
            "kind": "replay_report",
            "success": true,
            "run_id": run_id,
            "recovered": summary.event_count,
            "event_count": summary.event_count,
            "first_sequence": summary.first_sequence,
            "last_sequence": summary.last_sequence,
            "events": event_list,
            "terminal": summary.terminal_event,
            "terminal_event": summary.terminal_event,
            "terminal_status": summary.terminal_status,
            "event_counts": event_counts_json(summary),
            "action_counts": action_counts_json(summary),
            "recovery_summary": recovery::recovery_summary_report(events),
            "last_checkpoint": checkpoint::replay_checkpoint_report(events),
            "replay_safety": checkpoint::replay_safety_report(events, true),
            "recovery_error_class": "none"
        }),
        output,
    )
}

fn event_counts_json(summary: ReplayEventSummary) -> serde_json::Value {
    serde_json::json!({
        "step": summary.step_event_count,
        "action": summary.action_event_count,
        "slot": summary.slot_event_count
    })
}

fn action_counts_json(summary: ReplayEventSummary) -> serde_json::Value {
    serde_json::json!({
        "scheduled": summary.action_scheduled_count,
        "resolved": summary.action_resolved_count,
        "pending_unresolved": summary.pending_unresolved_action_count
    })
}

pub(super) fn replay_failure_context_report(
    events: &[vb_storage::JournalEvent],
) -> serde_json::Value {
    serde_json::json!({
        "available": true,
        "event_count": events.len(),
        "recovery_summary": recovery::recovery_summary_report(events),
        "last_checkpoint": checkpoint::replay_checkpoint_report(events),
        "replay_safety": checkpoint::replay_safety_report(events, false)
    })
}

fn write_replay_text_success(run_id: &str, events: &[vb_storage::JournalEvent]) {
    let summary = ReplayEventSummary::from_events(events);
    crate::outln!(
        "recovered {} event(s) for run {run_id}",
        summary.event_count
    );
    write_replay_text_summary(summary);
    recovery::write_replay_text_recovery_summary(events);
    checkpoint::write_replay_text_checkpoint(events);
    for event in events {
        print_event(event);
    }
    crate::outln!(
        "terminal: {}",
        optional_static_label(summary.terminal_event)
    );
    super::write_vb_kyyf_trace("replay", run_id, summary.event_count);
}

fn write_replay_text_summary(summary: ReplayEventSummary) {
    crate::outln!(
        "summary: event_count={} terminal_status={} terminal_event={} first_sequence={} last_sequence={} step_event_count={} action_event_count={} slot_event_count={} actions_scheduled={} actions_resolved={} pending_unresolved_actions={}",
        summary.event_count,
        summary.terminal_status,
        optional_static_label(summary.terminal_event),
        optional_sequence_label(summary.first_sequence),
        optional_sequence_label(summary.last_sequence),
        summary.step_event_count,
        summary.action_event_count,
        summary.slot_event_count,
        summary.action_scheduled_count,
        summary.action_resolved_count,
        summary.pending_unresolved_action_count
    );
}
