#![allow(unreachable_pub)]
#![forbid(unsafe_code)]

use super::{DiscoveryReport, GateEvidence, GateStatus, WhyFailed};
use std::path::PathBuf;

/// Convert a DiscoveryReport into a GateEvidence record.
/// Always succeeds (OBL-006: Kani proof of non-panic).
pub fn gate_evidence_from_report(report: &DiscoveryReport) -> GateEvidence {
    let status = if report.summary.invalid == 0 {
        GateStatus::Pass
    } else {
        GateStatus::Fail
    };

    let exit_code = if report.summary.invalid == 0 { 0 } else { 1 };

    let why_failed = if report.summary.invalid > 0 {
        let mut unique = report.errors.to_vec();
        unique.sort();
        unique.dedup();
        let detail = if unique.is_empty() {
            format!("{} contract(s) failed validation", report.summary.invalid)
        } else {
            format!(
                "{} contract(s) failed: {}",
                report.summary.invalid,
                unique.join(", ")
            )
        };
        Some(WhyFailed {
            gate_name: "contracts".to_string(),
            hint: detail,
            repair_command: "cargo xtask contracts --check".to_string(),
            variant: None,
            fixture_id: None,
            expected_gate: None,
        })
    } else {
        None
    };

    GateEvidence {
        kind: "contract-discovery".to_string(),
        gate_name: "contracts".to_string(),
        command: "cargo xtask contracts --dir contracts".to_string(),
        exit_code,
        log: PathBuf::from(".evidence/contracts/last_run.log"),
        status,
        why_failed,
    }
}
