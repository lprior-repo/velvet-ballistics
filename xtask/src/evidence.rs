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
use std::time::{Duration, Instant};

const GATE_TIMEOUT_SECS: u64 = 600;
const POLL_INTERVAL_MS: u64 = 50;

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
    /// Bead ID is invalid or would escape the evidence directory.
    InvalidBeadId { bead: String },
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
            Error::InvalidBeadId { bead } => {
                write!(f, "Invalid bead id: '{}'", bead)
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

/// Captured output from a real bounded command execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub type GateRunner = fn(&str, &[String], &Path) -> Result<GateEvidence>;

/// Executes a single gate command and serializes evidence.
pub fn run_gate(gate: &str, cmd: &[String], evidence_path: &Path) -> Result<GateEvidence> {
    let log_path = evidence_path.with_extension("log");
    let output = execute_command(gate, cmd, &log_path)?;
    let status = if output.exit_code == 0 {
        GateStatus::Pass
    } else {
        GateStatus::Fail
    };
    let mut evidence = GateEvidence {
        kind: gate.to_string(),
        gate_name: gate.to_string(),
        command: cmd.join(" "),
        exit_code: output.exit_code,
        log: log_path,
        status,
        why_failed: None,
    };
    evidence.why_failed = explain_failure(&evidence);
    write_evidence(&evidence, evidence_path)?;
    Ok(evidence)
}

fn execute_command(gate: &str, cmd: &[String], log_path: &Path) -> Result<CommandOutput> {
    let (program, args) = cmd.split_first().ok_or_else(|| Error::GateFailed {
        gate: gate.to_string(),
        exit_code: -1,
        log: log_path.to_path_buf(),
    })?;
    let mut child = std::process::Command::new(program)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|_e| Error::GateFailed {
            gate: gate.to_string(),
            exit_code: -1,
            log: log_path.to_path_buf(),
        })?;
    let started_at = Instant::now();
    let timeout = Duration::from_secs(GATE_TIMEOUT_SECS);
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                let output = child.wait_with_output().map_err(|_e| Error::GateFailed {
                    gate: gate.to_string(),
                    exit_code: -1,
                    log: log_path.to_path_buf(),
                })?;
                let exit_code = output.status.code().map_or(-1, |code| code);
                write_raw_log(log_path, &output.stdout, &output.stderr)?;
                return Ok(CommandOutput {
                    exit_code,
                    stdout: output.stdout,
                    stderr: output.stderr,
                });
            }
            Ok(None) if started_at.elapsed() >= timeout => {
                child.kill().map_err(|_e| Error::GateFailed {
                    gate: gate.to_string(),
                    exit_code: -1,
                    log: log_path.to_path_buf(),
                })?;
                child.wait().map_err(|_e| Error::GateFailed {
                    gate: gate.to_string(),
                    exit_code: -1,
                    log: log_path.to_path_buf(),
                })?;
                write_raw_log(log_path, b"gate timed out", &[])?;
                return Err(Error::GateTimeout {
                    gate: gate.to_string(),
                    duration_secs: GATE_TIMEOUT_SECS,
                });
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS)),
            Err(e) => {
                write_raw_log(log_path, e.to_string().as_bytes(), &[])?;
                return Err(Error::GateFailed {
                    gate: gate.to_string(),
                    exit_code: -1,
                    log: log_path.to_path_buf(),
                });
            }
        }
    }
}

fn write_raw_log(path: &Path, stdout: &[u8], stderr: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::EvidenceWriteFailed {
            gate: "raw-log".to_string(),
            path: path.to_path_buf(),
            cause: e.to_string(),
        })?;
    }
    let mut data = Vec::new();
    data.extend_from_slice(stdout);
    if !stderr.is_empty() {
        data.extend_from_slice(b"\n--- stderr ---\n");
        data.extend_from_slice(stderr);
    }
    std::fs::write(path, data).map_err(|e| Error::EvidenceWriteFailed {
        gate: "raw-log".to_string(),
        path: path.to_path_buf(),
        cause: e.to_string(),
    })
}

/// Returns the concrete command for a known gate. Unknown gates fail closed.
pub fn command_for_gate(gate: &str) -> Result<Vec<String>> {
    let cmd = match gate {
        "fmt" => ["moon", "run", ":fmt"].as_slice(),
        "check" => ["moon", "run", ":check"].as_slice(),
        "clippy" => ["moon", "run", ":lint-src"].as_slice(),
        "nextest" | "test" => ["moon", "run", ":test"].as_slice(),
        "forbidden-scan" => ["moon", "run", ":lint-src"].as_slice(),
        "hotpath-scan" => ["moon", "run", ":source-length"].as_slice(),
        "miri" => ["moon", "run", ":miri"].as_slice(),
        "mutants" => ["moon", "run", ":mutants-smoke"].as_slice(),
        "llvm-cov" | "coverage" => ["moon", "run", ":coverage"].as_slice(),
        "fuzz-build" => ["moon", "run", ":fuzz-smoke"].as_slice(),
        "supply-chain" => ["moon", "run", ":supply-chain"].as_slice(),
        "fuzz-smoke" => ["moon", "run", ":fuzz-smoke"].as_slice(),
        "mutants-smoke" => ["moon", "run", ":mutants-smoke"].as_slice(),
        "bench-build" => ["moon", "run", ":bench-build"].as_slice(),
        "feature-powerset" => ["moon", "run", ":feature-powerset"].as_slice(),
        "source-length" => ["moon", "run", ":source-length"].as_slice(),
        "maxperf" => ["moon", "run", ":maxperf"].as_slice(),
        other => {
            return Err(Error::SubcommandNotFound {
                name: other.to_string(),
            });
        }
    };
    Ok(cmd.iter().map(|part| (*part).to_string()).collect())
}

/// Validate bead IDs before building evidence paths.
pub fn validate_bead_id(bead_id: &str) -> Result<()> {
    let valid = !bead_id.is_empty()
        && !bead_id.contains("..")
        && bead_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
    if valid {
        Ok(())
    } else {
        Err(Error::InvalidBeadId {
            bead: bead_id.to_string(),
        })
    }
}

/// Runs all gates in a profile and aggregates evidence.
pub fn run_profile(
    profile: GateProfile,
    bead_id: Option<&str>,
    output_dir: &Path,
) -> Result<ProfileEvidence> {
    run_profile_with_runner(profile, bead_id, output_dir, run_gate)
}

/// Runs all gates in a profile using an injected runner.
pub fn run_profile_with_runner(
    profile: GateProfile,
    bead_id: Option<&str>,
    output_dir: &Path,
    runner: GateRunner,
) -> Result<ProfileEvidence> {
    let scope = bead_id.map_or("default", |id| id);
    validate_bead_id(scope)?;
    let mut gates = Vec::new();
    for gate_name in profile.gates() {
        let evidence_file = output_dir.join(format!("{}.yaml", gate_name));
        if let Some(parent) = evidence_file.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::BeadDirectoryCreationFailed {
                bead: scope.to_string(),
                cause: e.to_string(),
            })?;
        }
        let cmd = command_for_gate(gate_name)?;
        gates.push(runner(gate_name, &cmd, &evidence_file)?);
    }
    let all_passed = gates.iter().all(|g| g.status == GateStatus::Pass);
    let exit_code = if all_passed { 0 } else { 1 };
    let profile_evidence = ProfileEvidence {
        profile: format!("{:?}", profile),
        gates,
        exit_code,
    };
    write_profile_evidence(&profile_evidence, &output_dir.join(profile.evidence_file()))?;
    Ok(profile_evidence)
}

fn write_profile_evidence(evidence: &ProfileEvidence, path: &Path) -> Result<()> {
    let yaml = serde_saphyr::to_string(evidence).map_err(|e| Error::YamlSerializationFailed {
        gate: evidence.profile.clone(),
        cause: e.to_string(),
    })?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::EvidenceWriteFailed {
            gate: evidence.profile.clone(),
            path: path.to_path_buf(),
            cause: e.to_string(),
        })?;
    }
    std::fs::write(path, yaml).map_err(|e| Error::EvidenceWriteFailed {
        gate: evidence.profile.clone(),
        path: path.to_path_buf(),
        cause: e.to_string(),
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

    #[test]
    fn test_command_for_gate_known_gate_returns_command() {
        let result = command_for_gate("fmt");
        assert!(matches!(result, Ok(ref command) if command == &["moon", "run", ":fmt"]));
    }

    #[test]
    fn test_command_for_gate_unknown_gate_fails_closed() {
        let result = command_for_gate("unknown-gate");
        assert!(matches!(result, Err(Error::SubcommandNotFound { .. })));
    }

    // ========================================================================
    // validate_evidence_dir Tests (INV-001, ERR-003)
    // ========================================================================

    #[test]
    fn test_validate_evidence_dir_returns_missing_for_absent_file() {
        // Given: a directory with some evidence files but missing clippy
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let dir = temp.path();
        std::fs::write(dir.join("fmt.yaml"), "kind: fmt").expect("fixture should be written");
        let required_gates = vec!["fmt", "clippy", "nextest"];

        // When: validate_evidence_dir is called
        let result = validate_evidence_dir(&dir, &required_gates);

        // Then: returns MissingEvidence for clippy
        // RED_PHASE: Currently returns Ok(vec![]) - should return Err with MissingEvidence
        assert!(
            result.is_ok(),
            "validate_evidence_dir should return Ok(vec![]) or Err"
        );
        let errors = result.expect("validation should not fail");
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
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let dir = temp.path();
        let required_gates = vec!["fmt", "check", "clippy"];

        // When: validate_evidence_dir is called
        let result = validate_evidence_dir(&dir, &required_gates);

        // Then: returns MissingEvidence for all three gates
        // RED_PHASE: Currently returns Ok(vec![])
        assert!(result.is_ok());
        let errors = result.expect("validation should not fail");
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
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let output_dir = temp.path();

        // When: run_profile is called
        let result = run_profile_with_runner(profile, bead_id, output_dir, fake_runner);

        // Then: returns ProfileEvidence with all gates
        // RED_PHASE: Currently returns Error::SubcommandNotFound
        assert!(
            result.is_ok(),
            "run_profile should return Ok(ProfileEvidence), got: {:?}",
            result
        );
    }

    fn fake_runner(gate: &str, cmd: &[String], evidence_path: &Path) -> Result<GateEvidence> {
        let evidence = GateEvidence {
            kind: gate.to_string(),
            gate_name: gate.to_string(),
            command: cmd.join(" "),
            exit_code: 0,
            log: evidence_path.with_extension("log"),
            status: GateStatus::Pass,
            why_failed: None,
        };
        write_evidence(&evidence, evidence_path)?;
        Ok(evidence)
    }
}
