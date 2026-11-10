#![allow(unreachable_pub)]
#![forbid(unsafe_code)]

//! Contract-discovery: walk contracts/, validate schema_version + kind, run cue vet,
//! produce a DiscoveryReport and GateEvidence.

pub mod discovery;
pub mod evidence;
pub mod model;
pub mod version;

/// Re-export the GateEvidence, GateStatus, and WhyFailed types from the evidence module.
pub use crate::evidence::{GateEvidence, GateStatus, WhyFailed};
pub use discovery::discover_contracts;
#[cfg(test)]
pub(crate) use discovery::{extract_kind, extract_schema_version, is_contract_discovery_candidate};
pub use evidence::gate_evidence_from_report;
pub use model::{
    ContractError, ContractFile, ContractKind, DiscoveryReport, ReportSummary, VersionViolation,
};
pub use version::{
    SemverCmp, compare_semver, parse_schema_version, parse_vet_exit_code, run_cue_vet,
};

#[cfg(test)]
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

/// Handle the `contracts` xtask command.
pub fn cmd_contracts(dir: &str, json: bool, check: bool) -> anyhow::Result<()> {
    let contracts_dir = Path::new(dir);
    let report = discover_contracts(contracts_dir).map_err(|e| anyhow::anyhow!("{}", e))?;

    let _evidence = gate_evidence_from_report(&report);

    if json {
        let output = serde_json::to_string_pretty(&report)?;
        write_stdout(format_args!("{output}"))?;
    } else {
        write_stdout(format_args!(
            "contracts: {} total, {} valid, {} invalid",
            report.summary.total, report.summary.valid, report.summary.invalid
        ))?;
        if !report.errors.is_empty() {
            write_stdout(format_args!("Errors:"))?;
            for error in &report.errors {
                write_stdout(format_args!("  {error}"))?;
            }
        }
    }

    if check && report.summary.invalid > 0 {
        anyhow::bail!("{} contract(s) failed validation", report.summary.invalid);
    }

    Ok(())
}

fn write_stdout(args: std::fmt::Arguments<'_>) -> anyhow::Result<()> {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    handle
        .write_fmt(args)
        .map_err(|error| anyhow::anyhow!("Failed to write to stdout: {error}"))?;
    handle
        .write_all(b"\n")
        .map_err(|error| anyhow::anyhow!("Failed to write newline to stdout: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_schema_version_valid() {
        assert!(parse_schema_version("1.0.0").is_ok());
        assert!(parse_schema_version("0.1.0").is_ok());
        assert!(parse_schema_version("999.999.999").is_ok());
    }

    #[test]
    fn test_parse_schema_version_empty() {
        assert!(matches!(
            parse_schema_version(""),
            Err(ContractError::MissingSchemaVersion)
        ));
    }

    #[test]
    fn test_parse_schema_version_malformed() {
        assert!(matches!(
            parse_schema_version("1.0"),
            Err(ContractError::InvalidVersion { .. })
        ));
        assert!(matches!(
            parse_schema_version("1.0.0.0"),
            Err(ContractError::InvalidVersion { .. })
        ));
        assert!(matches!(
            parse_schema_version("abc"),
            Err(ContractError::InvalidVersion { .. })
        ));
        assert!(matches!(
            parse_schema_version("01.0.0"),
            Err(ContractError::InvalidVersion { .. })
        ));
    }

    #[test]
    fn test_parse_contract_kind_all() {
        for kind in ContractKind::all_values() {
            let s = kind.to_string();
            assert_eq!(ContractKind::parse(&s), Ok(*kind));
        }
    }

    #[test]
    fn test_parse_contract_kind_invalid() {
        assert!(ContractKind::parse("bogus").is_err());
        assert!(ContractKind::parse("").is_err());
    }

    #[test]
    fn test_compare_semver_equal() {
        assert_eq!(compare_semver("1.0.0", "1.0.0"), Ok(SemverCmp::Equal));
    }

    #[test]
    fn test_compare_semver_less() {
        assert_eq!(compare_semver("1.0.0", "2.0.0"), Ok(SemverCmp::Less));
        assert_eq!(compare_semver("1.0.0", "1.1.0"), Ok(SemverCmp::Less));
        assert_eq!(compare_semver("1.0.0", "1.0.1"), Ok(SemverCmp::Less));
    }

    #[test]
    fn test_compare_semver_greater() {
        assert_eq!(compare_semver("2.0.0", "1.0.0"), Ok(SemverCmp::Greater));
    }

    #[test]
    fn test_parse_vet_exit_code_zero() {
        assert!(parse_vet_exit_code(0).is_ok());
    }

    #[test]
    fn test_parse_vet_exit_code_nonzero() {
        assert!(parse_vet_exit_code(1).is_err());
        assert!(parse_vet_exit_code(-1).is_err());
    }

    #[test]
    fn test_gate_evidence_pass() {
        let report = DiscoveryReport {
            files: Vec::new(),
            errors: Vec::new(),
            summary: ReportSummary::new(),
        };
        let evidence = gate_evidence_from_report(&report);
        assert!(matches!(evidence.status, GateStatus::Pass));
        assert_eq!(evidence.exit_code, 0);
        assert!(evidence.why_failed.is_none());
    }

    #[test]
    fn test_gate_evidence_fail() {
        let report = DiscoveryReport {
            files: Vec::new(),
            errors: vec!["INVALID_KIND: bogus".to_string()],
            summary: ReportSummary {
                total: 1,
                valid: 0,
                invalid: 1,
                errors_by_kind: BTreeMap::from_iter(vec![("INVALID_KIND: bogus".to_string(), 1)]),
                version_violations: Vec::new(),
            },
        };
        let evidence = gate_evidence_from_report(&report);
        assert!(matches!(evidence.status, GateStatus::Fail));
        assert_eq!(evidence.exit_code, 1);
        assert!(evidence.why_failed.is_some());
    }

    #[test]
    fn test_report_summary_parity() {
        let summary = ReportSummary {
            total: 5,
            valid: 3,
            invalid: 2,
            errors_by_kind: BTreeMap::new(),
            version_violations: Vec::new(),
        };
        assert_eq!(summary.total, summary.valid + summary.invalid);
    }

    #[test]
    fn test_contract_kind_display() {
        assert_eq!(ContractKind::CliEnvelope.to_string(), "cli_envelope");
        assert_eq!(ContractKind::UiTokens.to_string(), "ui_tokens");
        assert_eq!(
            ContractKind::AcceptedArtifacts.to_string(),
            "accepted_artifacts"
        );
        assert_eq!(ContractKind::EvidenceBundle.to_string(), "evidence_bundle");
        assert_eq!(ContractKind::Diagnostics.to_string(), "diagnostics");
        assert_eq!(ContractKind::GateOutput.to_string(), "gate_output");
    }

    #[test]
    fn test_extract_kind_found() {
        let content = r#"kind: "cli_envelope"
schema_version: "1.0.0""#;
        assert_eq!(extract_kind(content), Some("cli_envelope".to_string()));
    }

    #[test]
    fn test_extract_kind_not_found() {
        let content = r#"package validation"#;
        assert_eq!(extract_kind(content), None);
    }

    #[test]
    fn test_extract_schema_version_found() {
        let content = r#"kind: "cli_envelope"
schema_version: "1.0.0""#;
        assert_eq!(extract_schema_version(content), "1.0.0");
    }

    #[test]
    fn test_extract_schema_version_not_found() {
        let content = r#"package validation"#;
        assert_eq!(extract_schema_version(content), "");
    }

    #[test]
    fn test_discover_nonexistent_dir() {
        let result = discover_contracts(Path::new("/tmp/does_not_exist_xyz"));
        assert!(result.is_err());
    }

    #[test]
    fn test_manifest_is_not_a_contract_discovery_candidate() {
        assert!(!is_contract_discovery_candidate(Path::new("manifest.cue")));
        assert!(is_contract_discovery_candidate(Path::new(
            "cli_envelope.cue"
        )));
    }

    #[test]
    fn test_contract_error_display() {
        assert_eq!(
            ContractError::MissingSchemaVersion.to_string(),
            "MISSING_SCHEMA_VERSION"
        );
        assert!(
            ContractError::InvalidKind {
                kind: "bogus".to_string(),
            }
            .to_string()
            .contains("INVALID_KIND")
        );
    }
}
