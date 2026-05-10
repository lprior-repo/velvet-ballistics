#![forbid(unsafe_code)]

//! Runtime engine with action-aware execution.
//!
//! This module is the public facade. Implementation is split into focused submodules:
//! - `types`: EvidenceCollector, RuntimeEngineError, RetryPolicy, RuntimeSignal
//! - `execute`: Node execution dispatch
//! - `drive`: Deterministic drive loop
//! - `action`: Action execution helpers
//! - `signal`: Signal conversion
//! - `helpers`: Mark step after signal
//! - `tests`: BDD and proptest tests

pub mod action;
pub mod drive;
pub mod execute;
pub mod helpers;
pub mod signal;
pub mod tests;
pub mod types;

// Re-export public types
pub use action::{
    compute_idempotency_key, execute_do, execute_do_without_contract, execute_error_handler,
    execute_retry_check, resolve_contract, resume_action_outcome,
};
pub use drive::{drive_deterministic_full, drive_with_actions};
pub use execute::execute_node_full;
pub use helpers::mark_step_after_signal;
pub use signal::runtime_from_core;
pub use types::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeEngineError, RuntimeEngineResult,
    RuntimeSignal,
};
