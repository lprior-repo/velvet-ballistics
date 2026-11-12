//! Production shutdown preserves suspended work and failure provenance.
use super::{Runtime, RuntimePendingBoundarySnapshot};
use crate::RuntimeError;
use vb_storage::JournalWriterFlushReport;

const DETAIL_LIMIT_PER_SHARD: usize = 64;

/// The operation that failed before shutdown could be acknowledged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownPhase {
    /// Draining a shard's bounded command queue, by runtime shard index.
    ShardDrain { shard: usize },
    /// Committing accepted journal writes and their durability barrier.
    JournalDrain,
}

/// A completed quiescence and durability barrier, with suspended work retained.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShutdownReport {
    commands_drained: usize,
    remaining: RuntimePendingBoundarySnapshot,
    timers_released: usize,
    journal: JournalWriterFlushReport,
}

impl ShutdownReport {
    #[must_use]
    pub const fn commands_drained(&self) -> usize {
        self.commands_drained
    }

    /// Boundary authorities captured before timer registration cleanup. Exact
    /// totals accompany at most 64 details per shard and an explicit truncation bit.
    #[must_use]
    pub const fn remaining(&self) -> &RuntimePendingBoundarySnapshot {
        &self.remaining
    }

    #[must_use]
    pub const fn timers_released(&self) -> usize {
        self.timers_released
    }

    #[must_use]
    pub const fn journal(&self) -> JournalWriterFlushReport {
        self.journal
    }
}

/// A failed shutdown never certifies durability or completion of active runs.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("shutdown failed during {phase:?}: {cause}")]
pub struct ShutdownFailure {
    phase: ShutdownPhase,
    #[source]
    cause: Box<RuntimeError>,
    remaining: RuntimePendingBoundarySnapshot,
    completed_shards: usize,
}

impl ShutdownFailure {
    #[must_use]
    pub const fn phase(&self) -> ShutdownPhase {
        self.phase
    }

    #[must_use]
    pub fn cause(&self) -> &RuntimeError {
        &self.cause
    }

    #[must_use]
    pub const fn remaining(&self) -> &RuntimePendingBoundarySnapshot {
        &self.remaining
    }

    #[must_use]
    pub const fn completed_shards(&self) -> usize {
        self.completed_shards
    }

    fn capture(
        runtime: &Runtime,
        phase: ShutdownPhase,
        cause: RuntimeError,
        completed_shards: usize,
    ) -> Self {
        Self {
            phase,
            cause: Box::new(cause),
            remaining: runtime.pending_boundary_snapshot(DETAIL_LIMIT_PER_SHARD),
            completed_shards,
        }
    }
}

pub(super) fn shutdown(runtime: &mut Runtime) -> Result<ShutdownReport, ShutdownFailure> {
    let commands_drained = quiesce(runtime)?;
    let journal = drain_journal(runtime)?;
    Ok(release_timers(runtime, commands_drained, journal))
}

fn quiesce(runtime: &mut Runtime) -> Result<usize, ShutdownFailure> {
    let mut drained = 0usize;
    for index in 0..runtime.shards.len() {
        drained = drained.saturating_add(quiesce_shard(runtime, index)?);
    }
    Ok(drained)
}

fn quiesce_shard(runtime: &mut Runtime, index: usize) -> Result<usize, ShutdownFailure> {
    runtime
        .shards
        .get_mut(index)
        .ok_or(RuntimeError::RunNotFound)
        .and_then(crate::shard::Shard::quiesce_for_shutdown)
        .map_err(|error| {
            ShutdownFailure::capture(
                runtime,
                ShutdownPhase::ShardDrain { shard: index },
                error,
                index,
            )
        })
}

fn drain_journal(runtime: &Runtime) -> Result<JournalWriterFlushReport, ShutdownFailure> {
    runtime.journal.drain_for_shutdown().map_err(|error| {
        ShutdownFailure::capture(
            runtime,
            ShutdownPhase::JournalDrain,
            error,
            runtime.shards.len(),
        )
    })
}

fn release_timers(
    runtime: &mut Runtime,
    commands_drained: usize,
    journal: JournalWriterFlushReport,
) -> ShutdownReport {
    let remaining = runtime.pending_boundary_snapshot(DETAIL_LIMIT_PER_SHARD);
    let timers_released = remaining.pending_timer_count();
    for shard in &mut runtime.shards {
        shard.clear_pending_timers();
    }
    ShutdownReport {
        commands_drained,
        remaining,
        timers_released,
        journal,
    }
}
