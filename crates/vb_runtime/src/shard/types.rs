#![forbid(unsafe_code)]
//! Single-threaded shard owning mutable run state directly.

use crossbeam_queue::ArrayQueue;
use indexmap::{IndexMap, IndexSet};
use std::time::Instant;
use vb_core::action::ActionContract;
use vb_core::capability::CapabilitySet;
use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;
use vb_storage::EventSeq;

use crate::RuntimeResult;
use crate::counters::ShardCounters;
use crate::frame_pool::FramePool;
use crate::journal::SharedRuntimeJournal;
use crate::primitives::collect::CollectStates;
use crate::trace::TraceRing;

// Aggregate resource model touchpoints for vb-qi37.2.1:
// ShardConfig aggregate_capacity, Shard active_usage, Shard reservations,
// RunState AggregateReservation, ShardStatus active_usage aggregate_capacity.

type FramePoolKey = (u16, u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PendingTimerKind {
    Wait,
    Ask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingTimer {
    pub step: StepIdx,
    pub kind: PendingTimerKind,
    pub generation: u64,
    pub deadline: Instant,
}

impl PendingTimer {
    #[must_use]
    pub fn matches_authority(
        self,
        generation: u64,
        deadline: Instant,
        kind: PendingTimerKind,
    ) -> bool {
        self.generation == generation && self.deadline == deadline && self.kind == kind
    }
}

/// Bounded command processed by a shard.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShardCommand {
    /// Submit a new run for execution.
    Submit {
        /// Run identifier chosen by the caller.
        run: RunId,
        /// Compiled workflow to execute.
        workflow: CompiledWorkflow,
        /// Capabilities granted to this run.
        caps: CapabilitySet,
    },
    /// Submit a run whose durable header was already persisted by the runtime shell.
    SubmitPrePersisted {
        /// Run identifier chosen by the caller.
        run: RunId,
        /// Compiled workflow to execute.
        workflow: CompiledWorkflow,
        /// Capabilities granted to this run.
        caps: CapabilitySet,
    },
    /// Submit a new run with runtime input slots already mapped by the caller.
    SubmitWithInputs {
        /// Run identifier chosen by the caller.
        run: RunId,
        /// Compiled workflow to execute.
        workflow: CompiledWorkflow,
        /// Initial slot values written before deterministic execution starts.
        inputs: Box<[(SlotIdx, SlotValue)]>,
        /// Capabilities granted to this run.
        caps: CapabilitySet,
    },
    /// Submit a new run with validated action contracts already bound.
    SubmitWithContracts {
        /// Run identifier chosen by the caller.
        run: RunId,
        /// Compiled workflow to execute.
        workflow: CompiledWorkflow,
        /// Capabilities granted to this run.
        caps: CapabilitySet,
        /// Validated action contracts for Do execution.
        action_contracts: Box<[ActionContract]>,
    },
    /// Submit a new run with input slots and validated action contracts already bound.
    SubmitWithInputsAndContracts {
        /// Run identifier chosen by the caller.
        run: RunId,
        /// Compiled workflow to execute.
        workflow: CompiledWorkflow,
        /// Initial slot values written before deterministic execution starts.
        inputs: Box<[(SlotIdx, SlotValue)]>,
        /// Capabilities granted to this run.
        caps: CapabilitySet,
        /// Validated action contracts for Do execution.
        action_contracts: Box<[ActionContract]>,
    },
    /// Resume a suspended run from its current program counter.
    Resume {
        /// Run identifier.
        run: RunId,
    },
    /// An external action completed.
    ActionCompleted {
        /// Ticket emitted by the suspended Do step.
        ticket: vb_core::action::ActionTicket,
        /// Typed action output payload.
        output: vb_core::action::ActionOutputReady,
    },
    /// An external action completed without a typed output payload.
    ActionCompletedLegacy {
        /// Run identifier.
        run: RunId,
        /// Step that was waiting for this action.
        step: StepIdx,
    },
    /// An external action failed.
    ActionFailed {
        /// Ticket for the action being failed.
        ticket: vb_core::action::ActionTicket,
        /// Typed failure payload.
        failure: vb_core::action::ActionFailure,
    },
    /// Public runtime facade action failure.
    RuntimeActionFailed {
        /// Ticket for the action being failed.
        ticket: vb_core::action::ActionTicket,
        /// Typed failure payload.
        failure: vb_core::action::ActionFailure,
    },
    /// An external ask was answered.
    AskAnswered {
        /// Typed ask answer payload.
        answer: AskAnswer,
    },
    /// A timer fired for a suspended run.
    TimerFired {
        /// Run identifier.
        run: RunId,
        /// Freshness generation captured when the timer was emitted.
        generation: u64,
        /// Deadline captured when the timer was emitted.
        deadline: Instant,
        /// Timer kind captured when the timer was emitted.
        kind: PendingTimerKind,
    },
    /// Cancel an active run.
    Cancel {
        /// Run identifier.
        run: RunId,
        /// Optional cancellation reason.
        reason: Option<String>,
    },
    /// Kill an active run unconditionally.
    Kill {
        /// Run identifier.
        run: RunId,
        /// Optional kill reason.
        reason: Option<String>,
    },
    /// Inspect run state for diagnostic purposes.
    Inspect {
        /// Run identifier.
        run: RunId,
        /// Caller correlation identifier echoed in the response.
        correlation: u64,
    },
    /// Shut down the shard gracefully.
    Shutdown,
}

/// Ticket identifying where an ask answer must resume execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AskTicket {
    /// Owning run.
    pub run: RunId,
    /// Step that issued the ask and is currently marked asking.
    pub ask_step: StepIdx,
    /// Step that consumes the answer slot, usually an AskResume node.
    pub resume_step: StepIdx,
}

/// Explicit ask answer contract. The caller supplies both payload and destination slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AskAnswer {
    /// Ask ticket proving the intended resume point.
    pub ticket: AskTicket,
    /// Slot that receives the answer before resuming.
    pub answer_slot: SlotIdx,
    /// Answer payload.
    pub value: SlotValue,
    /// Answer taint marker.
    pub taint: Taint,
    /// Encoded length of the answer payload in bytes.
    pub encoded_len: u32,
}

impl AskAnswer {
    /// Creates an answer when the caller has not precomputed encoded size.
    #[must_use]
    pub fn new(ticket: AskTicket, answer_slot: SlotIdx, value: SlotValue, taint: Taint) -> Self {
        Self {
            ticket,
            answer_slot,
            value,
            taint,
            encoded_len: 0,
        }
    }

    /// Creates an answer with explicit encoded payload length.
    #[must_use]
    pub fn with_encoded_len(
        ticket: AskTicket,
        answer_slot: SlotIdx,
        value: SlotValue,
        taint: Taint,
        encoded_len: u32,
    ) -> Self {
        Self {
            ticket,
            answer_slot,
            value,
            taint,
            encoded_len,
        }
    }
}

/// Mutable run state owned directly by the shard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunState {
    /// Active run frame.
    pub frame: RunFrame,
    /// Compiled workflow for this run.
    pub workflow: CompiledWorkflow,
    /// Cold value store for list, object, and blob handles.
    pub store: ValueStore,
    /// Per-Do-step attempt counters owned with the live frame.
    pub action_attempts: Box<[u16]>,
    /// Admission record for this run, if admission gating was performed.
    pub admission: Option<crate::admission::RunAdmission>,
    /// Per-run collect pagination state side table.
    pub collect_states: CollectStates,
    /// Validated action contracts used by Do execution.
    pub action_contracts: Box<[ActionContract]>,
}

/// Diagnostic snapshot returned by the Inspect command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectSnapshot {
    /// Run identifier.
    pub run: RunId,
    /// Caller correlation identifier.
    pub correlation: u64,
    /// Current program counter.
    pub pc: StepIdx,
    /// Number of executed transitions.
    pub executed: u64,
}

/// Bounded response produced by an inspect command.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InspectResponse {
    /// The run was active and a snapshot was captured.
    Found(InspectSnapshot),
    /// The run was not active on this shard.
    NotFound {
        /// Run identifier.
        run: RunId,
        /// Caller correlation identifier.
        correlation: u64,
    },
}

// ============================================================================
// RAII Introspection Registry
// ============================================================================

/// Outcome of an unregister operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnregisterOutcome {
    /// The handle was successfully unregistered.
    Unregistered,
    /// The handle was not found (no-op).
    Missing,
}

/// Outcome of a register operation when overlap is detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterOverlapOutcome {
    /// Registration was rejected due to conflict with existing registration.
    Conflict,
    /// Registration replaced the existing one with a new epoch.
    Replaced {
        /// The epoch of the replaced registration.
        old_epoch: u64,
        /// The epoch of the new registration.
        new_epoch: u64,
    },
}

/// Epoch-based handle for an introspection registration.
///
/// When dropped, the handle is automatically unregistered from the registry.
#[derive(Debug)]
pub struct InspectHandle {
    run: RunId,
    epoch: u64,
    registry: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<RunId, u64>>>,
}

impl InspectHandle {
    /// Returns the run identifier associated with this handle.
    #[must_use]
    pub fn run(&self) -> RunId {
        self.run
    }

    /// Returns the epoch of this handle.
    #[must_use]
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}

impl Drop for InspectHandle {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.registry.lock() {
            // Only remove if the epoch matches (handles stale drops correctly)
            if let Some(current_epoch) = guard.get(&self.run)
                && *current_epoch == self.epoch
            {
                guard.remove(&self.run);
            }
        }
    }
}

/// Registry for RAII-based introspection handles.
///
/// Provides epoch-based registration with automatic cleanup on guard drop.
/// Does NOT create global mutable run state - each registry instance is independent.
#[derive(Default)]
pub struct IntrospectionRegistry {
    inner: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<RunId, u64>>>,
    next_epoch: u64,
}

impl IntrospectionRegistry {
    /// Creates a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a handle for the given run.
    ///
    /// Returns the handle guard on success.
    pub fn register(&mut self, run: RunId) -> RuntimeResult<InspectHandle> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| crate::RuntimeError::JournalPoisoned)?;

        // Check if already registered
        if guard.contains_key(&run) {
            return Err(crate::RuntimeError::RunAlreadyExists);
        }

        let epoch = self.next_epoch;
        self.next_epoch = self.next_epoch.saturating_add(1);
        guard.insert(run, epoch);

        Ok(InspectHandle {
            run,
            epoch,
            registry: self.inner.clone(),
        })
    }

    /// Registers a handle for the given run, allowing epoch replacement on conflict.
    ///
    /// Returns outcome indicating whether registration succeeded, conflicted, or was replaced.
    pub fn register_with_overlap_policy(
        &mut self,
        run: RunId,
    ) -> RuntimeResult<(InspectHandle, Result<(), RegisterOverlapOutcome>)> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| crate::RuntimeError::JournalPoisoned)?;

        let (outcome, epoch) = if let Some(&old_epoch) = guard.get(&run) {
            // Overlap detected - replace with new epoch
            let new_epoch = self.next_epoch;
            self.next_epoch = self.next_epoch.saturating_add(1);
            guard.insert(run, new_epoch);
            (
                Err(RegisterOverlapOutcome::Replaced {
                    old_epoch,
                    new_epoch,
                }),
                new_epoch,
            )
        } else {
            // No overlap - insert with new epoch
            let epoch = self.next_epoch;
            self.next_epoch = self.next_epoch.saturating_add(1);
            guard.insert(run, epoch);
            (Ok(()), epoch)
        };

        Ok((
            InspectHandle {
                run,
                epoch,
                registry: self.inner.clone(),
            },
            outcome,
        ))
    }

    /// Unregisters a handle for the given run.
    ///
    /// Returns whether the handle was found and unregistered.
    pub fn unregister(&mut self, run: RunId) -> RuntimeResult<UnregisterOutcome> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| crate::RuntimeError::JournalPoisoned)?;

        if guard.remove(&run).is_some() {
            Ok(UnregisterOutcome::Unregistered)
        } else {
            Ok(UnregisterOutcome::Missing)
        }
    }

    /// Unregisters all handles.
    ///
    /// Returns the count of handles removed.
    pub fn unregister_all(&mut self) -> RuntimeResult<usize> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| crate::RuntimeError::JournalPoisoned)?;
        let count = guard.len();
        guard.clear();
        Ok(count)
    }

    /// Returns whether a run is currently visible to introspection.
    #[must_use]
    pub fn is_visible(&self, run: RunId) -> bool {
        if let Ok(guard) = self.inner.lock() {
            guard.contains_key(&run)
        } else {
            false
        }
    }
}

/// Snapshot formatting stays cold path (no computation on hot path).
pub struct InspectSnapshotFormatter;

impl InspectSnapshotFormatter {
    /// Formats a snapshot response into a string representation.
    ///
    /// This is a cold-path operation - called only when formatting output,
    /// not during the hot path of inspect operations.
    #[must_use]
    pub fn format_snapshot(run: RunId, response: &InspectResponse) -> String {
        match response {
            InspectResponse::Found(snap) => {
                format!(
                    "InspectSnapshot {{ run: {:?}, correlation: {}, pc: {:?}, executed: {} }}",
                    run, snap.correlation, snap.pc, snap.executed
                )
            }
            InspectResponse::NotFound { run, correlation } => {
                format!(
                    "NotFound {{ run: {:?}, correlation: {} }}",
                    run, correlation
                )
            }
        }
    }
}

/// Maximum bounded command queue capacity per shard.
pub const MAX_COMMAND_QUEUE_CAPACITY: usize = 65_536;

/// Returns true when a command queue capacity is inside the supported domain.
#[must_use]
pub const fn is_valid_command_queue_capacity(capacity: usize) -> bool {
    capacity > 0 && capacity <= MAX_COMMAND_QUEUE_CAPACITY
}

// ============================================================================
// ShardCommandQueue — domain wrapper around ArrayQueue<ShardCommand>
// ============================================================================

/// Domain-named wrapper around `crossbeam_queue::ArrayQueue<ShardCommand>`.
///
/// Provides a bounded, non-blocking command queue with domain-specific terminology
/// (`enqueue`, `pop`, `is_full`, `remaining_capacity`) and proper error taxonomy
/// (`RuntimeError::QueueFull`). This wrapper establishes the `ShardCommand` queue
/// as a first-class domain boundary rather than a raw field.
pub struct ShardCommandQueue {
    inner: ArrayQueue<ShardCommand>,
    /// Stored capacity to satisfy POST-001 and INV-001 invariants.
    capacity: usize,
}

impl ShardCommandQueue {
    /// Creates a new `ShardCommandQueue` with the given capacity.
    ///
    /// # Errors
    /// Returns `RuntimeError::CommandQueueCapacityExceeded` if `capacity` is 0
    /// or exceeds `MAX_COMMAND_QUEUE_CAPACITY`.
    pub fn new(capacity: usize) -> RuntimeResult<Self> {
        if !is_valid_command_queue_capacity(capacity) {
            return Err(crate::RuntimeError::CommandQueueCapacityExceeded {
                capacity,
                max: MAX_COMMAND_QUEUE_CAPACITY,
            });
        }
        Ok(Self {
            inner: ArrayQueue::new(capacity),
            capacity,
        })
    }

    /// Creates a command queue from an already-accepted shard configuration.
    ///
    /// `Shard::new` has historically been infallible and accepted `ShardConfig`
    /// by value. The validated constructor for externally supplied capacity is
    /// `ShardConfig::new`; this helper preserves `Shard::new`'s existing shape
    /// while placing the raw queue construction behind the domain wrapper.
    pub(crate) fn from_config(config: ShardConfig) -> Self {
        Self {
            inner: ArrayQueue::new(config.command_queue_capacity),
            capacity: config.command_queue_capacity,
        }
    }

    /// Enqueues a command. Returns `Ok(())` if the command was enqueued, or
    /// `Err(RuntimeError::QueueFull)` if the queue is at capacity.
    ///
    /// This operation is non-blocking and never allocates on failure.
    pub fn enqueue(&self, cmd: ShardCommand) -> RuntimeResult<()> {
        self.inner
            .push(cmd)
            .map_err(|_| crate::RuntimeError::QueueFull)
    }

    /// Dequeues the frontmost command, if any.
    ///
    /// Returns `Some(cmd)` in FIFO order, or `None` if the queue is empty.
    pub fn pop(&self) -> Option<ShardCommand> {
        self.inner.pop()
    }

    /// Returns the number of commands currently in the queue.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the queue contains no commands.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the capacity of this queue (set at construction).
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the number of remaining free slots in the queue.
    #[must_use]
    pub fn remaining_capacity(&self) -> usize {
        self.capacity.saturating_sub(self.inner.len())
    }

    /// Returns `true` if the queue is at capacity.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.inner.len() == self.capacity
    }

    /// Returns the compile-time bounded capacity limit (65536).
    ///
    /// This is the maximum capacity any `ShardCommandQueue` can be configured with.
    #[must_use]
    pub const fn bounded_capacity() -> usize {
        MAX_COMMAND_QUEUE_CAPACITY
    }
}

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

/// Lifecycle state of a run tracked by the runtime for resume eligibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuntimeState {
    /// Run was created but has not yet been started.
    Initial,
    /// Run is actively executing.
    Running,
    /// Run suspended and can be resumed.
    Resumable,
    /// Resume is in flight for this run.
    Resuming,
    /// Run terminated with a failure.
    Failed,
}

impl RuntimeState {
    /// Returns true if this state is a valid target for resume.
    #[must_use]
    pub fn is_resumable(&self) -> bool {
        matches!(self, Self::Resumable)
    }
}

/// Runtime events that drive state transitions in the RuntimeStateMachine.
/// Each variant corresponds to a distinct operational event in the shard lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuntimeEvent {
    /// A new run has been submitted and inserted into the shard.
    Submit,
    /// An existing run is being resumed from a suspended state.
    Resume,
    /// Resume journal append failed, revert to Resumable state.
    ResumeRollback,
    /// A run's deterministic execution is continuing after a drive tick.
    DriveContinue,
    /// A run has reached a terminal finished state.
    DriveFinished,
    /// A run is awaiting an external action response.
    AwaitAction,
    /// A run is awaiting a timer (wait or ask timeout).
    AwaitTimer,
    /// A run has reached a terminal failed state.
    Fail,
    /// Remove run from runtime_states tracking (terminal).
    TerminalRemove,
}

impl RuntimeEvent {
    /// Returns true if this event produces a terminal state (run is removed from runtime_states).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Fail | Self::TerminalRemove | Self::DriveFinished
        )
    }

    /// Returns true if this event sets a Resumable state.
    #[must_use]
    pub fn is_resumable(&self) -> bool {
        matches!(self, Self::AwaitAction | Self::AwaitTimer | Self::Resume)
    }
}

/// Status of a resume operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResumeStatus {
    /// Resume was accepted and the run was driven once.
    ///
    /// The post-drive lifecycle may be `Running`, `Resumable`, or terminal,
    /// depending on the deterministic engine signal emitted by that drive.
    Resumed,
    /// Run was already running when resume was attempted.
    AlreadyRunning,
}

/// Result of a successful resume operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeResult {
    /// The run identifier that was resumed.
    pub run_id: RunId,
    /// The status of the resume operation.
    pub status: ResumeStatus,
    /// Monotonic timestamp when the resume occurred.
    pub timestamp: u64,
}

/// Errors that can occur during a resume operation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResumeError {
    /// The run identifier was not found in the journal.
    RunIdNotFound {
        /// The run identifier that was not found.
        run_id: RunId,
    },
    /// The run is not in a resumable state.
    NotResumable {
        /// The run identifier.
        run_id: RunId,
        /// The current state of the run.
        current_state: RuntimeState,
    },
    /// Journal hydration is incomplete for this run.
    IncompleteHydration {
        /// The run identifier.
        run_id: RunId,
    },
    /// Failed to append the Resumed event to the journal.
    JournalAppendFailed,
    /// Failed to append the Resumed event with a preserved runtime source.
    JournalAppendFailedWithSource {
        /// Runtime failure that caused the journal append failure.
        source: Box<crate::RuntimeError>,
    },
    /// Failed to produce structured output.
    StructuredOutputFailed,
}

impl ResumeError {
    pub(crate) fn journal_append_failed_with_source(source: crate::RuntimeError) -> Self {
        Self::JournalAppendFailedWithSource {
            source: Box::new(source),
        }
    }

    /// Returns the runtime source bound to this resume journal failure on this thread.
    #[must_use]
    pub fn source_runtime_error(&self) -> Option<crate::RuntimeError> {
        match self {
            Self::JournalAppendFailedWithSource { source } => Some(source.as_ref().clone()),
            // NOTE: #[non_exhaustive] - new ResumeError variants return None for source_runtime_error.
            // Implementations should add explicit variant handling.
            _ => None,
        }
    }
}

// ============================================================================
// Numeric Timer Seam Types
// ============================================================================

/// A monotonically increasing timer tick value, counting logical time units.
///
/// Wraps a `u64` to provide type safety and checked arithmetic for deterministic
/// clock control. One tick represents one logical time unit in the deterministic
/// timer seam, operating alongside the existing wall-clock `Instant`-based timers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimerTick(u64);

impl TimerTick {
    /// Creates a new timer tick at the given value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the inner `u64` value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Advances the tick by a duration, returning the resulting tick.
    ///
    /// Returns `None` on overflow.
    #[must_use]
    pub fn checked_add(self, duration: TimerDuration) -> Option<Self> {
        self.0.checked_add(duration.get()).map(Self)
    }

    /// Returns `true` if this tick is at or past the given deadline.
    #[must_use]
    pub fn has_elapsed(self, deadline: TimerDeadline) -> bool {
        self.0 >= deadline.get()
    }
}

/// A timer duration measured in ticks, representing a span of logical time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimerDuration(u64);

impl TimerDuration {
    /// Creates a new duration with the given number of ticks.
    #[must_use]
    pub const fn new(ticks: u64) -> Self {
        Self(ticks)
    }

    /// Returns the inner `u64` value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the duration as a tick count.
    #[must_use]
    pub const fn as_ticks(self) -> u64 {
        self.0
    }

    /// Returns a zero-length duration.
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }
}

/// An absolute deadline in ticks, representing when a timer expires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimerDeadline(u64);

impl TimerDeadline {
    /// Creates a new deadline at the given tick value.
    #[must_use]
    pub const fn new(tick: u64) -> Self {
        Self(tick)
    }

    /// Returns the inner `u64` value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Creates a deadline by adding a duration to a tick.
    ///
    /// Returns `None` on overflow.
    #[must_use]
    pub fn from_tick_and_duration(tick: TimerTick, duration: TimerDuration) -> Option<Self> {
        tick.get().checked_add(duration.get()).map(Self)
    }

    /// Returns `true` if this deadline has passed relative to the given tick.
    #[must_use]
    pub fn is_past(self, current: TimerTick) -> bool {
        current.has_elapsed(self)
    }
}

/// Kind of timer managed by the numeric timer seam.
///
/// Used alongside the existing `PendingTimerKind` to provide a richer
/// timer taxonomy for deterministic execution control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TimerKind {
    /// Retry timer — combined wait/ask semantics for deterministic execution.
    Retry,
    /// Delayed action bound to a specific action identifier.
    DelayedAction(ActionId),
}


