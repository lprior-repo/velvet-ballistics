use serde::{Deserialize, Serialize};

/// Evidence gate failure types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceGateFailure {
    MissingSupplyChainEvidence,
    AuditFailure { tools: Vec<String> },
    MissingApiSurfaceEvidence,
    MissingSemverEvidence,
    MissingBloatAnalysis,
    MissingBenchmarkEvidence,
    MissingBenchmarkBaseline { benchmarks: Vec<String> },
    IncompleteBenchmarkMetadata { benchmarks: Vec<String> },
    MissingKernelPathEvidence { paths: Vec<String> },
}

impl std::fmt::Display for EvidenceGateFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSupplyChainEvidence => {
                write!(f, "supply-chain evidence is missing")
            }
            Self::AuditFailure { tools } => {
                write!(f, "audit failure in tools: {}", tools.join(", "))
            }
            Self::MissingApiSurfaceEvidence => {
                write!(f, "public API surface evidence is missing")
            }
            Self::MissingSemverEvidence => {
                write!(f, "semver stability evidence is missing")
            }
            Self::MissingBloatAnalysis => {
                write!(f, "binary bloat analysis is missing")
            }
            Self::MissingBenchmarkEvidence => {
                write!(f, "benchmark evidence is missing")
            }
            Self::MissingBenchmarkBaseline { benchmarks } => {
                write!(
                    f,
                    "benchmark baseline missing for: {}",
                    benchmarks.join(", ")
                )
            }
            Self::IncompleteBenchmarkMetadata { benchmarks } => {
                write!(
                    f,
                    "incomplete benchmark metadata for: {}",
                    benchmarks.join(", ")
                )
            }
            Self::MissingKernelPathEvidence { paths } => {
                write!(f, "kernel path evidence missing for: {}", paths.join(", "))
            }
        }
    }
}
