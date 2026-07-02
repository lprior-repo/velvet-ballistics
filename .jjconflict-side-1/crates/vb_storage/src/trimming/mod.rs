#![forbid(unsafe_code)]
//! Journal trimming with retention policy.

use crate::{EventSeq, JournalError};
use vb_core::RunId;

/// Retention policy for journal trimming.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrimPolicy {
    /// If true, skip runs that have no events to trim (no-op runs).
    pub skip_noop_runs: bool,
    /// Number of most-recent terminal runs per workflow to retain.
    /// A run is eligible for trimming only if it is NOT among the
    /// `retain_last_n_terminal` most recent terminal runs for its workflow.
    pub retain_last_n_terminal: u32,
}

impl Default for TrimPolicy {
    fn default() -> Self {
        Self {
            skip_noop_runs: true,
            retain_last_n_terminal: 10,
        }
    }
}

/// Errors that can occur during journal trimming.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TrimError {
    /// Fjall operation failed.
    #[error("fjall operation failed: {0}")]
    Fjall(#[from] fjall::Error),
    /// Journal operation failed.
    #[error("journal error: {0}")]
    Journal(#[from] JournalError),
    /// No durable snapshot found for run.
    #[error("no durable snapshot for run {run:?}")]
    NoDurableSnapshot {
        /// Run without a durable snapshot.
        run: RunId,
    },
    /// Retention policy blocks trimming this terminal run.
    #[error("retention policy blocks trim for run {run:?}")]
    RetentionPolicyBlocks {
        /// Run blocked by retention policy.
        run: RunId,
    },
    /// Trim operation was interrupted.
    #[error("trim operation incomplete")]
    IncompleteTrim {
        /// Number of events deleted before interruption.
        deleted_count: u64,
    },
}

impl TrimError {
    pub const NO_DURABLE_SNAPSHOT_CODE: vb_core::DiagnosticCode =
        vb_core::DiagnosticCode::new(0x4101);
    pub const RETENTION_POLICY_BLOCKS_CODE: vb_core::DiagnosticCode =
        vb_core::DiagnosticCode::new(0x4103);
    pub const INCOMPLETE_TRIM_CODE: vb_core::DiagnosticCode = vb_core::DiagnosticCode::new(0x4102);

    #[must_use]
    pub const fn diagnostic_code(&self) -> vb_core::DiagnosticCode {
        match self {
            Self::Fjall(_) => JournalError::FJALL_CODE,
            Self::Journal(inner) => inner.diagnostic_code(),
            Self::NoDurableSnapshot { .. } => Self::NO_DURABLE_SNAPSHOT_CODE,
            Self::RetentionPolicyBlocks { .. } => Self::RETENTION_POLICY_BLOCKS_CODE,
            Self::IncompleteTrim { .. } => Self::INCOMPLETE_TRIM_CODE,
        }
    }
}

/// Result type for trim operations.
pub type TrimResult<T> = Result<T, TrimError>;

/// Outcome of trimming a single run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrimmedRunResult {
    /// The run that was trimmed.
    pub run: RunId,
    /// Number of events deleted.
    pub deleted_count: u64,
    /// The snapshot sequence that served as the cutoff.
    pub cutoff_seq: EventSeq,
    /// Outcome status of the trim operation.
    pub status: TrimStatus,
}

/// Status of a trim operation for a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TrimStatus {
    /// Events were deleted.
    Trimmed,
    /// No events were eligible for deletion.
    NoOp,
}

/// Run eligibility for journal trimming.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TrimEligibility {
    /// Run can be trimmed up to the safe point.
    Eligible {
        /// The run identifier.
        run: RunId,
        /// Highest event sequence covered by a durable snapshot.
        safe_point: EventSeq,
        /// Number of events that would be deleted if trimmed.
        events_trimmable: u64,
    },
    /// Run cannot be trimmed due to a blocker.
    Blocked {
        /// The run identifier.
        run: RunId,
        /// The reason trimming is blocked.
        blocker: TrimBlocker,
    },
}

/// Reason a run cannot be trimmed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TrimBlocker {
    /// No durable snapshot exists for this run.
    NoDurableSnapshot,
    /// Retention policy protects this terminal run.
    RetentionPolicy {
        /// The retention count that blocked this run.
        retain_last_n_terminal: u32,
    },
}

/// Aggregate trim diagnostic for all runs in the journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrimDiagnostic {
    /// Per-run eligibility results.
    pub runs: Vec<TrimEligibility>,
    /// Total number of runs in the journal.
    pub total_runs: u64,
    /// Number of runs eligible for trimming.
    pub eligible_runs: u64,
    /// Number of runs blocked from trimming.
    pub blocked_runs: u64,
    /// Total events that would be deleted if all eligible runs were trimmed.
    pub total_events_trimmable: u64,
}

pub(crate) mod helpers;
pub(crate) mod logic;

#[cfg(test)]
mod tests;
