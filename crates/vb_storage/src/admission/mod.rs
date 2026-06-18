#![forbid(unsafe_code)]
//! Artifact admission and verification functions.
//!
//! Provides artifact submission and admission flows with policy-controlled durability.
//!
//! # Module layout
//!
//! - **types** – Domain data structures (`VerificationWarning`, `ProofFlag`,
//!   `VerificationProof`, `AcceptedArtifact`).
//! - **policy** – Gate count constants and policy-digest computation.
//! - **bytes** – Artifact byte validation (re-parse + checksum).
//! - **contracts** – Capability and idempotency extraction from action contracts.
//! - **flow** – Policy dispatch and artifact submission orchestration.
//! - **persistence** – Serialization and storage of accepted artifacts.
//! - **record** – Record validation and deserialization.
//! - **metadata** – Metadata hashing for immutability checks.
//! - **fuzz** – Internal fuzz-harness accessors (feature-gated).

pub mod types;

pub(crate) mod bytes;
pub(crate) mod contracts;
pub(crate) mod flow;
pub(crate) mod metadata;
pub(crate) mod persistence;
pub(crate) mod policy;
pub(crate) mod record;

#[cfg(feature = "fuzzing")]
pub(crate) mod fuzz_access {
    //! Internal fuzz-harness accessors; unavailable in normal Cargo builds.

    pub(crate) use super::fuzz::decode_accepted_artifact_envelope;
    pub(crate) use super::fuzz::reject_oversized_compiled_ir_value;
    pub(crate) use super::fuzz::validate_compiled_ir_record;
}

mod fuzz;

// =========================================================================
// Public API re-exports (crate::admission::* surface)
// =========================================================================

pub use types::{AcceptedArtifact, ProofFlag, VerificationProof, VerificationWarning};

pub use bytes::reject_oversized_compiled_ir_value;
pub use flow::{submit_artifact, submit_artifact_with_contracts};
pub use policy::compute_policy_digest;
pub use record::admit_compiled_artifact;
pub use record::decode_accepted_artifact_envelope;
pub use record::validate_compiled_ir_record;

pub(crate) use metadata::compute_artifact_metadata_hash;
pub(crate) use policy::is_accepted_gate_count;
pub(crate) use record::{
    missing_proof_flag, validate_accepted_artifact_digest, validate_accepted_artifact_metadata,
    validate_artifact_policy_digest,
};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
