#![forbid(unsafe_code)]

//! Simulation report types returned by the fault injection engine.

use serde::{Deserialize, Serialize};
use vb_core::ids::{ActionId, StepIdx};

use crate::fault_inject::types::{CheckpointSeq, CrashSeverity, FailureCode, NamedBoundary};

// ---------------------------------------------------------------------------
// Simulation report
// ---------------------------------------------------------------------------

/// Why a journal entry is missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissingReason {
    CrashBeforeAppend,
    AppendFailureTransient,
    AppendFailurePermanent,
    LockContentionExhausted,
}

/// One simulated journal entry produced by the engine.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum JournalOutcome {
    Appended {
        boundary: NamedBoundary,
        seq: u64,
    },
    Missing {
        boundary: NamedBoundary,
        reason: MissingReason,
    },
    Pending {
        boundary: NamedBoundary,
    },
    Corrupt {
        boundary: NamedBoundary,
    },
}

/// One observable outcome produced by the engine per applied fault.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FaultOutcome {
    Crashed {
        boundary: NamedBoundary,
        severity: CrashSeverity,
    },
    AppendFailed {
        boundary: NamedBoundary,
        transient: bool,
        attempts: u8,
    },
    LockResolved {
        boundary: NamedBoundary,
        attempts: u8,
    },
    LockExhausted {
        boundary: NamedBoundary,
        attempts: u8,
    },
    ActionFailed {
        action: ActionId,
        code: FailureCode,
    },
    TimedOut {
        step: StepIdx,
        delay_ticks: u32,
    },
    Restarted {
        checkpoint: CheckpointSeq,
    },
}

/// Final report returned by [`super::run_fault_injection`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FaultReport {
    pub seed: u64,
    pub events_applied: u32,
    pub runtime_steps: u32,
    pub journal_entries: Vec<JournalOutcome>,
    pub outcomes: Vec<FaultOutcome>,
    pub recovery_required: bool,
    /// Deterministic splitmix64 fingerprint of `(seed, journal_entries,
    /// outcomes)`. Two reports with identical fingerprints are
    /// byte-identical in their observable fields.
    pub schedule_hash: u64,
}
