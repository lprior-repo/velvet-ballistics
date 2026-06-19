#![forbid(unsafe_code)]
//! Summary and frame seed building for journal recovery.
//!
//! Organized into:
//! - `runtime_summary` — summary construction, admission recovery, event application
//! - `frame_seed` — accumulator, envelope view, frame seed builder, dimension helpers
//! - `slots` — slot recovery, taint extraction, pending actions, error mapping

mod frame_seed;
mod runtime_summary;
pub(crate) mod slots;

// Re-export public API
pub use frame_seed::{
    RecoveryFrameSeedBuilder, recover_runtime_frame_seed_from_events,
    recover_runtime_frame_seed_from_events_with_workflow, recovery_dimension_count_from_index,
    recovery_observed_dimension_is_positive, recovery_seed_dimensions_positive,
    reject_workflow_digest_mismatch,
};
pub use runtime_summary::{
    apply_summary_event, recover_run_admission_from_events, summarize_recovery_events,
};
pub use slots::pending_actions_from_events;

// Re-export items used by tests
pub(crate) use slots::{RecoveredSlots, replay_error_to_recovery};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
