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

impl ActionError {
    /// Runtime code for unresolved action references.
    pub const REFERENCE_MISSING_RUNTIME_CODE: &str = "REFERENCE_MISSING";
    /// Runtime code for action dispatch or encoding failures.
    pub const ACTION_FAILED_RUNTIME_CODE: &str = "ACTION_FAILED";
    /// Runtime code for oversized action payloads.
    pub const PAYLOAD_TOO_LARGE_RUNTIME_CODE: &str = "PAYLOAD_TOO_LARGE";
    /// Runtime code for bounded action queues at capacity.
    pub const QUEUE_FULL_RUNTIME_CODE: &str = "QUEUE_FULL";

    /// Returns the stable section 17 runtime code when this error has a direct mapping.
    #[must_use]
    pub const fn runtime_code(&self) -> Option<&'static str> {
        match self {
            Self::UnknownAction { .. } => Some(Self::REFERENCE_MISSING_RUNTIME_CODE),
            Self::PayloadTooLarge { .. } => Some(Self::PAYLOAD_TOO_LARGE_RUNTIME_CODE),
            Self::QueueFull => Some(Self::QUEUE_FULL_RUNTIME_CODE),
            Self::EncodingFailed | Self::DispatchFailed => Some(Self::ACTION_FAILED_RUNTIME_CODE),
            _ => None,
        }
    }
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
    fn action_error_unknown_action_exact_variant() -> Result<(), String> {
        let error = ActionError::UnknownAction {
            action: ActionId::new(42),
        };
        let ActionError::UnknownAction { action } = error else {
            return Err(String::from("expected UnknownAction variant"));
        };
        if action != ActionId::new(42) {
            return Err(String::from("unexpected action id"));
        }
        Ok(())
    }

    #[test]
    fn action_error_invalid_ticket_exact_variant() {
        let error = ActionError::InvalidTicket;
        assert_eq!(error, ActionError::InvalidTicket);
    }

    #[test]
    fn action_error_payload_too_large_exact_variant() -> Result<(), String> {
        let error = ActionError::PayloadTooLarge {
            max_bytes: 1024,
            actual_bytes: 2048,
        };
        let ActionError::PayloadTooLarge {
            max_bytes,
            actual_bytes,
        } = error
        else {
            return Err(String::from("expected PayloadTooLarge variant"));
        };
        if max_bytes != 1024 || actual_bytes != 2048 {
            return Err(String::from("unexpected payload size fields"));
        }
        Ok(())
    }

    #[test]
    fn action_error_output_slot_out_of_bounds_exact_variant() -> Result<(), String> {
        let error = ActionError::OutputSlotOutOfBounds {
            slot: 5,
            max_slots: 4,
        };
        let ActionError::OutputSlotOutOfBounds { slot, max_slots } = error else {
            return Err(String::from("expected OutputSlotOutOfBounds variant"));
        };
        if slot != 5 || max_slots != 4 {
            return Err(String::from("unexpected output slot bounds fields"));
        }
        Ok(())
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

    #[test]
    fn action_error_runtime_codes_cover_section_17_mappings() {
        assert_eq!(
            ActionError::UnknownAction {
                action: ActionId::new(9)
            }
            .runtime_code(),
            Some("REFERENCE_MISSING")
        );
        assert_eq!(
            ActionError::PayloadTooLarge {
                max_bytes: 1,
                actual_bytes: 2,
            }
            .runtime_code(),
            Some("PAYLOAD_TOO_LARGE")
        );
        assert_eq!(ActionError::QueueFull.runtime_code(), Some("QUEUE_FULL"));
        assert_eq!(
            ActionError::EncodingFailed.runtime_code(),
            Some("ACTION_FAILED")
        );
        assert_eq!(
            ActionError::DispatchFailed.runtime_code(),
            Some("ACTION_FAILED")
        );
    }

    #[test]
    fn action_error_runtime_codes_are_unique() {
        let codes = [
            ActionError::REFERENCE_MISSING_RUNTIME_CODE,
            ActionError::ACTION_FAILED_RUNTIME_CODE,
            ActionError::PAYLOAD_TOO_LARGE_RUNTIME_CODE,
            ActionError::QUEUE_FULL_RUNTIME_CODE,
        ];
        assert_eq!(codes.len(), 4);
        assert_eq!(
            codes
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            4
        );
    }

    #[test]
    fn action_error_runtime_code_is_absent_without_section_17_equivalent() {
        assert_eq!(ActionError::InvalidTicket.runtime_code(), None);
        assert_eq!(ActionError::CompletionAlreadyRecorded.runtime_code(), None);
    }

    // =========================================================================
    // Phase 2 adversarial BDD tests — action ABI security & taint vectors
    // =========================================================================

    // --- Idempotency key = 0 is not treated specially ---

    #[test]
    fn action_ticket_with_idempotency_key_zero_is_a_valid_ticket() {
        let ticket = ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(1),
            action: ActionId::new(5),
            attempt: 1,
            idempotency_key: 0,
        };
        assert_eq!(ticket.idempotency_key, 0);
        assert_eq!(ticket.run, RunId::new(1));
    }

    // --- Taint downgrade: Secret cannot become Clean through any operation ---

    #[test]
    fn deterministic_pure_propagate_cannot_downgrade_secret_to_clean() {
        let result = propagate_action_taint(Idempotency::DeterministicPure, Taint::Secret);
        assert_eq!(result, Taint::Secret);
    }

    #[test]
    fn deterministic_pure_propagate_cannot_downgrade_derived_to_clean() {
        let result =
            propagate_action_taint(Idempotency::DeterministicPure, Taint::DerivedFromSecret);
        assert_eq!(result, Taint::DerivedFromSecret);
    }

    #[test]
    fn idempotent_external_propagate_cannot_downgrade_secret_to_clean() {
        let result = propagate_action_taint(Idempotency::IdempotentExternal, Taint::Secret);
        assert_eq!(result, Taint::Secret);
    }

    #[test]
    fn idempotent_external_propagate_cannot_downgrade_derived_to_clean() {
        let result =
            propagate_action_taint(Idempotency::IdempotentExternal, Taint::DerivedFromSecret);
        assert_eq!(result, Taint::DerivedFromSecret);
    }

    // --- AtLeastOnceExternal always upgrades Secret to DerivedFromSecret ---

    #[test]
    fn at_least_once_secret_is_always_derived_never_secret() {
        let result = propagate_action_taint(Idempotency::AtLeastOnceExternal, Taint::Secret);
        assert_eq!(result, Taint::DerivedFromSecret);
        assert_ne!(result, Taint::Clean);
    }

    #[test]
    fn at_least_once_derived_remains_derived_never_clean() {
        let result =
            propagate_action_taint(Idempotency::AtLeastOnceExternal, Taint::DerivedFromSecret);
        assert_eq!(result, Taint::DerivedFromSecret);
        assert_ne!(result, Taint::Clean);
    }

    // --- Ticket reuse across runs: different run IDs are distinct tickets ---

    #[test]
    fn action_ticket_from_different_run_is_not_equal() {
        let ticket_a = ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(1),
            action: ActionId::new(5),
            attempt: 1,
            idempotency_key: 100,
        };
        let ticket_b = ActionTicket {
            run: RunId::new(2),
            step: StepIdx::new(0),
            seq: SeqNo::new(1),
            action: ActionId::new(5),
            attempt: 1,
            idempotency_key: 100,
        };
        assert_ne!(ticket_a, ticket_b);
    }

    // --- Payload too large error carries exact byte counts ---

    #[test]
    fn action_error_payload_too_large_reports_exact_overflow() {
        let error = ActionError::PayloadTooLarge {
            max_bytes: 1024,
            actual_bytes: 2048,
        };
        match error {
            ActionError::PayloadTooLarge {
                max_bytes,
                actual_bytes,
            } => {
                assert_eq!(max_bytes, 1024);
                assert_eq!(actual_bytes, 2048);
            }
            other => assert_eq!(
                other,
                ActionError::PayloadTooLarge {
                    max_bytes: 1024,
                    actual_bytes: 2048,
                }
            ),
        }
    }

    // --- Output slot out of bounds carries exact slot and max ---

    #[test]
    fn action_error_output_slot_out_of_bounds_reports_exact_boundary() {
        let error = ActionError::OutputSlotOutOfBounds {
            slot: 10,
            max_slots: 4,
        };
        match error {
            ActionError::OutputSlotOutOfBounds { slot, max_slots } => {
                assert_eq!(slot, 10);
                assert_eq!(max_slots, 4);
            }
            other => assert_eq!(
                other,
                ActionError::OutputSlotOutOfBounds {
                    slot: 10,
                    max_slots: 4,
                }
            ),
        }
    }

    // --- ActionContract with max_output_bytes = 0 is syntactically valid ---

    #[test]
    fn action_contract_with_zero_output_bytes_is_constructable() {
        let contract = ActionContract {
            id: ActionId::new(1),
            input_slot_count: 1,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 0,
            idempotency: Idempotency::DeterministicPure,
        };
        assert_eq!(contract.max_output_bytes, 0);
        assert_eq!(contract.output_slot_count, 0);
    }

    // --- ActionContract with timeout_ms = 0 is syntactically valid ---

    #[test]
    fn action_contract_with_zero_timeout_is_constructable() {
        let contract = ActionContract {
            id: ActionId::new(1),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 0,
            idempotency: Idempotency::AtLeastOnceExternal,
        };
        assert_eq!(contract.timeout_ms, 0);
    }

    // --- ActionOutputReady with Secret taint propagation ---

    #[test]
    fn action_output_ready_carries_secret_taint_without_downgrade() {
        let output = ActionOutputReady {
            output_slot: SlotIdx::new(0),
            value: SlotValue::I64(42),
            taint: Taint::Secret,
            encoded_len: 8,
        };
        assert_eq!(output.taint, Taint::Secret);
    }

    // --- ActionFailure with retryable flag ---

    #[test]
    fn action_failure_with_retryable_true_is_retryable() {
        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        assert!(failure.retryable);
    }

    // --- ActionOutcome equality for Suspended with ticket ---

    #[test]
    fn action_outcome_suspended_carries_ticket_identity() {
        let ticket = ActionTicket {
            run: RunId::new(42),
            step: StepIdx::new(3),
            seq: SeqNo::new(7),
            action: ActionId::new(1),
            attempt: 1,
            idempotency_key: 999,
        };
        let outcome = ActionOutcome::Suspended(ticket);
        match outcome {
            ActionOutcome::Suspended(t) => {
                assert_eq!(t.run, RunId::new(42));
                assert_eq!(t.idempotency_key, 999);
            }
            other => assert_eq!(other, ActionOutcome::Suspended(ticket)),
        }
    }

    // --- All ActionFailureCode variants have distinct repr values ---

    #[test]
    fn action_failure_code_repr_values_are_distinct() {
        use std::collections::BTreeSet;
        let codes = [
            ActionFailureCode::Rejected,
            ActionFailureCode::Timeout,
            ActionFailureCode::RateLimited,
            ActionFailureCode::ResourceExhausted,
            ActionFailureCode::ExternalUnavailable,
            ActionFailureCode::InvalidInput,
            ActionFailureCode::PermissionDenied,
            ActionFailureCode::Conflict,
            ActionFailureCode::Unknown,
        ];
        let reprs: BTreeSet<u8> = codes.iter().map(|c| failure_code_repr(*c)).collect();
        assert_eq!(reprs.len(), codes.len());
    }

    fn failure_code_repr(code: ActionFailureCode) -> u8 {
        match code {
            ActionFailureCode::Rejected => 0,
            ActionFailureCode::Timeout => 1,
            ActionFailureCode::RateLimited => 2,
            ActionFailureCode::ResourceExhausted => 3,
            ActionFailureCode::ExternalUnavailable => 4,
            ActionFailureCode::InvalidInput => 5,
            ActionFailureCode::PermissionDenied => 6,
            ActionFailureCode::Conflict => 7,
            ActionFailureCode::Unknown => 255,
        }
    }
}
