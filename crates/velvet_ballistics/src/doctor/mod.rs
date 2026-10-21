//! Module: doctor
#![forbid(unsafe_code)]

mod trim_check;

use crate::app_impl::prelude::*;
use crate::doctor_helpers::cmd_doctor_without_db;

use trim_check::{print_trim_summary_text, run_trim_eligibility_check};

pub(crate) fn open_doctor_journal(
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

pub(crate) fn cmd_doctor(db: Option<&std::path::Path>, output: OutputFormat) -> ExitCode {
    let Some(db) = db else {
        return cmd_doctor_without_db(output);
    };

    let mut checks = Vec::new();

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
            return report_doctor_failure(
                serde_json::json!({
                    "check": "open_journal",
                    "status": "fail",
                    "message": format!("cannot open journal at {}: {e}", db.display())
                }),
                format!("FAIL: cannot open journal at {}: {e}", db.display()),
                checks,
                output,
            );
        }
    };

    // Check 2: can we persist?
    if let Err(e) = journal.persist_strict() {
        return report_doctor_failure(
            serde_json::json!({
                "check": "strict_persist",
                "status": "fail",
                "message": format!("strict persist failed: {e}")
            }),
            format!("FAIL: strict persist failed: {e}"),
            checks,
            output,
        );
    }
    checks.push(serde_json::json!({
        "check": "strict_persist",
        "status": "pass",
        "message": "strict persist succeeded"
    }));

    // Check 3: can we write and read back an event?
    let test_run = vb_core::RunId::new(unique_doctor_run_id());
    let test_event = vb_storage::JournalEvent::RunAccepted {
        run: test_run,
        seq: vb_storage::EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0xAB; 32]),
    };

    if let Err(e) = journal.append_journaled(&test_event) {
        return report_doctor_failure(
            serde_json::json!({
                "check": "append_event",
                "status": "fail",
                "message": format!("cannot append test event: {e}")
            }),
            format!("FAIL: cannot append test event: {e}"),
            checks,
            output,
        );
    }
    checks.push(serde_json::json!({
        "check": "append_event",
        "status": "pass",
        "message": "journal append succeeded"
    }));

    if let Err(failure) = check_events_for_run(&journal, test_run) {
        return report_doctor_failure(failure.entry, failure.text_line, checks, output);
    }
    checks.push(serde_json::json!({
        "check": "read_back_event",
        "status": "pass",
        "message": "journal read-back succeeded"
    }));

    // Check 4: trim eligibility diagnostic (non-destructive)
    match run_trim_eligibility_check(&journal) {
        Ok(entry) => checks.push(entry),
        Err(failure) => {
            return report_doctor_failure(failure.entry, failure.text_line, checks, output);
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
        print_trim_summary_text(&journal);
        outln!("doctor: all checks passed");
    }
    ExitCode::SUCCESS
}

struct DoctorFailure {
    entry: serde_json::Value,
    text_line: String,
}

fn report_doctor_failure(
    entry: serde_json::Value,
    text_line: String,
    mut checks: Vec<serde_json::Value>,
    output: OutputFormat,
) -> ExitCode {
    checks.push(entry);
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({
                "success": false,
                "checks": checks
            }),
            output,
        );
    } else {
        errln!("{text_line}");
    }
    CliExitCode::StorageError.into()
}

fn check_events_for_run(
    journal: &vb_storage::FjallJournal,
    test_run: vb_core::RunId,
) -> Result<(), DoctorFailure> {
    match journal.events_for_run(test_run) {
        Ok(events) => {
            if events.is_empty() {
                Err(DoctorFailure {
                    entry: serde_json::json!({
                        "check": "read_back_event",
                        "status": "fail",
                        "message": "test event not found after append"
                    }),
                    text_line: "FAIL: test event not found after append".to_owned(),
                })
            } else {
                Ok(())
            }
        }
        Err(e) => Err(DoctorFailure {
            entry: serde_json::json!({
                "check": "read_back_event",
                "status": "fail",
                "message": format!("cannot read test run events: {e}")
            }),
            text_line: format!("FAIL: cannot read test run events: {e}"),
        }),
    }
}
