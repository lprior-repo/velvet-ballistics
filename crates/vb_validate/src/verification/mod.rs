#![forbid(unsafe_code)]
//! Verification modules for formal verification harnesses.
//!
//! These modules contain Kani proof harnesses and Verus binding proofs
//! for validating the correctness of gate implementations.
//!
//! Modules:
//! - `kani_idempotency_contract` - Kani harnesses for idempotency contract verification
//! - `kani_gate_08_accessor` - Kani harness for Gate 8 accessor validation
//! - `kani_gate_08_structural` - Kani structural harness for Gate 8 full WorkflowParts coverage
//! - `kani_step_primitives` - Kani harnesses for STEP_PRIMITIVES constant verification
//! - `gate_08_verus_proof` - Verus binding proof for Gate 8 accessor validation

#[cfg(kani)]
pub mod kani_idempotency_contract;

#[cfg(kani)]
pub mod kani_gate_08_accessor;

#[cfg(kani)]
pub mod kani_gate_08_structural;

#[cfg(kani)]
pub mod kani_step_primitives;

#[cfg(kani)]
pub mod gate_08_verus_proof;
