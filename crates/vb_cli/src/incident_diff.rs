//! CLI handler for the incident subcommand: opens a journal, builds the failure report, and emits it in the requested output format.

use crate::app_impl::prelude::*;

pub(crate) fn cmd_incident(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid = match parse_run_id(run_id, output) {
        Ok(id) => id,
        Err(code) => return code,
    };
    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            return storage_diagnostic(&format!("error opening journal at {}: {e}", db.display()), output);
        }
    };
    let events = match journal.events_for_run(rid) {
        Ok(evts) => evts,
        Err(e) => {
            return storage_diagnostic(&format!("error reading events for run {run_id}: {e}"), output);
        }
    };
    if events.is_empty() {
        return storage_diagnostic(&format!("no events found for run {run_id}"), output);
    }
    let report = commands_incident::build_incident_report(run_id, &events);
    emit_incident_report(&report, run_id, output);
    if report.failure_found {
        return CliExitCode::Success.into();
    }
    storage_diagnostic(&format!("run {run_id} has no failure event; not an incident"), output)
}

fn storage_diagnostic(message: &str, output: OutputFormat) -> ExitCode {
    if output == OutputFormat::Text {
        errln!("{message}");
    } else {
        json_error(
            &serde_json::json!({
                "success": false,
                "error": message,
            }),
            output,
        );
    }
    CliExitCode::StorageError.into()
}

fn emit_incident_report(
    report: &commands_incident::IncidentReport,
    run_id: &str,
    output: OutputFormat,
) -> ExitCode {
    let failed_step_val = match report.failed_at_step {
        Some(step) => serde_json::Value::Number(serde_json::Number::from(step)),
        None => serde_json::Value::Null,
    };

    let json_report = serde_json::json!({
        "run_id": report.run_id,
        "failure_code": report.failure_code,
        "failed_at_step": failed_step_val,
        "side_effects": report.side_effects,
        "repair_hints": report.repair_hints,
    });

    match output {
        OutputFormat::Yaml | OutputFormat::Postcard => {
            emit_json_or_return!(&json_report, output);
        }
        OutputFormat::Text => {
            outln!("incident report for run {run_id}");
            outln!("  failure_code:  {}", report.failure_code);
            match report.failed_at_step {
                Some(step) => outln!("  failed_at_step: {step}"),
                None => outln!("  failed_at_step: unknown"),
            }
            outln!("  side_effects:");
            if report.side_effects.is_empty() {
                outln!("    (none)");
            } else {
                for se in &report.side_effects {
                    let step = &se["step"];
                    let action = &se["action"];
                    let certainty = se["certainty"]
                        .as_str()
                        .map_or("unknown", std::convert::identity);
                    outln!("    step={step} action={action} certainty={certainty}");
                }
            }
            outln!("  repair_hints:");
            for hint in &report.repair_hints {
                let hint_str = hint.as_str().map_or("unknown", std::convert::identity);
                outln!("    - {hint_str}");
            }
        }
    }
    CliExitCode::Success.into()
}
