#![forbid(unsafe_code)]
//! Runtime admission control for workflow runs.
//!
//! `RunAdmission` records the artifact digest, granted capabilities,
//! and admission policy for each accepted run. `AdmissionError` enumerates
//! the reasons a submit may be rejected at the admission gate.
//!
//! The implementation is split across focused chunks under `parts/`.
//! All chunks share the parent module's `use` declarations and are
//! `include!`-d into this shell to keep the public API and tests
//! unchanged. Splitting by domain responsibility:
//!
//! - `chunk_001_types_errors_traits` - REQUIRED_GATE_COUNT, error enums,
//!   and the `ArtifactStore` / `AcceptedArtifactStore` trait surface.
//! - `chunk_002_records` - `RunAdmission` and `AdmissionBudgetRequest`
//!   value types and their accessors.
//! - `chunk_003_stores` - `AlwaysPresentArtifactStore`,
//!   `MissingAcceptedArtifactStore`, and `StorageArtifactStore`
//!   implementations of the artifact store traits.
//! - `chunk_004_validation` - envelope validation helpers and
//!   `ArtifactEnvelopeError` -> `AdmissionError` mapping.
//! - `chunk_005_admit_core` - the policy-dispatched
//!   `admit_run` / `admit_artifact_run` entry points and the strict
//!   `admit_artifact_run_with_certificate_floor` path.
//! - `chunk_006_admit_budget` - aggregate resource budget admission
//!   (`admit_run_with_budget`, `admit_run_with_budget_policy`,
//!   `map_budget_error`) and `check_capability`.

use std::sync::Arc;
use thiserror::Error;
use vb_core::budget::{
    AggregateBudgetError, AggregateResourceBudget, AggregateResourceCapacity,
    AggregateResourceUsage, BoundednessPolicy, validate_aggregate_budget,
};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_storage::EventSeq;

include!("admission/parts/chunk_001_types_errors_traits.rs");
include!("admission/parts/chunk_002_records.rs");
include!("admission/parts/chunk_003_stores.rs");
include!("admission/parts/chunk_004_validation.rs");
include!("admission/parts/chunk_005_admit_core.rs");
include!("admission/parts/chunk_006_admit_budget.rs");

#[cfg(test)]
#[path = "admission/tests.rs"]
mod tests;

#[cfg(test)]
mod artifact_envelope_tests {
    // Tests are in artifact_envelope_tests.rs
    // but we include them here via the module system.
    include!("admission/artifact_envelope_tests.rs");
}
