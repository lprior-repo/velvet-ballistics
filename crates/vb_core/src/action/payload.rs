use super::contract::ActionContract;
use super::error::{ActionError, ActionFailure, ActionResult};
use super::ticket::ActionTicket;
use crate::ids::{ActionId, RunId, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};
use serde::{Deserialize, Serialize};

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
    /// Encoded input payload length in bytes.
    encoded_len: EncodedActionInputLen,
    /// Ticket tracking this invocation.
    pub ticket: ActionTicket,
}

/// Encoded byte length for an action input, checked against the action contract.
///
/// Public callers cannot forge this from a caller-supplied numeric length:
///
/// ```compile_fail
/// use vb_core::action::{ActionContract, EncodedActionInputLen};
///
/// fn forge(contract: &ActionContract) {
///     let _ = EncodedActionInputLen::from_precomputed_len(1, contract);
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EncodedActionInputLen {
    bytes: u32,
    action: ActionId,
}

impl EncodedActionInputLen {
    /// Creates a checked length from a precomputed byte count at a trusted internal boundary.
    fn from_precomputed_len(encoded_len: u32, contract: &ActionContract) -> ActionResult<Self> {
        if encoded_len > contract.max_input_bytes {
            return Err(ActionError::PayloadTooLarge {
                max_bytes: contract.max_input_bytes,
                actual_bytes: encoded_len,
            });
        }
        Ok(Self {
            bytes: encoded_len,
            action: contract.id,
        })
    }

    /// Computes and checks the encoded length from actual boundary bytes.
    pub fn from_encoded_payload(
        encoded_payload: &[u8],
        contract: &ActionContract,
    ) -> ActionResult<Self> {
        let encoded_len =
            u32::try_from(encoded_payload.len()).map_err(|_| ActionError::PayloadTooLarge {
                max_bytes: contract.max_input_bytes,
                actual_bytes: u32::MAX,
            })?;
        Self::from_precomputed_len(encoded_len, contract)
    }

    /// Returns the checked byte count.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.bytes
    }

    /// Returns the action contract id used to check this length.
    #[must_use]
    pub const fn action(self) -> ActionId {
        self.action
    }
}

impl ActionInput {
    /// Creates an action input and binds its length to the actual encoded payload bytes.
    pub fn new(
        run: RunId,
        step: StepIdx,
        action: ActionId,
        input: SlotIdx,
        encoded_payload: &[u8],
        contract: &ActionContract,
        ticket: ActionTicket,
    ) -> ActionResult<Self> {
        if contract.id != action {
            return Err(ActionError::InvalidTicket);
        }
        let encoded_len = EncodedActionInputLen::from_encoded_payload(encoded_payload, contract)?;
        Self::from_checked_len(run, step, action, input, encoded_len, ticket)
    }

    /// Creates an action input from a privately checked encoded length.
    fn from_checked_len(
        run: RunId,
        step: StepIdx,
        action: ActionId,
        input: SlotIdx,
        encoded_len: EncodedActionInputLen,
        ticket: ActionTicket,
    ) -> ActionResult<Self> {
        if encoded_len.action() != action
            || ticket.run != run
            || ticket.step != step
            || ticket.action != action
        {
            return Err(ActionError::InvalidTicket);
        }
        Ok(Self {
            run,
            step,
            action,
            input,
            encoded_len,
            ticket,
        })
    }

    /// Returns the checked encoded input byte length.
    #[must_use]
    pub const fn encoded_len(&self) -> u32 {
        self.encoded_len.get()
    }

    /// Returns the proof-carrying checked encoded length.
    #[must_use]
    pub const fn encoded_input_len(&self) -> EncodedActionInputLen {
        self.encoded_len
    }
}

/// Output payload produced by a completed action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionOutput {
    /// Output slot to receive the result.
    pub output: SlotIdx,
    /// Completion status.
    pub status: ActionOutcome,
}

/// Successful action result with output value and metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

/// Terminal outcome of an action invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ActionOutcome {
    /// Action completed successfully with output.
    Ready(ActionOutputReady),
    /// Action is suspended awaiting external completion.
    Suspended(ActionTicket),
    /// Action failed.
    Failed(ActionFailure),
}
