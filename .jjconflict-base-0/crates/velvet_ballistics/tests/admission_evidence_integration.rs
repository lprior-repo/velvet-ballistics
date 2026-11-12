#![forbid(unsafe_code)]
//! Admission and evidence chain integration tests.
//!
//! These tests exercise end-to-end flows across multiple crates: submitting
//! artifacts, running workflows under various policies, verifying journal
//! evidence chains, capability enforcement, budget validation, and taint
//! propagation.

include!("admission_evidence_integration/chunk_001.rs");
include!("admission_evidence_integration/chunk_002.rs");
include!("admission_evidence_integration/chunk_003.rs");
include!("admission_evidence_integration/chunk_004.rs");
