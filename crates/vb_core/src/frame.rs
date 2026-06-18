#![forbid(unsafe_code)]

//! Bounded run-frame state for one shard-owned workflow run.
//!
//! This module is a thin facade that organizes frame-related types into
//! focused submodules:
//!
//! - **`step_state`** — `StepState` enum and transition predicate
//! - **`run_frame`** — `RunFrame` struct and full implementation
//! - **`verus_proofs`** — Verus formal verification (toolchain-gated)
//! - **`tests_and_verification`** — unit tests and Kani harnesses

// Core domain types and pure functions
pub mod step_state;
pub mod run_frame;

// Re-export core types at module level for backward compatibility
pub use step_state::{StepState, is_valid_step_state_transition};
pub use run_frame::RunFrame;

// Verification modules (toolchain gated)
#[cfg(verus)]
pub mod verus_proofs;

#[cfg(kani)]
mod tests_and_verification;
