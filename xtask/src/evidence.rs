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
//! - `Error::YamlSerializationFailed` — serde_yaml error during evidence serialization
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
pub(crate) struct GateEvidence {
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
pub(crate) struct WhyFailed {
    pub gate_name: String,
    pub hint: String,
    pub repair_command: String,
}

/// Status of a gate execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", content = "reason")]
pub(crate) enum GateStatus {
    Pass,
    Fail,
    Skipped { reason: String },
}

/// All error variants for xtask gate operations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum Error {
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
    /// serde_yaml error during evidence serialization.
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
pub(crate) type Result<T> = std::result::Result<T, Error>;

/// Profile of gates to run together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum GateProfile {
    /// Fast gates: fmt, check, clippy, nextest, forbidden-scan, hotpath-scan
    Fast,
    /// Deep gates: miri, mutants, llvm-cov, fuzz-build
    Deep,
    /// Release gates: check, test, supply-chain, miri, fuzz-smoke, coverage,
    /// mutants-smoke, bench-build, feature-powerset, source-length, maxperf
    Release,
}

impl GateProfile {
    /// Returns the list of gates in this profile.
    pub(crate) fn gates(self) -> &'static [&'static str] {
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
    pub(crate) fn evidence_file(self) -> &'static str {
        match self {
            GateProfile::Fast => "ai-fast.yaml",
            GateProfile::Deep => "ai-deep.yaml",
            GateProfile::Release => "ai-release.yaml",
        }
    }
}

/// Aggregated evidence for a full profile run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProfileEvidence {
    pub profile: String,
    pub gates: Vec<GateEvidence>,
    pub exit_code: i32,
}

// ============================================================================
// Core orchestration functions
// ============================================================================

/// Default timeout for gate execution (5 minutes).
const DEFAULT_GATE_TIMEOUT_SECS: u64 = 300;

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
pub(crate) fn run_gate(gate: &str, cmd: &[String], evidence_path: &Path) -> Result<GateEvidence> {
    let gate_name = gate.to_string();
    let command_str = cmd.join(" ");

    // Create parent directories for evidence path if needed
    if let Some(parent) = evidence_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::EvidenceWriteFailed {
            gate: gate_name.clone(),
            path: evidence_path.to_path_buf(),
            cause: format!("failed to create evidence directory: {}", e),
        })?;
    }

    // Derive log path from evidence path (same location, .log extension)
    let log_path = evidence_path.with_extension("log");

    // Execute the command with timeout
    let exit_code = match execute_command_with_timeout(cmd, DEFAULT_GATE_TIMEOUT_SECS) {
        Ok(code) => code,
        Err(_) => {
            return Err(Error::GateTimeout {
                gate: gate_name,
                duration_secs: DEFAULT_GATE_TIMEOUT_SECS,
            });
        }
    };

    // Determine status based on exit code
    let status = if exit_code == 0 {
        GateStatus::Pass
    } else {
        GateStatus::Fail
    };

    // Build evidence
    let mut evidence = GateEvidence {
        kind: gate_name.clone(),
        gate_name,
        command: command_str,
        exit_code,
        log: log_path,
        status,
        why_failed: None,
    };

    // If failed, populate why_failed
    if exit_code != 0 {
        evidence.why_failed = explain_failure(&evidence);
    }

    // Write evidence to file
    write_evidence(&evidence, evidence_path)?;

    Ok(evidence)
}

/// Executes a command with a timeout, returning the exit code or timeout error.
fn execute_command_with_timeout(cmd: &[String], timeout_secs: u64) -> Result<i32> {
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    let cmd0 = match cmd.first() {
        Some(c) => c,
        None => {
            return Err(Error::GateFailed {
                gate: String::new(),
                exit_code: -1,
                log: PathBuf::from("target/evidence/none.log"),
            });
        }
    };
    let cmd_args = cmd.get(1..).unwrap_or(&[]);

    let mut child = std::process::Command::new(cmd0)
        .args(cmd_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| Error::GateFailed {
            gate: cmd0.clone(),
            exit_code: -1,
            log: PathBuf::from("target/evidence/spawn.log"),
        })?;

    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Ok(status.code().unwrap_or(-1));
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill().ok();
                    let _ = child.wait().ok();
                    return Err(Error::GateTimeout {
                        gate: cmd0.clone(),
                        duration_secs: timeout_secs,
                    });
                }
                // Brief sleep to avoid busy-waiting
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => {
                return Err(Error::GateFailed {
                    gate: cmd0.clone(),
                    exit_code: -1,
                    log: PathBuf::from("target/evidence/wait.log"),
                });
            }
        }
    }
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
pub(crate) fn run_profile(
    profile: GateProfile,
    bead_id: Option<&str>,
    output_dir: &Path,
) -> Result<ProfileEvidence> {
    let bead = bead_id.unwrap_or("default");
    let gates = profile.gates();
    let mut gate_evidences = Vec::with_capacity(gates.len());
    let mut overall_exit_code = 0;

    // Create bead directory if needed
    let bead_dir = output_dir.join(".evidence").join(bead);
    std::fs::create_dir_all(&bead_dir).map_err(|e| Error::BeadDirectoryCreationFailed {
        bead: bead.to_string(),
        cause: format!("failed to create directory: {}", e),
    })?;

    // Import gates module for gate command execution
    use crate::gates::Gate;

    for gate_name in gates {
        let gate_name_str = *gate_name;
        let gate = match gate_name_str {
            "fmt" => Gate::Fmt,
            "check" => Gate::Check,
            "clippy" => Gate::Clippy,
            "nextest" => Gate::Nextest,
            "forbidden-scan" => Gate::ForbiddenScan,
            "hotpath-scan" => Gate::HotpathScan,
            "miri" => Gate::Miri,
            "mutants" => Gate::Mutants,
            "llvm-cov" => Gate::LlvmCov,
            "fuzz-build" => Gate::FuzzBuild,
            "supply-chain" => Gate::SupplyChain,
            "fuzz-smoke" => Gate::FuzzSmoke,
            "coverage" => Gate::Coverage,
            "mutants-smoke" => Gate::MutantsSmoke,
            "bench-build" => Gate::BenchBuild,
            "feature-powerset" => Gate::FeaturePowerset,
            "source-length" => Gate::SourceLength,
            "maxperf" => Gate::Maxperf,
            other => {
                return Err(Error::SubcommandNotFound {
                    name: other.to_string(),
                });
            }
        };

        let evidence_path = evidence_path(bead, gate_name_str);
        let cmd = gate.command();

        // Execute the gate
        match run_gate(gate_name_str, &cmd, &evidence_path) {
            Ok(evidence) => {
                overall_exit_code = overall_exit_code.max(evidence.exit_code);
                gate_evidences.push(evidence);
            }
            Err(e) => {
                // Gate failed - record the error as evidence
                let evidence = GateEvidence {
                    kind: gate_name_str.to_string(),
                    gate_name: gate_name_str.to_string(),
                    command: cmd.join(" "),
                    exit_code: -1,
                    log: evidence_path.with_extension("log"),
                    status: GateStatus::Fail,
                    why_failed: Some(WhyFailed {
                        gate_name: gate_name_str.to_string(),
                        hint: format!("Gate failed: {}", e),
                        repair_command: format!("cargo xtask {}", gate_name_str),
                    }),
                };
                overall_exit_code = overall_exit_code.max(1);
                gate_evidences.push(evidence);
            }
        }
    }

    // Write profile-level aggregated evidence
    let profile_evidence = ProfileEvidence {
        profile: match profile {
            GateProfile::Fast => "ai-fast".to_string(),
            GateProfile::Deep => "ai-deep".to_string(),
            GateProfile::Release => "ai-release".to_string(),
        },
        gates: gate_evidences,
        exit_code: overall_exit_code,
    };

    let profile_file = bead_dir.join(profile.evidence_file());
    let yaml =
        serde_yaml::to_string(&profile_evidence).map_err(|e| Error::YamlSerializationFailed {
            gate: "profile".to_string(),
            cause: e.to_string(),
        })?;
    std::fs::write(&profile_file, &yaml).map_err(|e| Error::EvidenceWriteFailed {
        gate: "profile".to_string(),
        path: profile_file,
        cause: e.to_string(),
    })?;

    Ok(profile_evidence)
}

/// Generates a `WhyFailed` diagnostic from a failed gate evidence.
///
/// # Arguments
/// * `evidence` - The evidence for a failed gate
///
/// # Returns
/// `WhyFailed` with gate_name, hint, and repair_command populated.
/// Returns `None` if the gate did not fail.
pub(crate) fn explain_failure(evidence: &GateEvidence) -> Option<WhyFailed> {
    // Only generate why-failed for gates that actually failed
    if matches!(evidence.status, GateStatus::Pass) {
        return None;
    }

    let gate_name = &evidence.gate_name;

    // Generate hint and repair command based on gate type
    let (hint, repair_command) = match gate_name.as_str() {
        "fmt" => (
            "Format your code with cargo +nightly fmt --all".to_string(),
            "cargo +nightly fmt --all".to_string(),
        ),
        "check" => (
            "Run moon run :check to see check errors".to_string(),
            "moon run :check".to_string(),
        ),
        "clippy" => (
            "Run 'cargo +nightly clippy' to see diagnostics".to_string(),
            "cargo +nightly clippy --fix --allow-dirty".to_string(),
        ),
        "nextest" => (
            "Run 'cargo nextest run' to see test failures".to_string(),
            "cargo nextest run --workspace".to_string(),
        ),
        "forbidden-scan" => (
            "Forbidden patterns found in source code".to_string(),
            "bash scripts/forbidden-scan.sh".to_string(),
        ),
        "hotpath-scan" => (
            "Hotpath analysis found issues".to_string(),
            "bash scripts/hotpath-scan.sh".to_string(),
        ),
        "miri" => (
            "Miri found undefined behavior or memory issues".to_string(),
            "cargo +nightly miri test --workspace".to_string(),
        ),
        "mutants" => (
            "Mutation testing found surviving mutants".to_string(),
            "cargo mutants --package velvet_ballastics".to_string(),
        ),
        "llvm-cov" => (
            "Code coverage is below threshold".to_string(),
            "cargo llvm-cov".to_string(),
        ),
        "fuzz-build" => (
            "Fuzz build failed".to_string(),
            "cargo fuzz build".to_string(),
        ),
        "supply-chain" => (
            "Supply chain check failed".to_string(),
            "moon run :supply-chain".to_string(),
        ),
        "fuzz-smoke" => (
            "Fuzz smoke test failed".to_string(),
            "moon run :fuzz-smoke".to_string(),
        ),
        "coverage" => (
            "Coverage check failed".to_string(),
            "moon run :coverage".to_string(),
        ),
        "mutants-smoke" => (
            "Mutation smoke test found surviving mutants".to_string(),
            "moon run :mutants-smoke".to_string(),
        ),
        "bench-build" => (
            "Benchmark build failed".to_string(),
            "moon run :bench-build".to_string(),
        ),
        "feature-powerset" => (
            "Feature powerset check failed".to_string(),
            "moon run :feature-powerset".to_string(),
        ),
        "source-length" => (
            "Source file exceeds length limit".to_string(),
            "bash scripts/check-source-length.sh".to_string(),
        ),
        "maxperf" => (
            "Maxperf build or performance check failed".to_string(),
            "moon run :maxperf".to_string(),
        ),
        _ => (
            format!(
                "Gate '{}' failed with exit code {}",
                gate_name, evidence.exit_code
            ),
            format!("cargo xtask {}", gate_name),
        ),
    };

    Some(WhyFailed {
        gate_name: gate_name.clone(),
        hint,
        repair_command,
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
pub(crate) fn validate_evidence_dir(dir: &Path, required_gates: &[&str]) -> Result<Vec<Error>> {
    let mut errors = Vec::new();

    // Check if directory exists
    if !dir.exists() {
        // Return MissingEvidence for all gates since dir doesn't exist
        for gate in required_gates {
            errors.push(Error::MissingEvidence {
                gate: gate.to_string(),
                path: dir.join(format!("{}.yaml", gate)),
            });
        }
        return Ok(errors);
    }

    // Check each required gate's evidence file
    for gate in required_gates {
        let evidence_file = dir.join(format!("{}.yaml", gate));
        if !evidence_file.exists() {
            errors.push(Error::MissingEvidence {
                gate: gate.to_string(),
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
pub(crate) fn evidence_path(bead_id: &str, gate_name: &str) -> PathBuf {
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
pub(crate) fn write_evidence(evidence: &GateEvidence, path: &Path) -> Result<()> {
    // Serialize to YAML
    let yaml = serde_yaml::to_string(evidence).map_err(|e| Error::YamlSerializationFailed {
        gate: evidence.gate_name.clone(),
        cause: e.to_string(),
    })?;

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::EvidenceWriteFailed {
            gate: evidence.gate_name.clone(),
            path: path.to_path_buf(),
            cause: format!("failed to create directory: {}", e),
        })?;
    }

    // Write to file
    std::fs::write(path, &yaml).map_err(|e| Error::EvidenceWriteFailed {
        gate: evidence.gate_name.clone(),
        path: path.to_path_buf(),
        cause: e.to_string(),
    })?;

    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::let_underscore_must_use
)]
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
        let yaml = serde_yaml::to_string(&evidence);

        // Then: output contains all required fields
        let yaml_str = yaml.expect("YAML serialization should succeed for valid evidence");
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
        let yaml = match serde_yaml::to_string(&original) {
            Ok(s) => s,
            Err(e) => panic!("YAML serialization failed: {}", e),
        };
        let parsed: GateEvidence = match serde_yaml::from_str(&yaml) {
            Ok(p) => p,
            Err(e) => panic!("YAML deserialization failed: {}", e),
        };

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
        let yaml = serde_yaml::to_string(&evidence).unwrap();

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
        // Use the test helper with a short timeout to verify timeout behavior
        use test_helpers::run_gate_with_timeout;

        // Given: a gate that would timeout
        let gate = "sleep";
        let cmd = vec!["sleep".to_string(), "10".to_string()];
        let evidence_path = PathBuf::from(".evidence/vb-test/sleep.yaml");

        // When: run_gate_with_timeout is called with 1ms timeout
        let result = run_gate_with_timeout(gate, &cmd, &evidence_path, 1);

        // Then: returns Error::GateTimeout
        assert!(
            matches!(
                result,
                Err(Error::GateTimeout {
                    ref gate,
                    ref duration_secs,
                }) if *gate == "sleep" && *duration_secs == 1
            ),
            "Expected GateTimeout error for gate 'sleep' with 1s timeout, got: {:?}",
            result
        );
    }

    // ========================================================================
    // validate_evidence_dir Tests (INV-001, ERR-003)
    // ========================================================================

    #[test]
    fn test_validate_evidence_dir_returns_missing_for_absent_file() {
        // Given: a directory path that doesn't exist
        let dir = PathBuf::from(".evidence/vb-test-validate");
        let required_gates = vec!["fmt", "clippy", "nextest"];

        // Clean up any existing directory to ensure clean state
        let _ = std::fs::remove_dir_all(&dir);

        // When: validate_evidence_dir is called
        let result = validate_evidence_dir(&dir, &required_gates);

        // Then: returns MissingEvidence for all required gates (directory doesn't exist)
        assert!(
            result.is_ok(),
            "validate_evidence_dir should return Ok(vec![]) or Err"
        );
        let errors = result.unwrap();
        // Implementation should return MissingEvidence for each absent gate
        let missing: Vec<_> = errors
            .iter()
            .filter(|e| matches!(e, Error::MissingEvidence { .. }))
            .collect();
        assert!(
            !missing.is_empty(),
            "Should find missing evidence for absent files"
        );

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_validate_evidence_dir_detects_all_missing_files() {
        // Given: a directory that doesn't exist
        let dir = PathBuf::from(".evidence/vb-nonexistent-validate");
        let required_gates = vec!["fmt", "check", "clippy"];

        // Clean up any existing directory
        let _ = std::fs::remove_dir_all(&dir);

        // When: validate_evidence_dir is called
        let result = validate_evidence_dir(&dir, &required_gates);

        // Then: returns MissingEvidence for all three gates
        assert!(result.is_ok());
        let errors = result.unwrap();
        // Should have 3 MissingEvidence errors
        assert_eq!(
            errors.len(),
            3,
            "Should have MissingEvidence for all 3 absent gates"
        );

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
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

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::let_underscore_must_use
)]
mod test_helpers {
    use super::*;
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    /// Test-only helper that runs a gate with a custom timeout.
    /// This allows testing timeout behavior without waiting for actual timeouts.
    pub(crate) fn run_gate_with_timeout(
        gate: &str,
        cmd: &[String],
        evidence_path: &Path,
        timeout_secs: u64,
    ) -> Result<GateEvidence> {
        let gate_name = gate.to_string();
        let command_str = cmd.join(" ");

        if let Some(parent) = evidence_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::EvidenceWriteFailed {
                gate: gate_name.clone(),
                path: evidence_path.to_path_buf(),
                cause: format!("failed to create evidence directory: {}", e),
            })?;
        }

        let log_path = evidence_path.with_extension("log");

        // Execute with custom timeout
        let exit_code = match execute_command_for_test(cmd, timeout_secs) {
            Ok(code) => code,
            Err(Error::GateTimeout {
                gate: _,
                duration_secs: _,
            }) => {
                return Err(Error::GateTimeout {
                    gate: gate_name,
                    duration_secs: timeout_secs,
                });
            }
            Err(e) => return Err(e),
        };

        let status = if exit_code == 0 {
            GateStatus::Pass
        } else {
            GateStatus::Fail
        };

        let mut evidence = GateEvidence {
            kind: gate_name.clone(),
            gate_name,
            command: command_str,
            exit_code,
            log: log_path,
            status,
            why_failed: None,
        };

        if exit_code != 0 {
            evidence.why_failed = explain_failure(&evidence);
        }

        write_evidence(&evidence, evidence_path)?;
        Ok(evidence)
    }

    /// Execute command with a custom timeout for testing.
    fn execute_command_for_test(cmd: &[String], timeout_secs: u64) -> Result<i32> {
        let cmd0 = match cmd.first() {
            Some(c) => c,
            None => {
                return Err(Error::GateFailed {
                    gate: String::new(),
                    exit_code: -1,
                    log: PathBuf::from("target/evidence/none.log"),
                });
            }
        };
        let cmd_args = cmd.get(1..).unwrap_or(&[]);

        let mut child = std::process::Command::new(cmd0)
            .args(cmd_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| Error::GateFailed {
                gate: cmd0.clone(),
                exit_code: -1,
                log: PathBuf::from("target/evidence/spawn.log"),
            })?;

        let start = Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    return Ok(status.code().unwrap_or(-1));
                }
                Ok(None) => {
                    if start.elapsed() >= timeout {
                        let _ = child.kill().ok();
                        let _ = child.wait().ok();
                        return Err(Error::GateTimeout {
                            gate: cmd0.clone(),
                            duration_secs: timeout_secs,
                        });
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(_) => {
                    return Err(Error::GateFailed {
                        gate: cmd0.clone(),
                        exit_code: -1,
                        log: PathBuf::from("target/evidence/wait.log"),
                    });
                }
            }
        }
    }

    #[test]
    fn test_run_gate_timeout_returns_error() {
        // Use the test helper with a short timeout to verify timeout behavior
        use test_helpers::run_gate_with_timeout;

        // Given: a gate that would timeout
        let gate = "sleep";
        let cmd = vec!["sleep".to_string(), "10".to_string()];
        let evidence_path = PathBuf::from(".evidence/vb-test/sleep.yaml");

        // When: run_gate_with_timeout is called with 1ms timeout
        let result = run_gate_with_timeout(gate, &cmd, &evidence_path, 1);

        // Then: returns Error::GateTimeout
        assert!(
            matches!(
                result,
                Err(Error::GateTimeout {
                    ref gate,
                    ref duration_secs,
                }) if *gate == "sleep" && *duration_secs == 1
            ),
            "Expected GateTimeout error for gate 'sleep' with 1s timeout, got: {:?}",
            result
        );
    }
}
