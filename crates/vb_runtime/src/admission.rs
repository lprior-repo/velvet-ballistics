#![forbid(unsafe_code)]
//! Runtime admission control for workflow runs.
//!
//! `RunAdmission` records the artifact digest, granted capabilities,
//! and admission policy for each accepted run. `AdmissionError` enumerates
//! the reasons a submit may be rejected at the admission gate.

#[path = "admission/admission.rs"]
mod admission_core;
mod budget_error_map;
mod errors;
mod guards;
mod stores;
mod types;
mod validation;

// Public re-exports for crate users
pub use admission_core::{
    admit_artifact_run, admit_artifact_run_with_certificate_floor, admit_run,
    admit_run_with_budget, admit_run_with_budget_policy, check_capability,
    per_workflow_step_ceiling, preflight_step_budget,
};
pub use errors::{AdmissionError, ArtifactEnvelopeError, map_artifact_envelope_error};
pub use stores::{
    AcceptedArtifactStore, AlwaysPresentArtifactStore, ArtifactStore, MissingAcceptedArtifactStore,
    SharedAcceptedArtifactStore, SharedArtifactStore, StorageArtifactStore,
};
#[cfg(feature = "test-util")]
pub use types::empty_workflow;
pub use types::{AdmissionBudgetRequest, REQUIRED_GATE_COUNT, RunAdmission};
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
#[path = "admission/step_budget_tests/mod.rs"]
mod step_budget_tests;

#[cfg(test)]
#[path = "admission/step_budget_policy_tests.rs"]
mod step_budget_policy_tests;

#[cfg(test)]
#[path = "admission/artifact_envelope_tests.rs"]
mod artifact_envelope_tests;
