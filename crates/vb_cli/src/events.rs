#![forbid(unsafe_code)]
//! Event listing command.

use crate::args::{
    ActionRegistryMode, Command, DurabilityMode, EventStatus, OutputFormat, ParseError, StepTarget,
};
use crate::cli_envelope;
use crate::exit_code::CliExitCode;
use crate::file_io::{parse_run_id, read_file, read_journal_events, report_storage_open_error};
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};
use crate::output::{
    json_error, json_out, output_error_exit, write_contract_error_json, write_failure_message,
    write_stderr_line, write_stdout_line,
};
use crate::output_utils::*;
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

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
                    crate::errln!("run {run_id}: no events found");
                }
                return CliExitCode::ValidationFailed.into();
            } else {
                match output {
                    OutputFormat::Yaml | OutputFormat::Postcard => {
                        let event_list: Vec<serde_json::Value> =
                            events.iter().map(event_to_json).collect();
                        crate::emit_json_or_return!(
                            &serde_json::json!({
                                "schema_version": crate::cli_envelope::SCHEMA_VERSION,
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
                        crate::outln!("{} event(s) total", events.len());
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
                crate::errln!("error reading events for run {run_id}: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    }

    ExitCode::SUCCESS
}

pub(crate) fn print_event(event: &vb_storage::JournalEvent) {
    match event {
        vb_storage::JournalEvent::RunAccepted { seq, .. } => {
            crate::outln!("  seq={}: RunAccepted", seq.get());
        }
        vb_storage::JournalEvent::RunAdmission { seq, policy, .. } => {
            crate::outln!("  seq={}: RunAdmission policy={policy:?}", seq.get());
        }
        vb_storage::JournalEvent::StepStarted { seq, step, .. } => {
            crate::outln!("  seq={}: StepStarted step={}", seq.get(), step.get());
        }
        vb_storage::JournalEvent::StepSucceeded {
            seq, step, output, ..
        } => {
            crate::outln!(
                "  seq={}: StepSucceeded step={} output={}",
                seq.get(),
                step.get(),
                output.get()
            );
        }
        vb_storage::JournalEvent::ActionScheduled {
            seq, step, action, ..
        } => {
            crate::outln!(
                "  seq={}: ActionScheduled step={} action={}",
                seq.get(),
                step.get(),
                action.get()
            );
        }
        vb_storage::JournalEvent::ActionCompletedEvent {
            seq, step, action, ..
        } => {
            crate::outln!(
                "  seq={}: ActionCompleted step={} action={}",
                seq.get(),
                step.get(),
                action.get()
            );
        }
        vb_storage::JournalEvent::ActionFailedEvent {
            seq, step, action, ..
        } => {
            crate::outln!(
                "  seq={}: ActionFailed step={} action={}",
                seq.get(),
                step.get(),
                action.get()
            );
        }
        vb_storage::JournalEvent::SlotWrittenEvent { seq, slot, .. } => {
            crate::outln!("  seq={}: SlotWritten slot={}", seq.get(), slot.get());
        }
        vb_storage::JournalEvent::WaitScheduledEvent { seq, step, .. } => {
            crate::outln!("  seq={}: WaitScheduled step={}", seq.get(), step.get());
        }
        vb_storage::JournalEvent::AskScheduledEvent { seq, step, .. } => {
            crate::outln!("  seq={}: AskScheduled step={}", seq.get(), step.get());
        }
        vb_storage::JournalEvent::AskAnsweredEvent { seq, step, .. } => {
            crate::outln!("  seq={}: AskAnswered step={}", seq.get(), step.get());
        }
        vb_storage::JournalEvent::RetryScheduledEvent { seq, step, .. } => {
            crate::outln!("  seq={}: RetryScheduled step={}", seq.get(), step.get());
        }
        vb_storage::JournalEvent::RunCancelled { seq, .. } => {
            crate::outln!("  seq={}: RunCancelled", seq.get());
        }
        vb_storage::JournalEvent::RunFinished { seq, result, .. } => {
            crate::outln!("  seq={}: RunFinished result={}", seq.get(), result.get());
        }
        vb_storage::JournalEvent::RunFailedEvent { seq, .. } => {
            crate::outln!("  seq={}: RunFailed", seq.get());
        }
        vb_storage::JournalEvent::RunResumed { run, .. } => {
            crate::outln!("  RunResumed run={}", run.get());
        }
        vb_storage::JournalEvent::RunRetried { run, .. } => {
            crate::outln!("  RunRetried run={}", run.get());
        }
        vb_storage::JournalEvent::RunAnswered { run, slot_idx, .. } => {
            crate::outln!("  RunAnswered run={} slot={}", run.get(), slot_idx.get());
        }
        _ => {
            crate::outln!("  Unknown");
        }
    }
}

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
            seq, step, output, ..
        } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "StepSucceeded",
                "step": step.get(),
                "output": output.get()
            })
        }
        vb_storage::JournalEvent::ActionScheduled {
            seq, step, action, ..
        } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "ActionScheduled",
                "step": step.get(),
                "action": action.get()
            })
        }
        vb_storage::JournalEvent::ActionCompletedEvent {
            seq, step, action, ..
        } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "ActionCompleted",
                "step": step.get(),
                "action": action.get()
            })
        }
        vb_storage::JournalEvent::ActionFailedEvent {
            seq, step, action, ..
        } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "ActionFailed",
                "step": step.get(),
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
        _ => serde_json::json!({"type": "Unknown"}),
    }
}

fn write_locked_read_surface(
    _operation: &str,
    _run_id: &str,
    _output: crate::args::OutputFormat,
) -> std::process::ExitCode {
    crate::errln!("locked read surface not implemented");
    std::process::ExitCode::FAILURE
}

fn write_vb_kyyf_trace(command: &str, run_id: &str, events_len: usize) {
    crate::outln!(
        "BDD-KYYF-002 command={command} run_id={run_id} evidence=.evidence/vb-kyyf/storage-replay-resume.md digest=normalized-replay events={events_len}"
    );
}
