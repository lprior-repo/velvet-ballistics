#![forbid(unsafe_code)]
//! Single-threaded shard owning mutable run state directly.

use crossbeam_queue::ArrayQueue;
use indexmap::{IndexMap, IndexSet};
use std::time::Instant;
use vb_core::action::ActionContract;
use vb_core::capability::CapabilitySet;
use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
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

/// Internal recovery command payload used to keep recovery evidence grouped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecoverRunCommand {
    /// Run identifier being recovered.
    pub(crate) run: RunId,
    /// Hydrated run frame from journal replay.
    pub(crate) frame: RunFrame,
    /// Durable accepted-artifact digest from `RunAdmission`.
    pub(crate) artifact_digest: WorkflowDigest,
    /// Workflow/source digest retained for artifact binding validation.
    pub(crate) workflow_digest: WorkflowDigest,
    /// Next durable sequence number after the recovered prefix.
    pub(crate) next_seq: EventSeq,
    /// Recovered collect pagination side table from durable frame extras.
    pub(crate) collect_states: CollectStates,
    /// Recovered suspended boundary, if durable evidence parked the run.
    pub(crate) boundary: crate::recovery::RecoveredRunBoundary,
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
    /// Recover a run from durable Fjall journal evidence.
    ///
    /// The runtime has already reconstructed a `RunFrame` from the journal
    /// replay and is injecting it directly into the shard's frame pool.
    /// The durable accepted-artifact digest is used to look up the compiled
    /// workflow; the workflow digest is retained for source-binding validation.
    Recover {
        /// Run identifier being recovered.
        run: RunId,
        /// Hydrated run frame from journal replay.
        frame: RunFrame,
        /// Durable accepted-artifact digest from `RunAdmission`.
        artifact_digest: WorkflowDigest,
        /// Workflow digest for artifact store lookup during recovery.
        workflow_digest: WorkflowDigest,
        /// Next durable sequence number after the recovered prefix.
        next_seq: EventSeq,
        /// Recovered collect pagination side table from durable frame extras.
        collect_states: CollectStates,
        /// Recovered suspended boundary, if durable evidence parked the run.
        boundary: crate::recovery::RecoveredRunBoundary,
    },
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

    /// Acquires the registry's inner mutex, recovering from any prior
    /// poison state. The standard `Mutex::lock` returns `PoisonError`
    /// forever once a holder panics; this helper mirrors the recovery
    /// pattern already used by `action_queue` so the admission gate does
    /// not stay permanently disabled after a panic.
    fn lock_or_recover(
        inner: &std::sync::Arc<std::sync::Mutex<std::collections::HashMap<RunId, u64>>>,
    ) -> std::sync::MutexGuard<'_, std::collections::HashMap<RunId, u64>> {
        match inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// Registers a handle for the given run.
    ///
    /// Returns the handle guard on success. Recovers transparently from
    /// a poisoned mutex so a single panicking holder cannot permanently
    /// disable the admission gate (RA-014).
    pub fn register(&mut self, run: RunId) -> RuntimeResult<InspectHandle> {
        let mut guard = Self::lock_or_recover(&self.inner);

        // Check if already registered
        if guard.contains_key(&run) {
            return Err(crate::RuntimeError::RunAlreadyExists);
        }

        let epoch = self.next_epoch;
        self.next_epoch = self
            .next_epoch
            .checked_add(1)
            .ok_or(crate::RuntimeError::IntrospectionEpochExhausted)?;
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
        let mut guard = Self::lock_or_recover(&self.inner);

        let (outcome, epoch) = if let Some(&old_epoch) = guard.get(&run) {
            // Overlap detected - replace with new epoch
            let new_epoch = self.next_epoch;
            self.next_epoch = self
                .next_epoch
                .checked_add(1)
                .ok_or(crate::RuntimeError::IntrospectionEpochExhausted)?;
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
            self.next_epoch = self
                .next_epoch
                .checked_add(1)
                .ok_or(crate::RuntimeError::IntrospectionEpochExhausted)?;
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
        let mut guard = Self::lock_or_recover(&self.inner);

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
        let mut guard = Self::lock_or_recover(&self.inner);
        let count = guard.len();
        guard.clear();
        Ok(count)
    }

    /// Returns whether a run is currently visible to introspection.
    #[must_use]
    pub fn is_visible(&self, run: RunId) -> bool {
        match self.inner.lock() {
            Ok(guard) => guard.contains_key(&run),
            Err(poisoned) => poisoned.into_inner().contains_key(&run),
        }
    }
}

#[cfg(test)]
impl IntrospectionRegistry {
    /// Returns a clone of the inner `Arc<Mutex<...>>` for tests that need
    /// to poison the registry from another thread. Production code MUST
    /// NOT use this — it is compiled out of release builds.
    pub(crate) fn inner_arc_for_test(
        &self,
    ) -> std::sync::Arc<std::sync::Mutex<std::collections::HashMap<RunId, u64>>> {
        self.inner.clone()
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
    pub fn format_snapshot(response: &InspectResponse) -> String {
        match response {
            InspectResponse::Found(snap) => {
                format!(
                    "InspectSnapshot {{ run: {:?}, correlation: {}, pc: {:?}, executed: {} }}",
                    snap.run, snap.correlation, snap.pc, snap.executed
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
    /// Last frame executed count already reflected in shard counters by run.
    pub(crate) accounted_executed_steps: IndexMap<RunId, u64>,
    pub(crate) pending_timers: IndexMap<RunId, PendingTimer>,
    /// In-flight `ActionTicket`s by run. Inserted when `await_action`
    /// journal appends an `ActionScheduledTicket`; cleared when the
    /// matching completion/failure/abandon event is journaled.
    pub(crate) pending_actions: IndexMap<RunId, vb_core::action::ActionTicket>,
    /// Per-run action ABI digests computed at admission/recovery time.
    /// The schedule/completion hot path only performs bounded lookup.
    pub(crate) action_abi_digests: IndexMap<RunId, Box<[(ActionId, WorkflowDigest)]>>,
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
    /// A pre-commit resume or drive append failed; revert to retryable state.
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
        matches!(
            self,
            Self::AwaitAction | Self::AwaitTimer | Self::ResumeRollback
        )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RuntimeError;
    use std::time::Instant;

    // ---- PendingTimer ----

    #[test]
    fn pending_timer_constructor_sets_fields_correctly() {
        let deadline = Instant::now();
        let timer = PendingTimer {
            step: vb_core::ids::StepIdx::new(3),
            kind: PendingTimerKind::Ask,
            generation: 42,
            deadline,
        };
        assert_eq!(timer.step, vb_core::ids::StepIdx::new(3));
        assert_eq!(timer.kind, PendingTimerKind::Ask);
        assert_eq!(timer.generation, 42);
        assert_eq!(timer.deadline, deadline);
    }

    #[test]
    fn pending_timer_matches_authority_when_all_fields_match() {
        let deadline = Instant::now();
        let timer = PendingTimer {
            step: vb_core::ids::StepIdx::ZERO,
            kind: PendingTimerKind::Wait,
            generation: 5,
            deadline,
        };
        assert!(timer.matches_authority(5, deadline, PendingTimerKind::Wait));
    }

    #[test]
    fn pending_timer_matches_authority_rejects_wrong_generation() {
        let timer = PendingTimer {
            step: vb_core::ids::StepIdx::ZERO,
            kind: PendingTimerKind::Wait,
            generation: 5,
            deadline: Instant::now(),
        };
        assert!(!timer.matches_authority(6, timer.deadline, PendingTimerKind::Wait));
    }

    #[test]
    fn pending_timer_matches_authority_rejects_wrong_kind() {
        let timer = PendingTimer {
            step: vb_core::ids::StepIdx::ZERO,
            kind: PendingTimerKind::Wait,
            generation: 3,
            deadline: Instant::now(),
        };
        assert!(!timer.matches_authority(3, timer.deadline, PendingTimerKind::Ask));
    }

    #[test]
    fn pending_timer_is_copy() {
        let t1 = PendingTimer {
            step: vb_core::ids::StepIdx::new(1),
            kind: PendingTimerKind::Ask,
            generation: 7,
            deadline: Instant::now(),
        };
        let t2 = t1;
        assert_eq!(t1.generation, t2.generation);
        assert_eq!(t1.step, t2.step);
        assert_eq!(t1.kind, t2.kind);
    }

    // ---- PendingTimerKind ----

    #[test]
    fn pending_timer_kind_wait_and_ask_are_distinct() {
        assert_ne!(PendingTimerKind::Wait, PendingTimerKind::Ask);
        assert_eq!(PendingTimerKind::Wait, PendingTimerKind::Wait);
        assert_eq!(PendingTimerKind::Ask, PendingTimerKind::Ask);
    }

    // ---- is_valid_command_queue_capacity ----

    #[test]
    fn is_valid_capacity_accepts_one() {
        assert!(is_valid_command_queue_capacity(1));
    }

    #[test]
    fn is_valid_capacity_accepts_max() {
        assert!(is_valid_command_queue_capacity(MAX_COMMAND_QUEUE_CAPACITY));
    }

    #[test]
    fn is_valid_capacity_rejects_zero() {
        assert!(!is_valid_command_queue_capacity(0));
    }

    #[test]
    fn is_valid_capacity_rejects_exceeding_max() {
        assert!(!is_valid_command_queue_capacity(
            MAX_COMMAND_QUEUE_CAPACITY + 1
        ));
    }

    // ---- ShardCommandQueue ----

    #[test]
    fn command_queue_new_accepts_valid_capacity() {
        let result = ShardCommandQueue::new(64);
        match result {
            Ok(q) => {
                assert_eq!(q.capacity(), 64);
                assert!(q.is_empty());
                assert!(!q.is_full());
            }
            Err(_e) => panic!("unexpected error constructing queue"),
        }
    }

    #[test]
    fn command_queue_new_rejects_zero_capacity() {
        let result = ShardCommandQueue::new(0);
        match result {
            Err(RuntimeError::CommandQueueCapacityExceeded { capacity, max }) => {
                assert_eq!(capacity, 0);
                assert_eq!(max, MAX_COMMAND_QUEUE_CAPACITY);
            }
            _other => panic!("unexpected result constructing queue"),
        }
    }

    #[test]
    fn command_queue_new_rejects_exceeding_max() {
        let result = ShardCommandQueue::new(MAX_COMMAND_QUEUE_CAPACITY + 1);
        assert!(result.is_err());
        match result {
            Err(RuntimeError::CommandQueueCapacityExceeded { .. }) => {}
            _other => panic!("unexpected result constructing queue"),
        }
    }

    #[test]
    fn command_queue_remaining_capacity_decreases_after_enqueue() {
        let q = ShardCommandQueue::new(2).unwrap();
        assert_eq!(q.remaining_capacity(), 2);
        let cmd = ShardCommand::Shutdown;
        assert_eq!(q.enqueue(cmd), Ok(()));
        assert_eq!(q.remaining_capacity(), 1);
    }

    #[test]
    fn command_queue_is_full_after_filling_to_capacity() {
        let q = ShardCommandQueue::new(1).unwrap();
        assert!(!q.is_full());
        assert_eq!(q.enqueue(ShardCommand::Shutdown), Ok(()));
        assert!(q.is_full());
    }

    #[test]
    fn command_queue_enqueue_rejects_when_full() {
        let q = ShardCommandQueue::new(1).unwrap();
        assert_eq!(q.enqueue(ShardCommand::Shutdown), Ok(()));
        let result = q.enqueue(ShardCommand::Shutdown);
        assert_eq!(result, Err(RuntimeError::QueueFull));
    }

    #[test]
    fn command_queue_pop_returns_fifo_order() {
        let q = ShardCommandQueue::new(2).unwrap();
        let cmd1 = ShardCommand::Shutdown;
        let cmd2 = ShardCommand::Cancel {
            run: vb_core::ids::RunId::new(1),
            reason: None,
        };
        assert_eq!(q.enqueue(cmd1.clone()), Ok(()));
        assert_eq!(q.enqueue(cmd2.clone()), Ok(()));
        assert_eq!(q.pop(), Some(cmd1));
        assert_eq!(q.pop(), Some(cmd2));
        assert!(q.is_empty());
    }

    #[test]
    fn command_queue_len_accurately_reports_count() {
        let q = ShardCommandQueue::new(8).unwrap();
        assert_eq!(q.len(), 0);
        assert_eq!(q.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(q.len(), 1);
        assert_eq!(q.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(q.len(), 2);
        q.pop();
        assert_eq!(q.len(), 1);
    }

    // ---- ShardConfig ----

    #[test]
    fn shard_config_default_has_valid_capacity() {
        let config = ShardConfig::default();
        assert!(config.command_queue_capacity > 0);
        assert!(is_valid_command_queue_capacity(
            config.command_queue_capacity
        ));
    }

    // ---- RuntimeState ----

    #[test]
    fn runtime_state_resumable_is_resumable() {
        assert!(RuntimeState::Resumable.is_resumable());
    }

    #[test]
    fn runtime_state_running_is_not_resumable() {
        assert!(!RuntimeState::Running.is_resumable());
    }

    #[test]
    fn runtime_state_failed_is_not_resumable() {
        assert!(!RuntimeState::Failed.is_resumable());
    }

    #[test]
    fn runtime_state_initial_is_not_resumable() {
        assert!(!RuntimeState::Initial.is_resumable());
    }

    // ---- RuntimeEvent ----

    #[test]
    fn runtime_event_await_action_is_resumable() {
        assert!(RuntimeEvent::AwaitAction.is_resumable());
    }

    #[test]
    fn runtime_event_await_timer_is_resumable() {
        assert!(RuntimeEvent::AwaitTimer.is_resumable());
    }

    #[test]
    fn runtime_event_fail_is_terminal() {
        assert!(RuntimeEvent::Fail.is_terminal());
    }

    #[test]
    fn runtime_event_drive_finished_is_terminal() {
        assert!(RuntimeEvent::DriveFinished.is_terminal());
    }

    // ---- Numeric Timer Types ----

    #[test]
    fn timer_tick_new_returns_expected_value() {
        let tick = TimerTick::new(42);
        assert_eq!(tick.get(), 42);
    }

    #[test]
    fn timer_tick_checked_add_succeeds() {
        let tick = TimerTick::new(10);
        let dur = TimerDuration::new(5);
        let result = tick.checked_add(dur);
        assert_eq!(result, Some(TimerTick::new(15)));
    }

    #[test]
    fn timer_tick_checked_add_returns_none_on_overflow() {
        let tick = TimerTick::new(u64::MAX);
        let dur = TimerDuration::new(1);
        assert_eq!(tick.checked_add(dur), None);
    }

    #[test]
    fn timer_tick_has_elapsed_when_tick_eq_deadline() {
        let tick = TimerTick::new(100);
        let deadline = TimerDeadline::new(100);
        assert!(tick.has_elapsed(deadline));
    }

    #[test]
    fn timer_tick_has_elapsed_when_tick_past_deadline() {
        let tick = TimerTick::new(101);
        let deadline = TimerDeadline::new(100);
        assert!(tick.has_elapsed(deadline));
    }

    #[test]
    fn timer_tick_has_not_elapsed_when_before_deadline() {
        let tick = TimerTick::new(99);
        let deadline = TimerDeadline::new(100);
        assert!(!tick.has_elapsed(deadline));
    }

    #[test]
    fn timer_tick_ord_sorts_correctly() {
        let a = TimerTick::new(10);
        let b = TimerTick::new(20);
        assert!(a < b);
        assert_eq!(a, TimerTick::new(10));
    }

    #[test]
    fn timer_duration_new_and_get() {
        let dur = TimerDuration::new(30);
        assert_eq!(dur.get(), 30);
        assert_eq!(dur.as_ticks(), 30);
    }

    #[test]
    fn timer_duration_zero_is_zero() {
        let dur = TimerDuration::zero();
        assert_eq!(dur.get(), 0);
    }

    #[test]
    fn timer_duration_ord_sorts_correctly() {
        let a = TimerDuration::new(5);
        let b = TimerDuration::new(10);
        assert!(a < b);
    }

    #[test]
    fn timer_deadline_new_and_get() {
        let dl = TimerDeadline::new(77);
        assert_eq!(dl.get(), 77);
    }

    #[test]
    fn timer_deadline_from_tick_and_duration_succeeds() {
        let tick = TimerTick::new(10);
        let dur = TimerDuration::new(20);
        let deadline = TimerDeadline::from_tick_and_duration(tick, dur);
        assert_eq!(deadline, Some(TimerDeadline::new(30)));
    }

    #[test]
    fn timer_deadline_from_tick_and_duration_returns_none_on_overflow() {
        let tick = TimerTick::new(u64::MAX);
        let dur = TimerDuration::new(1);
        assert_eq!(TimerDeadline::from_tick_and_duration(tick, dur), None);
    }

    #[test]
    fn timer_deadline_is_past_when_current_tick_is_equal() {
        let current = TimerTick::new(50);
        let deadline = TimerDeadline::new(50);
        assert!(deadline.is_past(current));
    }

    #[test]
    fn timer_deadline_is_past_when_current_tick_exceeds() {
        let current = TimerTick::new(51);
        let deadline = TimerDeadline::new(50);
        assert!(deadline.is_past(current));
    }

    #[test]
    fn timer_deadline_is_not_past_when_current_tick_before() {
        let current = TimerTick::new(49);
        let deadline = TimerDeadline::new(50);
        assert!(!deadline.is_past(current));
    }

    #[test]
    fn timer_deadline_ord_sorts_correctly() {
        let a = TimerDeadline::new(15);
        let b = TimerDeadline::new(25);
        assert!(a < b);
    }

    #[test]
    fn timer_kind_retry_and_delayed_action_are_distinct() {
        let kind1 = TimerKind::Retry;
        let kind2 = TimerKind::DelayedAction(ActionId::new(1));
        assert_ne!(kind1, kind2);
    }

    #[test]
    fn timer_kind_delayed_action_preserves_action_id() {
        let action = ActionId::new(42);
        let kind = TimerKind::DelayedAction(action);
        match kind {
            TimerKind::DelayedAction(aid) => assert_eq!(aid, ActionId::new(42)),
            _ => panic!("expected DelayedAction variant"),
        }
    }

    #[test]
    fn timer_tick_copy_and_eq_preserves_value() {
        let t1 = TimerTick::new(5);
        let t2 = t1;
        assert_eq!(t1, t2);
    }

    #[test]
    fn timer_duration_copy_and_eq_preserves_value() {
        let d1 = TimerDuration::new(10);
        let d2 = d1;
        assert_eq!(d1, d2);
    }

    #[test]
    fn timer_deadline_copy_and_eq_preserves_value() {
        let dl1 = TimerDeadline::new(20);
        let dl2 = dl1;
        assert_eq!(dl1, dl2);
    }

    // ---- Numeric Timer Boundary Coverage ----

    #[test]
    fn timer_tick_zero_get_returns_zero() {
        assert_eq!(TimerTick::new(0).get(), 0);
    }

    #[test]
    fn timer_tick_max_get_returns_max() {
        assert_eq!(TimerTick::new(u64::MAX).get(), u64::MAX);
    }

    #[test]
    fn timer_tick_checked_add_zero_returns_self() {
        let tick = TimerTick::new(7);
        assert_eq!(tick.checked_add(TimerDuration::zero()), Some(tick));
    }

    #[test]
    fn timer_tick_checked_add_zero_to_zero_returns_zero() {
        let tick = TimerTick::new(0);
        assert_eq!(
            tick.checked_add(TimerDuration::zero()),
            Some(TimerTick::new(0))
        );
    }

    #[test]
    fn timer_tick_checked_add_max_minus_one_plus_one_returns_max() {
        let tick = TimerTick::new(u64::MAX - 1);
        let dur = TimerDuration::new(1);
        assert_eq!(tick.checked_add(dur), Some(TimerTick::new(u64::MAX)));
    }

    #[test]
    fn timer_tick_checked_add_max_plus_zero_returns_max() {
        let tick = TimerTick::new(u64::MAX);
        assert_eq!(
            tick.checked_add(TimerDuration::zero()),
            Some(TimerTick::new(u64::MAX))
        );
    }

    #[test]
    fn timer_tick_checked_add_zero_plus_max_overflows() {
        // 0 + u64::MAX = u64::MAX (no overflow)
        let tick = TimerTick::new(0);
        let dur = TimerDuration::new(u64::MAX);
        assert_eq!(tick.checked_add(dur), Some(TimerTick::new(u64::MAX)));
    }

    #[test]
    fn timer_tick_has_elapsed_zero_vs_zero_is_true() {
        assert!(TimerTick::new(0).has_elapsed(TimerDeadline::new(0)));
    }

    #[test]
    fn timer_tick_has_elapsed_zero_vs_one_is_false() {
        assert!(!TimerTick::new(0).has_elapsed(TimerDeadline::new(1)));
    }

    #[test]
    fn timer_tick_has_elapsed_max_vs_max_is_true() {
        assert!(TimerTick::new(u64::MAX).has_elapsed(TimerDeadline::new(u64::MAX)));
    }

    #[test]
    fn timer_tick_has_elapsed_max_vs_max_minus_one_is_true() {
        assert!(TimerTick::new(u64::MAX).has_elapsed(TimerDeadline::new(u64::MAX - 1)));
    }

    #[test]
    fn timer_tick_has_elapsed_max_minus_one_vs_max_is_false() {
        assert!(!TimerTick::new(u64::MAX - 1).has_elapsed(TimerDeadline::new(u64::MAX)));
    }

    #[test]
    fn timer_tick_partial_cmp_is_consistent_with_ord() {
        let a = TimerTick::new(5);
        let b = TimerTick::new(10);
        assert_eq!(a.partial_cmp(&b), Some(std::cmp::Ordering::Less));
        assert_eq!(b.partial_cmp(&a), Some(std::cmp::Ordering::Greater));
        assert_eq!(a.partial_cmp(&a), Some(std::cmp::Ordering::Equal));
        // partial_cmp of equal values
        assert_eq!(
            TimerTick::new(3).partial_cmp(&TimerTick::new(3)),
            Some(std::cmp::Ordering::Equal)
        );
    }

    #[test]
    fn timer_tick_hash_is_consistent_with_eq() {
        use std::hash::{Hash, Hasher};
        // Two equal values should have the same hash
        let t1 = TimerTick::new(42);
        let t2 = TimerTick::new(42);
        let mut h1 = std::collections::hash_map::DefaultHasher::new();
        let mut h2 = std::collections::hash_map::DefaultHasher::new();
        t1.hash(&mut h1);
        t2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn timer_duration_max_get_returns_max() {
        assert_eq!(TimerDuration::new(u64::MAX).get(), u64::MAX);
    }

    #[test]
    fn timer_duration_one_get_returns_one() {
        assert_eq!(TimerDuration::new(1).get(), 1);
    }

    #[test]
    fn timer_duration_partial_cmp_is_consistent_with_ord() {
        let a = TimerDuration::new(3);
        let b = TimerDuration::new(7);
        assert_eq!(a.partial_cmp(&b), Some(std::cmp::Ordering::Less));
        assert_eq!(b.partial_cmp(&a), Some(std::cmp::Ordering::Greater));
    }

    #[test]
    fn timer_duration_hash_is_consistent_with_eq() {
        use std::hash::{Hash, Hasher};
        let d1 = TimerDuration::new(10);
        let d2 = TimerDuration::new(10);
        let mut h1 = std::collections::hash_map::DefaultHasher::new();
        let mut h2 = std::collections::hash_map::DefaultHasher::new();
        d1.hash(&mut h1);
        d2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn timer_deadline_max_get_returns_max() {
        assert_eq!(TimerDeadline::new(u64::MAX).get(), u64::MAX);
    }

    #[test]
    fn timer_deadline_zero_get_returns_zero() {
        assert_eq!(TimerDeadline::new(0).get(), 0);
    }

    #[test]
    fn timer_deadline_from_tick_and_duration_zero_plus_zero() {
        let tick = TimerTick::new(0);
        let dur = TimerDuration::new(0);
        assert_eq!(
            TimerDeadline::from_tick_and_duration(tick, dur),
            Some(TimerDeadline::new(0))
        );
    }

    #[test]
    fn timer_deadline_from_tick_and_duration_one_plus_max_overflows() {
        let tick = TimerTick::new(1);
        let dur = TimerDuration::new(u64::MAX);
        assert_eq!(TimerDeadline::from_tick_and_duration(tick, dur), None);
    }

    #[test]
    fn timer_deadline_from_tick_and_duration_max_plus_max_overflows() {
        let tick = TimerTick::new(u64::MAX);
        let dur = TimerDuration::new(u64::MAX);
        assert_eq!(TimerDeadline::from_tick_and_duration(tick, dur), None);
    }

    #[test]
    fn timer_deadline_from_tick_and_duration_max_minus_two_plus_one() {
        let tick = TimerTick::new(u64::MAX - 2);
        let dur = TimerDuration::new(1);
        assert_eq!(
            TimerDeadline::from_tick_and_duration(tick, dur),
            Some(TimerDeadline::new(u64::MAX - 1))
        );
    }

    #[test]
    fn timer_deadline_is_past_zero_vs_zero_is_true() {
        assert!(TimerDeadline::new(0).is_past(TimerTick::new(0)));
    }

    #[test]
    fn timer_deadline_is_past_max_vs_max_is_true() {
        assert!(TimerDeadline::new(u64::MAX).is_past(TimerTick::new(u64::MAX)));
    }

    #[test]
    fn timer_deadline_is_past_one_vs_zero_is_false() {
        assert!(!TimerDeadline::new(1).is_past(TimerTick::new(0)));
    }

    #[test]
    fn timer_deadline_partial_cmp_is_consistent_with_ord() {
        let a = TimerDeadline::new(5);
        let b = TimerDeadline::new(10);
        assert_eq!(a.partial_cmp(&b), Some(std::cmp::Ordering::Less));
    }

    #[test]
    fn timer_deadline_hash_is_consistent_with_eq() {
        use std::hash::{Hash, Hasher};
        let dl1 = TimerDeadline::new(15);
        let dl2 = TimerDeadline::new(15);
        let mut h1 = std::collections::hash_map::DefaultHasher::new();
        let mut h2 = std::collections::hash_map::DefaultHasher::new();
        dl1.hash(&mut h1);
        dl2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    // ---- TimerKind variant coverage ----

    #[test]
    fn timer_kind_retry_equals_retry() {
        assert_eq!(TimerKind::Retry, TimerKind::Retry);
    }

    #[test]
    fn timer_kind_delayed_action_equals_same_action_id() {
        assert_eq!(
            TimerKind::DelayedAction(ActionId::new(7)),
            TimerKind::DelayedAction(ActionId::new(7))
        );
    }

    #[test]
    fn timer_kind_delayed_action_differs_with_different_action_id() {
        assert_ne!(
            TimerKind::DelayedAction(ActionId::new(1)),
            TimerKind::DelayedAction(ActionId::new(2))
        );
    }

    #[test]
    fn timer_kind_clone_preserves_value() {
        let k1 = TimerKind::DelayedAction(ActionId::new(99));
        let k2 = k1;
        assert_eq!(k1, k2);
    }

    // ---- Debug format does not panic ----

    #[test]
    fn timer_tick_debug_format() {
        let tick = TimerTick::new(42);
        let s = format!("{:?}", tick);
        assert!(s.contains("42"));
    }

    #[test]
    fn timer_duration_debug_format() {
        let dur = TimerDuration::new(10);
        let s = format!("{:?}", dur);
        assert!(s.contains("10"));
    }

    #[test]
    fn timer_deadline_debug_format() {
        let dl = TimerDeadline::new(7);
        let s = format!("{:?}", dl);
        assert!(s.contains("7"));
    }

    #[test]
    fn timer_kind_debug_format() {
        let k = TimerKind::Retry;
        let s = format!("{:?}", k);
        assert!(!s.is_empty());

        let k2 = TimerKind::DelayedAction(ActionId::new(42));
        let s2 = format!("{:?}", k2);
        assert!(s2.contains("42"));
    }

    // ---- IntrospectionRegistry epoch saturation (RS-214) ----

    /// Constructs an `IntrospectionRegistry` whose internal `next_epoch` is
    /// pre-seeded to a chosen value. Used to exercise the saturation boundary
    /// without simulating 2^64 register calls.
    fn registry_with_next_epoch(next_epoch: u64) -> IntrospectionRegistry {
        IntrospectionRegistry {
            inner: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            next_epoch,
        }
    }

    /// RS-214: `register` must return `IntrospectionEpochExhausted` (typed)
    /// once `next_epoch == u64::MAX`, instead of silently reusing the same
    /// epoch for the new handle.
    #[test]
    fn introspection_register_returns_typed_error_when_next_epoch_is_max() {
        let mut registry = registry_with_next_epoch(u64::MAX);
        let run = vb_core::ids::RunId::new(9101);

        let result = registry.register(run);

        match result {
            Err(RuntimeError::IntrospectionEpochExhausted) => {}
            Err(other) => panic!("expected IntrospectionEpochExhausted, got {other:?}"),
            Ok(handle) => panic!(
                "expected IntrospectionEpochExhausted, got Ok handle with epoch {}",
                handle.epoch()
            ),
        }

        // The exhausted sentinel must persist: subsequent registers must keep
        // failing rather than silently reusing u64::MAX.
        let again = registry.register(vb_core::ids::RunId::new(9102));
        assert!(
            matches!(again, Err(RuntimeError::IntrospectionEpochExhausted)),
            "post-exhaustion registers must also return IntrospectionEpochExhausted, got {again:?}"
        );

        // No handle should have been inserted into the registry.
        assert!(!registry.is_visible(run));
    }

    /// RS-214: `register_with_overlap_policy` (no overlap branch) must return
    /// `IntrospectionEpochExhausted` at the saturation boundary.
    #[test]
    fn introspection_register_with_overlap_policy_returns_typed_error_on_saturation() {
        let mut registry = registry_with_next_epoch(u64::MAX);
        let run = vb_core::ids::RunId::new(9201);

        let result = registry.register_with_overlap_policy(run);

        match result {
            Err(RuntimeError::IntrospectionEpochExhausted) => {}
            Err(other) => panic!("expected IntrospectionEpochExhausted, got {other:?}"),
            Ok(_) => panic!("expected IntrospectionEpochExhausted, got Ok(_) at saturation"),
        }

        assert!(!registry.is_visible(run));
    }

    /// RS-214: `register_with_overlap_policy` (overlap branch) must return
    /// `IntrospectionEpochExhausted` at the saturation boundary even when the
    /// caller supplies an existing registration. This is the precise path that
    /// would have aliased the live handle's epoch under saturating arithmetic.
    #[test]
    fn introspection_register_with_overlap_policy_overlap_branch_returns_typed_error_on_saturation()
    {
        let mut registry = registry_with_next_epoch(u64::MAX - 1);
        let run = vb_core::ids::RunId::new(9301);

        // First register succeeds and advances next_epoch to u64::MAX.
        let _first = registry
            .register(run)
            .expect("first register at u64::MAX - 1 must succeed");
        assert_eq!(registry.next_epoch_for_test(), u64::MAX);

        // Second call (overlap branch) must fail with the typed exhaustion error
        // instead of reusing u64::MAX as the new epoch for the replacement handle.
        let result = registry.register_with_overlap_policy(run);
        match result {
            Err(RuntimeError::IntrospectionEpochExhausted) => {}
            Err(other) => {
                panic!("expected IntrospectionEpochExhausted on overlap branch, got {other:?}")
            }
            Ok(_) => panic!("expected IntrospectionEpochExhausted on overlap branch, got Ok(_)"),
        }

        // The original handle's epoch must remain unchanged.
        assert_eq!(_first.epoch(), u64::MAX - 1);
    }

    impl IntrospectionRegistry {
        #[cfg(test)]
        fn next_epoch_for_test(&self) -> u64 {
            self.next_epoch
        }
    }
}

// ============================================================================
// Regression tests for RA-014: lock_admission permanently bricks shard
// admission on mutex poison.
//
// Before the fix, `IntrospectionRegistry::register` / `unregister` /
// `unregister_all` / `register_with_overlap_policy` all used the pattern
//   `self.inner.lock().map_err(|_| RuntimeError::JournalPoisoned)?`
// which discards the poisoned guard. Once poisoned, every subsequent call
// hits a still-poisoned mutex and returns `Err(JournalPoisoned)` forever,
// permanently disabling the admission gate for that shard.
//
// After the fix, the registry recovers the inner data via
// `PoisonError::into_inner()` and continues accepting admissions. The
// follow-up tests pin down that behavior.
// ============================================================================
#[cfg(test)]
mod introspection_poison_regression_tests {
    use super::*;
    use crate::RuntimeError;
    use std::panic::AssertUnwindSafe;
    use std::sync::Arc;
    use std::thread;

    /// Spawns a thread that locks the shared mutex and panics, leaving the
    /// mutex poisoned. `catch_unwind` swallows the panic so the test thread
    /// is not poisoned. The function returns once the spawned thread has
    /// finished and the mutex is verifiably poisoned.
    fn poison_arc_mutex<T: Send + 'static>(mutex: Arc<std::sync::Mutex<T>>) {
        let handle = thread::spawn(move || {
            // Lock and immediately panic to poison the mutex. The panic
            // itself is the source of the poison; we use `catch_unwind` to
            // keep the test process alive.
            let _guard = mutex.lock().expect("acquire lock to poison");
            panic!("intentional poison for RA-014 regression test");
        });
        let result = std::panic::catch_unwind(AssertUnwindSafe(move || {
            // Wait for the spawner to finish panicking.
            let _ = handle.join();
        }));
        // The spawned thread panicked; that is the intended outcome.
        let _ = result;
    }

    #[test]
    fn register_recovers_after_mutex_poison() {
        // Given: a fresh registry whose inner mutex is poisoned.
        let mut registry = IntrospectionRegistry::new();
        poison_arc_mutex(registry.inner_arc_for_test());

        // When: register is called after the poison.
        let result = registry.register(RunId::new(1));

        // Then: registration succeeds because the registry recovers
        // the poisoned guard rather than returning JournalPoisoned.
        assert!(
            result.is_ok(),
            "register must recover from mutex poison, got {result:?}"
        );
        assert!(registry.is_visible(RunId::new(1)));
    }

    #[test]
    fn register_with_overlap_recovers_after_mutex_poison() {
        let mut registry = IntrospectionRegistry::new();
        poison_arc_mutex(registry.inner_arc_for_test());

        let result = registry.register_with_overlap_policy(RunId::new(7));
        assert!(
            result.is_ok(),
            "register_with_overlap_policy must recover, got {result:?}"
        );
    }

    #[test]
    fn unregister_recovers_after_mutex_poison() {
        let mut registry = IntrospectionRegistry::new();
        poison_arc_mutex(registry.inner_arc_for_test());

        let result = registry.unregister(RunId::new(3));
        assert!(result.is_ok(), "unregister must recover, got {result:?}");
    }

    #[test]
    fn unregister_all_recovers_after_mutex_poison() {
        let mut registry = IntrospectionRegistry::new();
        poison_arc_mutex(registry.inner_arc_for_test());

        let result = registry.unregister_all();
        assert!(
            result.is_ok(),
            "unregister_all must recover, got {result:?}"
        );
    }

    #[test]
    fn admission_continues_after_poison_recovery() {
        // Given: a registry with one registered run, then a poison event.
        let mut registry = IntrospectionRegistry::new();
        let initial = registry.register(RunId::new(100));
        assert!(initial.is_ok(), "initial register must succeed");

        // Drop the handle so the run state can be re-registered later.
        drop(initial);

        poison_arc_mutex(registry.inner_arc_for_test());

        // When: we register again after the poison.
        let after_poison = registry.register(RunId::new(101));

        // Then: registration succeeds and the second run is visible.
        assert!(
            after_poison.is_ok(),
            "post-poison register must succeed, got {after_poison:?}"
        );
        assert!(registry.is_visible(RunId::new(101)));
    }

    #[test]
    fn register_rejects_run_already_exists_after_poison_recovery() {
        // Given: a registry with an actively-held registration, poisoned,
        // then we try to register the same run again. The handle stays
        // alive so the registry still holds the prior mapping.
        let mut registry = IntrospectionRegistry::new();
        let first = registry.register(RunId::new(42));
        assert!(first.is_ok(), "first register must succeed");

        poison_arc_mutex(registry.inner_arc_for_test());

        // When: we attempt to register the same run again post-recovery.
        let dup = registry.register(RunId::new(42));

        // Then: recovery exposes the actual state, so the duplicate
        // detection still rejects the second registration with the typed
        // RunAlreadyExists error rather than a misleading JournalPoisoned.
        assert!(
            matches!(dup, Err(RuntimeError::RunAlreadyExists)),
            "post-recovery duplicate must surface RunAlreadyExists, got {dup:?}"
        );

        // Keep `first` alive until the assertions are done so the handle's
        // RAII Drop does not race with the recovery test.
        drop(first);
    }
}
// ============================================================================
// Regression test for vb-8ilqu: InspectSnapshotFormatter::format_snapshot
// must source the `run` from the snapshot in the Found branch, never from
// a separately-supplied external parameter.
// ============================================================================
#[cfg(test)]
mod format_snapshot_uses_snap_run {
    use super::*;

    /// The Found branch must cite the snapshot's own `run`, not an external
    /// value. Construct a snapshot whose `snap.run` is `B` and verify the
    /// formatted output contains `B` rather than any other value the
    /// previous external-parameter signature would have shadowed.
    #[test]
    fn found_branch_uses_snap_run_not_external() {
        let snap = InspectSnapshot {
            run: RunId::new(7777),
            correlation: 99,
            pc: StepIdx::new(3),
            executed: 5,
        };
        let response = InspectResponse::Found(snap);

        let formatted = InspectSnapshotFormatter::format_snapshot(&response);

        assert!(
            formatted.contains("7777"),
            "formatted output must cite snap.run (7777), got: {formatted}"
        );
        assert!(
            formatted.contains("InspectSnapshot"),
            "formatted output should be the Found-branch shape, got: {formatted}"
        );
    }

    /// Two snapshots with different `snap.run` values must format to
    /// different strings — proves the formatter does not collapse to a
    /// single fixed value.
    #[test]
    fn found_branch_distinguishes_distinct_snap_runs() {
        let response_a = InspectResponse::Found(InspectSnapshot {
            run: RunId::new(1),
            correlation: 0,
            pc: StepIdx::ZERO,
            executed: 0,
        });
        let response_b = InspectResponse::Found(InspectSnapshot {
            run: RunId::new(2),
            correlation: 0,
            pc: StepIdx::ZERO,
            executed: 0,
        });

        let formatted_a = InspectSnapshotFormatter::format_snapshot(&response_a);
        let formatted_b = InspectSnapshotFormatter::format_snapshot(&response_b);

        assert_ne!(
            formatted_a, formatted_b,
            "two snapshots with distinct snap.run must format differently"
        );
        assert!(formatted_a.contains("RunId(1)"));
        assert!(formatted_b.contains("RunId(2)"));
    }
}
