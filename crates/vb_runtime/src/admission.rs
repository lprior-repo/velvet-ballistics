#![forbid(unsafe_code)]
//! Runtime admission control for workflow runs.
//!
//! `RunAdmission` records the artifact digest, granted capabilities,
//! and admission policy for each accepted run. `AdmissionError` enumerates
//! the reasons a submit may be rejected at the admission gate.

mod guards;
mod admission;
mod errors;
mod stores;
mod types;
mod validation;

// Public re-exports for crate users
pub use admission::{
    admit_artifact_run, admit_artifact_run_with_certificate_floor, admit_run,
    admit_run_with_budget, admit_run_with_budget_policy, check_capability,
};
pub use errors::{map_artifact_envelope_error, ArtifactEnvelopeError, AdmissionError};
pub use stores::{
    AcceptedArtifactStore, AlwaysPresentArtifactStore, ArtifactStore,
    MissingAcceptedArtifactStore, SharedAcceptedArtifactStore, SharedArtifactStore,
    StorageArtifactStore,
};
pub use types::{AdmissionBudgetRequest, RunAdmission, REQUIRED_GATE_COUNT};
pub use validation::validate_accepted_artifact_envelope;

// Re-export vb_core types for test compatibility
pub use vb_core::budget::{AggregateResourceBudget, AggregateResourceCapacity, BoundednessPolicy};
pub use vb_core::capability::{Capability, CapabilitySet};
pub use vb_core::ids::{ActionId, RunId, WorkflowDigest};
pub use vb_core::policy::RuntimePolicy;

// Test module paths
#[cfg(test)]
#[path = "admission/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "admission/artifact_envelope_tests.rs"]
mod artifact_envelope_tests;
