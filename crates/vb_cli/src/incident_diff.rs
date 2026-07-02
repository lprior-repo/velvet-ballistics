//! Module: incident_diff
//!
//! [`cmd_incident`] is structured as a strict classify-then-emit pipeline:
//!
//! 1. [`load_run_events`]  → loads the journal events for the run
//! 2. [`classify_run_events`] → produces a [`RunClassification`] verdict
//! 3. emit a response **only** after the verdict is known
//!
//! Splitting the prior monolithic `cmd_incident` into these named
//! helpers fixes review-rejection blocker 1 ("cmd_incident emits
//! report before deciding non-incident failure") because the report is
//! now only emitted when `classification.kind` is
//! [`RunClassification::Incident`]. Splitting the function also
//! addresses blocker 3 ("split overlong functions") by keeping each
//! phase under 25 lines of hot logic.

use crate::app_impl::prelude::*;
use vb_storage::journal::incident::IncidentAnalysis;

/// Classified outcome of running [`vb_storage::analyze_incident_events`]
/// against one run's journal events.
///
/// Drives the classify-then-emit pipeline in [`cmd_incident`]. The CLI
/// uses this verdict to decide whether to emit a full incident report
/// or a clean "not an incident" response. Constructing this before any
/// output is emitted closes the historical race where the report was
/// always emitted and only the exit code reflected the incident
/// classification.
#[derive(Debug, Clone)]
pub(crate) enum RunClassification {
    /// The event stream contains a recognized failure event.
    Incident(IncidentAnalysis),
    /// The event stream contains no recognized failure event.
    NotAnIncident,
    /// The event stream is empty (no events at all).
    NoEvents,
}

/// Open the journal at `db` and return the events for `rid`. Emits a
/// diagnostic on failure; returns `Err(exit)` so the caller propagates
/// the same exit code.
///
/// Extracted from `cmd_incident` so the classification pipeline is not
/// interleaved with input plumbing. The two I/O paths (journal-open
/// and event-read) are split into [`open_journal_or_diagnostic`] and
/// [`read_events_or_diagnostic`] so each error branch stays small.
fn load_run_events(
    run_id: &str,
    rid: vb_core::RunId,
    db: &std::path::Path,
    output: OutputFormat,
) -> Result<Vec<vb_storage::JournalEvent>, ExitCode> {
    let journal = open_journal_or_diagnostic(db, output)?;
    read_events_or_diagnostic(&journal, run_id, rid, output)
}

/// Open the journal at `db`. On failure, emit a structured diagnostic
/// and return the propagation exit code.
fn open_journal_or_diagnostic(
    db: &std::path::Path,
    output: OutputFormat,
) -> Result<vb_storage::FjallJournal, ExitCode> {
    vb_storage::FjallJournal::open(db, None).map_err(|e| -> ExitCode {
        let message = format!("error opening journal at {}: {e}", db.display());
        emit_storage_diagnostic(&message, output);
        CliExitCode::StorageError.into()
    })
}

/// Read events for `rid` from `journal`. On failure, emit a structured
/// diagnostic and return the propagation exit code.
fn read_events_or_diagnostic(
    journal: &vb_storage::FjallJournal,
    run_id: &str,
    rid: vb_core::RunId,
    output: OutputFormat,
) -> Result<Vec<vb_storage::JournalEvent>, ExitCode> {
    journal.events_for_run(rid).map_err(|e| -> ExitCode {
        let message = format!("error reading events for run {run_id}: {e}");
        emit_storage_diagnostic(&message, output);
        CliExitCode::StorageError.into()
    })
}

/// Emit a storage-error diagnostic in the requested output format.
fn emit_storage_diagnostic(message: &str, output: OutputFormat) {
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({"success": false, "error": message}),
            output,
        );
    } else {
        errln!("{message}");
    }
}

/// Classify the run's event stream.
///
/// This is the single decision point that closes
/// review-rejection blocker 1: the verdict is fully computed before
/// any output is emitted. The caller decides whether to emit a report
/// (only when [`RunClassification::Incident`]) or a "not an incident"
/// response.
#[must_use]
fn classify_run_events(events: &[vb_storage::JournalEvent]) -> RunClassification {
    if events.is_empty() {
        return RunClassification::NoEvents;
    }
    let analysis = vb_storage::analyze_incident_events(events);
    if analysis.failure_found {
        RunClassification::Incident(analysis)
    } else {
        RunClassification::NotAnIncident
    }
}

/// Emit the diagnostic for "no events found for run" case. The helper
/// keeps `cmd_incident` under 25 lines and pins the diagnostic message
/// to a stable string.
fn emit_no_events_diagnostic(run_id: &str, output: OutputFormat) {
    let message = format!("no events found for run {run_id}");
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({"success": false, "error": message}),
            output,
        );
    } else {
        errln!("{message}");
    }
}

/// Emit the diagnostic for "not an incident" case. Kept separate so
/// `cmd_incident` itself only orchestrates.
fn emit_non_incident_diagnostic(run_id: &str, output: OutputFormat) {
    let message = format!("run {run_id} has no failure event; not an incident");
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({"success": false, "error": message}),
            output,
        );
    } else {
        errln!("{message}");
    }
}

/// Emit the full incident report. Only called when the classification
/// is [`RunClassification::Incident`]; before the refactor the report
/// was always emitted and only the exit code reflected the verdict.
///
/// Returns the [`ExitCode`] chosen by [`emit_json_or_return!`] when
/// emission fails, otherwise [`CliExitCode::Success`]. The early-return
/// (`return ...;`) inside `emit_json_or_return!` propagates the
/// output-error exit code through this helper.
///
/// Thin dispatcher: delegates to per-format helpers so each branch is
/// reviewable in isolation. The JSON branch covers YAML + Postcard
/// (both are envelope-based serializations of the same payload).
fn emit_incident_report(
    report: &commands_incident::IncidentReport,
    output: OutputFormat,
) -> ExitCode {
    if output == OutputFormat::Text {
        emit_incident_report_text(report);
        CliExitCode::Success.into()
    } else {
        emit_incident_report_json(report, output)
    }
}

/// Map a report's `failed_at_step` to its JSON representation.
fn failed_at_step_to_json(report: &commands_incident::IncidentReport) -> serde_json::Value {
    match report.failed_at_step {
        Some(step) => serde_json::Value::Number(serde_json::Number::from(step)),
        None => serde_json::Value::Null,
    }
}

/// Build the full JSON envelope payload for an incident report.
fn build_incident_report_json(report: &commands_incident::IncidentReport) -> serde_json::Value {
    serde_json::json!({
        "run_id": report.run_id,
        "failure_code": report.failure_code,
        "failure_found": report.failure_found,
        "failed_at_step": failed_at_step_to_json(report),
        "side_effects": report.side_effects,
        "repair_hints": report.repair_hints,
    })
}

/// Emit the JSON / YAML / Postcard branch of the incident report.
///
/// Returns [`ExitCode`] so the caller's [`emit_json_or_return!`]
/// macro-induced early return can propagate cleanly. On success the
/// caller dispatches to [`emit_incident_report_text`] instead.
fn emit_incident_report_json(
    report: &commands_incident::IncidentReport,
    output: OutputFormat,
) -> ExitCode {
    let json_report = build_incident_report_json(report);
    emit_json_or_return!(&json_report, output);
    CliExitCode::Success.into()
}

/// Emit the human-readable Text branch of the incident report.
fn emit_incident_report_text(report: &commands_incident::IncidentReport) {
    outln!("incident report for run {}", report.run_id);
    outln!("  failure_code:  {}", report.failure_code);
    match report.failed_at_step {
        Some(step) => outln!("  failed_at_step: {step}"),
        None => outln!("  failed_at_step: unknown"),
    }
    outln!("  side_effects:");
    format_side_effects(report);
    outln!("  repair_hints:");
    for hint in &report.repair_hints {
        let hint_str = hint.as_str().map_or("unknown", std::convert::identity);
        outln!("    - {hint_str}");
    }
}

/// Format the side-effect rows for the Text branch.
fn format_side_effects(report: &commands_incident::IncidentReport) {
    if report.side_effects.is_empty() {
        outln!("    (none)");
        return;
    }
    for se in &report.side_effects {
        let step = &se["step"];
        let action = &se["action"];
        let certainty = se["certainty"]
            .as_str()
            .map_or("unknown", std::convert::identity);
        outln!("    step={step} action={action} certainty={certainty}");
    }
}

pub(crate) fn cmd_incident(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid = match parse_run_id(run_id, output) {
        Ok(id) => id,
        Err(code) => return code,
    };

    // 1. Load events (storage-layer I/O).
    let events = match load_run_events(run_id, rid, db, output) {
        Ok(evts) => evts,
        Err(code) => return code,
    };

    // 2. Classify BEFORE emitting anything. Verdict decides whether
    //    the report is rendered at all.
    let classification = classify_run_events(&events);
    match classification {
        RunClassification::NoEvents => {
            emit_no_events_diagnostic(run_id, output);
            CliExitCode::StorageError.into()
        }
        RunClassification::NotAnIncident => {
            // No report emission — the prior review rejected this code
            // path because the report was always rendered before this
            // decision. Emit only the diagnostic.
            emit_non_incident_diagnostic(run_id, output);
            CliExitCode::StorageError.into()
        }
        RunClassification::Incident(analysis) => {
            // 3. Emit the report only after the verdict is "incident".
            let report = commands_incident::IncidentReport::from_analysis(run_id, analysis);
            emit_incident_report(&report, output)
        }
    }
}
