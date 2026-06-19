#![forbid(unsafe_code)]

use super::report::replay_failure_context_report;
use crate::args::OutputFormat;
use crate::exit_code::{CliExitCode, recovery_error_exit_code};
use crate::output::{json_out_exit, write_stderr_line, write_structured_stderr};
use std::process::ExitCode;

pub(super) struct ReplayFailureOutcome {
    pub(super) code: CliExitCode,
    pub(super) message: String,
    structured: serde_json::Value,
}

pub(super) fn replay_context_events(
    journal: &vb_storage::FjallJournal,
    run: vb_core::RunId,
) -> Option<Vec<vb_storage::JournalEvent>> {
    match journal.events_for_run(run) {
        Ok(events) if events.is_empty() => None,
        Ok(events) => Some(events),
        Err(_error) => None,
    }
}

pub(super) fn write_replay_error(
    run_id: &str,
    error: &vb_storage::recovery::RecoveryError,
    context_events: Option<&[vb_storage::JournalEvent]>,
    output: OutputFormat,
) -> ExitCode {
    let outcome = replay_failure_outcome(run_id, error, context_events);
    render_replay_failure(&outcome, output);
    outcome.code.into()
}

pub(super) fn replay_failure_outcome(
    run_id: &str,
    error: &vb_storage::recovery::RecoveryError,
    context_events: Option<&[vb_storage::JournalEvent]>,
) -> ReplayFailureOutcome {
    match error {
        vb_storage::recovery::RecoveryError::NoRecoveryData { .. } => {
            replay_no_recovery_outcome(run_id)
        }
        other => replay_recovery_error_outcome(run_id, other, context_events),
    }
}

pub(super) fn replay_no_recovery_outcome(run_id: &str) -> ReplayFailureOutcome {
    let message = format!("run {run_id}: no events found");
    let code = CliExitCode::ValidationFailed;
    ReplayFailureOutcome {
        code,
        structured: crate::output_utils::diagnostic_value(&message, code),
        message,
    }
}

fn replay_recovery_error_outcome(
    run_id: &str,
    error: &vb_storage::recovery::RecoveryError,
    context_events: Option<&[vb_storage::JournalEvent]>,
) -> ReplayFailureOutcome {
    let code = recovery_error_exit_code(error);
    let message = format!("error replaying run {run_id}: {error}");
    let context = context_events.map_or_else(
        || serde_json::json!({"available": false}),
        replay_failure_context_report,
    );
    ReplayFailureOutcome {
        code,
        structured: replay_error_json(run_id, error, &message, context, code),
        message,
    }
}

fn replay_error_json(
    run_id: &str,
    error: &vb_storage::recovery::RecoveryError,
    message: &str,
    context: serde_json::Value,
    code: CliExitCode,
) -> serde_json::Value {
    serde_json::json!({
        "schema_version": crate::cli_envelope::SCHEMA_VERSION,
        "kind": "replay_report",
        "success": false,
        "run_id": run_id,
        "status": "recovery_error",
        "recovered": 0,
        "event_count": 0,
        "first_sequence": Option::<u64>::None,
        "last_sequence": Option::<u64>::None,
        "terminal": Option::<&str>::None,
        "terminal_event": Option::<&str>::None,
        "terminal_status": "none",
        "event_counts": {"step": 0, "action": 0, "slot": 0},
        "action_counts": {"scheduled": 0, "resolved": 0, "pending_unresolved": 0},
        "events": [],
        "context": context,
        "recovery_error_class": recovery_error_class(error),
        "exit_code": u8::from(code),
        "error": message
    })
}

fn render_replay_failure(outcome: &ReplayFailureOutcome, output: OutputFormat) {
    match output {
        OutputFormat::Text => crate::errln!("{}", outcome.message),
        OutputFormat::Yaml | OutputFormat::Postcard => {
            if let Err(error) = write_structured_stderr(&outcome.structured, output) {
                write_stderr_line(format_args!(
                    "replay structured error output failed: {error}"
                ));
            }
        }
    }
}

fn recovery_error_class(error: &vb_storage::recovery::RecoveryError) -> &'static str {
    match error {
        vb_storage::recovery::RecoveryError::Journal(_) => "journal",
        vb_storage::recovery::RecoveryError::WorkflowSourceDigestMismatch { .. }
        | vb_storage::recovery::RecoveryError::CompiledIrDigestMismatch { .. } => {
            "workflow_digest_mismatch"
        }
        vb_storage::recovery::RecoveryError::ActionAbiMismatch { .. } => "action_abi_mismatch",
        vb_storage::recovery::RecoveryError::PolicyDigestMismatch { .. }
        | vb_storage::recovery::RecoveryError::PolicyDigestUnavailable { .. }
        | vb_storage::recovery::RecoveryError::PolicyDigestExpectationMissing { .. } => {
            "policy_digest_error"
        }
        vb_storage::recovery::RecoveryError::FullDigestCheckConfigMissing => {
            "full_digest_config_missing"
        }
        vb_storage::recovery::RecoveryError::RunAdmissionArtifactDigestMismatch { .. } => {
            "run_admission_artifact_digest_mismatch"
        }
        vb_storage::recovery::RecoveryError::NonIdempotentActionBlocked { .. } => {
            "non_idempotent_action_blocked"
        }
        vb_storage::recovery::RecoveryError::ReplayDivergence { .. } => "replay_divergence",
        vb_storage::recovery::RecoveryError::SlotTaintReadFailed { .. }
        | vb_storage::recovery::RecoveryError::CorruptSlotTaint { .. } => "slot_taint_error",
        vb_storage::recovery::RecoveryError::NoRecoveryData { .. } => "no_recovery_data",
        vb_storage::recovery::RecoveryError::CorruptSnapshot { .. } => "corrupt_snapshot",
        vb_storage::recovery::RecoveryError::TerminalStateMismatch { .. } => {
            "terminal_state_mismatch"
        }
        vb_storage::recovery::RecoveryError::FrameDimensionOverflow { .. } => {
            "frame_dimension_overflow"
        }
        _ => "unknown_recovery_error",
    }
}

pub(crate) fn write_locked_read_surface(
    command: &'static str,
    run_id: &str,
    output: OutputFormat,
) -> ExitCode {
    match output {
        OutputFormat::Text => {
            crate::outln!(
                "{command} run {run_id}: storage is held by an active writer; public CLI surface is available"
            );
            super::write_vb_kyyf_trace(command, run_id, 0);
            ExitCode::SUCCESS
        }
        OutputFormat::Yaml | OutputFormat::Postcard => json_out_exit(
            &serde_json::json!({
                "run_id": run_id,
                "command": command,
                "status": "writer_lock_held",
                "surface": "available"
            }),
            output,
        ),
    }
}
