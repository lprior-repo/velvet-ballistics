//! Execution trace command.
    run_id: &str,
    db: &std::path::Path,
    output: OutputFormat,
    filters: commands_journal::TraceFilters,
) -> ExitCode {
    let events = match read_journal_events(run_id, db, output) {
        Ok(ev) => ev,
        Err(code) => return code,
    };
    let trace = commands_journal::filter_trace(commands_journal::build_trace(&events), filters);
    if trace.is_empty() {
        if output != OutputFormat::Text {
            emit_json_or_return!(
                &serde_json::json!({
                    "schema_version": cli_envelope::SCHEMA_VERSION,
                    "kind": "trace_report",
                    "run_id": run_id,
                    "trace": [],
                    "total": 0
                }),
                output,
            );
        } else {
            outln!("no events found for run {run_id}");
        }
        return CliExitCode::Success.into();
    }
    match output {
        OutputFormat::Yaml | OutputFormat::Postcard => {
            let entries: Vec<serde_json::Value> = trace.iter().map(trace_entry_to_json).collect();
            emit_json_or_return!(
                &serde_json::json!({
                    "schema_version": cli_envelope::SCHEMA_VERSION,
                    "kind": "trace_report",
                    "run_id": run_id,
                    "trace": entries,
                    "total": trace.len()
                }),
                output,
            );
        }
        OutputFormat::Text => {
            outln!("execution trace for run {run_id}");
            for e in &trace {
                match e.step {
                    Some(step) => outln!(
                        "  [{}] {} step {} (seq {})",
                        e.index,
                        e.event_type,
                        step,
                        e.seq
                    ),
                    None => outln!("  [{}] {} (seq {})", e.index, e.event_type, e.seq),
                }
            }
            outln!("{} event(s) total", trace.len());
        }
    }
    CliExitCode::Success.into()
}

/// Convert a structured trace entry to its JSON representation.
pub(crate) fn trace_entry_to_json(entry: &commands_journal::TraceEntry) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    map.insert("seq".into(), serde_json::Value::from(entry.seq));
    map.insert("type".into(), serde_json::Value::from(entry.event_type));
    if let Some(step) = entry.step {
        map.insert("step".into(), serde_json::Value::from(step));
    }
    if let Some(status) = entry.status {
        map.insert("status".into(), serde_json::Value::from(status.as_str()));
    }
    if let Some(action) = entry.action {
        map.insert("action".into(), serde_json::Value::from(action));
    }
    for (k, v) in &entry.extra_json {
        map.insert((*k).into(), v.clone());
    }
    serde_json::Value::Object(map)
}

