#![forbid(unsafe_code)]

//! Action ABI contract for the do/retry/on_error primitives.

use crate::ids::{ActionId, BlobId, RunId, SeqNo, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};
use serde::{Deserialize, Serialize};

/// Declares how an action behaves with respect to repeated execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Idempotency {
    /// Pure deterministic computation with no side effects.
    DeterministicPure = 0,
    /// External call that is idempotent when retried with the same key.
    IdempotentExternal = 1,
    /// External call that may execute more than once; at-least-once delivery.
    AtLeastOnceExternal = 2,
}

/// Static contract describing an action's resource and correctness bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActionContract {
    /// Numeric action identifier used for dispatch.
    pub id: ActionId,
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

/// Result alias for action operations.
pub type ActionResult<T> = Result<T, ActionError>;

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

/// Failure details for a rejected, timed-out, or errored action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionFailure {
    /// Machine-readable failure code.
    pub code: ActionFailureCode,
    /// Whether this failure can be retried.
    pub retryable: bool,
    /// Taint of the input that caused the failure.
    pub taint: Taint,
    /// Optional detail blob for diagnostics.
    pub detail: Option<BlobId>,
    /// Encoded byte length of the failure payload.
    pub encoded_len: u32,
}

/// Machine-readable action failure codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ActionFailureCode {
    /// Action was rejected by the handler before execution.
    Rejected = 0,
    /// Action exceeded its timeout deadline.
    Timeout = 1,
    /// Action was rate-limited by the external service.
    RateLimited = 2,
    /// External resource was exhausted.
    ResourceExhausted = 3,
    /// External service was unavailable.
    ExternalUnavailable = 4,
    /// Input payload failed validation.
    InvalidInput = 5,
    /// Caller lacked permission for this action.
    PermissionDenied = 6,
    /// Optimistic concurrency conflict.
    Conflict = 7,
    /// Unspecified or unclassified failure.
    Unknown = 255,
}

/// Typed errors from the action subsystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionError {
    /// The requested action is not registered.
    UnknownAction {
        /// Action identifier that was not found.
        action: ActionId,
    },
    /// The supplied ticket does not match any in-flight action.
    InvalidTicket,
    /// Input payload exceeds the contract's declared byte limit.
    PayloadTooLarge {
        /// Declared maximum bytes.
        max_bytes: u32,
        /// Actual payload bytes.
        actual_bytes: u32,
    },
    /// Output slot index exceeds the contract's declared output count.
    OutputSlotOutOfBounds {
        /// Requested slot index.
        slot: u16,
        /// Declared output slot count.
        max_slots: u16,
    },
    /// Replay was blocked because the action is not idempotent.
    NonIdempotentReplayBlocked,
    /// Completion was already recorded for this ticket.
    CompletionAlreadyRecorded,
    /// Action dispatch queue is at capacity.
    QueueFull,
    /// Output encoding failed.
    EncodingFailed,
    /// Action dispatch failed internally.
    DispatchFailed,
}

/// Terminal outcome of an action invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionOutcome {
    /// Action completed successfully with output.
    Ready(ActionOutputReady),
    /// Action is suspended awaiting external completion.
    Suspended(ActionTicket),
    /// Action failed.
    Failed(ActionFailure),
}

/// Computes the output taint for an action given its idempotency and input taint.
///
/// Rules:
/// - DeterministicPure and IdempotentExternal: output taint >= input taint (join).
/// - AtLeastOnceExternal: DerivedFromSecret when any input is Secret/DerivedFromSecret.
/// - Clean result from tainted input is rejected unless the action declares declassification
///   (not modeled here; caller must validate).
#[must_use]
pub const fn propagate_action_taint(idempotency: Idempotency, input_taint: Taint) -> Taint {
    match idempotency {
        Idempotency::DeterministicPure | Idempotency::IdempotentExternal => join_taint(input_taint),
        Idempotency::AtLeastOnceExternal => match input_taint {
            Taint::Clean => Taint::Clean,
            Taint::Secret | Taint::DerivedFromSecret => Taint::DerivedFromSecret,
        },
    }
}

/// Returns the least upper bound of the input taint and the output's own taint.
/// Since deterministic/idempotent actions propagate taint upward, the output
/// is always >= the input taint.
const fn join_taint(input: Taint) -> Taint {
    input
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_clean_stays_clean() {
        let result = propagate_action_taint(Idempotency::DeterministicPure, Taint::Clean);
        assert_eq!(result, Taint::Clean);
    }

    #[test]
    fn deterministic_secret_stays_secret() {
        let result = propagate_action_taint(Idempotency::DeterministicPure, Taint::Secret);
        assert_eq!(result, Taint::Secret);
    }

    #[test]
    fn deterministic_derived_stays_derived() {
        let result =
            propagate_action_taint(Idempotency::DeterministicPure, Taint::DerivedFromSecret);
        assert_eq!(result, Taint::DerivedFromSecret);
    }

    #[test]
    fn idempotent_clean_stays_clean() {
        let result = propagate_action_taint(Idempotency::IdempotentExternal, Taint::Clean);
        assert_eq!(result, Taint::Clean);
    }

    #[test]
    fn at_least_once_secret_becomes_derived() {
        let result = propagate_action_taint(Idempotency::AtLeastOnceExternal, Taint::Secret);
        assert_eq!(result, Taint::DerivedFromSecret);
    }

    #[test]
    fn at_least_once_derived_stays_derived() {
        let result =
            propagate_action_taint(Idempotency::AtLeastOnceExternal, Taint::DerivedFromSecret);
        assert_eq!(result, Taint::DerivedFromSecret);
    }

    #[test]
    fn at_least_once_clean_stays_clean() {
        let result = propagate_action_taint(Idempotency::AtLeastOnceExternal, Taint::Clean);
        assert_eq!(result, Taint::Clean);
    }

    // -- ActionError exact variant assertions --

    #[test]
    fn action_error_unknown_action_exact_variant() {
        let error = ActionError::UnknownAction {
            action: ActionId::new(42),
        };
        let ActionError::UnknownAction { action } = error else {
            panic!("expected UnknownAction variant");
        };
        assert_eq!(action, ActionId::new(42));
    }

    #[test]
    fn action_error_invalid_ticket_exact_variant() {
        let error = ActionError::InvalidTicket;
        assert_eq!(error, ActionError::InvalidTicket);
    }

    #[test]
    fn action_error_payload_too_large_exact_variant() {
        let error = ActionError::PayloadTooLarge {
            max_bytes: 1024,
            actual_bytes: 2048,
        };
        let ActionError::PayloadTooLarge {
            max_bytes,
            actual_bytes,
        } = error
        else {
            panic!("expected PayloadTooLarge variant");
        };
        assert_eq!(max_bytes, 1024);
        assert_eq!(actual_bytes, 2048);
    }

    #[test]
    fn action_error_output_slot_out_of_bounds_exact_variant() {
        let error = ActionError::OutputSlotOutOfBounds {
            slot: 5,
            max_slots: 4,
        };
        let ActionError::OutputSlotOutOfBounds { slot, max_slots } = error else {
            panic!("expected OutputSlotOutOfBounds variant");
        };
        assert_eq!(slot, 5);
        assert_eq!(max_slots, 4);
    }

    #[test]
    fn action_error_non_idempotent_replay_blocked_exact_variant() {
        let error = ActionError::NonIdempotentReplayBlocked;
        assert_eq!(error, ActionError::NonIdempotentReplayBlocked);
    }

    #[test]
    fn action_error_completion_already_recorded_exact_variant() {
        let error = ActionError::CompletionAlreadyRecorded;
        assert_eq!(error, ActionError::CompletionAlreadyRecorded);
    }

    #[test]
    fn action_error_queue_full_exact_variant() {
        let error = ActionError::QueueFull;
        assert_eq!(error, ActionError::QueueFull);
    }

    #[test]
    fn action_error_encoding_failed_exact_variant() {
        let error = ActionError::EncodingFailed;
        assert_eq!(error, ActionError::EncodingFailed);
    }

    #[test]
    fn action_error_dispatch_failed_exact_variant() {
        let error = ActionError::DispatchFailed;
        assert_eq!(error, ActionError::DispatchFailed);
    }
}
