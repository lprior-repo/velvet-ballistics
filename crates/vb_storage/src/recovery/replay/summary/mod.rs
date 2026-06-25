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

use crate::recovery::types::{
    RecoveredPendingAction, RecoveredRunAdmission, RecoveredSlotEntry, RecoveredStepEntry,
    RecoveredStepState, RecoveryError, RecoveryFrameSeed, RecoveryResult, RecoveryRuntimeSummary,
    RecoveryTerminalState, UnsupportedRecoveryState,
};
use crate::{EventSeq, JournalEvent};

use derive::reject_workflow_digest_mismatch;
use hydrate::{RecoveredSlots, replay_error_to_recovery};

pub mod accumulator;
pub mod apply;
pub mod derive;
pub mod hydrate;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub use apply::{apply_summary_event, summarize_recovery_events};
pub use derive::{
    RecoveryFrameSeedBuilder, recover_run_admission_from_events,
    recover_runtime_frame_seed_from_events, recover_runtime_frame_seed_from_events_with_workflow,
    recovery_dimension_count_from_index, recovery_observed_dimension_is_positive,
    recovery_seed_dimensions_positive,
};
pub(crate) use hydrate::legacy_slot_taint;
