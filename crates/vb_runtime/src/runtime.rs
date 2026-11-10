#![forbid(unsafe_code)]
//! Multi-shard runtime routing commands to correct shards.

use std::num::NonZeroUsize;
use vb_core::action::{ActionContract, ActionFailure, ActionOutputReady, ActionTicket};
use vb_core::capability::CapabilitySet;
use vb_core::ids::{RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::manual_gate::GateDecisionInput;
use vb_core::value::SlotValue;
use vb_core::workflow::CompiledWorkflow;

use crate::counters::{CounterSnapshot, RuntimeMetricsSnapshot, ShardMetricsSnapshot};
use crate::journal::SharedRuntimeJournal;
use crate::shard::timer_wheel::TimerEntry;
use crate::shard::{
    AskAnswer, InspectResponse, Shard, ShardCommand, ShardConfig, ShardDirective,
    ShardPendingBoundarySnapshot,
};
use crate::trace::TraceEvent;
use crate::{RuntimeError, RuntimeResult};

pub mod completion_observation;
mod owned_activation;
mod shutdown;
pub use shutdown::{ShutdownFailure, ShutdownPhase, ShutdownReport};

struct RecoveryArtifactWorkflow {
    artifact: vb_storage::admission::AcceptedArtifact,
    workflow: CompiledWorkflow,
}

struct RecoveryPrefixEvidence {
    events: Vec<vb_storage::JournalEvent>,
    admission: crate::admission::RunAdmission,
    workflow_digest: WorkflowDigest,
    next_seq: vb_storage::EventSeq,
}

struct RecoveredSeedProductContext {
    run: RunId,
    prefix: RecoveryPrefixEvidence,
    /// Typed frame-seed product. The storage-side cannot-resume witness is
    /// carried on `seed.cannot_resume_state()` and survives through the
    /// runtime classification step (`product_from_recovered_context` →
    /// `RuntimeRecoveryProduct::CannotResume`). Holding the typed product
    /// here is what closes the FINDING-002 typestate bypass: the raw
    /// `RecoveryFrameSeed` DTO no longer leaks into the runtime flow.
    seed: vb_storage::recovery::RecoveryFrameSeedProduct,
    boundary: crate::recovery::RecoveredRunBoundary,
}

/// Lossless conversion of a `u32` integer to its exact `f32` representation.
///
/// `From<u32> for f32` is NOT implemented by the Rust standard library
/// (only `From<u8>`, `From<u16>`, `From<i8>`, `From<i16>` exist for `f32`).
/// For values in `[0, 2^24)` — which includes the full RA-003 trace-ring
/// domain (`cap <= 2^20`, `len <= cap`) — the IEEE-754 single-precision
/// encoding fits the integer exactly, so this helper produces a result
/// bit-identical to `(n as f32)` without using an `as`-cast and without
/// tripping `clippy::as_conversions`. All integer arithmetic uses
/// `u32::checked_*` / `u32::saturating_*` so `clippy::arithmetic_side_effects`
/// is also satisfied.
fn u32_to_f32_exact(n: u32) -> f32 {
    if n == 0 {
        return 0.0_f32;
    }
    // `e = floor(log2(n))`. For n in [1, 2^32), `leading_zeros` is in [0, 31],
    // so `e` is in [0, 31]. The `31 - ...` formula is the bit-width (32) minus
    // 1 (for the implicit leading one) minus `leading_zeros`.
    let e = u32::checked_sub(31, n.leading_zeros()).unwrap_or(0);
    let biased_exp = u32::saturating_add(e, 127);
    let power = 1_u32.checked_shl(e).unwrap_or(1);
    let mantissa = u32::checked_sub(n, power)
        .unwrap_or(0)
        .checked_shl(23_u32.saturating_sub(e))
        .unwrap_or(0);
    f32::from_bits((biased_exp << 23) | mantissa)
}

/// Summary of an active run on a shard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveRunSummary {
    /// Run identifier.
    pub run_id: RunId,
    /// Compiled workflow digest.
    pub workflow: vb_core::WorkflowDigest,
    /// Number of steps in the workflow.
    pub step_count: u16,
    /// Steps that reached a terminal state (Succeeded, Failed, Skipped, or Cancelled).
    pub steps_completed: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePendingBoundarySnapshot {
    shards: Box<[ShardPendingBoundarySnapshot]>,
    command_queue_depth: usize,
    active_run_count: usize,
    pending_timer_count: usize,
    pending_action_count: usize,
    pending_ask_count: usize,
    truncated: bool,
}

impl RuntimePendingBoundarySnapshot {
    fn new(
        shards: Box<[ShardPendingBoundarySnapshot]>,
        totals: RuntimePendingBoundaryTotals,
    ) -> Self {
        Self {
            shards,
            command_queue_depth: totals.command_queue_depth,
            active_run_count: totals.active_run_count,
            pending_timer_count: totals.pending_timer_count,
            pending_action_count: totals.pending_action_count,
            pending_ask_count: totals.pending_ask_count,
            truncated: totals.truncated,
        }
    }

    #[must_use]
    pub fn shards(&self) -> &[ShardPendingBoundarySnapshot] {
        &self.shards
    }

    #[must_use]
    pub const fn command_queue_depth(&self) -> usize {
        self.command_queue_depth
    }

    #[must_use]
    pub const fn active_run_count(&self) -> usize {
        self.active_run_count
    }

    #[must_use]
    pub const fn pending_timer_count(&self) -> usize {
        self.pending_timer_count
    }

    #[must_use]
    pub const fn pending_action_count(&self) -> usize {
        self.pending_action_count
    }

    #[must_use]
    pub const fn pending_ask_count(&self) -> usize {
        self.pending_ask_count
    }

    #[must_use]
    pub const fn truncated(&self) -> bool {
        self.truncated
    }
}

#[derive(Default)]
struct RuntimePendingBoundaryTotals {
    command_queue_depth: usize,
    active_run_count: usize,
    pending_timer_count: usize,
    pending_action_count: usize,
    pending_ask_count: usize,
    truncated: bool,
}

impl RuntimePendingBoundaryTotals {
    fn add_shard(&mut self, shard: &ShardPendingBoundarySnapshot) {
        self.command_queue_depth = self
            .command_queue_depth
            .saturating_add(shard.command_queue_depth());
        self.active_run_count = self
            .active_run_count
            .saturating_add(shard.active_run_count());
        self.pending_timer_count = self
            .pending_timer_count
            .saturating_add(shard.pending_timer_count());
        self.pending_action_count = self
            .pending_action_count
            .saturating_add(shard.pending_action_count());
        self.pending_ask_count = self
            .pending_ask_count
            .saturating_add(shard.pending_ask_count());
        self.truncated |= shard.truncated();
    }
}

fn shard_id_from_index(index: usize) -> u32 {
    u32::try_from(index).map_or(u32::MAX, core::convert::identity)
}

/// Multi-shard runtime.
pub struct Runtime {
    shards: Vec<Shard>,
    shard_count: usize,
    journal: SharedRuntimeJournal,
    policy: vb_core::policy::RuntimePolicy,
    live_policy: crate::policy::LivePolicy,
}

include!("runtime/construction.rs");

include!("runtime/submission.rs");

include!("runtime/cancel_recovery.rs");

include!("runtime/retry_validation.rs");

include!("runtime/recovery_product.rs");

include!("runtime/recovery_seed.rs");

include!("runtime/tick.rs");

include!("runtime/actions.rs");

include!("runtime/metrics.rs");

include!("runtime/shutdown_shard.rs");

#[path = "runtime/live_policy_dispatch.rs"]
mod live_policy_dispatch;
#[path = "runtime/registered_executor.rs"]
mod registered_executor;
#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
pub use registered_executor::RegisteredExecutionError;

#[cfg(test)]
#[path = "runtime/shutdown_tests.rs"]
mod shutdown_tests;
