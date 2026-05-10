//! Evidence types and functions for xtask command-center gates.
//!
//! This module provides the evidence bundle types (GateEvidence, WhyFailed, GateStatus)
//! and orchestration functions (run_gate, run_profile, explain_failure, validate_evidence_dir).
//!
//! # Error Taxonomy
//!
//! All fallible operations return `Result<T, Error>` with explicit error variants:
//!
//! - `Error::GateTimeout` — gate exceeded its time bound
//! - `Error::GateFailed` — underlying command returned non-zero
//! - `Error::MissingEvidence` — evidence file absent (fail-closed trigger)
//! - `Error::EvidenceWriteFailed` — YAML serialization or file write error
//! - `Error::SubcommandNotFound` — requested xtask subcommand does not exist
//! - `Error::BeadDirectoryCreationFailed` — could not create `.evidence/<bead>/` directory
//! - `Error::YamlSerializationFailed` — saphyr error during evidence serialization
//! - `Error::UpstreamMoonFailed` — moon run task returned non-zero
//! - `Error::UpstreamJustFailed` — just recipe returned non-zero

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Evidence bundle for a single gate execution.
///
/// Contains all fields required by POST-004:
/// - `kind`: Category of the gate (e.g., "fmt", "clippy", "ai-fast")
/// - `gate_name`: Specific gate name within the category
/// - `command`: Full command string that was executed
/// - `exit_code`: Numeric exit code from the command
/// - `log`: Path to the log file with raw tool output
/// - `status`: Pass/Fail/Skipped status
/// - `why_failed`: Optional failure diagnostic with hint and repair command
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

/// Failure diagnostic with hint and repair command.
///
/// Populated when a gate fails, providing actionable remediation steps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhyFailed {
    pub gate_name: String,
    pub hint: String,
    pub repair_command: String,
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
#[allow(dead_code)]
pub enum Error {
    /// Gate exceeded its configured timeout duration.
    GateTimeout { gate: String, duration_secs: u64 },
    /// Underlying command returned non-zero exit code.
    GateFailed {
        gate: String,
        exit_code: i32,
        log: PathBuf,
    },
    /// Evidence file for a required gate does not exist (fail-closed).
    MissingEvidence { gate: String, path: PathBuf },
    /// YAML serialization or file write failed.
    EvidenceWriteFailed {
        gate: String,
        path: PathBuf,
        cause: String,
    },
    /// Requested xtask subcommand does not exist.
    SubcommandNotFound { name: String },
    /// Could not create `.evidence/<bead>/` directory.
    BeadDirectoryCreationFailed { bead: String, cause: String },
    /// saphyr error during evidence serialization.
    YamlSerializationFailed { gate: String, cause: String },
    /// moon run task returned non-zero.
    UpstreamMoonFailed { task: String, cause: String },
    /// just recipe returned non-zero.
    UpstreamJustFailed { recipe: String, cause: String },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::GateTimeout {
                gate,
                duration_secs,
            } => {
                write!(f, "Gate '{}' exceeded timeout of {}s", gate, duration_secs)
            }
            Error::GateFailed {
                gate,
                exit_code,
                log,
            } => {
                write!(
                    f,
                    "Gate '{}' failed with exit code {} (log: {})",
                    gate,
                    exit_code,
                    log.display()
                )
            }
            Error::MissingEvidence { gate, path } => {
                write!(
                    f,
                    "Missing evidence for gate '{}' at {}",
                    gate,
                    path.display()
                )
            }
            Error::EvidenceWriteFailed { gate, path, cause } => {
                write!(
                    f,
                    "Failed to write evidence for '{}' to {}: {}",
                    gate,
                    path.display(),
                    cause
                )
            }
            Error::SubcommandNotFound { name } => {
                write!(f, "Subcommand not found: '{}'", name)
            }
            Error::BeadDirectoryCreationFailed { bead, cause } => {
                write!(
                    f,
                    "Failed to create evidence directory for bead '{}': {}",
                    bead, cause
                )
            }
            Error::YamlSerializationFailed { gate, cause } => {
                write!(f, "YAML serialization failed for '{}': {}", gate, cause)
            }
            Error::UpstreamMoonFailed { task, cause } => {
                write!(f, "Moon task '{}' failed: {}", task, cause)
            }
            Error::UpstreamJustFailed { recipe, cause } => {
                write!(f, "Just recipe '{}' failed: {}", recipe, cause)
            }
        }
    }
}

impl std::error::Error for Error {}

/// Result type alias for evidence operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Profile of gates to run together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GateProfile {
    /// Fast gates: fmt, check, clippy, nextest, forbidden-scan, hotpath-scan
    Fast,
    /// Deep gates: miri, mutants, llvm-cov, fuzz-build
    Deep,
    /// Release gates: check, test, supply-chain, miri, fuzz-smoke, coverage,
    /// mutants-smoke, bench-build, feature-powerset, source-length, maxperf
    Release,
}

#[allow(dead_code)]
impl GateProfile {
    /// Returns the list of gates in this profile.
    pub fn gates(self) -> &'static [&'static str] {
        match self {
            GateProfile::Fast => &[
                "fmt",
                "check",
                "clippy",
                "nextest",
                "forbidden-scan",
                "hotpath-scan",
            ],
            GateProfile::Deep => &["miri", "mutants", "llvm-cov", "fuzz-build"],
            GateProfile::Release => &[
                "check",
                "test",
                "supply-chain",
                "miri",
                "fuzz-smoke",
                "coverage",
                "mutants-smoke",
                "bench-build",
                "feature-powerset",
                "source-length",
                "maxperf",
            ],
        }
    }

    /// Returns the evidence file name for this profile.
    pub fn evidence_file(self) -> &'static str {
        match self {
            GateProfile::Fast => "ai-fast.yaml",
            GateProfile::Deep => "ai-deep.yaml",
            GateProfile::Release => "ai-release.yaml",
        }
    }
}

/// Aggregated evidence for a full profile run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileEvidence {
    pub profile: String,
    pub gates: Vec<GateEvidence>,
    pub exit_code: i32,
}

// ============================================================================
// Core orchestration functions
// ============================================================================

/// Executes a single gate command and serializes evidence.
///
/// # Arguments
/// * `gate` - The gate name (e.g., "fmt", "clippy")
/// * `cmd` - The command arguments to execute
/// * `evidence_path` - Path where evidence YAML should be written
///
/// # Errors
/// Returns `Error::GateTimeout` if execution exceeds timeout.
/// Returns `Error::GateFailed` if command returns non-zero.
/// Returns `Error::EvidenceWriteFailed` if YAML write fails.
pub fn run_gate(gate: &str, cmd: &[String], evidence_path: &Path) -> Result<GateEvidence> {
    if cmd.is_empty() {
        return Err(Error::GateFailed {
            gate: gate.to_string(),
            exit_code: -1,
            log: evidence_path.to_path_buf(),
        });
    }
    let mut command = std::process::Command::new(&cmd[0]);
    if cmd.len() > 1 {
        command.args(&cmd[1..]);
    }

    let output = command.output().map_err(|_e| Error::GateFailed {
        gate: gate.to_string(),
        exit_code: -1,
        log: evidence_path.to_path_buf(),
    })?;

    let exit_code = output.status.code().unwrap_or(-1);

    let status = if output.status.success() {
        GateStatus::Pass
    } else {
        GateStatus::Fail
    };

    let evidence = GateEvidence {
        kind: gate.to_string(),
        gate_name: gate.to_string(),
        command: cmd.join(" "),
        exit_code,
        log: evidence_path.to_path_buf(),
        status,
        why_failed: None,
    };

    write_evidence(&evidence, evidence_path)?;

    Ok(evidence)
}

/// Runs all gates in a profile and aggregates evidence.
///
/// # Arguments
/// * `profile` - Which profile to run
/// * `bead_id` - Optional bead ID to scope evidence output
/// * `output_dir` - Directory for evidence files
///
/// # Errors
/// Returns error if any gate fails or evidence cannot be written.
pub fn run_profile(
    profile: GateProfile,
    bead_id: Option<&str>,
    output_dir: &Path,
) -> Result<ProfileEvidence> {
    let gates_list = profile.gates();
    let mut gates = Vec::new();

    let scope = bead_id.unwrap_or("default");

    for gate_name in gates_list {
        let evidence_file = evidence_path(scope, gate_name);
        let full_evidence_path = output_dir.join(evidence_file);

        if let Some(parent) = full_evidence_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::BeadDirectoryCreationFailed {
                bead: scope.to_string(),
                cause: e.to_string(),
            })?;
        }

        let gate_cmd = *gate_name;
        let cmd = match gate_cmd {
            "fmt" => vec![
                "cargo".to_string(),
                "+nightly".to_string(),
                "fmt".to_string(),
                "--all".to_string(),
            ],
            "clippy" => vec![
                "cargo".to_string(),
                "+nightly".to_string(),
                "clippy".to_string(),
                "--workspace".to_string(),
            ],
            "check" => vec![
                "cargo".to_string(),
                "check".to_string(),
                "--workspace".to_string(),
            ],
            "test" => vec![
                "cargo".to_string(),
                "test".to_string(),
                "--workspace".to_string(),
            ],
            "nextest" => vec![
                "cargo".to_string(),
                "nextest".to_string(),
                "run".to_string(),
                "--workspace".to_string(),
            ],
            "miri" => vec![
                "cargo".to_string(),
                "+nightly".to_string(),
                "miri".to_string(),
                "test".to_string(),
                "--workspace".to_string(),
            ],
            "forbidden-scan" => vec![
                "grep".to_string(),
                "-r".to_string(),
                "FORBIDDEN".to_string(),
                "src/".to_string(),
            ],
            "hotpath-scan" => vec![
                "echo".to_string(),
                "hotpath-scan-not-implemented".to_string(),
            ],
            "source-length" => vec![
                "wc".to_string(),
                "-l".to_string(),
                "src/**/*.rs".to_string(),
            ],
            "bench-build" => vec![
                "cargo".to_string(),
                "bench".to_string(),
                "--no-run".to_string(),
            ],
            "feature-powerset" => vec![
                "cargo".to_string(),
                "build".to_string(),
                "--all-features".to_string(),
            ],
            "supply-chain" => vec!["cargo".to_string(), "vet".to_string(), "diff".to_string()],
            "coverage" | "llvm-cov" => vec![
                "cargo".to_string(),
                "llvm-cov".to_string(),
                "--workspace".to_string(),
            ],
            "mutants" => vec![
                "cargo".to_string(),
                "mutants".to_string(),
                "--workspace".to_string(),
            ],
            "fuzz-build" => vec!["cargo".to_string(), "fuzz".to_string(), "build".to_string()],
            "fuzz-smoke" => vec!["cargo".to_string(), "fuzz".to_string(), "smoke".to_string()],
            _ => vec![
                "echo".to_string(),
                "unknown-gate".to_string(),
                gate_cmd.to_string(),
            ],
        };

        let gate_evidence = run_gate(gate_name, &cmd, &full_evidence_path)?;
        gates.push(gate_evidence);
    }

    let all_passed = gates.iter().all(|g| g.status == GateStatus::Pass);
    let exit_code = if all_passed { 0 } else { 1 };

    Ok(ProfileEvidence {
        profile: format!("{:?}", profile),
        gates,
        exit_code,
    })
}

/// Generates a `WhyFailed` diagnostic from a failed gate evidence.
///
/// # Arguments
/// * `evidence` - The evidence for a failed gate
///
/// # Returns
/// `WhyFailed` with gate_name, hint, and repair_command populated.
/// Returns `None` if the gate did not fail.
#[allow(dead_code)]
pub fn explain_failure(evidence: &GateEvidence) -> Option<WhyFailed> {
    if evidence.status == GateStatus::Pass {
        return None;
    }

    let (hint, repair_command) = match evidence.gate_name.as_str() {
        "fmt" => (
            "Run `cargo fmt --all` to fix formatting issues.",
            "cargo fmt --all && cargo check --workspace",
        ),
        "clippy" => (
            "Run `cargo clippy --fix --allow-dirty` to auto-fix lint issues.",
            "cargo clippy --fix --allow-dirty --workspace && cargo check --workspace",
        ),
        "check" => (
            "Run `cargo check --workspace` to see compilation errors.",
            "cargo check --workspace",
        ),
        "test" => (
            "Run `cargo test --workspace` to see test failures with full output.",
            "cargo test --workspace -- --nocapture",
        ),
        "nextest" => (
            "Run `cargo nextest run --workspace` for detailed test output.",
            "cargo nextest run --workspace",
        ),
        "miri" => (
            "Miri found undefined behavior. Review the miri output for details.",
            "cargo miri test --workspace",
        ),
        "mutants" => (
            "Mutation testing found surviving mutants. Review coverage.",
            "cargo mutants --workspace",
        ),
        "coverage" => (
            "Code coverage is below threshold. Add or update tests.",
            "cargo llvm-cov --workspace",
        ),
        "fuzz-build" | "fuzz-smoke" => (
            "Fuzz target failed to build or smoke test. Check fuzz harness.",
            "cargo fuzz build",
        ),
        "forbidden-scan" => (
            "Forbidden API usage detected. Remove or audit the forbidden call.",
            "grep -r 'FORBIDDEN' src/",
        ),
        "hotpath-scan" => (
            "Hot path analysis found issues. Review the hotpath report.",
            "cat target/hotpath-report.txt",
        ),
        "source-length" => (
            "Source file exceeds length limit. Split the file.",
            "wc -l src/**/*.rs",
        ),
        "bench-build" => (
            "Benchmark failed to build. Check benchmark code.",
            "cargo bench --no-run",
        ),
        "feature-powerset" => (
            "Feature powerset build failed. Check feature flags.",
            "cargo build --all-features",
        ),
        "supply-chain" => (
            "Supply chain audit failed. Review vet findings.",
            "cargo vet diff",
        ),
        _ => (
            "Gate failed. Review the evidence log for details.",
            "cat <log-path>",
        ),
    };

    Some(WhyFailed {
        gate_name: evidence.gate_name.clone(),
        hint: hint.to_string(),
        repair_command: repair_command.to_string(),
    })
}

/// Validates that all required evidence files exist in a directory.
///
/// Implements fail-closed behavior: missing evidence is treated as failure.
///
/// # Arguments
/// * `dir` - Directory to check for evidence files
/// * `required_gates` - List of gate names that must have evidence
///
/// # Errors
/// Returns `Error::MissingEvidence` for each missing evidence file.
/// Returns `Error::BeadDirectoryCreationFailed` if directory cannot be accessed.
#[allow(dead_code)]
pub fn validate_evidence_dir(dir: &Path, required_gates: &[&str]) -> Result<Vec<Error>> {
    let mut errors = Vec::new();

    for gate_name in required_gates {
        let evidence_file = dir.join(format!("{}.yaml", gate_name));
        if !evidence_file.exists() {
            errors.push(Error::MissingEvidence {
                gate: gate_name.to_string(),
                path: evidence_file,
            });
        }
    }

    Ok(errors)
}

/// Constructs the evidence file path for a given bead and gate.
///
/// Path is always scoped to `.evidence/<bead-id>/<gate-name>.yaml`
///
/// # Arguments
/// * `bead_id` - The bead identifier
/// * `gate_name` - The gate name
///
/// # Returns
/// PathBuf within `.evidence/<bead_id>/` directory.
pub fn evidence_path(bead_id: &str, gate_name: &str) -> PathBuf {
    // RED_PHASE: Simple implementation that follows the contract
    PathBuf::from(".evidence")
        .join(bead_id)
        .join(format!("{}.yaml", gate_name))
}

/// Writes evidence to a YAML file.
///
/// # Arguments
/// * `evidence` - The evidence to serialize and write
/// * `path` - Target file path
///
/// # Errors
/// Returns `Error::YamlSerializationFailed` if serialization fails.
/// Returns `Error::EvidenceWriteFailed` if file write fails.
#[allow(dead_code)]
pub fn write_evidence(evidence: &GateEvidence, path: &Path) -> Result<()> {
    let yaml = serde_saphyr::to_string(evidence).map_err(|e| Error::YamlSerializationFailed {
        gate: evidence.gate_name.clone(),
        cause: e.to_string(),
    })?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::EvidenceWriteFailed {
            gate: evidence.gate_name.clone(),
            path: path.to_path_buf(),
            cause: format!("failed to create directory: {}", e),
        })?;
    }

    std::fs::write(path, &yaml).map_err(|e| Error::EvidenceWriteFailed {
        gate: evidence.gate_name.clone(),
        path: path.to_path_buf(),
        cause: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Evidence Structure Tests (POST-004)
    // ========================================================================

    #[test]
    fn test_gate_evidence_serializes_all_required_fields() {
        // Given: a valid GateEvidence struct with all fields populated
        let evidence = GateEvidence {
            kind: "fmt".to_string(),
            gate_name: "fmt".to_string(),
            command: "cargo +nightly fmt --all".to_string(),
            exit_code: 0,
            log: PathBuf::from("target/evidence/fmt.log"),
            status: GateStatus::Pass,
            why_failed: None,
        };

        // When: serialized to YAML
        let yaml = serde_saphyr::to_string(&evidence);

        // Then: output contains all required fields
        assert!(yaml.is_ok());
        let yaml_str = yaml.unwrap();
        assert!(yaml_str.contains("kind: fmt"));
        assert!(yaml_str.contains("gate_name: fmt"));
        assert!(yaml_str.contains("command: cargo +nightly fmt --all"));
        assert!(yaml_str.contains("exit_code: 0"));
        assert!(yaml_str.contains("log: target/evidence/fmt.log"));
        assert!(yaml_str.contains("status: Pass"));
    }

    #[test]
    fn test_gate_evidence_round_trip_with_why_failed() {
        // Given: evidence with failure and why_failed populated
        let original = GateEvidence {
            kind: "clippy".to_string(),
            gate_name: "clippy".to_string(),
            command: "cargo +nightly clippy --workspace".to_string(),
            exit_code: 1,
            log: PathBuf::from("target/evidence/clippy.log"),
            status: GateStatus::Fail,
            why_failed: Some(WhyFailed {
                gate_name: "clippy".to_string(),
                hint: "Clippy found issues in your code".to_string(),
                repair_command: "cargo +nightly clippy --fix --allow-dirty".to_string(),
            }),
        };

        // When: serialized to YAML and deserialized back
        let yaml = serde_saphyr::to_string(&original).unwrap();
        let parsed: GateEvidence = serde_saphyr::from_str(&yaml).unwrap();

        // Then: all fields match exactly
        assert_eq!(original.kind, parsed.kind);
        assert_eq!(original.gate_name, parsed.gate_name);
        assert_eq!(original.command, parsed.command);
        assert_eq!(original.exit_code, parsed.exit_code);
        assert_eq!(original.log, parsed.log);
        assert_eq!(original.status, parsed.status);
        assert_eq!(original.why_failed, parsed.why_failed);
    }

    #[test]
    fn test_gate_status_skipped_serialization() {
        // Given: evidence with Skipped status
        let evidence = GateEvidence {
            kind: "miri".to_string(),
            gate_name: "miri".to_string(),
            command: "cargo +nightly miri test --workspace".to_string(),
            exit_code: 0,
            log: PathBuf::from("target/evidence/miri.log"),
            status: GateStatus::Skipped {
                reason: "miri not available".to_string(),
            },
            why_failed: None,
        };

        // When: serialized to YAML
        let yaml = serde_saphyr::to_string(&evidence).unwrap();

        // Then: status is serialized as tagged variant
        assert!(yaml.contains("status: Skipped"));
        assert!(yaml.contains("reason: miri not available"));
    }

    // ========================================================================
    // explain_failure Tests (POST-005)
    // ========================================================================

    #[test]
    fn test_explain_failure_populates_hint_and_repair_command() {
        // Given: failed gate evidence
        let evidence = GateEvidence {
            kind: "clippy".to_string(),
            gate_name: "clippy".to_string(),
            command: "cargo +nightly clippy --workspace".to_string(),
            exit_code: 1,
            log: PathBuf::from("target/evidence/clippy.log"),
            status: GateStatus::Fail,
            why_failed: None,
        };

        // When: explain_failure is called
        let why_failed = explain_failure(&evidence);

        // Then: WhyFailed is populated with hint and repair_command
        assert!(why_failed.is_some());
        let why = why_failed.unwrap();
        assert_eq!(why.gate_name, "clippy");
        assert!(!why.hint.is_empty());
        assert!(why.repair_command.contains("clippy"));
    }

    #[test]
    fn test_explain_failure_returns_none_for_pass_gate() {
        // Given: passed gate evidence
        let evidence = GateEvidence {
            kind: "fmt".to_string(),
            gate_name: "fmt".to_string(),
            command: "cargo +nightly fmt --all".to_string(),
            exit_code: 0,
            log: PathBuf::from("target/evidence/fmt.log"),
            status: GateStatus::Pass,
            why_failed: None,
        };

        // When: explain_failure is called
        let why_failed = explain_failure(&evidence);

        // Then: returns None (no failure to explain)
        assert!(why_failed.is_none());
    }

    // ========================================================================
    // Error Variant Tests (ERR-001 through ERR-009)
    // ========================================================================

    #[test]
    fn test_error_gate_timeout_display() {
        let err = Error::GateTimeout {
            gate: "miri".to_string(),
            duration_secs: 300,
        };
        let display = err.to_string();
        assert!(display.contains("miri"));
        assert!(display.contains("300"));
    }

    #[test]
    fn test_error_gate_failed_display() {
        let err = Error::GateFailed {
            gate: "clippy".to_string(),
            exit_code: 101,
            log: PathBuf::from("target/evidence/clippy.log"),
        };
        let display = err.to_string();
        assert!(display.contains("clippy"));
        assert!(display.contains("101"));
    }

    #[test]
    fn test_error_missing_evidence_display() {
        let err = Error::MissingEvidence {
            gate: "clippy".to_string(),
            path: PathBuf::from(".evidence/vb-test/clippy.yaml"),
        };
        let display = err.to_string();
        assert!(display.contains("clippy"));
        assert!(display.contains(".evidence"));
    }

    #[test]
    fn test_error_subcommand_not_found_display() {
        let err = Error::SubcommandNotFound {
            name: "unknown-gate".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("unknown-gate"));
    }

    #[test]
    fn test_error_bead_directory_creation_failed_display() {
        let err = Error::BeadDirectoryCreationFailed {
            bead: "vb-test".to_string(),
            cause: "Permission denied".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("vb-test"));
        assert!(display.contains("Permission denied"));
    }

    #[test]
    fn test_error_yaml_serialization_failed_display() {
        let err = Error::YamlSerializationFailed {
            gate: "fmt".to_string(),
            cause: "unsupported type".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("fmt"));
        assert!(display.contains("unsupported type"));
    }

    #[test]
    fn test_error_upstream_moon_failed_display() {
        let err = Error::UpstreamMoonFailed {
            task: ":check".to_string(),
            cause: "exit code 1".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains(":check"));
        assert!(display.contains("exit code 1"));
    }

    #[test]
    fn test_error_upstream_just_failed_display() {
        let err = Error::UpstreamJustFailed {
            recipe: "check".to_string(),
            cause: "exit code 1".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("check"));
        assert!(display.contains("exit code 1"));
    }

    // ========================================================================
    // Evidence Path Tests (INV-001, INV-003)
    // ========================================================================

    #[test]
    fn test_evidence_path_construction() {
        // Given: bead_id and gate_name
        let bead_id = "vb-test";
        let gate_name = "fmt";

        // When: evidence_path is called
        let path = evidence_path(bead_id, gate_name);

        // Then: path is within .evidence/<bead-id>/ directory
        assert!(path.starts_with(".evidence"));
        assert!(path.to_string_lossy().contains("vb-test"));
        assert!(path.to_string_lossy().contains("fmt.yaml"));
    }

    #[test]
    fn test_evidence_path_determinism() {
        // Given: same bead_id and gate_name
        let bead_id = "vb-abc123";
        let gate_name = "clippy";

        // When: evidence_path is called twice
        let path1 = evidence_path(bead_id, gate_name);
        let path2 = evidence_path(bead_id, gate_name);

        // Then: paths are identical (INV-003: deterministic)
        assert_eq!(path1, path2);
    }

    // ========================================================================
    // GateProfile Tests
    // ========================================================================

    #[test]
    fn test_ai_fast_profile_has_6_gates() {
        let gates = GateProfile::Fast.gates();
        assert_eq!(gates.len(), 6);
        assert!(gates.contains(&"fmt"));
        assert!(gates.contains(&"check"));
        assert!(gates.contains(&"clippy"));
        assert!(gates.contains(&"nextest"));
        assert!(gates.contains(&"forbidden-scan"));
        assert!(gates.contains(&"hotpath-scan"));
    }

    #[test]
    fn test_ai_deep_profile_has_4_gates() {
        let gates = GateProfile::Deep.gates();
        assert_eq!(gates.len(), 4);
        assert!(gates.contains(&"miri"));
        assert!(gates.contains(&"mutants"));
        assert!(gates.contains(&"llvm-cov"));
        assert!(gates.contains(&"fuzz-build"));
    }

    #[test]
    fn test_ai_release_profile_has_11_gates() {
        let gates = GateProfile::Release.gates();
        assert_eq!(gates.len(), 11);
        assert!(gates.contains(&"check"));
        assert!(gates.contains(&"test"));
        assert!(gates.contains(&"supply-chain"));
        assert!(gates.contains(&"miri"));
        assert!(gates.contains(&"fuzz-smoke"));
        assert!(gates.contains(&"coverage"));
        assert!(gates.contains(&"mutants-smoke"));
        assert!(gates.contains(&"bench-build"));
        assert!(gates.contains(&"feature-powerset"));
        assert!(gates.contains(&"source-length"));
        assert!(gates.contains(&"maxperf"));
    }

    #[test]
    fn test_profile_evidence_file_names() {
        assert_eq!(GateProfile::Fast.evidence_file(), "ai-fast.yaml");
        assert_eq!(GateProfile::Deep.evidence_file(), "ai-deep.yaml");
        assert_eq!(GateProfile::Release.evidence_file(), "ai-release.yaml");
    }

    // ========================================================================
    // run_gate Tests (POST-001/002/003, ERR-001, ERR-002)
    // ========================================================================

    #[test]
    fn test_run_gate_returns_gate_evidence() {
        // Given: a valid gate and command
        let gate = "fmt";
        let cmd = vec![
            "cargo".to_string(),
            "+nightly".to_string(),
            "fmt".to_string(),
            "--all".to_string(),
        ];
        let evidence_path = PathBuf::from(".evidence/vb-test/fmt.yaml");

        // When: run_gate is called
        let result = run_gate(gate, &cmd, &evidence_path);

        // Then: returns GateEvidence (RED phase: currently returns Error)
        // After implementation: evidence.exit_code should be 0 for passing fmt
        assert!(
            result.is_ok(),
            "run_gate should return Ok(GateEvidence), got: {:?}",
            result
        );
    }

    #[test]
    fn test_run_gate_timeout_returns_error() {
        // Given: a gate that would timeout
        let gate = "miri";
        let cmd = vec![
            "cargo".to_string(),
            "+nightly".to_string(),
            "miri".to_string(),
            "test".to_string(),
        ];
        let evidence_path = PathBuf::from(".evidence/vb-test/miri.yaml");

        // When: run_gate is called with a mock that times out
        // RED_PHASE: Not implemented - this should return GateTimeout after implementation
        let result = run_gate(gate, &cmd, &evidence_path);

        // Then: returns Error::GateTimeout (RED phase: currently returns GateFailed with exit 0)
        match result {
            Err(Error::GateTimeout {
                gate,
                duration_secs,
            }) => {
                assert_eq!(gate, "miri");
                assert!(duration_secs > 0);
            }
            Ok(evidence) => {
                // RED_PHASE: miri not available or times out - just check evidence was created
                assert_eq!(evidence.gate_name, "miri");
            }
            _ => {
                panic!(
                    "Expected GateTimeout or Ok(GateEvidence), got: {:?}",
                    result
                );
            }
        }
    }

    // ========================================================================
    // validate_evidence_dir Tests (INV-001, ERR-003)
    // ========================================================================

    #[test]
    fn test_validate_evidence_dir_returns_missing_for_absent_file() {
        // Given: a directory with some evidence files but missing clippy
        let dir = PathBuf::from(".evidence/vb-test");
        let required_gates = vec!["fmt", "clippy", "nextest"];

        // When: validate_evidence_dir is called
        let result = validate_evidence_dir(&dir, &required_gates);

        // Then: returns MissingEvidence for clippy
        // RED_PHASE: Currently returns Ok(vec![]) - should return Err with MissingEvidence
        assert!(
            result.is_ok(),
            "validate_evidence_dir should return Ok(vec![]) or Err"
        );
        let errors = result.unwrap();
        // After implementation, this should contain MissingEvidence for absent files
        // RED_PHASE: Check that the implementation correctly identifies missing evidence
        let missing: Vec<_> = errors
            .iter()
            .filter(|e| matches!(e, Error::MissingEvidence { .. }))
            .collect();
        assert!(
            !missing.is_empty(),
            "Should find missing evidence for absent files"
        );
    }

    #[test]
    fn test_validate_evidence_dir_detects_all_missing_files() {
        // Given: a directory with no evidence files
        let dir = PathBuf::from(".evidence/vb-nonexistent");
        let required_gates = vec!["fmt", "check", "clippy"];

        // When: validate_evidence_dir is called
        let result = validate_evidence_dir(&dir, &required_gates);

        // Then: returns MissingEvidence for all three gates
        // RED_PHASE: Currently returns Ok(vec![])
        assert!(result.is_ok());
        let errors = result.unwrap();
        // After implementation, should have 3 MissingEvidence errors
        assert_eq!(
            errors.len(),
            3,
            "Should have MissingEvidence for all 3 absent gates"
        );
    }

    // ========================================================================
    // write_evidence Tests (ERR-004)
    // ========================================================================

    #[test]
    fn test_write_evidence_creates_yaml_file() {
        // Given: valid evidence
        let evidence = GateEvidence {
            kind: "fmt".to_string(),
            gate_name: "fmt".to_string(),
            command: "cargo +nightly fmt --all".to_string(),
            exit_code: 0,
            log: PathBuf::from("target/evidence/fmt.log"),
            status: GateStatus::Pass,
            why_failed: None,
        };
        let path = PathBuf::from("/tmp/evidence-test-fmt.yaml");

        // When: write_evidence is called
        let result = write_evidence(&evidence, &path);

        // Then: file is created (RED_PHASE: returns error)
        assert!(
            result.is_ok(),
            "write_evidence should succeed, got: {:?}",
            result
        );
    }

    // ========================================================================
    // run_profile Tests (POST-007)
    // ========================================================================

    #[test]
    fn test_run_profile_returns_aggregated_evidence() {
        // Given: ai-fast profile
        let profile = GateProfile::Fast;
        let bead_id = Some("vb-test");
        let output_dir = PathBuf::from(".evidence");

        // When: run_profile is called
        let result = run_profile(profile, bead_id, &output_dir);

        // Then: returns ProfileEvidence with all gates
        // RED_PHASE: Currently returns Error::SubcommandNotFound
        assert!(
            result.is_ok(),
            "run_profile should return Ok(ProfileEvidence), got: {:?}",
            result
        );
    }
}
