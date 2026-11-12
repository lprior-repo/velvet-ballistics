use super::error::ActionFailure;
use super::payload::ActionOutputReady;
use super::ticket::ActionTicket;
use crate::ids::{ActionId, SlotIdx, StepIdx};
use serde::{Deserialize, Serialize};

/// Journal events for Do-node action lifecycle.
///
/// These events are recorded for crash recovery. The journal records the
/// suspension (ticket issuance) and the terminal outcome (success or failure).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ActionJournalEvent {
    /// Engine suspended on a Do node, issuing an action ticket.
    Suspended {
        /// Ticket identifying the in-flight action.
        ticket: ActionTicket,
        /// Monotonic per-step attempt number captured for replay.
        attempt: u16,
        /// Action contract ID for dispatch routing.
        action: ActionId,
        /// Input slot carrying the action payload.
        input_slot: SlotIdx,
        /// Output slot to receive the result on completion.
        output_slot: SlotIdx,
        /// Step that triggered the suspension.
        step: StepIdx,
    },
    /// Action completed successfully with output.
    Completed {
        /// Ticket of the completed action.
        ticket: ActionTicket,
        /// Monotonic per-step attempt number captured for replay.
        attempt: u16,
        /// Successful output payload written by the action.
        output: ActionOutputReady,
    },
    /// Action failed terminally.
    Failed {
        /// Ticket of the failed action.
        ticket: ActionTicket,
        /// Monotonic per-step attempt number captured for replay.
        attempt: u16,
        /// Output slot targeted by the action.
        output_slot: SlotIdx,
        /// Failure payload reported by the action lifecycle.
        failure: ActionFailure,
    },
}
