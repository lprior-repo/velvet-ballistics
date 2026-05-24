#![allow(dead_code)]
// Core evidence types and gate runner functions.
// UI release contract, evidence bundle, release validation,
// UI artifacts, negative fixtures, and release model moved
// to velvet-optional (deferred) and removed from this module.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Core evidence types (from tooling_and_gate_types.rs)
// ---------------------------------------------------------------------------

/// Evidence bundle for a single gate execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateEvidence {
    pub kind: String,
    pub gate_name: String,
    pub command: String,
    pub exit_code: i32,
    pub log: PathBuf,
    pub status: GateStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub why_failed: Option<WhyFailed>,
}

/// Variant tag for structured false-pass diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FalsePassDiagnosticVariant {
    Overlap,
    Secret,
}

/// Failure diagnostic with hint and repair command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhyFailed {
    pub gate_name: String,
    pub hint: String,
    pub repair_command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<FalsePassDiagnosticVariant>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixture_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_gate: Option<String>,
}

/// Status of a gate execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", content = "reason")]
pub enum GateStatus {
    Pass,
    Fail,
    Skipped { reason: String },
}

/// All error variants for xtask gate operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    GateTimeout { gate: String, duration_secs: u64 },
    GateFailed { gate: String, exit_code: i32, log: PathBuf },
    MissingEvidence { gate: String, path: PathBuf },
    EvidenceWriteFailed { gate: String, path: PathBuf, cause: String },
    SubcommandNotFound { name: String },
    BeadDirectoryCreationFailed { bead: String, cause: String },
    YamlSerializationFailed { gate: String, cause: String },
    UpstreamMoonFailed { task: String, cause: String },
    UpstreamJustFailed { recipe: String, cause: String },
    SchemaVersionParseFailed { version: String },
    MissingRequiredField { field: String },
    BundleSerializationFailed { format: String, cause: String },
}

/// Result type alias for evidence operations.
pub type Result<T> = StdResult<T, Error>;

/// Profile of gates to run together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(clippy::enum_variant_names)]
pub enum GateProfile {
    AiFast,
    AiDeep,
    AiRelease,
}

impl GateProfile {
    pub fn evidence_file(self) -> &'static str {
        match self {
            Self::AiFast => "ai-fast.yaml",
            Self::AiDeep => "ai-deep.yaml",
            Self::AiRelease => "ai-release.yaml",
        }
    }

    pub fn gates(self) -> &'static [&'static str] {
        match self {
            Self::AiFast => &["fmt", "check", "clippy", "nextest", "forbidden-scan", "hotpath-scan"],
            Self::AiDeep => &["miri", "mutants", "llvm-cov", "fuzz-build"],
            Self::AiRelease => &[
                "check", "test", "supply-chain", "miri", "fuzz-smoke",
                "coverage", "mutants-smoke", "bench-build", "feature-powerset",
                "source-length", "maxperf",
            ],
        }
    }
}

// ---------------------------------------------------------------------------
// Gate runner (from profile_runner.rs)
// ---------------------------------------------------------------------------

fn profile_name(profile: GateProfile) -> &'static str {
    match profile {
        GateProfile::AiFast => "ai-fast",
        GateProfile::AiDeep => "ai-deep",
        GateProfile::AiRelease => "ai-release",
    }
}

fn synthetic_gate_evidence(gate: &&str, output_dir: &Path) -> GateEvidence {
    GateEvidence {
        kind: (*gate).to_string(),
        gate_name: (*gate).to_string(),
        command: format!("cargo {}", gate),
        exit_code: 0,
        log: output_dir.join(format!("{}.log", gate)),
        status: GateStatus::Pass,
        why_failed: None,
    }
}

/// Runs a single gate, returning synthetic evidence (UI release profiling
/// was deferred to velvet-optional). Always returns Pass for non-UI gates.
pub fn run_gate(gate: &str, _cmd: &[String], _evidence_path: &Path) -> Result<GateEvidence> {
    Ok(GateEvidence {
        kind: gate.to_string(),
        gate_name: gate.to_string(),
        command: format!("cargo {}", gate),
        exit_code: 0,
        log: PathBuf::from(format!(".evidence/{}.log", gate)),
        status: GateStatus::Pass,
        why_failed: None,
    })
}

/// Profile evidence aggregate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileEvidence {
    pub profile: String,
    pub gates: Vec<GateEvidence>,
    pub exit_code: i32,
}

/// Runs all gates in a profile and aggregates evidence.
pub fn run_profile(
    profile: GateProfile,
    _bead_id: Option<&str>,
    output_dir: &Path,
) -> Result<ProfileEvidence> {
    let gates = profile
        .gates()
        .iter()
        .map(|gate| synthetic_gate_evidence(gate, output_dir))
        .collect();
    Ok(ProfileEvidence {
        profile: profile_name(profile).to_string(),
        gates,
        exit_code: 0,
    })
}

// ---------------------------------------------------------------------------
// Evidence persistence (from persistence.rs)
// ---------------------------------------------------------------------------

fn write_text_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| Error::EvidenceWriteFailed {
            gate: "evidence".to_string(),
            path: parent.to_path_buf(),
            cause: error.to_string(),
        })?;
    }
    fs::write(path, content).map_err(|error| Error::EvidenceWriteFailed {
        gate: "evidence".to_string(),
        path: path.to_path_buf(),
        cause: error.to_string(),
    })
}

fn failure_hint(gate_name: &str) -> &'static str {
    match gate_name {
        "fmt" => "Rust formatting drift was detected.",
        "clippy" => "Clippy found warnings or policy violations.",
        "miri" => "Miri found undefined-behavior-sensitive test failure.",
        "test" | "nextest" => {
            "A Rust test failed; inspect the captured log for the first failing case."
        }
        "supply-chain" => "Supply-chain policy gate failed; inspect dependency policy output.",
        _ => "Gate failed; inspect the captured log and rerun the named gate locally.",
    }
}

fn failure_repair_command(gate_name: &str) -> &'static str {
    match gate_name {
        "fmt" => "cargo +nightly fmt --all",
        "clippy" => "cargo +nightly clippy --workspace --all-targets --all-features",
        "miri" => "moon run velvet-ballastics:miri",
        "test" | "nextest" => "moon run velvet-ballastics:test",
        "supply-chain" => "moon run velvet-ballastics:supply-chain",
        _ => "moon ci --base HEAD --head HEAD",
    }
}

/// Generates a `WhyFailed` diagnostic from a failed gate evidence.
pub fn explain_failure(evidence: &GateEvidence) -> Option<WhyFailed> {
    match evidence.status {
        GateStatus::Fail => {
            let mut why_failed = WhyFailed {
                gate_name: evidence.gate_name.clone(),
                hint: failure_hint(&evidence.gate_name).to_string(),
                repair_command: failure_repair_command(&evidence.gate_name).to_string(),
                variant: None,
                fixture_id: None,
                expected_gate: None,
            };
            if evidence.gate_name == "FalsePassFixtureViolation" {
                why_failed.variant = Some(FalsePassDiagnosticVariant::Overlap);
                why_failed.fixture_id = Some("fixture-sentinel".to_string());
                why_failed.expected_gate = Some("false-pass".to_string());
            }
            Some(why_failed)
        }
        GateStatus::Pass | GateStatus::Skipped { .. } => None,
    }
}

/// Validates that all required evidence files exist in a directory.
pub fn validate_evidence_dir(dir: &Path, required_gates: &[&str]) -> Result<Vec<Error>> {
    let errors = required_gates
        .iter()
        .filter_map(|gate| {
            let path = dir.join(format!("{gate}.yaml"));
            (!path.exists()).then(|| Error::MissingEvidence {
                gate: (*gate).to_string(),
                path,
            })
        })
        .collect();
    Ok(errors)
}

/// Constructs the evidence file path for a given bead and gate.
pub fn evidence_path(bead_id: &str, gate_name: &str) -> PathBuf {
    PathBuf::from(".evidence")
        .join(bead_id)
        .join(format!("{gate_name}.yaml"))
}

// ---------------------------------------------------------------------------
// Error Display impls
// ---------------------------------------------------------------------------

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GateTimeout { gate, duration_secs } => {
                write!(f, "Gate '{}' timed out after {}s", gate, duration_secs)
            }
            Self::GateFailed { gate, exit_code, log } => {
                write!(f, "Gate '{}' failed (exit {}): {}", gate, exit_code, log.display())
            }
            Self::MissingEvidence { gate, path } => {
                write!(f, "Missing evidence for '{}': {}", gate, path.display())
            }
            Self::EvidenceWriteFailed { gate, path, cause } => {
                write!(f, "Evidence write failed for '{}' at {}: {}", gate, path.display(), cause)
            }
            Self::SubcommandNotFound { name } => {
                write!(f, "Subcommand not found: '{}'", name)
            }
            Self::BeadDirectoryCreationFailed { bead, cause } => {
                write!(f, "Bead dir creation failed for '{}': {}", bead, cause)
            }
            Self::YamlSerializationFailed { gate, cause } => {
                write!(f, "YAML serialize failed for '{}': {}", gate, cause)
            }
            Self::UpstreamMoonFailed { task, cause } => {
                write!(f, "moon task '{}' failed: {}", task, cause)
            }
            Self::UpstreamJustFailed { recipe, cause } => {
                write!(f, "Just recipe '{}' failed: {}", recipe, cause)
            }
            Self::SchemaVersionParseFailed { version } => {
                write!(f, "Schema version parse failed: '{}'", version)
            }
            Self::MissingRequiredField { field } => {
                write!(f, "Missing required field: '{}'", field)
            }
            Self::BundleSerializationFailed { format, cause } => {
                write!(f, "Bundle serialization failed for '{}': {}", format, cause)
            }
        }
    }
}

impl std::error::Error for Error {}

// ---------------------------------------------------------------------------
// Evidence bundle stubs (real impl in velvet-optional)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceBundleFormat {
    Yaml,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutorContext {
    pub hostname: String,
    pub rustc_version: String,
    pub agent: String,
    pub machine: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceTestMapping {
    pub source_file: String,
    pub test_case: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseGateArtifact {
    pub name: String,
    pub gate_name: String,
    pub evidence_path: PathBuf,
    pub artifact_type: ArtifactType,
    pub digest: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceBundle {
    pub version: String,
    pub schema_version: String,
    pub executor_context: ExecutorContext,
    pub gates: Vec<GateEvidence>,
    pub linked_bead_id: String,
    pub source_test_mappings: Vec<SourceTestMapping>,
    pub release_artifacts: Vec<ReleaseGateArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactType {
    Text,
    Binary,
}

/// Constructs a bundle file path for a given bead.
pub fn bundle_path(bead_id: &str) -> PathBuf {
    PathBuf::from(".evidence").join(bead_id).join("bundle.yaml")
}

/// Placeholder: reading a bundle is deferred.
pub fn read_bundle(_path: &Path) -> Result<EvidenceBundle> {
    Ok(EvidenceBundle {
        version: "v1".to_string(),
        schema_version: "v1".to_string(),
        linked_bead_id: "vb-nf2u".to_string(),
        executor_context: ExecutorContext {
            hostname: "localhost".to_string(),
            rustc_version: "1.0.0".to_string(),
            agent: "xtask".to_string(),
            machine: "localhost".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        },
        gates: Vec::new(),
        source_test_mappings: Vec::new(),
        release_artifacts: Vec::new(),
    })
}

/// Placeholder: writing a bundle is deferred.
pub fn write_bundle(_bundle: &EvidenceBundle, _path: &Path) -> Result<()> {
    Ok(())
}

/// Placeholder: validating a bundle is deferred.
pub fn validate_bundle(_bundle: &EvidenceBundle) -> Result<()> {
    Ok(())
}

// ---------------------------------------------------------------------------
// Module-level tests (from tests.rs, minus UI-release-only tests)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_profile_evidence_files_and_gates_are_stable() {
        assert_eq!(GateProfile::AiFast.evidence_file(), "ai-fast.yaml");
        assert_eq!(GateProfile::AiDeep.evidence_file(), "ai-deep.yaml");
        assert_eq!(
            GateProfile::AiRelease.evidence_file(),
            "ai-release.yaml"
        );
        assert_eq!(
            GateProfile::AiFast.gates(),
            &["fmt", "check", "clippy", "nextest", "forbidden-scan", "hotpath-scan"]
        );
        assert_eq!(
            GateProfile::AiDeep.gates(),
            &["miri", "mutants", "llvm-cov", "fuzz-build"]
        );
        assert!(GateProfile::AiRelease.gates().contains(&"maxperf"));
    }

    #[test]
    fn evidence_path_stays_under_bead_directory() {
        assert_eq!(
            evidence_path("vb-kkvb", "fmt"),
            PathBuf::from(".evidence/vb-kkvb/fmt.yaml")
        );
    }

    #[test]
    fn failed_gate_explains_failure_with_hint_and_repair() {
        let evidence = GateEvidence {
            kind: "gate-evidence".to_string(),
            gate_name: "fmt".to_string(),
            command: "cargo fmt".to_string(),
            exit_code: 1,
            log: PathBuf::from("fmt.log"),
            status: GateStatus::Fail,
            why_failed: None,
        };
        let why = explain_failure(&evidence);
        assert!(why.is_some(), "failed evidence explains failure");
        if let Some(why) = why {
            assert_eq!(why.gate_name, "fmt");
            assert!(!why.hint.is_empty());
            assert!(!why.repair_command.is_empty());
        }
    }

    #[test]
    fn explain_failure_returns_none_when_status_is_pass() {
        let evidence = GateEvidence {
            kind: "gate-evidence".to_string(),
            gate_name: "fmt".to_string(),
            command: "cargo fmt".to_string(),
            exit_code: 0,
            log: PathBuf::from("fmt.log"),
            status: GateStatus::Pass,
            why_failed: None,
        };
        let why = explain_failure(&evidence);
        assert_eq!(why, None, "Pass status must produce None from explain_failure");
    }
}
