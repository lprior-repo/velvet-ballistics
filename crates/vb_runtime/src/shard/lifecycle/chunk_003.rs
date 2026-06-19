// Lifecycle chunk 003: Action completion preflight and error handling.
//
// Split into submodules for maintainability:
// - [`chunk_003_completion`](chunk_003_completion) - Action completion preflight logic
// - [`chunk_003_error`](chunk_003_error) - Error handler application
// - [`chunk_003_kani`](chunk_003_kani) - Kani proof harnesses

use vb_core::ValueStore;
use vb_core::action::{ActionFailure, ActionOutputReady, ActionTicket};
use vb_core::capability::CapabilitySet;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::CompiledWorkflow;

use crate::engine::{
    EvidenceCollector, RetryPolicy, RuntimeEngineResult, RuntimeSignal, drive_deterministic_full,
};
use crate::journal::RuntimeJournalEvent;
use crate::trace::TraceEvent;
use crate::{RuntimeError, RuntimeResult};

use crate::primitives::collect::CollectStates;
use crate::shard::types::{
    AskAnswer, PendingTimerKind, ResumeError, ResumeResult, ResumeStatus, RunState, RuntimeEvent,
    RuntimeState, Shard,
};

mod chunk_003_completion;
mod chunk_003_error;
// HVR-PO-RUNTIME-001/HVR-PO-RUNTIME-002/HVR-PO-RUNTIME-006: keep legacy lifecycle Kani harnesses out of the vb-god2f feature lane.
#[cfg(all(kani, not(feature = "vb-god2f-action-completion")))]
mod chunk_003_kani;

// Re-export for convenience
pub(crate) use chunk_003_completion::{
    ActionFailureOutcome, current_timestamp, preflight_action_completion, retry_is_available,
};
#[cfg(test)]
pub(crate) use chunk_003_completion::reject_taint_downgrade;
pub(crate) use chunk_003_error::apply_error_handler;
