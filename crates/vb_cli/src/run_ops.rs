#![forbid(unsafe_code)]
//! Run operations: retry, resume, answer, cancel.

fn cmd_retry(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let events = match read_journal_events(run_id, db, output) {
        Ok(ev) => ev,
        Err(code) => return code,
    };
    if events.is_empty() {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({ "success": false, "error": format!("run {run_id} not found") }),
                output,
            );
        } else {
            errln!("run {run_id}: no events found");
        }
        return CliExitCode::StorageError.into();
    }
    let analysis = commands_journal::analyze_retry(&events);
    if !analysis.can_retry {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({ "success": false, "error": format!("run {run_id} {}", analysis.reason) }),
                output,
            );
        } else {
            errln!("run {run_id} {}", analysis.reason);
        }
        return CliExitCode::ValidationFailed.into();
    }
    let resume_step = analysis.last_successful_step.map(|s| s.saturating_add(1));
    if output != OutputFormat::Text {
        emit_json_or_return!(
            &serde_json::json!({
                "run_id": run_id, "failed_at_step": analysis.failed_at_step,
                "last_successful_step": analysis.last_successful_step,
                "resume_from_step": resume_step, "events": events.len()
            }),
            output,
        );
    } else {
        match (analysis.failed_at_step, analysis.last_successful_step) {
            (Some(fail), Some(last)) => {
                outln!("Run {run_id} failed at step {fail}. Last successful: step {last}.")
            }
            (Some(fail), None) => {
                outln!("Run {run_id} failed at step {fail}. No successful steps recorded.")
            }
            (None, Some(last)) => outln!("Run {run_id} failed. Last successful: step {last}."),
            (None, None) => outln!("Run {run_id} failed. No step progress recorded."),
        }
        match resume_step {
            Some(step) => outln!("Retry would resume from step {step} with recovered state."),
            None => outln!("Retry would resume from the beginning."),
        }
    }
    ExitCode::SUCCESS
}


fn cmd_resume(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let events = match read_journal_events(run_id, db, output) {
        Ok(ev) => ev,
        Err(code) => return code,
    };
    if events.is_empty() {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({ "success": false, "error": format!("run {run_id} not found") }),
                output,
            );
        } else {
            errln!("run {run_id}: no events found");
        }
        return CliExitCode::StorageError.into();
    }
    let analysis = commands_journal::analyze_resume(&events);
    if !analysis.can_resume {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({ "success": false, "error": format!("run {run_id} {}", analysis.reason) }),
                output,
            );
        } else {
            errln!("run {run_id} {}", analysis.reason);
        }
        return CliExitCode::ValidationFailed.into();
    }
    let resume_step = analysis.suspended_at_step;
    if output != OutputFormat::Text {
        emit_json_or_return!(
            &serde_json::json!({
                "run_id": run_id, "suspended_at_step": analysis.suspended_at_step,
                "status": "suspended", "resume_from_step": resume_step, "events": events.len()
            }),
            output,
        );
    } else {
        match resume_step {
            Some(step) => outln!(
                "Run {run_id} suspended at step {step}. Resume would continue from step {step} with recovered state."
            ),
            None => outln!(
                "Run {run_id} is active but no explicit suspension point found. Resume would continue from current state."
            ),
        }
    }
    ExitCode::SUCCESS
}


fn cmd_answer(
    run_id: &str,
    step: u16,
    value_file: &std::path::Path,
    db: &std::path::Path,
    output: OutputFormat,
) -> ExitCode {
    // Parse run_id
    let rid = match run_id.parse::<u64>() {
        Ok(id) => vb_core::RunId::new(id),
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("invalid run_id '{run_id}': {e}")
                    }),
                    output,
                );
            } else {
                errln!("invalid run_id '{run_id}': {e}");
            }
            return CliExitCode::ValidationFailed.into();
        }
    };

    // Read value_file as bytes (expected to be postcard-encoded SlotValue)
    let answer_bytes = match std::fs::read(value_file) {
        Ok(bytes) => bytes,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error reading value file {}: {e}", value_file.display())
                    }),
                    output,
                );
            } else {
                errln!("error reading value file {}: {e}", value_file.display());
            }
            return CliExitCode::ValidationFailed.into();
        }
    };

    // Derive IPC socket path from db path: <db_parent>/<db_stem>.sock
    // e.g., /var/lib/vb/run.db -> /var/lib/vb/run.sock
    let socket_path = db.with_extension("sock");

    // Connect to the IPC server
    let mut client = match IpcClient::connect(&socket_path) {
        Ok(c) => c,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error connecting to IPC server at {}: {e}", socket_path.display())
                    }),
                    output,
                );
            } else {
                errln!(
                    "error connecting to IPC server at {}: {e}",
                    socket_path.display()
                );
            }
            return CliExitCode::IpcError.into();
        }
    };

    // Construct the IPC payload
    let payload = IpcPayload::AnswerAsk {
        run_id: rid,
        ticket: step.into(),
        answer: answer_bytes,
        taint: None,
    };

    // Send the command
    if let Err(e) = client.send_command(IpcCommand::AnswerAsk, 0, &payload) {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": format!("error sending answer: {e}")
                }),
                output,
            );
        } else {
            errln!("error sending answer: {e}");
        }
        return CliExitCode::IpcError.into();
    }

    // Receive and process the response
    match client.recv_response(vb_ipc::MaxPayloadBytes::DEFAULT) {
        Ok((_header, response)) => match response {
            vb_ipc::server::IpcResponse::AcceptedRun { run_id: _ } => {
                if output != OutputFormat::Text {
                    emit_json_or_return!(
                        &serde_json::json!({
                            "success": true,
                            "run_id": rid.get()
                        }),
                        output,
                    );
                } else {
                    outln!("answer accepted for run {}", rid.get());
                }
                ExitCode::SUCCESS
            }
            vb_ipc::server::IpcResponse::RuntimeError { message } => {
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "error": message
                        }),
                        output,
                    );
                } else {
                    errln!("runtime error: {message}");
                }
                CliExitCode::RuntimeFailed.into()
            }
            vb_ipc::server::IpcResponse::PayloadError {
                diagnostic: _,
                message,
            } => {
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "error": message
                        }),
                        output,
                    );
                } else {
                    errln!("payload error: {message}");
                }
                CliExitCode::ValidationFailed.into()
            }
            other => {
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "error": format!("unexpected response: {other:?}")
                        }),
                        output,
                    );
                } else {
                    errln!("unexpected response: {other:?}");
                }
                CliExitCode::RuntimeFailed.into()
            }
        },
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error receiving response: {e}")
                    }),
                    output,
                );
            } else {
                errln!("error receiving response: {e}");
            }
            CliExitCode::IpcError.into()
        }
    }
}


fn run_is_terminal(events: &[vb_storage::JournalEvent]) -> bool {
    events.iter().any(|e| {
        matches!(
            e,
            vb_storage::JournalEvent::RunFinished { .. }
                | vb_storage::JournalEvent::RunFailedEvent { .. }
                | vb_storage::JournalEvent::RunCancelled { .. }
        )
    })
}


fn format_cancel_output(
    run_id: &str,
    reason: Option<&str>,
    note: &str,
    output: OutputFormat,
) -> ExitCode {
    if output != OutputFormat::Text {
        emit_json_or_return!(
            &serde_json::json!({
                "success": true,
                "run_id": run_id,
                "status": "cancelled",
                "reason": reason,
                "note": note,
            }),
            output,
        );
        ExitCode::SUCCESS
    } else {
        let detail = match reason {
            Some(r) => format!(" (reason: {r})"),
            None => String::new(),
        };
        outln!("Run {run_id} cancelled{detail} ({note})");
        ExitCode::SUCCESS
    }
}


fn write_cancel_event(
    journal: &vb_storage::FjallJournal,
    rid: vb_core::RunId,
    reason: Option<String>,
    events: &[vb_storage::JournalEvent],
) -> Result<(), vb_storage::JournalError> {
    let next_seq = match events.last() {
        Some(e) => e.seq().get().saturating_add(1),
        None => 0,
    };
    let event = vb_storage::JournalEvent::RunCancelled {
        run: rid,
        seq: vb_storage::EventSeq::new(next_seq),
        attempt: 1,
        reason,
    };
    journal.append_journaled(&event)
}


fn cmd_cancel(
    run_id: &str,
    db: &std::path::Path,
    reason: Option<String>,
    output: OutputFormat,
) -> ExitCode {
    let rid = match parse_run_id(run_id, output) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            report_storage_open_error(&e, db, output);
            return CliExitCode::StorageError.into();
        }
    };

    let events = match journal.events_for_run(rid) {
        Ok(ev) => ev,
        Err(e) => {
            let message = format!("error reading run {run_id}: {e}");
            if output != OutputFormat::Text {
                write_failure_message(&message, output, CliExitCode::StorageError);
            } else {
                errln!("{message}");
            }
            return CliExitCode::StorageError.into();
        }
    };

    // Idempotent: no events means run never existed.
    if events.is_empty() {
        return format_cancel_output(
            run_id,
            reason.as_deref(),
            "run not found, idempotent",
            output,
        );
    }

    // Idempotent: already terminal.
    if run_is_terminal(&events) {
        return format_cancel_output(
            run_id,
            reason.as_deref(),
            "already terminal, idempotent",
            output,
        );
    }

    if let Err(e) = write_cancel_event(&journal, rid, reason.clone(), &events) {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": format!("error writing cancel event: {e}")
                }),
                output,
            );
        } else {
            errln!("error writing cancel event: {e}");
        }
        return CliExitCode::StorageError.into();
    }

    format_cancel_output(run_id, reason.as_deref(), "cancelled", output)
}

