//! Single-threaded shard owning mutable run state directly.

use crossbeam_queue::ArrayQueue;
use indexmap::IndexMap;
use vb_core::capability::CapabilitySet;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow};

use crate::counters::ShardCounters;
use crate::frame_pool::FramePool;
use crate::journal::{NoopRuntimeJournal, RuntimeJournalEvent, SharedRuntimeJournal};
use crate::trace::{TraceEvent, TraceRing};
use crate::{RuntimeError, RuntimeResult};

type FramePoolKey = (u16, u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PendingTimerKind {
    Wait,
    Ask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingTimer {
    pub step: StepIdx,
    pub kind: PendingTimerKind,
}

/// Bounded command processed by a shard.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// An external ask was answered.
    AskAnswered {
        /// Typed ask answer payload.
        answer: AskAnswer,
    },
    /// A timer fired for a suspended run.
    TimerFired {
        /// Run identifier.
        run: RunId,
    },
    /// Cancel an active run.
    Cancel {
        /// Run identifier.
        run: RunId,
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
    pub(crate) action_attempts: Box<[u16]>,
    /// Admission record for this run, if admission gating was performed.
    pub admission: Option<crate::admission::RunAdmission>,
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

/// Maximum bounded command queue capacity per shard.
pub const MAX_COMMAND_QUEUE_CAPACITY: usize = 65_536;

/// Single-threaded shard owning all mutable run state.
pub struct Shard {
    pub(crate) command_queue: ArrayQueue<ShardCommand>,
    pub(crate) runs: IndexMap<RunId, RunState>,
    pub(crate) pending_timers: IndexMap<RunId, PendingTimer>,
    pub(crate) frame_pools: IndexMap<FramePoolKey, FramePool>,
    pub(crate) trace_ring: TraceRing,
    pub(crate) counters: ShardCounters,
    pub(crate) step_budget_per_tick: u64,
    pub(crate) max_active_runs: usize,
    pub(crate) policy: vb_core::policy::RuntimePolicy,
    pub(crate) artifact_store: crate::admission::SharedArtifactStore,
    pub(crate) inspect_response: Option<InspectResponse>,
    pub(crate) shutting_down: bool,
    pub(crate) journal: SharedRuntimeJournal,
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
