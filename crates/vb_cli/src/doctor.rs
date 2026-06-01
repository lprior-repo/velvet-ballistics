#![forbid(unsafe_code)]
//! Diagnostic check command.

fn open_doctor_journal(
    db: &std::path::Path,
) -> Result<vb_storage::FjallJournal, vb_storage::JournalError> {
    for delay in [
        std::time::Duration::from_millis(5),
        std::time::Duration::from_millis(25),
    ] {
        match vb_storage::FjallJournal::open(db, None) {
            Ok(journal) => return Ok(journal),
            Err(vb_storage::JournalError::ProcessLockHeld { .. }) => std::thread::sleep(delay),
            Err(err) => return Err(err),
        }
    }

    vb_storage::FjallJournal::open(db, None)
}


fn cmd_doctor(db: Option<&std::path::Path>, output: OutputFormat) -> ExitCode {
    let Some(db) = db else {
        return cmd_doctor_without_db(output);
    };

    let mut checks = Vec::new();
    let _success = true;

    // Check 1: can we open the journal?
    let journal = match open_doctor_journal(db) {
        Ok(j) => {
            checks.push(serde_json::json!({
                "check": "open_journal",
                "status": "pass",
                "message": format!("journal opened at {}", db.display())
            }));
            j
        }
        Err(e) => {
            checks.push(serde_json::json!({
                "check": "open_journal",
                "status": "fail",
                "message": format!("cannot open journal at {}: {e}", db.display())
            }));
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "checks": checks
                    }),
                    output,
                );
            } else {
                errln!("FAIL: cannot open journal at {}: {e}", db.display());
            }
            return CliExitCode::StorageError.into();
        }
    };

    // Check 2: can we persist?
    match journal.persist_strict() {
        Ok(()) => {
            checks.push(serde_json::json!({
                "check": "strict_persist",
                "status": "pass",
                "message": "strict persist succeeded"
            }));
        }
        Err(e) => {
            checks.push(serde_json::json!({
                "check": "strict_persist",
                "status": "fail",
                "message": format!("strict persist failed: {e}")
            }));
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "checks": checks
                    }),
                    output,
                );
            } else {
                errln!("FAIL: strict persist failed: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    }

    // Check 3: can we write and read back an event?
    let test_run = vb_core::RunId::new(unique_doctor_run_id());
    let test_event = vb_storage::JournalEvent::RunAccepted {
        run: test_run,
        seq: vb_storage::EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0xAB; 32]),
    };

    if let Err(e) = journal.append_journaled(&test_event) {
        checks.push(serde_json::json!({
            "check": "append_event",
            "status": "fail",
            "message": format!("cannot append test event: {e}")
        }));
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "checks": checks
                }),
                output,
            );
        } else {
            errln!("FAIL: cannot append test event: {e}");
        }
        return CliExitCode::StorageError.into();
    }
    checks.push(serde_json::json!({
        "check": "append_event",
        "status": "pass",
        "message": "journal append succeeded"
    }));

    match journal.events_for_run(test_run) {
        Ok(events) => {
            if events.is_empty() {
                checks.push(serde_json::json!({
                    "check": "read_back_event",
                    "status": "fail",
                    "message": "test event not found after append"
                }));
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "checks": checks
                        }),
                        output,
                    );
                } else {
                    errln!("FAIL: test event not found after append");
                }
                return CliExitCode::StorageError.into();
            }
            checks.push(serde_json::json!({
                "check": "read_back_event",
                "status": "pass",
                "message": format!("journal read-back returned {} event(s)", events.len())
            }));
        }
        Err(e) => {
            checks.push(serde_json::json!({
                "check": "read_back_event",
                "status": "fail",
                "message": format!("cannot read test run events: {e}")
            }));
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "checks": checks
                    }),
                    output,
                );
            } else {
                errln!("FAIL: cannot read test run events: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    }

    // Check 4: trim eligibility diagnostic (non-destructive)
    match journal.trim_eligibility_diagnostic(vb_storage::TrimPolicy::default()) {
        Ok(diag) => {
            let mut runs = Vec::new();
            for run in &diag.runs {
                match run {
                    vb_storage::TrimEligibility::Eligible {
                        run: r,
                        safe_point,
                        events_trimmable,
                    } => {
                        runs.push(serde_json::json!({
                            "run": r.get(),
                            "status": "eligible",
                            "safe_point": safe_point.get(),
                            "events_trimmable": events_trimmable
                        }));
                    }
                    vb_storage::TrimEligibility::Blocked { run: r, blocker } => {
                        let blocker_name = match blocker {
                            vb_storage::TrimBlocker::NoDurableSnapshot => "no_durable_snapshot",
                            vb_storage::TrimBlocker::RetentionPolicy { .. } => "retention_policy",
                            _ => "unknown",
                        };
                        runs.push(serde_json::json!({
                            "run": r.get(),
                            "status": "blocked",
                            "blocker": blocker_name
                        }));
                    }
                    _ => {
                        runs.push(serde_json::json!({
                            "status": "unknown"
                        }));
                    }
                }
            }
            checks.push(serde_json::json!({
                "check": "trim_eligibility",
                "status": "pass",
                "message": format!(
                    "trim eligibility: {} total, {} eligible, {} blocked, {} events trimmable",
                    diag.total_runs, diag.eligible_runs, diag.blocked_runs, diag.total_events_trimmable
                ),
                "total_runs": diag.total_runs,
                "eligible_runs": diag.eligible_runs,
                "blocked_runs": diag.blocked_runs,
                "total_events_trimmable": diag.total_events_trimmable,
                "runs": runs
            }));
        }
        Err(e) => {
            checks.push(serde_json::json!({
                "check": "trim_eligibility",
                "status": "fail",
                "message": format!("trim eligibility diagnostic failed: {e}")
            }));
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "checks": checks
                    }),
                    output,
                );
            } else {
                errln!("FAIL: trim eligibility diagnostic failed: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    }

    checks.push(serde_json::json!({
        "check": "all",
        "status": "pass",
        "message": "all checks passed"
    }));

    if output != OutputFormat::Text {
        emit_json_or_return!(
            &serde_json::json!({
                "success": true,
                "checks": checks
            }),
            output,
        );
    } else {
        // Print trim eligibility summary in text mode
        if let Ok(diag) = journal.trim_eligibility_diagnostic(vb_storage::TrimPolicy::default()) {
            outln!(
                "doctor: trim eligibility — {} total, {} eligible, {} blocked, {} events trimmable",
                diag.total_runs,
                diag.eligible_runs,
                diag.blocked_runs,
                diag.total_events_trimmable
            );
            for run in &diag.runs {
                match run {
                    vb_storage::TrimEligibility::Eligible {
                        run: r,
                        safe_point,
                        events_trimmable,
                    } => {
                        outln!(
                            "doctor:   run {} eligible — safe_point={} events_trimmable={}",
                            r.get(),
                            safe_point.get(),
                            events_trimmable
                        );
                    }
                    vb_storage::TrimEligibility::Blocked { run: r, blocker } => {
                        let blocker_name = match blocker {
                            vb_storage::TrimBlocker::NoDurableSnapshot => "no_durable_snapshot",
                            vb_storage::TrimBlocker::RetentionPolicy { .. } => "retention_policy",
                            _ => "unknown",
                        };
                        outln!(
                            "doctor:   run {} blocked — blocker={}",
                            r.get(),
                            blocker_name
                        );
                    }
                    _ => {
                        outln!("doctor:   unknown trim eligibility");
                    }
                }
            }
        }
        outln!("doctor: all checks passed");
    }
    ExitCode::SUCCESS
}


fn unique_doctor_run_id() -> u64 {
    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return u64::MAX;
    };
    match u64::try_from(now.as_nanos()) {
        Ok(value) => value,
        Err(_) => now.as_secs(),
    }
}

// --- Helpers ---

