#![forbid(unsafe_code)]
//! Summary and frame seed building for journal recovery.
//!
//! Provides:
//! - Runtime summary construction from events
//! - Frame seed building for live-frame reconstruction
//!
//! Organized into:
//! - `apply`: event-to-summary application
//! - `derive`: top-level recovery entry points and frame seed builder
//! - `accumulator`: `FrameSeedAccumulator` state machine
//! - `hydrate`: slot hydration, taint recovery, and replay error mapping

#[cfg(test)]
use crate::JournalEvent;
#[cfg(test)]
use crate::recovery::types::{
    RecoveredRunAdmission, RecoveredSlotEntry, RecoveredStepState, RecoveryError,
    RecoveryRuntimeSummary, UnsupportedRecoveryState,
};

#[cfg(test)]
use derive::reject_workflow_digest_mismatch;
#[cfg(test)]
use hydrate::{RecoveredSlots, replay_error_to_recovery};

pub mod accumulator;
pub mod apply;
pub mod derive;
pub mod dimensions;
pub mod hydrate;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub use apply::{apply_summary_event, summarize_recovery_events};
pub use derive::{
    RecoveryFrameSeedBuilder, recover_raw_runtime_frame_seed_from_events,
    recover_raw_runtime_frame_seed_from_events_with_workflow, recover_run_admission_from_events,
    recover_runtime_frame_seed_from_events, recover_runtime_frame_seed_from_events_with_workflow,
};
pub use dimensions::{
    recovery_dimension_count_from_index, recovery_observed_dimension_is_positive,
    recovery_seed_dimensions_positive,
};
#[cfg(test)]
pub(crate) use hydrate::legacy_slot_taint;
