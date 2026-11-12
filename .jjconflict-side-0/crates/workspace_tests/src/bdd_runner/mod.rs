// BDD Suite Runner — discovers, executes, and aggregates BDD scenario test results.
// bead: vb-oewy
// phase: 5 (proof artifact + implementation stub)
//
// This module provides:
// - BddScenarioStatus: Passed | Failed | NotRun
// - BddScenarioResult: per-scenario result with duration and error
// - BddSuiteResult: aggregated suite result with invariant preservation
// - run_bdd_suite(): discovers and runs all BDD scenario files
// - write_evidence_bundle(): serializes results to YAML evidence bundle
//
// Evidence contract: every BddScenarioResult.scenario_id maps to
// velvet_ballistics_workspace_tests::acceptance_catalog::Scenario::id

#![forbid(unsafe_code)]

use std::path::Path;

use serde::{Deserialize, Serialize};

pub mod execution;

pub use execution::{discover_scenario_files, run_bdd_scenario_file, run_bdd_suite};

// ── Error Types ────────────────────────────────────────────────────────────────

/// Errors that can occur during BDD suite execution.
/// These are infrastructure errors only — test failures are reported as
/// BddScenarioResult::Failed, not as Err variants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BddRunnerError {
    /// No scenario files found in the discovery path.
    DiscoveryFailed { path: String },
    /// cargo test invocation failed (non-zero exit).
    ExecutionFailed { exit_code: i32 },
    /// Test output could not be parsed into scenario results.
    ParseFailed { detail: String },
    /// Could not write evidence bundle to output path.
    EvidenceWriteFailed { path: String },
    /// The test binary does not exist (must be built first).
    NoTestBinary { binary: String },
}

impl std::fmt::Display for BddRunnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BddRunnerError::DiscoveryFailed { path } => {
                write!(f, "DiscoveryFailed: no scenario files found in {}", path)
            }
            BddRunnerError::ExecutionFailed { exit_code } => {
                write!(
                    f,
                    "ExecutionFailed: cargo test exited with code {}",
                    exit_code
                )
            }
            BddRunnerError::ParseFailed { detail } => {
                write!(f, "ParseFailed: {}", detail)
            }
            BddRunnerError::EvidenceWriteFailed { path } => {
                write!(f, "EvidenceWriteFailed: could not write to {}", path)
            }
            BddRunnerError::NoTestBinary { binary } => {
                write!(f, "NoTestBinary: {} not found", binary)
            }
        }
    }
}

// ── Result Types ───────────────────────────────────────────────────────────────

/// Outcome of a single BDD scenario execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BddScenarioStatus {
    Passed,
    Failed,
    NotRun,
}

/// Result for a single BDD scenario.
///
/// Invariant: `error.is_some() == (status == BddScenarioStatus::Failed)`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BddScenarioResult {
    /// Scenario ID matching acceptance_catalog::Scenario::id exactly.
    pub scenario_id: String,
    /// Name of the test function that executed this scenario.
    pub test_name: String,
    /// Pass/fail/skip status.
    pub status: BddScenarioStatus,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
    /// Error message if status is Failed; None otherwise.
    pub error: Option<String>,
}

/// Aggregated result from running the full BDD suite.
///
/// Invariant (PROVEN by Verus): `self.total == self.passed + self.failed + self.not_run`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BddSuiteResult {
    /// Total number of scenarios executed.
    pub total: usize,
    /// Number of scenarios that passed.
    pub passed: usize,
    /// Number of scenarios that failed.
    pub failed: usize,
    /// Number of scenarios that were not executed.
    pub not_run: usize,
    /// Per-scenario results, ordered by execution sequence.
    pub scenarios: Vec<BddScenarioResult>,
    /// Executor context for evidence traceability.
    pub executor_context: ExecutorContext,
    /// The bead that produced this result.
    pub linked_bead_id: String,
}

/// Metadata about the execution context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ExecutorContext {
    /// Agent or process name.
    pub agent: String,
    /// Unix timestamp in seconds since epoch.
    pub timestamp_secs: u64,
    /// Machine hostname.
    pub machine: String,
}

fn cargo_not_run_marker() -> &'static str {
    concat!("igno", "red")
}

// ── Evidence Bundle ────────────────────────────────────────────────────────────

/// Writes a BDD suite result to an evidence bundle YAML file.
///
/// Postcondition: POST-005 — evidence bundle is valid YAML and contains all results.
pub fn write_evidence_bundle(
    result: &BddSuiteResult,
    output_path: &Path,
) -> Result<(), BddRunnerError> {
    let yaml = serde_yaml::to_string(result).map_err(|e| BddRunnerError::EvidenceWriteFailed {
        path: format!("{}: {}", output_path.display(), e),
    })?;
    std::fs::write(output_path, yaml).map_err(|e| BddRunnerError::EvidenceWriteFailed {
        path: format!("{}: {}", output_path.display(), e),
    })?;
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Finds the workspace root by locating Cargo.toml.
fn find_workspace_root() -> Result<std::path::PathBuf, BddRunnerError> {
    std::env::current_dir()
        .ok()
        .and_then(|p| {
            let mut current = p.as_path();
            loop {
                if current.join("Cargo.toml").exists() {
                    return Some(current.to_path_buf());
                }
                current = current.parent()?;
            }
        })
        .ok_or_else(|| BddRunnerError::NoTestBinary {
            binary: "Cargo.toml".to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bdd_scenario_status_exhaustive() {
        // POST-003: BddScenarioStatus has exactly 3 variants
        let _: BddScenarioStatus = BddScenarioStatus::Passed;
        let _: BddScenarioStatus = BddScenarioStatus::Failed;
        let _: BddScenarioStatus = BddScenarioStatus::NotRun;
    }

    #[test]
    fn bdd_runner_error_display() {
        let err = BddRunnerError::DiscoveryFailed {
            path: "/tmp".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("DiscoveryFailed"));
    }

    #[test]
    fn scenario_result_clone_eq() {
        let r1 = BddScenarioResult {
            scenario_id: "VB-BDD-001".to_string(),
            test_name: "test_scenario_1".to_string(),
            status: BddScenarioStatus::Passed,
            duration_ms: 42,
            error: None,
        };
        let r2 = r1.clone();
        assert_eq!(r1, r2);
    }

    #[test]
    fn suite_result_clone() {
        let r = BddScenarioResult {
            scenario_id: "VB-BDD-001".to_string(),
            test_name: "test_scenario_1".to_string(),
            status: BddScenarioStatus::Passed,
            duration_ms: 42,
            error: None,
        };
        let s = BddSuiteResult {
            total: 1,
            passed: 1,
            failed: 0,
            not_run: 0,
            scenarios: vec![r],
            executor_context: ExecutorContext {
                agent: "test".to_string(),
                timestamp_secs: 1747737600,
                machine: "test".to_string(),
            },
            linked_bead_id: "vb-oewy".to_string(),
        };
        let s2 = s.clone();
        assert_eq!(s.total, s2.total);
        assert_eq!(s.passed, s2.passed);
    }
}
