use crate::app_impl::prelude::*;
use crate::commands_incident::{self, IncidentReport};
use vb_storage::{FjallJournal, JournalEvent};

pub(crate) fn cmd_incident(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid = match parse_run_id(run_id, output) {
        Ok(id) => id,
        Err(code) => return code,
    };
    match load_events(run_id, rid, db, output) {
        Ok(events) => report_events(run_id, &events, output),
        Err(code) => code,
    }
}

fn load_events(
    run_id: &str,
    rid: vb_core::RunId,
    db: &std::path::Path,
    output: OutputFormat,
) -> Result<Vec<JournalEvent>, ExitCode> {
    check_journal_dir(db, output)?;
    let journal = FjallJournal::open(db, None).map_err(|error| open_error(db, output, error))?;
    let events = journal
        .events_for_run_full(rid)
        .map_err(|error| read_error(run_id, output, error))?;
    if events.is_empty() {
        return Err(no_events(run_id, output));
    }
    Ok(events)
}

fn open_error(
    db: &std::path::Path,
    output: OutputFormat,
    error: impl std::fmt::Display,
) -> ExitCode {
    report_error(
        output,
        format!("error opening journal at {}: {error}", db.display()),
    );
    CliExitCode::StorageError.into()
}

fn read_error(run_id: &str, output: OutputFormat, error: impl std::fmt::Display) -> ExitCode {
    report_error(
        output,
        format!("error reading events for run {run_id}: {error}"),
    );
    CliExitCode::StorageError.into()
}

fn no_events(run_id: &str, output: OutputFormat) -> ExitCode {
    report_error(output, format!("no events found for run {run_id}"));
    CliExitCode::StorageError.into()
}

fn report_error(output: OutputFormat, message: String) {
    if output == OutputFormat::Text {
        errln!("{message}");
    } else {
        json_error(
            &serde_json::json!({"success": false, "error": message}),
            output,
        );
    }
}

fn report_events(run_id: &str, events: &[JournalEvent], output: OutputFormat) -> ExitCode {
    let report = commands_incident::build_incident_report(run_id, events);
    if !report.failure_found {
        return no_failure(run_id, output);
    }
    emit_report(run_id, &report, output)
}

fn no_failure(run_id: &str, output: OutputFormat) -> ExitCode {
    report_error(
        output,
        format!("run {run_id} has no failure event; not an incident"),
    );
    CliExitCode::StorageError.into()
}

fn emit_report(run_id: &str, report: &IncidentReport, output: OutputFormat) -> ExitCode {
    let value = report_value(report);
    match output {
        OutputFormat::Yaml | OutputFormat::Postcard => emit_json_or_return!(&value, output),
        OutputFormat::Text => emit_text_report(run_id, report),
    }
    CliExitCode::Success.into()
}

fn report_value(report: &IncidentReport) -> serde_json::Value {
    let failed_step = report.failed_at_step.map_or_else(
        || serde_json::Value::Null,
        |step| serde_json::Value::Number(serde_json::Number::from(step)),
    );
    serde_json::json!({
        "run_id": report.run_id,
        "failure_code": report.failure_code,
        "failed_at_step": failed_step,
        "side_effects": report.side_effects,
        "repair_hints": report.repair_hints,
        "journal_tail": report.journal_tail,
    })
}

fn emit_text_report(run_id: &str, report: &IncidentReport) {
    outln!("incident report for run {run_id}");
    outln!("  failure_code:  {}", report.failure_code);
    emit_failed_step(report.failed_at_step);
    emit_side_effects(&report.side_effects);
    emit_repair_hints(&report.repair_hints);
    emit_journal_tail(&report.journal_tail);
}

fn emit_failed_step(step: Option<u16>) {
    match step {
        Some(value) => outln!("  failed_at_step: {value}"),
        None => outln!("  failed_at_step: unknown"),
    }
}

fn emit_side_effects(effects: &[serde_json::Value]) {
    outln!("  side_effects:");
    if effects.is_empty() {
        outln!("    (none)");
    }
    effects.iter().for_each(|effect| outln!("    {effect}"));
}

fn emit_repair_hints(hints: &[serde_json::Value]) {
    outln!("  repair_hints:");
    hints.iter().for_each(|hint| {
        outln!(
            "    - {}",
            hint.as_str().map_or("unknown", std::convert::identity)
        )
    });
}

fn emit_journal_tail(events: &[serde_json::Value]) {
    outln!("  journal_tail:");
    events.iter().for_each(|event| outln!("    {event}"));
}
