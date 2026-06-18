//! Journal events for Do-node action lifecycle.
//!
//! These events are recorded for crash recovery. The journal records the
//! suspension (ticket issuance) and the terminal outcome (success or failure).

use crate::action::classification::RetryPolicy;
use crate::action::failure::ActionFailureCode;
use crate::action::model::ActionTicket;
use crate::ids::{ActionId, SlotIdx, StepIdx};
use crate::value::Taint;
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
        /// Output slot written by the action.
        output_slot: SlotIdx,
        /// Taint propagated from input to output.
        output_taint: Taint,
    },
    /// Action failed terminally.
    Failed {
        /// Ticket of the failed action.
        ticket: ActionTicket,
        /// Monotonic per-step attempt number captured for replay.
        attempt: u16,
        /// Failure code for diagnostics.
        code: ActionFailureCode,
        /// Whether the failure is retryable.
        retry_policy: RetryPolicy,
    },
}
