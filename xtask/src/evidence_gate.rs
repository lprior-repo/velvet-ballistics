//! Evidence gate types and validation for supply-chain, API, semver, bloat, and performance evidence.

mod bundle;
mod criterion;
mod failure;
mod types;

pub use bundle::EvidenceBundle;
pub use criterion::{enrich_benchmark_evidence, parse_criterion_output, required_kernel_groups};
pub use failure::EvidenceGateFailure;
pub use types::{
    ApiSurfaceRecord, AuditResult, BenchmarkEvidence, BloatContributor, BloatRecord, SemverRecord,
};

#[cfg(test)]
#[path = "evidence_gate/tests.rs"]
mod tests;
