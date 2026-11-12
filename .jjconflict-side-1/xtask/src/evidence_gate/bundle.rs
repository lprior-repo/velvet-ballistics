use serde::{Deserialize, Serialize};

use super::{
    ApiSurfaceRecord, AuditResult, BenchmarkEvidence, BloatRecord, EvidenceGateFailure,
    SemverRecord,
};

/// Evidence bundle collecting all release evidence categories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBundle {
    pub supply_chain_audits: Vec<AuditResult>,
    pub api_surface: Option<ApiSurfaceRecord>,
    pub semver_record: Option<SemverRecord>,
    pub bloat_analysis: Option<BloatRecord>,
    pub benchmark_evidence: Vec<BenchmarkEvidence>,
    pub kernel_paths_covered: Vec<String>,
    pub captured_at: String,
    pub toolchain: String,
    pub host_cpu: String,
}

impl EvidenceBundle {
    pub fn new(toolchain: String, host_cpu: String, captured_at: String) -> Self {
        Self {
            supply_chain_audits: Vec::new(),
            api_surface: None,
            semver_record: None,
            bloat_analysis: None,
            benchmark_evidence: Vec::new(),
            kernel_paths_covered: Vec::new(),
            captured_at,
            toolchain,
            host_cpu,
        }
    }

    /// Check if all required evidence categories are present.
    pub fn is_complete(&self) -> bool {
        !self.supply_chain_audits.is_empty()
            && self.api_surface.is_some()
            && self.semver_record.is_some()
            && self.bloat_analysis.is_some()
            && !self.benchmark_evidence.is_empty()
            && !self.kernel_paths_covered.is_empty()
    }

    /// Check if any supply-chain audit failed.
    pub fn has_audit_failure(&self) -> bool {
        self.supply_chain_audits.iter().any(|a| !a.passed)
    }

    /// Check if any benchmark evidence is missing baseline metadata.
    pub fn has_missing_benchmark_baseline(&self) -> bool {
        self.benchmark_evidence.iter().any(|b| !b.has_baseline())
    }

    /// Check if any benchmark evidence is missing required metadata.
    pub fn has_incomplete_benchmark_metadata(&self) -> bool {
        self.benchmark_evidence.iter().any(|b| !b.is_complete())
    }

    /// Validate all evidence gates. Returns a list of gate failures.
    pub fn validate_gates(&self) -> Vec<EvidenceGateFailure> {
        let mut failures = Vec::new();

        if self.supply_chain_audits.is_empty() {
            failures.push(EvidenceGateFailure::MissingSupplyChainEvidence);
        }

        if self.has_audit_failure() {
            let failed_tools: Vec<&str> = self
                .supply_chain_audits
                .iter()
                .filter(|a| !a.passed)
                .map(|a| a.tool.as_str())
                .collect();
            failures.push(EvidenceGateFailure::AuditFailure {
                tools: failed_tools.iter().map(|s| s.to_string()).collect(),
            });
        }

        if self.api_surface.is_none() {
            failures.push(EvidenceGateFailure::MissingApiSurfaceEvidence);
        }

        if self.semver_record.is_none() {
            failures.push(EvidenceGateFailure::MissingSemverEvidence);
        }

        if self.bloat_analysis.is_none() {
            failures.push(EvidenceGateFailure::MissingBloatAnalysis);
        }

        if self.benchmark_evidence.is_empty() {
            failures.push(EvidenceGateFailure::MissingBenchmarkEvidence);
        }

        if self.has_missing_benchmark_baseline() {
            let missing: Vec<&str> = self
                .benchmark_evidence
                .iter()
                .filter(|b| !b.has_baseline())
                .map(|b| b.name.as_str())
                .collect();
            failures.push(EvidenceGateFailure::MissingBenchmarkBaseline {
                benchmarks: missing.iter().map(|s| s.to_string()).collect(),
            });
        }

        if self.has_incomplete_benchmark_metadata() {
            let incomplete: Vec<&str> = self
                .benchmark_evidence
                .iter()
                .filter(|b| !b.is_complete())
                .map(|b| b.name.as_str())
                .collect();
            failures.push(EvidenceGateFailure::IncompleteBenchmarkMetadata {
                benchmarks: incomplete.iter().map(|s| s.to_string()).collect(),
            });
        }

        let required_kernel_paths = [
            "yaml_parse",
            "compile_validate",
            "expression",
            "runtime_core",
            "storage_ipc",
        ];
        let missing_kernel: Vec<&str> = required_kernel_paths
            .iter()
            .filter(|p| !self.kernel_paths_covered.iter().any(|k| k.contains(**p)))
            .copied()
            .collect();
        if !missing_kernel.is_empty() {
            failures.push(EvidenceGateFailure::MissingKernelPathEvidence {
                paths: missing_kernel.iter().map(|s| s.to_string()).collect(),
            });
        }

        failures
    }

    /// Returns true if all evidence gates pass.
    pub fn all_gates_pass(&self) -> bool {
        self.validate_gates().is_empty()
    }
}
