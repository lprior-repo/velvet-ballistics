//! Trim eligibility check extracted from `cmd_doctor` so the file stays under
//! the 300-line first-party source-length cap.
#![forbid(unsafe_code)]

/// JSON check entry for a successful trim eligibility diagnostic.
pub(crate) fn run_trim_eligibility_check(
    journal: &vb_storage::FjallJournal,
) -> Result<serde_json::Value, TrimCheckFailure> {
    let diag = journal
        .trim_eligibility_diagnostic(vb_storage::TrimPolicy::default())
        .map_err(|e| TrimCheckFailure {
            entry: serde_json::json!({
                "check": "trim_eligibility",
                "status": "fail",
                "message": format!("trim eligibility diagnostic failed: {e}")
            }),
            text_line: format!("FAIL: trim eligibility diagnostic failed: {e}"),
        })?;

    let runs = build_trim_runs_json(&diag);
    Ok(serde_json::json!({
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
    }))
}

/// Serialize each `TrimEligibility` entry into a JSON value.
pub(crate) fn build_trim_runs_json(diag: &vb_storage::TrimDiagnostic) -> Vec<serde_json::Value> {
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
    runs
}

/// Print the per-run trim eligibility summary in text mode.
pub(crate) fn print_trim_summary_text(journal: &vb_storage::FjallJournal) {
    let Ok(diag) = journal.trim_eligibility_diagnostic(vb_storage::TrimPolicy::default()) else {
        return;
    };
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

/// Doctor check failure payload.
pub(crate) struct TrimCheckFailure {
    pub entry: serde_json::Value,
    pub text_line: String,
}
