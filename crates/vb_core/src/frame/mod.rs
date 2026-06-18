//! Bounded run-frame state for one shard-owned workflow run.
//!
//! This module organizes the `RunFrame` runtime state into focused submodules:
//!
//! - **`run_frame`** — `RunFrame` struct definition
//! - **`lifecycle`** — construction and reinitialization
//! - **`accessors`** — const fn getters for frame identity and dimensions
//! - **`parallel`** — parallel in-flight branch tracking
//! - **`pc`** — program counter management and execution counter
//! - **`slots`** — slot I/O, taint handling, and snapshot views
//! - **`transitions`** — step state machine transitions and validation
//!
//! **`step_state`** lives at the crate root of this module as the shared enum.

#![forbid(unsafe_code)]

// Core domain types and pure functions
pub mod step_state;

// RunFrame submodules
pub mod accessors;
pub mod lifecycle;
pub mod parallel;
pub mod pc;
pub mod run_frame;
pub mod slots;
pub mod transitions;

// Re-export core types at module level for backward compatibility
pub use run_frame::RunFrame;
pub use step_state::{StepState, is_valid_step_state_transition};

// Verification modules (toolchain gated)
#[cfg(verus)]
pub mod verus_proofs;

#[cfg(kani)]
mod tests_and_verification;
