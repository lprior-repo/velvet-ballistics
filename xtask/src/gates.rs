//! Gate command wrappers for xtask command-center gates.
//!
//! This module provides the individual gate implementations that execute
//! the actual quality checks (fmt, clippy, nextest, miri, etc.).
//! RED_PHASE: All gate runners are stubs that return SubcommandNotFound.

#![allow(dead_code)]
//!
//! Each gate follows the pattern:
//! 1. Execute the underlying command
//! 2. Capture exit code and log output
//! 3. Return evidence bundle via `run_gate`

use crate::evidence::{run_gate, GateEvidence, Result};

/// Gate identifiers matching Section 77.1 requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gate {
    Fmt,
    Check,
    Clippy,
    Nextest,
    ForbiddenScan,
    HotpathScan,
    Miri,
    Mutants,
    LlvmCov,
    FuzzBuild,
    SupplyChain,
    FuzzSmoke,
    Coverage,
    MutantsSmoke,
    BenchBuild,
    FeaturePowerset,
    SourceLength,
    Maxperf,
}

impl Gate {
    /// Returns the gate name string.
    pub fn name(self) -> &'static str {
        match self {
            Gate::Fmt => "fmt",
            Gate::Check => "check",
            Gate::Clippy => "clippy",
            Gate::Nextest => "nextest",
            Gate::ForbiddenScan => "forbidden-scan",
            Gate::HotpathScan => "hotpath-scan",
            Gate::Miri => "miri",
            Gate::Mutants => "mutants",
            Gate::LlvmCov => "llvm-cov",
            Gate::FuzzBuild => "fuzz-build",
            Gate::SupplyChain => "supply-chain",
            Gate::FuzzSmoke => "fuzz-smoke",
            Gate::Coverage => "coverage",
            Gate::MutantsSmoke => "mutants-smoke",
            Gate::BenchBuild => "bench-build",
            Gate::FeaturePowerset => "feature-powerset",
            Gate::SourceLength => "source-length",
            Gate::Maxperf => "maxperf",
        }
    }

    /// Returns the command arguments to execute.
    pub fn command(self) -> Vec<String> {
        match self {
            Gate::Fmt => vec![
                "cargo".to_string(),
                "+nightly".to_string(),
                "fmt".to_string(),
                "--all".to_string(),
            ],
            Gate::Check => vec!["moon".to_string(), "run".to_string(), ":check".to_string()],
            Gate::Clippy => vec![
                "cargo".to_string(),
                "+nightly".to_string(),
                "clippy".to_string(),
                "--workspace".to_string(),
            ],
            Gate::Nextest => vec![
                "cargo".to_string(),
                "nextest".to_string(),
                "run".to_string(),
                "--workspace".to_string(),
            ],
            Gate::ForbiddenScan => vec![
                "bash".to_string(),
                "scripts/forbidden-scan.sh".to_string(),
            ],
            Gate::HotpathScan => vec![
                "bash".to_string(),
                "scripts/hotpath-scan.sh".to_string(),
            ],
            Gate::Miri => vec![
                "cargo".to_string(),
                "+nightly".to_string(),
                "miri".to_string(),
                "test".to_string(),
                "--workspace".to_string(),
            ],
            Gate::Mutants => vec![
                "cargo".to_string(),
                "mutants".to_string(),
                "--package".to_string(),
                "velvet_ballastics".to_string(),
            ],
            Gate::LlvmCov => vec!["cargo".to_string(), "llvm-cov".to_string()],
            Gate::FuzzBuild => vec!["cargo".to_string(), "fuzz".to_string(), "build".to_string()],
            Gate::SupplyChain => vec!["moon".to_string(), "run".to_string(), ":supply-chain".to_string()],
            Gate::FuzzSmoke => vec!["moon".to_string(), "run".to_string(), ":fuzz-smoke".to_string()],
            Gate::Coverage => vec!["moon".to_string(), "run".to_string(), ":coverage".to_string()],
            Gate::MutantsSmoke => vec![
                "moon".to_string(),
                "run".to_string(),
                ":mutants-smoke".to_string(),
            ],
            Gate::BenchBuild => vec!["moon".to_string(), "run".to_string(), ":bench-build".to_string()],
            Gate::FeaturePowerset => vec![
                "moon".to_string(),
                "run".to_string(),
                ":feature-powerset".to_string(),
            ],
            Gate::SourceLength => vec![
                "bash".to_string(),
                "scripts/check-source-length.sh".to_string(),
            ],
            Gate::Maxperf => vec!["moon".to_string(), "run".to_string(), ":maxperf".to_string()],
        }
    }

    /// Returns the evidence file name for this gate.
    pub fn evidence_file(self) -> String {
        format!("{}.yaml", self.name())
    }
}

/// Runs the fmt gate.
///
/// Executes `cargo +nightly fmt --all` and returns evidence.
pub fn run_fmt_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    let gate = Gate::Fmt;
    let evidence_path = crate::evidence::evidence_path(
        bead_id.unwrap_or("default"),
        gate.name(),
    );
    run_gate(gate.name(), &gate.command(), &evidence_path)
}

/// Runs the check gate.
///
/// Executes `moon run :check` and returns evidence.
pub fn run_check_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    let gate = Gate::Check;
    let evidence_path = crate::evidence::evidence_path(
        bead_id.unwrap_or("default"),
        gate.name(),
    );
    run_gate(gate.name(), &gate.command(), &evidence_path)
}

/// Runs the clippy gate.
///
/// Executes `cargo +nightly clippy --workspace` and returns evidence.
pub fn run_clippy_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    let gate = Gate::Clippy;
    let evidence_path = crate::evidence::evidence_path(
        bead_id.unwrap_or("default"),
        gate.name(),
    );
    run_gate(gate.name(), &gate.command(), &evidence_path)
}

/// Runs the nextest gate.
///
/// Executes `cargo nextest run --workspace` and returns evidence.
pub fn run_nextest_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    let gate = Gate::Nextest;
    let evidence_path = crate::evidence::evidence_path(
        bead_id.unwrap_or("default"),
        gate.name(),
    );
    run_gate(gate.name(), &gate.command(), &evidence_path)
}

/// Runs the forbidden-scan gate.
///
/// Executes the forbidden pattern scan script and returns evidence.
pub fn run_forbidden_scan_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    let gate = Gate::ForbiddenScan;
    let evidence_path = crate::evidence::evidence_path(
        bead_id.unwrap_or("default"),
        gate.name(),
    );
    run_gate(gate.name(), &gate.command(), &evidence_path)
}

/// Runs the hotpath-scan gate.
///
/// Executes the hotpath scan script and returns evidence.
pub fn run_hotpath_scan_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    let gate = Gate::HotpathScan;
    let evidence_path = crate::evidence::evidence_path(
        bead_id.unwrap_or("default"),
        gate.name(),
    );
    run_gate(gate.name(), &gate.command(), &evidence_path)
}

/// Runs the miri gate.
///
/// Executes `cargo +nightly miri test --workspace` and returns evidence.
pub fn run_miri_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    let gate = Gate::Miri;
    let evidence_path = crate::evidence::evidence_path(
        bead_id.unwrap_or("default"),
        gate.name(),
    );
    run_gate(gate.name(), &gate.command(), &evidence_path)
}

/// Runs the mutants gate.
///
/// Executes `cargo mutants --package velvet_ballastics` and returns evidence.
pub fn run_mutants_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    let gate = Gate::Mutants;
    let evidence_path = crate::evidence::evidence_path(
        bead_id.unwrap_or("default"),
        gate.name(),
    );
    run_gate(gate.name(), &gate.command(), &evidence_path)
}

/// Runs the llvm-cov gate.
///
/// Executes `cargo llvm-cov` and returns evidence.
pub fn run_llvm_cov_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    let gate = Gate::LlvmCov;
    let evidence_path = crate::evidence::evidence_path(
        bead_id.unwrap_or("default"),
        gate.name(),
    );
    run_gate(gate.name(), &gate.command(), &evidence_path)
}

/// Runs the fuzz-build gate.
///
/// Executes `cargo fuzz build` and returns evidence.
pub fn run_fuzz_build_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    let gate = Gate::FuzzBuild;
    let evidence_path = crate::evidence::evidence_path(
        bead_id.unwrap_or("default"),
        gate.name(),
    );
    run_gate(gate.name(), &gate.command(), &evidence_path)
}

/// Runs the supply-chain gate.
///
/// Delegates to moon `:supply-chain` and returns evidence.
pub fn run_supply_chain_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    let gate = Gate::SupplyChain;
    let evidence_path = crate::evidence::evidence_path(
        bead_id.unwrap_or("default"),
        gate.name(),
    );
    run_gate(gate.name(), &gate.command(), &evidence_path)
}

/// Runs the fuzz-smoke gate.
///
/// Delegates to moon `:fuzz-smoke` and returns evidence.
pub fn run_fuzz_smoke_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    let gate = Gate::FuzzSmoke;
    let evidence_path = crate::evidence::evidence_path(
        bead_id.unwrap_or("default"),
        gate.name(),
    );
    run_gate(gate.name(), &gate.command(), &evidence_path)
}

/// Runs the coverage gate.
///
/// Delegates to moon `:coverage` and returns evidence.
pub fn run_coverage_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    let gate = Gate::Coverage;
    let evidence_path = crate::evidence::evidence_path(
        bead_id.unwrap_or("default"),
        gate.name(),
    );
    run_gate(gate.name(), &gate.command(), &evidence_path)
}

/// Runs the mutants-smoke gate.
///
/// Delegates to moon `:mutants-smoke` and returns evidence.
pub fn run_mutants_smoke_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    let gate = Gate::MutantsSmoke;
    let evidence_path = crate::evidence::evidence_path(
        bead_id.unwrap_or("default"),
        gate.name(),
    );
    run_gate(gate.name(), &gate.command(), &evidence_path)
}

/// Runs the bench-build gate.
///
/// Delegates to moon `:bench-build` and returns evidence.
pub fn run_bench_build_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    let gate = Gate::BenchBuild;
    let evidence_path = crate::evidence::evidence_path(
        bead_id.unwrap_or("default"),
        gate.name(),
    );
    run_gate(gate.name(), &gate.command(), &evidence_path)
}

/// Runs the feature-powerset gate.
///
/// Delegates to moon `:feature-powerset` and returns evidence.
pub fn run_feature_powerset_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    let gate = Gate::FeaturePowerset;
    let evidence_path = crate::evidence::evidence_path(
        bead_id.unwrap_or("default"),
        gate.name(),
    );
    run_gate(gate.name(), &gate.command(), &evidence_path)
}

/// Runs the source-length gate.
///
/// Executes `bash scripts/check-source-length.sh` and returns evidence.
pub fn run_source_length_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    let gate = Gate::SourceLength;
    let evidence_path = crate::evidence::evidence_path(
        bead_id.unwrap_or("default"),
        gate.name(),
    );
    run_gate(gate.name(), &gate.command(), &evidence_path)
}

/// Runs the maxperf gate.
///
/// Executes the maxperf build and returns evidence.
pub fn run_maxperf_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    let gate = Gate::Maxperf;
    let evidence_path = crate::evidence::evidence_path(
        bead_id.unwrap_or("default"),
        gate.name(),
    );
    run_gate(gate.name(), &gate.command(), &evidence_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Gate Name Tests
    // ========================================================================

    #[test]
    fn test_gate_names_match_expected_identifiers() {
        assert_eq!(Gate::Fmt.name(), "fmt");
        assert_eq!(Gate::Check.name(), "check");
        assert_eq!(Gate::Clippy.name(), "clippy");
        assert_eq!(Gate::Nextest.name(), "nextest");
        assert_eq!(Gate::ForbiddenScan.name(), "forbidden-scan");
        assert_eq!(Gate::HotpathScan.name(), "hotpath-scan");
        assert_eq!(Gate::Miri.name(), "miri");
        assert_eq!(Gate::Mutants.name(), "mutants");
        assert_eq!(Gate::LlvmCov.name(), "llvm-cov");
        assert_eq!(Gate::FuzzBuild.name(), "fuzz-build");
        assert_eq!(Gate::SupplyChain.name(), "supply-chain");
        assert_eq!(Gate::FuzzSmoke.name(), "fuzz-smoke");
        assert_eq!(Gate::Coverage.name(), "coverage");
        assert_eq!(Gate::MutantsSmoke.name(), "mutants-smoke");
        assert_eq!(Gate::BenchBuild.name(), "bench-build");
        assert_eq!(Gate::FeaturePowerset.name(), "feature-powerset");
        assert_eq!(Gate::SourceLength.name(), "source-length");
        assert_eq!(Gate::Maxperf.name(), "maxperf");
    }

    // ========================================================================
    // Gate Command Tests (POST-001/002/003)
    // ========================================================================

    #[test]
    fn test_fmt_gate_command() {
        let cmd = Gate::Fmt.command();
        assert!(cmd.contains(&"cargo".to_string()));
        assert!(cmd.contains(&"+nightly".to_string()));
        assert!(cmd.contains(&"fmt".to_string()));
        assert!(cmd.contains(&"--all".to_string()));
    }

    #[test]
    fn test_clippy_gate_command() {
        let cmd = Gate::Clippy.command();
        assert!(cmd.contains(&"cargo".to_string()));
        assert!(cmd.contains(&"+nightly".to_string()));
        assert!(cmd.contains(&"clippy".to_string()));
        assert!(cmd.contains(&"--workspace".to_string()));
    }

    #[test]
    fn test_miri_gate_command() {
        let cmd = Gate::Miri.command();
        assert!(cmd.contains(&"cargo".to_string()));
        assert!(cmd.contains(&"+nightly".to_string()));
        assert!(cmd.contains(&"miri".to_string()));
        assert!(cmd.contains(&"test".to_string()));
        assert!(cmd.contains(&"--workspace".to_string()));
    }

    #[test]
    fn test_ai_fast_gates_all_implemented() {
        let gates = [
            Gate::Fmt,
            Gate::Check,
            Gate::Clippy,
            Gate::Nextest,
            Gate::ForbiddenScan,
            Gate::HotpathScan,
        ];
        for gate in gates {
            let cmd = gate.command();
            assert!(!cmd.is_empty(), "Gate {} should have a command", gate.name());
            let evidence_file = gate.evidence_file();
            assert!(evidence_file.ends_with(".yaml"), "Evidence file should end with .yaml");
        }
    }

    #[test]
    fn test_ai_deep_gates_all_implemented() {
        let gates = [Gate::Miri, Gate::Mutants, Gate::LlvmCov, Gate::FuzzBuild];
        for gate in gates {
            let cmd = gate.command();
            assert!(!cmd.is_empty(), "Gate {} should have a command", gate.name());
        }
    }

    #[test]
    fn test_ai_release_gates_all_implemented() {
        let gates = [
            Gate::Check,
            Gate::Nextest, // test
            Gate::SupplyChain,
            Gate::Miri,
            Gate::FuzzSmoke,
            Gate::Coverage,
            Gate::MutantsSmoke,
            Gate::BenchBuild,
            Gate::FeaturePowerset,
            Gate::SourceLength,
            Gate::Maxperf,
        ];
        for gate in gates {
            let cmd = gate.command();
            assert!(!cmd.is_empty(), "Gate {} should have a command", gate.name());
        }
    }

    // ========================================================================
    // Individual Gate Runner Tests (POST-001/002/003)
    // ========================================================================

    #[test]
    fn test_run_fmt_gate_returns_evidence() {
        let result = run_fmt_gate(Some("vb-test"));
        // RED_PHASE: Currently returns Error::GateFailed { exit_code: 0, ... }
        // After implementation: should return Ok(GateEvidence) with exit_code=0
        assert!(result.is_ok(), "run_fmt_gate should return Ok(GateEvidence), got: {:?}", result);
    }

    #[test]
    fn test_run_clippy_gate_returns_evidence() {
        let result = run_clippy_gate(Some("vb-test"));
        // RED_PHASE: Currently returns Error
        // After implementation: should return Ok(GateEvidence)
        assert!(result.is_ok(), "run_clippy_gate should return Ok(GateEvidence), got: {:?}", result);
    }

    #[test]
    fn test_run_nextest_gate_returns_evidence() {
        let result = run_nextest_gate(Some("vb-test"));
        // RED_PHASE: Currently returns Error
        assert!(result.is_ok(), "run_nextest_gate should return Ok(GateEvidence), got: {:?}", result);
    }

    #[test]
    fn test_run_miri_gate_returns_evidence() {
        let result = run_miri_gate(Some("vb-test"));
        // RED_PHASE: Currently returns Error
        assert!(result.is_ok(), "run_miri_gate should return Ok(GateEvidence), got: {:?}", result);
    }

    // ========================================================================
    // Evidence File Name Tests
    // ========================================================================

    #[test]
    fn test_gate_evidence_file_names() {
        assert_eq!(Gate::Fmt.evidence_file(), "fmt.yaml");
        assert_eq!(Gate::Clippy.evidence_file(), "clippy.yaml");
        assert_eq!(Gate::Miri.evidence_file(), "miri.yaml");
        assert_eq!(Gate::ForbiddenScan.evidence_file(), "forbidden-scan.yaml");
        assert_eq!(Gate::HotpathScan.evidence_file(), "hotpath-scan.yaml");
    }
}
