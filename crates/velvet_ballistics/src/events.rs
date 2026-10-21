//! Module: events

use crate::app_impl::prelude::*;
use crate::inspect::write_vb_kyyf_trace;
use crate::replay::{event_to_json, write_locked_read_surface};

pub(crate) fn cmd_events(
    run_id: &str,
    db: &std::path::Path,
    output: OutputFormat,
    status: Option<EventStatus>,
    limit: Option<i64>,
) -> ExitCode {
    let _status_filter = status.map(|value| value.as_str());
    let _limit_filter = limit;
    let rid = match parse_run_id(run_id, output) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(vb_storage::JournalError::ProcessLockHeld { .. }) => {
            return write_locked_read_surface("events", run_id, output);
        }
        Err(error) => {
            report_storage_open_error(&error, db, output);
            return CliExitCode::StorageError.into();
        }
    };

    match journal.events_for_run(rid) {
        Ok(events) => {
            if events.is_empty() {
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "run_id": run_id,
                            "status": "not_found",
                            "events": [],
                            "total": 0,
                            "error": format!("run {run_id}: no events found")
                        }),
                        output,
                    );
                } else {
                    errln!("run {run_id}: no events found");
                }
                return CliExitCode::ValidationFailed.into();
            } else {
                match output {
                    OutputFormat::Yaml | OutputFormat::Postcard => {
                        let event_list: Vec<serde_json::Value> =
                            events.iter().map(event_to_json).collect();
                        emit_json_or_return!(
                            &serde_json::json!({
                                "schema_version": cli_envelope::SCHEMA_VERSION,
                                "kind": "events_report",
                                "run_id": run_id,
                                "events": event_list,
                                "total": events.len()
                            }),
                            output,
                        );
                    }
                    OutputFormat::Text => {
                        for event in &events {
                            print_event(event);
                        }
                        outln!("{} event(s) total", events.len());
                        write_vb_kyyf_trace("events", run_id, events.len());
                    }
                }
            }
        }
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error reading events for run {run_id}: {e}")
                    }),
                    output,
                );
            } else {
                errln!("error reading events for run {run_id}: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    }

    ExitCode::SUCCESS
}

pub(crate) fn print_event(event: &vb_storage::JournalEvent) {
    match event {
        vb_storage::JournalEvent::RunAccepted { seq, .. } => {
            outln!("  seq={}: RunAccepted", seq.get());
        }
        vb_storage::JournalEvent::RunAdmission { seq, policy, .. } => {
            outln!("  seq={}: RunAdmission policy={policy:?}", seq.get());
        }
        vb_storage::JournalEvent::StepStarted { seq, step, .. } => {
            outln!("  seq={}: StepStarted step={}", seq.get(), step.get());
        }
        vb_storage::JournalEvent::StepSucceeded {
            seq, step, output, ..
        } => {
            outln!(
                "  seq={}: StepSucceeded step={} output={}",
                seq.get(),
                step.get(),
                output.get()
            );
        }
        vb_storage::JournalEvent::ActionScheduled {
            seq, step, action, ..
        } => {
            outln!(
                "  seq={}: ActionScheduled step={} action={}",
                seq.get(),
                step.get(),
                action.get()
            );
        }
        vb_storage::JournalEvent::ActionCompletedEvent {
            seq, step, action, ..
        } => {
            outln!(
                "  seq={}: ActionCompleted step={} action={}",
                seq.get(),
                step.get(),
                action.get()
            );
        }
        vb_storage::JournalEvent::ActionFailedEvent {
            seq, step, action, ..
        } => {
            outln!(
                "  seq={}: ActionFailed step={} action={}",
                seq.get(),
                step.get(),
                action.get()
            );
        }
        vb_storage::JournalEvent::SlotWrittenEvent { seq, slot, .. } => {
            outln!("  seq={}: SlotWritten slot={}", seq.get(), slot.get());
        }
        vb_storage::JournalEvent::WaitScheduledEvent { seq, step, .. } => {
            outln!("  seq={}: WaitScheduled step={}", seq.get(), step.get());
        }
        vb_storage::JournalEvent::AskScheduledEvent { seq, step, .. } => {
            outln!("  seq={}: AskScheduled step={}", seq.get(), step.get());
        }
        vb_storage::JournalEvent::AskAnsweredEvent { seq, step, .. } => {
            outln!("  seq={}: AskAnswered step={}", seq.get(), step.get());
        }
        vb_storage::JournalEvent::RetryScheduledEvent { seq, step, .. } => {
            outln!("  seq={}: RetryScheduled step={}", seq.get(), step.get());
        }
        vb_storage::JournalEvent::RunCancelled { seq, .. } => {
            outln!("  seq={}: RunCancelled", seq.get());
        }
        vb_storage::JournalEvent::RunFinished { seq, result, .. } => {
            outln!("  seq={}: RunFinished result={}", seq.get(), result.get());
        }
        vb_storage::JournalEvent::RunFailedEvent { seq, .. } => {
            outln!("  seq={}: RunFailed", seq.get());
        }
        vb_storage::JournalEvent::RunResumed { run, .. } => {
            outln!("  RunResumed run={}", run.get());
        }
        vb_storage::JournalEvent::RunRetried { run, .. } => {
            outln!("  RunRetried run={}", run.get());
        }
        vb_storage::JournalEvent::RunAnswered { run, slot_idx, .. } => {
            outln!("  RunAnswered run={} slot={}", run.get(), slot_idx.get());
        }
        _ => {
            outln!("  Unknown");
        }
    }
}
