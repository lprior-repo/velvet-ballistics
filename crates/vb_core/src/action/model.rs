//! Core domain model: contracts, tickets, outputs, outcomes.

use crate::action::classification::{ActionName, Idempotency, MockMarker, RetrySafety, SideEffect};
use crate::action::failure::ActionFailure;
use crate::capability::Capability;
use crate::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};
use serde::{Deserialize, Serialize};

/// Static contract describing an action's resource and correctness bounds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActionContract {
    /// Numeric action identifier used for dispatch.
    pub id: ActionId,
    /// Action name used for name-based lookup.
    pub name: ActionName,
    /// Number of input slots consumed.
    pub input_slot_count: u16,
    /// Number of output slots produced.
    pub output_slot_count: u16,
    /// Maximum encoded input byte length.
    pub max_input_bytes: u32,
    /// Maximum encoded output byte length.
    pub max_output_bytes: u32,
    /// Maximum wall-clock time for one attempt in milliseconds.
    pub timeout_ms: u64,
    /// Idempotency classification for retry and taint propagation.
    pub idempotency: Idempotency,
    /// Side-effect classification for retry safety decisions.
    pub side_effect: SideEffect,
    /// Retry safety classification for the verification gate.
    pub retry_safety: RetrySafety,
    /// Required capabilities for this action.
    pub required_capabilities: Box<[Capability]>,
}

/// Input payload for one action invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionInput {
    /// Owning run.
    pub run: RunId,
    /// Step that issued the action.
    pub step: StepIdx,
    /// Action being invoked.
    pub action: ActionId,
    /// Input slot carrying the payload.
    pub input: SlotIdx,
    /// Ticket tracking this invocation.
    pub ticket: ActionTicket,
}

/// Output payload produced by a completed action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionOutput {
    /// Output slot to receive the result.
    pub output: SlotIdx,
    /// Completion status.
    pub status: ActionOutcome,
}

/// Successful action result with output value and metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionOutputReady {
    /// Output slot receiving the result value.
    pub output_slot: SlotIdx,
    /// Result value produced by the action.
    pub value: SlotValue,
    /// Taint propagated from input to output.
    pub taint: Taint,
    /// Encoded byte length of the output payload.
    pub encoded_len: u32,
}

/// Unique ticket tracking one action invocation across suspension boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActionTicket {
    /// Owning run.
    pub run: RunId,
    /// Step that issued the action.
    pub step: StepIdx,
    /// Monotonic sequence within the run.
    pub seq: SeqNo,
    /// Action being invoked.
    pub action: ActionId,
    /// Current attempt number (1-indexed).
    pub attempt: u16,
    /// Idempotency key for deduplication and replay.
    pub idempotency_key: u128,
    /// Maximum attempts allowed (capacity bound from retry policy).
    pub capacity: u16,
    /// Mock marker assigned by dispatch when the action uses a mock handler.
    pub mock: MockMarker,
}

impl Default for ActionTicket {
    fn default() -> Self {
        Self {
            run: RunId::new(0),
            step: StepIdx::new(0),
            seq: SeqNo::new(0),
            action: ActionId::new(0),
            attempt: 0,
            idempotency_key: 0,
            capacity: 0,
            mock: MockMarker::default(),
        }
    }
}

/// Terminal outcome of an action invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ActionOutcome {
    /// Action completed successfully with output.
    Ready(ActionOutputReady),
    /// Action is suspended awaiting external completion.
    Suspended(ActionTicket),
    /// Action failed.
    Failed(ActionFailure),
}
