//! Module: doctor_helpers

use crate::app_impl::prelude::*;

pub(crate) fn cmd_doctor_without_db(output: OutputFormat) -> ExitCode {
    let remediation = "rerun with `doctor --db <path>` to verify Fjall journal storage";
    let checks = vec![serde_json::json!({
        "check": "database_path",
        "status": "skip",
        "category": "missing_db",
        "message": "no --db <path> provided; persistent journal checks skipped",
        "remediation": remediation
    })];

    if output != OutputFormat::Text {
        emit_json_or_return!(
            &serde_json::json!({
                "success": true,
                "mode": "stateless",
                "category": "missing_db",
                "checks": checks,
                "remediation": remediation
            }),
            output,
        );
    } else {
        outln!("doctor: no --db <path> provided; persistent journal checks skipped");
        outln!("doctor: {remediation}");
    }

    ExitCode::SUCCESS
}
