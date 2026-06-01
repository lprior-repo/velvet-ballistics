use crate::args::OutputFormat;
use crate::exit_code::CliExitCode;
use crate::file_io::parse_run_id;
use crate::output::json_error;
use std::process::ExitCode;

fn cmd_diff(run_a: &str, run_b: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid_a = match parse_run_id(run_a, output) {
        Ok(id) => id,
        Err(code) => return code,
    };
    let rid_b = match parse_run_id(run_b, output) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({"success": false, "error": format!("error opening journal at {}: {e}", db.display())}),
                    output,
                );
            } else {
                crate::errln!("error opening journal at {}: {e}", db.display());
            }
            return CliExitCode::StorageError.into();
        }
    };

    let events_a = match journal.events_for_run(rid_a) {
        Ok(events) => events,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({"success": false, "error": format!("error reading run {run_a}: {e}")}),
                    output,
                );
            } else {
                crate::errln!("error reading run {run_a}: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    };

    let events_b = match journal.events_for_run(rid_b) {
        Ok(events) => events,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({"success": false, "error": format!("error reading run {run_b}: {e}")}),
                    output,
                );
            } else {
                crate::errln!("error reading run {run_b}: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    };

    let result = crate::commands_diff::compute_diff(&events_a, &events_b);

    match output {
        OutputFormat::Yaml | OutputFormat::Postcard => {
            crate::emit_json_or_return!(
                &serde_json::json!({
                    "schema_version": crate::cli_envelope::SCHEMA_VERSION,
                    "kind": "diff_report",
                    "run_a": run_a,
                    "run_b": run_b,
                    "events_a": result.events_a,
                    "events_b": result.events_b,
                    "diffs": result.diffs,
                    "total_differences": result.diffs.len()
                }),
                output,
            );
        }
        OutputFormat::Text => {
            crate::outln!("diff: run {run_a} vs run {run_b}");
            crate::outln!("  events: {} vs {}", result.events_a, result.events_b);
            if result.diffs.is_empty() {
                crate::outln!("  no differences found");
            } else {
                for diff in &result.diffs {
                    print_diff_entry(diff);
                }
                crate::outln!("  {} difference(s) total", result.diffs.len());
            }
        }
    }
    CliExitCode::Success.into()
}

fn print_diff_entry(diff: &serde_json::Value) {
    let kind = str_field(diff, "kind", "unknown");
    match kind {
        "only_in_a" => {
            let idx = u64_field(diff, "index");
            crate::outln!("  [{idx}] - only in run A");
        }
        "only_in_b" => {
            let idx = u64_field(diff, "index");
            crate::outln!("  [{idx}] + only in run B");
        }
        "changed" => {
            let idx = u64_field(diff, "index");
            crate::outln!("  [{idx}] ~ changed");
        }
        "step_missing_in_b" => {
            let s = u64_field(diff, "step");
            crate::outln!("  step {s}: - present in run A only");
        }
        "step_missing_in_a" => {
            let s = u64_field(diff, "step");
            crate::outln!("  step {s}: + present in run B only");
        }
        "step_outcome_differs" => {
            let s = u64_field(diff, "step");
            let oa = str_field(diff, "outcome_a", "?");
            let ob = str_field(diff, "outcome_b", "?");
            crate::outln!("  step {s}: ~ {oa} vs {ob}");
        }
        "slot_missing_in_b" => {
            let s = u64_field(diff, "slot");
            crate::outln!("  slot {s}: - present in run A only");
        }
        "slot_missing_in_a" => {
            let s = u64_field(diff, "slot");
            crate::outln!("  slot {s}: + present in run B only");
        }
        "slot_value_differs" => {
            let s = u64_field(diff, "slot");
            let va = str_field(diff, "value_a", "?");
            let vb = str_field(diff, "value_b", "?");
            crate::outln!("  slot {s}: ~ {va} vs {vb}");
        }
        _ => {
            crate::outln!("  unknown diff kind: {kind}");
        }
    }
}

fn str_field<'value>(
    value: &'value serde_json::Value,
    field: &str,
    fallback: &'static str,
) -> &'value str {
    value
        .get(field)
        .and_then(|entry| entry.as_str())
        .map_or(fallback, std::convert::identity)
}

fn u64_field(value: &serde_json::Value, field: &str) -> u64 {
    value
        .get(field)
        .and_then(|entry| entry.as_u64())
        .map_or(0, std::convert::identity)
}
