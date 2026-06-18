#![forbid(unsafe_code)]
//! Recovery type definitions, split by responsibility.
//!
//! - `error` – `RecoveryError` and `RecoveryResult`
//! - `state` – terminal states, summaries, admission, hydration, step/slot entries,
//!   frame seeds, and unsupported-state flags
//! - `snapshot` – `RunSnapshot`
//! - `replay` – `ActionReplayTracker` and replay effect enum
//! - `digest` – `DigestCheck` levels and `DigestCheckConfig`

pub mod digest;
pub mod error;
pub mod replay;
pub mod snapshot;
pub mod state;

// ============================================================================
// Re-exports – error
// ============================================================================

pub use error::{RecoveryError, RecoveryResult};

// ============================================================================
// Re-exports – state
// ============================================================================

pub use state::{
    RecoveredPendingAction, RecoveredRunAdmission, RecoveredSlotEntry, RecoveredStepEntry,
    RecoveredStepState, RecoveryFrameSeed, RecoveryHydration, RecoveryRuntimeSummary,
    RecoveryTerminalState, UnsupportedRecoveryState,
};

// ============================================================================
// Re-exports – snapshot
// ============================================================================

pub use snapshot::RunSnapshot;

// ============================================================================
// Re-exports – replay
// ============================================================================

pub(crate) use replay::ActionReplayEffect;
pub use replay::ActionReplayTracker;

// ============================================================================
// Re-exports – digest
// ============================================================================

pub use digest::{DigestCheck, DigestCheckConfig};
