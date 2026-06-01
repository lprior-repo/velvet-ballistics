#![forbid(unsafe_code)]
//! Incident analysis and diff commands.

fn cmd_incident(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid = match parse_run_id(run_id, output) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error opening journal at {}: {e}", db.display())
                    }),
                    output,
                );
            } else {
                errln!("error opening journal at {}: {e}", db.display());
            }
            return CliExitCode::StorageError.into();
        }
    };

    let events = match journal.events_for_run(rid) {
        Ok(evts) => evts,
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
    };

    if events.is_empty() {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": format!("no events found for run {run_id}")
                }),
                output,
            );
        } else {
            errln!("no events found for run {run_id}");
        }
        return CliExitCode::StorageError.into();
    }

    let report = commands_incident::build_incident_report(run_id, &events);
    let failed_step_val = report
        .failed_at_step
        .map(|s| serde_json::Value::Number(serde_json::Number::from(s)))
        .unwrap_or(serde_json::Value::Null);

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
                    // WAIVER: Option::unwrap_or is not Result::unwrap — no panic path.
                    // This is safe fallback for missing JSON fields in CLI report display.
                    let certainty = se["certainty"].as_str().unwrap_or("unknown");
                    outln!("    step={step} action={action} certainty={certainty}");
                }
            }
            outln!("  repair_hints:");
            for hint in &report.repair_hints {
                // WAIVER: Option::unwrap_or is not Result::unwrap — no panic path.
                // This is safe fallback for missing hint strings in CLI report display.
                let hint_str = hint.as_str().unwrap_or("unknown");
                outln!("    - {hint_str}");
            }
        }
    }

    if report.failure_found {
        CliExitCode::Success.into()
    } else {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": format!("run {run_id} has no failure event; not an incident")
                }),
                output,
            );
        } else {
            errln!("run {run_id} has no failure event; not an incident");
        }
        CliExitCode::StorageError.into()
    }
}


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
                errln!("error opening journal at {}: {e}", db.display());
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
                errln!("error reading run {run_a}: {e}");
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
                errln!("error reading run {run_b}: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    };

    let result = commands_diff::compute_diff(&events_a, &events_b);

    match output {
        OutputFormat::Yaml | OutputFormat::Postcard => {
            emit_json_or_return!(
                &serde_json::json!({
                    "schema_version": cli_envelope::SCHEMA_VERSION,
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
            outln!("diff: run {run_a} vs run {run_b}");
            outln!("  events: {} vs {}", result.events_a, result.events_b);
            if result.diffs.is_empty() {
                outln!("  no differences found");
            } else {
                for diff in &result.diffs {
                    print_diff_entry(diff);
                }
                outln!("  {} difference(s) total", result.diffs.len());
            }
        }
    }
    CliExitCode::Success.into()
}


fn print_diff_entry(diff: &serde_json::Value) {
    let kind = diff
        .get("kind")
        .and_then(|k| k.as_str())
        .unwrap_or("unknown");
    match kind {
        "only_in_a" => {
            let idx = diff.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
            outln!("  [{idx}] - only in run A");
        }
        "only_in_b" => {
            let idx = diff.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
            outln!("  [{idx}] + only in run B");
        }
        "changed" => {
            let idx = diff.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
            outln!("  [{idx}] ~ changed");
        }
        "step_missing_in_b" => {
            let s = diff.get("step").and_then(|v| v.as_u64()).unwrap_or(0);
            outln!("  step {s}: - present in run A only");
        }
        "step_missing_in_a" => {
            let s = diff.get("step").and_then(|v| v.as_u64()).unwrap_or(0);
            outln!("  step {s}: + present in run B only");
        }
        "step_outcome_differs" => {
            let s = diff.get("step").and_then(|v| v.as_u64()).unwrap_or(0);
            let oa = diff
                .get("outcome_a")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let ob = diff
                .get("outcome_b")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            outln!("  step {s}: ~ {oa} vs {ob}");
        }
        "slot_missing_in_b" => {
            let s = diff.get("slot").and_then(|v| v.as_u64()).unwrap_or(0);
            outln!("  slot {s}: - present in run A only");
        }
        "slot_missing_in_a" => {
            let s = diff.get("slot").and_then(|v| v.as_u64()).unwrap_or(0);
            outln!("  slot {s}: + present in run B only");
        }
        "slot_value_differs" => {
            let s = diff.get("slot").and_then(|v| v.as_u64()).unwrap_or(0);
            let va = diff.get("value_a").and_then(|v| v.as_str()).unwrap_or("?");
            let vb = diff.get("value_b").and_then(|v| v.as_str()).unwrap_or("?");
            outln!("  slot {s}: ~ {va} vs {vb}");
        }
        _ => {
            outln!("  unknown diff kind: {kind}");
        }
    }
}

