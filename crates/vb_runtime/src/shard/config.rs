#![forbid(unsafe_code)]
//! Shard configuration and main Shard struct.

use indexmap::IndexMap;

use vb_core::ids::RunId;
#[cfg(feature = "test-util")]
use vb_core::workflow::CompiledWorkflow;
use vb_storage::EventSeq;

use crate::counters::ShardCounters;
use crate::frame_pool::FramePool;
use crate::journal::{RuntimeJournalEvent, SharedRuntimeJournal};
use crate::trace::TraceRing;

use super::bounded_outcomes::{BoundedOutcomeIndex, DEFAULT_MAX_TERMINAL_OUTCOMES};
use super::lru_ring::{DEFAULT_MAX_TERMINAL_RUNS, DEFAULT_TERMINAL_RUNS_TTL_TICKS, LruRing};

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
    /// Number of ticks over which to coalesce journal events into a single
    /// batch commit. When equal to 1 (the default), each command is
    /// journalized individually. When greater than 1, commands dispatched
    /// within the window are accumulated and written atomically when the
    /// window expires.
    pub coalesce_window_ticks: u32,
    /// Number of execution steps between periodic snapshots.
    ///
    /// A value of `0` disables periodic mid-run snapshots entirely.
    /// A value of `1` snapshots after every completed step (valid but costly).
    /// A value greater than `1` snapshots after every N completed steps.
    ///
    /// This is immutable after shard creation and applies uniformly to all
    /// runs in the shard.
    pub snapshot_interval_steps: u64,
    /// Bounded capacity of the terminal-runs LRU ring (MEM-01).
    ///
    /// When the ring reaches this capacity and no TTL-expired entry
    /// can be evicted, further inserts return
    /// `RuntimeError::TerminalRunsLruFull` (the strict
    /// `terminal_runs_try_insert` path) or are force-inserted with
    /// `lru_capacity_overflows` incremented (the legacy
    /// `terminal_runs_insert` path).
    pub max_terminal_runs: usize,
    /// TTL in ticks for terminal-runs LRU entries (MEM-01).
    ///
    /// Entries inserted more than this many ticks in the past are
    /// evicted by the lazy sweep before a new insert checks capacity.
    /// The default of `86_400` matches a 1-tick/second operating mode;
    /// operators using faster tick rates should scale accordingly.
    pub terminal_runs_ttl_ticks: u64,
    /// Bounded capacity for the terminal-outcomes side table (RQ-W0-10).
    ///
    /// When the outcome map reaches this capacity, the oldest entry is
    /// evicted FIFO before a new insert is recorded. Defaults to
    /// `DEFAULT_MAX_TERMINAL_OUTCOMES` (matching `max_terminal_runs`); a
    /// value of `0` is treated as `1` so the map remains usable for
    /// minimal configurations.
    pub max_terminal_outcomes: usize,
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

/// Returns true when a coalesce window tick count is valid.
///
/// A value of 1 means no coalescing (each command is journalized individually).
/// Values greater than 1 enable coalescing over the specified number of ticks.
#[must_use]
pub const fn is_valid_coalesce_window_ticks(count: u32) -> bool {
    count > 0
}

/// Returns true when a `max_terminal_runs` capacity is positive.
///
/// A zero-capacity terminal-runs ring is rejected at the runtime boundary
/// instead of being silently normalised by `LruRing::try_new`.
#[must_use]
pub const fn is_valid_max_terminal_runs(capacity: usize) -> bool {
    capacity > 0
}

/// Validates every field of `config` and returns a single typed error that
/// aggregates **all** invalid fields, in field-declaration order.
///
/// This is the unified `ShardConfig` validator. It supersedes
/// `validate_shard_config_inputs`, which used early returns and silently
/// skipped every field after the first failure (RS-217: shard config
/// validation omitted fields). All public construction paths
/// (`ShardConfig::new`, `ShardConfig::new_full`, and
/// `Shard::new_with_journal_and_artifact_store`) call this method so
/// struct-literal config bypass cannot sneak invalid combinations past
/// the validator.
///
/// Rejected fields:
/// - `command_queue_capacity == 0`
/// - `command_queue_capacity > MAX_COMMAND_QUEUE_CAPACITY`
/// - `trace_capacity == 0`
/// - `step_budget_per_tick == 0`
/// - `max_active_runs == 0`
/// - `coalesce_window_ticks == 0`
/// - `max_terminal_runs == 0`
///
/// Accepted at any value:
/// - `snapshot_interval_steps == 0` (disables periodic snapshots)
/// - `terminal_runs_ttl_ticks == 0` (disables TTL expiry)
/// - `max_terminal_outcomes == 0` (treated as 1 by `BoundedOutcomeIndex`)
pub fn validate_shard_config(config: &ShardConfig) -> Result<(), crate::RuntimeError> {
    let mut errors: Vec<crate::RuntimeError> = Vec::new();

    if !is_valid_command_queue_capacity(config.command_queue_capacity) {
        errors.push(crate::RuntimeError::CommandQueueCapacityExceeded {
            capacity: config.command_queue_capacity,
            max: MAX_COMMAND_QUEUE_CAPACITY,
        });
    }
    if !is_valid_trace_capacity(config.trace_capacity) {
        errors.push(crate::RuntimeError::UnsupportedOperation {
            operation: "trace_capacity_zero",
        });
    }
    if !is_valid_step_budget_per_tick(config.step_budget_per_tick) {
        errors.push(crate::RuntimeError::UnsupportedOperation {
            operation: "step_budget_per_tick_zero",
        });
    }
    if config.max_active_runs == 0 {
        errors.push(crate::RuntimeError::ActiveRunCapacityZero);
    }
    if !is_valid_coalesce_window_ticks(config.coalesce_window_ticks) {
        errors.push(crate::RuntimeError::UnsupportedOperation {
            operation: "coalesce_window_ticks_zero",
        });
    }
    if !is_valid_max_terminal_runs(config.max_terminal_runs) {
        errors.push(crate::RuntimeError::LruRingCapacityZero);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(crate::RuntimeError::ConfigInvalid { errors })
    }
}

/// Backwards-compatible helper that validates the legacy six-field inputs.
///
/// New code MUST call [`validate_shard_config`] instead so that all
/// fields are validated together. This helper remains for callers that
/// only have the six capacity inputs available; it aggregates any
/// invalid input into a single typed error, in declaration order.
#[allow(clippy::too_many_arguments)]
pub fn validate_shard_config_inputs(
    command_queue_capacity: usize,
    trace_capacity: usize,
    step_budget_per_tick: u64,
    max_active_runs: usize,
    coalesce_window_ticks: u32,
    max_terminal_runs: usize,
) -> Result<(), crate::RuntimeError> {
    validate_shard_config(&ShardConfig {
        command_queue_capacity,
        trace_capacity,
        step_budget_per_tick,
        max_active_runs,
        policy: vb_core::policy::RuntimePolicy::Strict,
        coalesce_window_ticks,
        snapshot_interval_steps: 0,
        max_terminal_runs,
        terminal_runs_ttl_ticks: DEFAULT_TERMINAL_RUNS_TTL_TICKS,
        max_terminal_outcomes: super::bounded_outcomes::DEFAULT_MAX_TERMINAL_OUTCOMES,
    })
}

impl ShardConfig {
    /// Validates this `ShardConfig` end-to-end and returns a single typed
    /// error that aggregates **all** invalid fields.
    ///
    /// This is the unified entry point called by `ShardConfig::new`,
    /// `ShardConfig::new_full`, and `Shard::new_with_journal_and_artifact_store`.
    /// Struct-literal construction followed by `validate()` produces the
    /// same aggregated error report as the constructors, closing the gap
    /// described in RS-217.
    pub fn validate(&self) -> Result<(), crate::RuntimeError> {
        validate_shard_config(self)
    }
}

impl Default for ShardConfig {
    fn default() -> Self {
        Self {
            command_queue_capacity: 1024,
            trace_capacity: 4096,
            step_budget_per_tick: 1000,
            max_active_runs: 1024,
            policy: vb_core::policy::RuntimePolicy::Strict,
            // Flipped from 1 to 10 by the P2-14c batched-atomicity bench
            // (see `.evidence/batched_atomicity_bench.json`): A/B measurement
            // shows the coalescing layer provides >= 3x I/O reduction on
            // submit + 100 actions. The 78 test sites that explicitly set
            // `coalesce_window_ticks: 1` continue to get the no-coalescing
            // behavior they assert against; the three sites that use
            // `..ShardConfig::default()` (vb_ipc server trace test, the
            // workspace strict-admission test, and the step-budget helper)
            // are verified to remain green by the gates below.
            coalesce_window_ticks: 10,
            snapshot_interval_steps: 0,
            max_terminal_runs: DEFAULT_MAX_TERMINAL_RUNS,
            terminal_runs_ttl_ticks: DEFAULT_TERMINAL_RUNS_TTL_TICKS,
            max_terminal_outcomes: DEFAULT_MAX_TERMINAL_OUTCOMES,
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
    /// Number of execution steps between periodic snapshots.
    ///
    /// A value of `0` disables periodic mid-run snapshots entirely.
    /// A value of `1` snapshots after every completed step (valid but costly).
    /// A value greater than `1` snapshots after every N completed steps.
    ///
    /// This is immutable after shard creation and applies uniformly to all
    /// runs in the shard.
    pub snapshot_interval_steps: u64,
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
    /// Terminal run ids retained as a bounded LRU ring (MEM-01).
    ///
    /// Capacity and TTL are configured via `ShardConfig::max_terminal_runs`
    /// and `ShardConfig::terminal_runs_ttl_ticks`. Idempotent membership
    /// is preserved by the underlying `IndexSet`; insertion-order is
    /// tracked separately so TTL sweeps can drop the oldest entries
    /// before capacity is consulted.
    pub(crate) terminal_runs: LruRing<RunId>,
    /// Recorded terminal outcome per run id, populated when a run is moved
    /// into `terminal_runs` via cancel/kill/finish/fail.
    ///
    /// Bounded by `max_terminal_outcomes` to prevent unbounded growth when
    /// `terminal_runs_insert` force-grows past `max_terminal_runs` under
    /// sustained load. Newest outcomes win on collision; oldest outcomes
    /// are evicted FIFO when capacity is reached (RQ-W0-10, companion to
    /// MEM-01).
    pub(crate) terminal_outcomes: BoundedOutcomeIndex,
    /// Next durable journal sequence by run, owned by this shard.
    pub(crate) journal_sequences: IndexMap<RunId, EventSeq>,
    pub(crate) pending_timers: IndexMap<RunId, PendingTimer>,
    pub(crate) frame_pools: IndexMap<FramePoolKey, FramePool>,
    pub(crate) trace_ring: TraceRing,
    pub(crate) counters: ShardCounters,
    pub(crate) step_budget_per_tick: u64,
    pub(crate) max_active_runs: usize,
    pub(crate) coalesce_window_ticks: u32,
    pub(crate) policy: vb_core::policy::RuntimePolicy,
    pub(crate) snapshot_interval_steps: u64,
    /// Current logical tick used as the LRU ring's clock source.
    pub(crate) current_tick: TimerTick,
    pub(crate) artifact_store: crate::admission::SharedAcceptedArtifactStore,
    pub(crate) inspect_response: Option<InspectResponse>,
    /// Atomic shutdown flag. Using `AtomicBool` makes the
    /// `enqueue`/`tick` race-free: producers can observe the same
    /// transition as the dispatcher without holding the dispatcher
    /// mutex. The dispatcher remains the only writer.
    pub(crate) shutting_down: std::sync::atomic::AtomicBool,
    pub(crate) journal: SharedRuntimeJournal,
    /// Remaining ticks in the current coalesce window.
    ///
    /// When `coalesce_window_ticks` exceeds 1, this counter decrements
    /// each tick. When it reaches zero, the buffered events are flushed
    /// atomically via `append_sequenced_batch`.
    pub(crate) current_coalesce_window_remaining: u32,
    /// Buffered journal events collected during the coalesce window.
    ///
    /// Each entry pairs a journal event with its per-run starting
    /// sequence so that the batch flush can assign correct sequences
    /// to every event, regardless of which run it belongs to.
    pub(crate) coalesce_buffer: Vec<(RuntimeJournalEvent, EventSeq)>,
    /// Admission gate lock held for the duration of preflight+enqueue.
    ///
    /// `Runtime::submit_*` methods acquire this lock before evaluating the
    /// admission preflight and hold it until the `ShardCommand` is enqueued.
    /// The lock guarantees that two concurrent submits targeting the same
    /// shard cannot squeeze in between the preflight and the enqueue, so the
    /// budget reservation is atomic with the queue commit. The shard's tick
    /// loop does NOT take this lock, so admission is decoupled from run
    /// execution.
    pub(crate) admission_lock: std::sync::Mutex<()>,
    /// Recovered workflows keyed by run, populated during `Runtime::recover`.
    #[cfg(feature = "test-util")]
    pub(crate) pending_workflows: IndexMap<RunId, CompiledWorkflow>,
}

// ============================================================================
// Re-exports for Shard struct dependencies
// ============================================================================

pub use super::run_state::{InspectResponse, RunState, RuntimeState, TerminalOutcome};
pub use super::timer::{PendingTimer, TimerTick};
