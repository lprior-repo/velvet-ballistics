#![allow(unreachable_pub)]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

/// Recognised contract kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractKind {
    CliEnvelope,
    UiTokens,
    AcceptedArtifacts,
    EvidenceBundle,
    Diagnostics,
    GateOutput,
}

impl ContractKind {
    /// All recognised values in ordinal order.
    pub const fn all_values() -> &'static [Self] {
        &[
            Self::CliEnvelope,
            Self::UiTokens,
            Self::AcceptedArtifacts,
            Self::EvidenceBundle,
            Self::Diagnostics,
            Self::GateOutput,
        ]
    }

    /// Stable wire spelling for this contract kind.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CliEnvelope => "cli_envelope",
            Self::UiTokens => "ui_tokens",
            Self::AcceptedArtifacts => "accepted_artifacts",
            Self::EvidenceBundle => "evidence_bundle",
            Self::Diagnostics => "diagnostics",
            Self::GateOutput => "gate_output",
        }
    }

    /// Convert a string slice into a recognised ContractKind, or the string
    /// itself when it is not recognised (for error reporting).
    pub fn parse(s: &str) -> Result<Self, String> {
        Self::all_values()
            .iter()
            .copied()
            .find(|kind| kind.as_str() == s)
            .ok_or_else(|| s.to_string())
    }
}

impl fmt::Display for ContractKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single contract file discovered under contracts/.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractFile {
    pub path: PathBuf,
    pub schema_version: String,
    pub kind: ContractKind,
    pub vet_errors: Vec<String>,
}

/// A version monotonicity breach.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionViolation {
    pub file: PathBuf,
    pub expected: String,
    pub actual: String,
    pub detail: String,
}

/// Summary counters produced by discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total: u32,
    pub valid: u32,
    pub invalid: u32,
    /// BTreeMap ensures deterministic JSON key order.
    pub errors_by_kind: BTreeMap<String, u32>,
    pub version_violations: Vec<VersionViolation>,
}

impl Default for ReportSummary {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportSummary {
    pub fn new() -> Self {
        Self {
            total: 0,
            valid: 0,
            invalid: 0,
            errors_by_kind: BTreeMap::new(),
            version_violations: Vec::new(),
        }
    }
}

/// Full discovery report.
///
/// errors is `Vec<String>` (not `Vec<ValidationError>`) because
/// `ValidationError` from `vb_validate` does not implement Serialize/Deserialize.
/// We store the Display representation instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryReport {
    pub files: Vec<ContractFile>,
    pub errors: Vec<String>,
    pub summary: ReportSummary,
}

/// Contract-discovery validation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractError {
    MissingSchemaVersion,
    InvalidVersion {
        version: String,
    },
    InvalidKind {
        kind: String,
    },
    CueVetFailed {
        file: String,
    },
    VersionMonotonicityBreach {
        file: String,
        expected: String,
        actual: String,
    },
}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSchemaVersion => write!(f, "MISSING_SCHEMA_VERSION"),
            Self::InvalidVersion { version } => write!(f, "INVALID_VERSION: {version}"),
            Self::InvalidKind { kind } => write!(f, "INVALID_KIND: {kind}"),
            Self::CueVetFailed { file } => write!(f, "CUE_VET_FAILED: {file}"),
            Self::VersionMonotonicityBreach {
                file,
                expected,
                actual,
            } => write!(
                f,
                "VERSION_MONOTONICITY_BREACH: {file} expected {expected} got {actual}"
            ),
        }
    }
}
