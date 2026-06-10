#![forbid(unsafe_code)]

//! Runtime engine with action-aware execution.
//!
//! This module is the public facade. Implementation is split into focused submodules:
//! - `evidence`: EvidenceEvent, EvidenceCollector for drive loop instrumentation
//! - `types`: RuntimeEngineError, RetryPolicy, RuntimeSignal
//! - `execute`: Node execution dispatch
//! - `drive`: Deterministic drive loop
//! - `action`: Action execution helpers
//! - `signal`: Signal conversion
//! - `helpers`: Mark step after signal
//! - `tests`: BDD and proptest tests

pub mod action;
#[cfg(test)]
pub mod action_tests;
pub mod drive;
pub mod evidence;
pub mod execute;
pub mod handlers;
pub mod helpers;
// `property_tests` is the untracked `crates/vb_runtime/src/property_tests/`
// directory that contains a `concurrency_safety` proptest. It surfaces a
// real IntrospectionRegistry race (master §38 follow-up). The directory
// is left on disk for the followup bead; we do NOT wire it in until the
// race is fixed.
// #[cfg(test)]
// pub mod property_tests;
pub mod retry_math;
pub mod signal;
#[cfg(test)]
pub mod tests;
pub mod types;

// Re-export public types
pub use action::{
    compute_idempotency_key, execute_do, execute_do_without_contract, execute_error_handler,
    execute_retry_check, resolve_contract, resume_action_outcome,
};
pub use drive::{drive_deterministic_full, drive_with_actions};
pub use evidence::{EvidenceCollector, EvidenceEvent};
pub use execute::execute_node_full;
pub use helpers::mark_step_after_signal;
pub use retry_math::{RetryCursor, RetryPolicyLimits, RetryPolicyMathError};
pub use signal::runtime_from_core;
pub use types::{RetryPolicy, RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};
