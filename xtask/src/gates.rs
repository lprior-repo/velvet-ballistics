//! Gate command wrappers for xtask command-center gates.

use crate::evidence::{GateEvidence, Result, command_for_gate, evidence_path, run_gate};

/// Gate identifiers matching command-center requirements.
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
    pub fn command(self) -> Result<Vec<String>> {
        command_for_gate(self.name())
    }

    /// Returns the evidence file name for this gate.
    pub fn evidence_file(self) -> String {
        format!("{}.yaml", self.name())
    }
}

pub fn run_fmt_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_gate_by_id(Gate::Fmt, bead_id)
}

pub fn run_check_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_gate_by_id(Gate::Check, bead_id)
}

pub fn run_clippy_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_gate_by_id(Gate::Clippy, bead_id)
}

pub fn run_nextest_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_gate_by_id(Gate::Nextest, bead_id)
}

pub fn run_forbidden_scan_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_gate_by_id(Gate::ForbiddenScan, bead_id)
}

pub fn run_hotpath_scan_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_gate_by_id(Gate::HotpathScan, bead_id)
}

pub fn run_miri_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_gate_by_id(Gate::Miri, bead_id)
}

pub fn run_mutants_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_gate_by_id(Gate::Mutants, bead_id)
}

pub fn run_llvm_cov_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_gate_by_id(Gate::LlvmCov, bead_id)
}

pub fn run_fuzz_build_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_gate_by_id(Gate::FuzzBuild, bead_id)
}

pub fn run_supply_chain_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_gate_by_id(Gate::SupplyChain, bead_id)
}

pub fn run_fuzz_smoke_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_gate_by_id(Gate::FuzzSmoke, bead_id)
}

pub fn run_coverage_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_gate_by_id(Gate::Coverage, bead_id)
}

pub fn run_mutants_smoke_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_gate_by_id(Gate::MutantsSmoke, bead_id)
}

pub fn run_bench_build_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_gate_by_id(Gate::BenchBuild, bead_id)
}

pub fn run_feature_powerset_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_gate_by_id(Gate::FeaturePowerset, bead_id)
}

pub fn run_source_length_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_gate_by_id(Gate::SourceLength, bead_id)
}

pub fn run_maxperf_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_gate_by_id(Gate::Maxperf, bead_id)
}

fn run_gate_by_id(gate: Gate, bead_id: Option<&str>) -> Result<GateEvidence> {
    let evidence_path = evidence_path(bead_scope(bead_id), gate.name());
    let command = gate.command()?;
    run_gate(gate.name(), &command, &evidence_path)
}

fn bead_scope(bead_id: Option<&str>) -> &str {
    bead_id.map_or("default", |id| id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_names_match_expected_identifiers() {
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

    #[test]
    fn gate_commands_delegate_to_fail_closed_mapper() {
        assert_eq!(Gate::Fmt.command(), command_for_gate("fmt"));
        assert_eq!(Gate::Clippy.command(), command_for_gate("clippy"));
        assert_eq!(Gate::Miri.command(), command_for_gate("miri"));
    }

    #[test]
    fn ai_fast_gates_all_have_commands_without_running_them() {
        let gates = [
            Gate::Fmt,
            Gate::Check,
            Gate::Clippy,
            Gate::Nextest,
            Gate::ForbiddenScan,
            Gate::HotpathScan,
        ];

        assert!(
            gates
                .iter()
                .all(|gate| matches!(gate.command(), Ok(ref command) if !command.is_empty()))
        );
        assert!(
            gates
                .iter()
                .all(|gate| gate.evidence_file().ends_with(".yaml"))
        );
    }

    #[test]
    fn ai_deep_gates_all_have_commands_without_running_them() {
        let gates = [Gate::Miri, Gate::Mutants, Gate::LlvmCov, Gate::FuzzBuild];
        assert!(
            gates
                .iter()
                .all(|gate| matches!(gate.command(), Ok(ref command) if !command.is_empty()))
        );
    }

    #[test]
    fn ai_release_gates_all_have_commands_without_running_them() {
        let gates = [
            Gate::Check,
            Gate::Nextest,
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

        assert!(
            gates
                .iter()
                .all(|gate| matches!(gate.command(), Ok(ref command) if !command.is_empty()))
        );
    }

    #[test]
    fn runner_scopes_evidence_without_spawning_gate() {
        let gate = Gate::Fmt;
        let scoped = evidence_path(bead_scope(Some("vb-test")), gate.name());
        assert_eq!(
            scoped,
            std::path::PathBuf::from(".evidence/vb-test/fmt.yaml")
        );
        assert!(matches!(gate.command(), Ok(ref command) if command == &["moon", "run", ":fmt"]));
    }

    #[test]
    fn gate_evidence_file_names() {
        assert_eq!(Gate::Fmt.evidence_file(), "fmt.yaml");
        assert_eq!(Gate::Clippy.evidence_file(), "clippy.yaml");
        assert_eq!(Gate::Miri.evidence_file(), "miri.yaml");
        assert_eq!(Gate::ForbiddenScan.evidence_file(), "forbidden-scan.yaml");
        assert_eq!(Gate::HotpathScan.evidence_file(), "hotpath-scan.yaml");
    }
}
