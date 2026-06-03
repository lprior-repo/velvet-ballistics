#![forbid(unsafe_code)]
//! Shard configuration and main Shard struct.

use indexmap::{IndexMap, IndexSet};

use vb_core::ids::RunId;
use vb_storage::EventSeq;

use crate::counters::ShardCounters;
use crate::frame_pool::FramePool;
use crate::journal::SharedRuntimeJournal;
use crate::trace::TraceRing;

// Re-export from queue for ShardConfig
pub use super::queue::{
    MAX_COMMAND_QUEUE_CAPACITY, ShardCommandQueue, is_valid_command_queue_capacity,
};

// ============================================================================
// ShardConfig
// ============================================================================

/// Shard configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShardConfig {
    /// Bounded capacity for the command queue.
    pub command_queue_capacity: usize,
    /// Bounded capacity for the trace ring.
    pub trace_capacity: usize,
    /// Maximum steps to execute per tick.
    pub step_budget_per_tick: u64,
    /// Maximum active runs admitted to this shard.
    pub max_active_runs: usize,
    /// Admission policy governing artifact verification.
    pub policy: vb_core::policy::RuntimePolicy,
}

/// Returns true when a trace ring capacity can retain at least one trace event.
#[must_use]
pub const fn is_valid_trace_capacity(capacity: usize) -> bool {
    capacity > 0
}

/// Returns true when a shard tick can attempt at least one deterministic step.
#[must_use]
pub const fn is_valid_step_budget_per_tick(budget: u64) -> bool {
    budget > 0
}

impl Default for ShardConfig {
    fn default() -> Self {
        Self {
            command_queue_capacity: 1024,
            trace_capacity: 4096,
            step_budget_per_tick: 1000,
            max_active_runs: 1024,
            policy: vb_core::policy::RuntimePolicy::Strict,
        }
    }
}

// ============================================================================
// ShardStatus and ShardHealth
// ============================================================================

/// Read-only shard health snapshot for operator status reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShardStatus {
    /// Human-readable health label.
    pub health: ShardHealth,
    /// True when the shard can continue processing ticks.
    pub running: bool,
    /// True after graceful shutdown begins.
    pub shutting_down: bool,
    /// Current command queue depth.
    pub command_queue_depth: usize,
    /// Total command queue capacity.
    pub command_queue_capacity: usize,
    /// Number of active runs owned by the shard.
    pub active_runs: usize,
    /// Configured active-run ceiling.
    pub max_active_runs: usize,
    /// Configured trace ring capacity.
    pub trace_capacity: usize,
    /// Count of trace events dropped due to ring overflow.
    pub trace_dropped: u64,
    /// Maximum execution steps attempted per tick.
    pub step_budget_per_tick: u64,
    /// Runtime admission policy.
    pub runtime_policy: vb_core::policy::RuntimePolicy,
}

/// Coarse health label for a shard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShardHealth {
    /// Shard is accepting ticks.
    Running,
    /// Shard has begun graceful shutdown.
    ShuttingDown,
}

// ============================================================================
// Shard
// ============================================================================

type FramePoolKey = (u16, u16);

/// Single-threaded shard owning all mutable run state.
pub struct Shard {
    pub(crate) command_queue: ShardCommandQueue,
    pub runs: IndexMap<RunId, RunState>,
    /// Per-run lifecycle state tracking for resume eligibility.
    pub(crate) runtime_states: IndexMap<RunId, RuntimeState>,
    /// Terminal run ids retained as direct runtime state, independent of trace retention.
    pub(crate) terminal_runs: IndexSet<RunId>,
    /// Next durable journal sequence by run, owned by this shard.
    pub(crate) journal_sequences: IndexMap<RunId, EventSeq>,
    pub(crate) pending_timers: IndexMap<RunId, PendingTimer>,
    pub(crate) frame_pools: IndexMap<FramePoolKey, FramePool>,
    pub(crate) trace_ring: TraceRing,
    pub(crate) counters: ShardCounters,
    pub(crate) step_budget_per_tick: u64,
    pub(crate) max_active_runs: usize,
    pub(crate) policy: vb_core::policy::RuntimePolicy,
    pub(crate) artifact_store: crate::admission::SharedAcceptedArtifactStore,
    pub(crate) inspect_response: Option<InspectResponse>,
    pub(crate) shutting_down: bool,
    pub(crate) current_tick: TimerTick,
    pub(crate) journal: SharedRuntimeJournal,
}

// ============================================================================
// Re-exports for Shard struct dependencies
// ============================================================================

pub use super::run_state::{InspectResponse, RunState, RuntimeState};
pub use super::timer::{PendingTimer, TimerTick};
