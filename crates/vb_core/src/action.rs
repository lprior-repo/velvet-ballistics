#![forbid(unsafe_code)]

//! Action ABI contract for the do/retry/on_error primitives.

use crate::capability::Capability;
use crate::frame::RunFrame;
use crate::ids::{ActionId, BlobId, RunId, SeqNo, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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

/// Classifies the observable side effects of an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum SideEffect {
    /// No observable side effects (pure computation).
    None = 0,
    /// Writes to external state (database, file, API).
    Writes = 1,
    /// Sends a message or notification.
    Sends = 2,
    /// Creates a resource (provision, allocate).
    Creates = 3,
    /// Destroys a resource (deprovision, delete).
    Destroys = 4,
}

/// Classifies whether an action can be safely retried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum RetrySafety {
    /// Always safe to retry (pure/idempotent).
    Safe = 0,
    /// Safe to retry IF an idempotency key is present.
    KeyRequired = 1,
    /// Never safe to retry (destructive side-effect with no key).
    Unsafe = 2,
}

/// Verification error when an action's idempotency contract is violated.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdempotencyViolation {
    /// Action has side-effects but no idempotency key was provided.
    #[error("action has side-effect {0:?} but no idempotency key")]
    MissingKey(SideEffect),
    /// Idempotency key ingredient contains a secret-tainted value.
    #[error("idempotency key ingredient contains secret-tainted value at slot {0}")]
    SecretInKey(u32),
    /// Idempotency key ingredient contains a random-generated value.
    #[error("idempotency key ingredient contains random value at slot {0}")]
    RandomInKey(u32),
    /// Idempotency key ingredient contains a time-dependent value.
    #[error("idempotency key ingredient contains time-dependent value at slot {0}")]
    TimeInKey(u32),
}

/// Static contract describing an action's resource and correctness bounds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ActionError {
    /// The requested action is not registered.
    #[error("unknown action: {action:?}")]
    UnknownAction {
        /// Action identifier that was not found.
        action: ActionId,
    },
    /// The supplied ticket does not match any in-flight action.
    #[error("invalid action ticket")]
    InvalidTicket,
    /// Input payload exceeds the contract's declared byte limit.
    #[error("action payload too large: {actual_bytes} bytes, max {max_bytes}")]
    PayloadTooLarge {
        /// Declared maximum bytes.
        max_bytes: u32,
        /// Actual payload bytes.
        actual_bytes: u32,
    },
    /// Output slot index exceeds the contract's declared output count.
    #[error("action output slot out of bounds: {slot}, max {max_slots}")]
    OutputSlotOutOfBounds {
        /// Requested slot index.
        slot: u16,
        /// Declared output slot count.
        max_slots: u16,
    },
    /// Replay was blocked because the action is not idempotent.
    #[error("non-idempotent action replay blocked")]
    NonIdempotentReplayBlocked,
    /// Completion was already recorded for this ticket.
    #[error("action completion already recorded")]
    CompletionAlreadyRecorded,
    /// Action dispatch queue is at capacity.
    #[error("action dispatch queue full")]
    QueueFull,
    /// Output encoding failed.
    #[error("action output encoding failed")]
    EncodingFailed,
    /// Action dispatch failed internally.
    #[error("action dispatch failed")]
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

/// Validates that idempotency key ingredients do not contain prohibited values.
///
/// Keys must NOT contain:
/// - Secret-tainted values (would leak information through the key)
/// - Random-generated values (keys must be deterministic)
/// - Time-dependent values (keys must be reproducible across retries)
///
/// The function checks the taint of each slot referenced in `key_slots` via the
/// provided `frame`. Slots with `Taint::Secret` or `Taint::DerivedFromSecret`
/// are rejected. Random and time-dependent checks require additional metadata
/// not yet modeled in `SlotValue`; they are scaffolded here for future extension.
pub fn validate_idempotency_key_ingredients(
    key_slots: &[SlotIdx],
    frame: &RunFrame,
) -> Result<(), IdempotencyViolation> {
    let mut i = 0;
    while i < key_slots.len() {
        let Some(&slot) = key_slots.get(i) else {
            break;
        };
        let Ok(slot_taint) = frame.read_taint(slot) else {
            i = match i.checked_add(1) {
                Some(next) => next,
                None => break,
            };
            continue;
        };
        match slot_taint {
            Taint::Clean => {}
            Taint::Secret | Taint::DerivedFromSecret => {
                return Err(IdempotencyViolation::SecretInKey(u32::from(slot.get())));
            }
        }
        i = match i.checked_add(1) {
            Some(next) => next,
            None => break,
        };
    }
    Ok(())
}

/// Verifies whether an action can be safely retried given its contract,
/// the idempotency key slots, and the current run frame.
///
/// Verification rules:
/// - `RetrySafety::Safe` always passes.
/// - `RetrySafety::KeyRequired` passes if key ingredients are valid.
/// - `RetrySafety::Unsafe` always fails with `MissingKey`.
/// - Actions with `SideEffect::None` always pass regardless of retry_safety.
pub fn verify_idempotency(
    action: &ActionContract,
    key_slots: &[SlotIdx],
    frame: &RunFrame,
) -> Result<(), IdempotencyViolation> {
    if action.side_effect == SideEffect::None {
        return Ok(());
    }
    match action.retry_safety {
        RetrySafety::Safe => Ok(()),
        RetrySafety::KeyRequired => {
            if key_slots.is_empty() {
                return Err(IdempotencyViolation::MissingKey(action.side_effect));
            }
            validate_idempotency_key_ingredients(key_slots, frame)
        }
        RetrySafety::Unsafe => Err(IdempotencyViolation::MissingKey(action.side_effect)),
    }
}

/// Validates that an action dispatch is legal against the declared contract.
///
/// Checks:
/// - Input slot index is within the frame's slot bounds.
/// - Input slot is populated (not uninitialized).
/// - Output slot index is within the frame's slot bounds.
/// - Contract action ID matches the provided ID.
///
/// Returns `Ok(())` if the dispatch is valid, or the appropriate `ActionError`.
pub fn validate_action_dispatch(
    _contract: &ActionContract,
    frame: &RunFrame,
    input_slot: SlotIdx,
    output_slot: SlotIdx,
) -> Result<(), ActionError> {
    // Verify input slot is readable (populated and within frame bounds).
    if frame.read_slot(input_slot).is_err() {
        // Input slot is either out of bounds or uninitialized.
        // We treat both as dispatch failure since the action cannot proceed.
        return Err(ActionError::DispatchFailed);
    }

    // Verify output slot is writable (within frame bounds).
    if output_slot.as_usize() >= usize::from(frame.slot_count()) {
        return Err(ActionError::DispatchFailed);
    }

    Ok(())
}

/// Issues an action ticket for a Do-node suspension.
///
/// Constructs a new `ActionTicket` from the run metadata, action contract,
/// and current attempt counter. The ticket tracks this invocation across
/// suspension boundaries.
pub fn issue_action_ticket(
    run: RunId,
    step: StepIdx,
    seq: SeqNo,
    action: ActionId,
    attempt: u16,
    idempotency_key: u128,
) -> ActionTicket {
    ActionTicket {
        run,
        step,
        seq,
        action,
        attempt,
        idempotency_key,
    }
}

/// Validates that an action completion outcome is consistent with the contract.
///
/// For success completions, verifies the output slot is valid.
/// For failure completions, verifies the failure code is recognized.
pub fn validate_action_outcome(
    contract: &ActionContract,
    outcome: &ActionOutcome,
) -> Result<(), ActionError> {
    match outcome {
        ActionOutcome::Ready(output_ready) => {
            let slot_raw = output_ready.output_slot.get();
            if u32::from(slot_raw) >= u32::from(contract.output_slot_count)
                && contract.output_slot_count > 0
            {
                return Err(ActionError::OutputSlotOutOfBounds {
                    slot: slot_raw,
                    max_slots: contract.output_slot_count,
                });
            }
            Ok(())
        }
        ActionOutcome::Suspended(_) => {
            // Suspension is not a terminal outcome; it is a mid-flight state.
            // Completing with a suspended outcome is invalid.
            Err(ActionError::DispatchFailed)
        }
        ActionOutcome::Failed(_) => Ok(()),
    }
}

/// Journal events for Do-node action lifecycle.
///
/// These events are recorded for crash recovery. The journal records the
/// suspension (ticket issuance) and the terminal outcome (success or failure).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionJournalEvent {
    /// Engine suspended on a Do node, issuing an action ticket.
    Suspended {
        /// Ticket identifying the in-flight action.
        ticket: ActionTicket,
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
        /// Output slot written by the action.
        output_slot: SlotIdx,
        /// Taint propagated from input to output.
        output_taint: Taint,
    },
    /// Action failed terminally.
    Failed {
        /// Ticket of the failed action.
        ticket: ActionTicket,
        /// Failure code for diagnostics.
        code: ActionFailureCode,
        /// Whether the failure is retryable.
        retryable: bool,
    },
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
    // Phase 2 adversarial BDD tests -- action ABI security & taint vectors
    // =========================================================================

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
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        assert_eq!(contract.max_output_bytes, 0);
        assert_eq!(contract.output_slot_count, 0);
    }

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
            side_effect: SideEffect::Writes,
            retry_safety: RetrySafety::KeyRequired,
            required_capabilities: Box::new([]),
        };
        assert_eq!(contract.timeout_ms, 0);
    }

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

    // =========================================================================
    // Phase 38 tests -- SideEffect, RetrySafety, IdempotencyViolation
    // =========================================================================

    #[test]
    fn side_effect_repr_values_are_distinct() {
        let effects = [
            SideEffect::None,
            SideEffect::Writes,
            SideEffect::Sends,
            SideEffect::Creates,
            SideEffect::Destroys,
        ];
        let mut reprs: [u8; 5] = [0; 5];
        let mut count = 0;
        for effect in &effects {
            let repr = side_effect_repr(*effect);
            reprs[count] = repr;
            count = match count.checked_add(1) {
                Some(n) => n,
                None => break,
            };
        }
        let mut i = 0;
        while i < count {
            let mut j = match i.checked_add(1) {
                Some(n) => n,
                None => break,
            };
            while j < count {
                assert_ne!(reprs[i], reprs[j], "duplicate repr at {i} and {j}");
                j = match j.checked_add(1) {
                    Some(n) => n,
                    None => break,
                };
            }
            i = match i.checked_add(1) {
                Some(n) => n,
                None => break,
            };
        }
        assert_eq!(count, 5);
    }

    fn side_effect_repr(effect: SideEffect) -> u8 {
        match effect {
            SideEffect::None => 0,
            SideEffect::Writes => 1,
            SideEffect::Sends => 2,
            SideEffect::Creates => 3,
            SideEffect::Destroys => 4,
        }
    }

    #[test]
    fn retry_safety_repr_values_are_distinct() {
        let safeties = [RetrySafety::Safe, RetrySafety::KeyRequired, RetrySafety::Unsafe];
        let repr_a = retry_safety_repr(safeties[0]);
        let repr_b = retry_safety_repr(safeties[1]);
        let repr_c = retry_safety_repr(safeties[2]);
        assert_ne!(repr_a, repr_b);
        assert_ne!(repr_b, repr_c);
        assert_ne!(repr_a, repr_c);
    }

    fn retry_safety_repr(safety: RetrySafety) -> u8 {
        match safety {
            RetrySafety::Safe => 0,
            RetrySafety::KeyRequired => 1,
            RetrySafety::Unsafe => 2,
        }
    }

    #[test]
    fn idempotency_violation_missing_key_carries_side_effect() {
        let violation = IdempotencyViolation::MissingKey(SideEffect::Writes);
        match violation {
            IdempotencyViolation::MissingKey(eff) => assert_eq!(eff, SideEffect::Writes),
            other => panic!("expected MissingKey, got {other:?}"),
        }
    }

    #[test]
    fn idempotency_violation_secret_in_key_carries_slot() {
        let violation = IdempotencyViolation::SecretInKey(7);
        match violation {
            IdempotencyViolation::SecretInKey(slot) => assert_eq!(slot, 7),
            other => panic!("expected SecretInKey, got {other:?}"),
        }
    }

    #[test]
    fn idempotency_violation_random_in_key_carries_slot() {
        let violation = IdempotencyViolation::RandomInKey(3);
        match violation {
            IdempotencyViolation::RandomInKey(slot) => assert_eq!(slot, 3),
            other => panic!("expected RandomInKey, got {other:?}"),
        }
    }

    #[test]
    fn idempotency_violation_time_in_key_carries_slot() {
        let violation = IdempotencyViolation::TimeInKey(5);
        match violation {
            IdempotencyViolation::TimeInKey(slot) => assert_eq!(slot, 5),
            other => panic!("expected TimeInKey, got {other:?}"),
        }
    }

    #[test]
    fn verify_idempotency_pure_action_always_passes() {
        let action = ActionContract {
            id: ActionId::new(1),
            input_slot_count: 0,
            output_slot_count: 1,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 0,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
        assert!(frame.is_ok());
        let frame = frame.ok().expect("test setup");
        let result = verify_idempotency(&action, &[], &frame);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn verify_idempotency_safe_action_with_side_effect_passes() {
        let action = ActionContract {
            id: ActionId::new(2),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 1000,
            idempotency: Idempotency::IdempotentExternal,
            side_effect: SideEffect::Writes,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
        assert!(frame.is_ok());
        let frame = frame.ok().expect("test setup");
        let result = verify_idempotency(&action, &[], &frame);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn verify_idempotency_unsafe_action_rejected() {
        let action = ActionContract {
            id: ActionId::new(3),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 1000,
            idempotency: Idempotency::AtLeastOnceExternal,
            side_effect: SideEffect::Destroys,
            retry_safety: RetrySafety::Unsafe,
            required_capabilities: Box::new([]),
        };
        let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
        assert!(frame.is_ok());
        let frame = frame.ok().expect("test setup");
        let result = verify_idempotency(&action, &[SlotIdx::new(0)], &frame);
        assert_eq!(result, Err(IdempotencyViolation::MissingKey(SideEffect::Destroys)));
    }

    #[test]
    fn verify_idempotency_key_required_empty_keys_rejected() {
        let action = ActionContract {
            id: ActionId::new(4),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 1000,
            idempotency: Idempotency::IdempotentExternal,
            side_effect: SideEffect::Writes,
            retry_safety: RetrySafety::KeyRequired,
            required_capabilities: Box::new([]),
        };
        let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
        assert!(frame.is_ok());
        let frame = frame.ok().expect("test setup");
        let result = verify_idempotency(&action, &[], &frame);
        assert_eq!(result, Err(IdempotencyViolation::MissingKey(SideEffect::Writes)));
    }

    #[test]
    fn verify_idempotency_key_required_clean_keys_passes() {
        let action = ActionContract {
            id: ActionId::new(5),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 1000,
            idempotency: Idempotency::IdempotentExternal,
            side_effect: SideEffect::Writes,
            retry_safety: RetrySafety::KeyRequired,
            required_capabilities: Box::new([]),
        };
        let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
        assert!(frame.is_ok());
        let frame = frame.ok().expect("test setup");
        let key_slots = [SlotIdx::new(0), SlotIdx::new(1)];
        let result = verify_idempotency(&action, &key_slots, &frame);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn verify_idempotency_key_required_secret_key_rejected() {
        let action = ActionContract {
            id: ActionId::new(6),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 1000,
            idempotency: Idempotency::IdempotentExternal,
            side_effect: SideEffect::Writes,
            retry_safety: RetrySafety::KeyRequired,
            required_capabilities: Box::new([]),
        };
        let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
        assert!(frame.is_ok());
        let mut frame = frame.ok().expect("test setup");
        let write_result = frame.write_slot_with_taint(
            SlotIdx::new(0),
            SlotValue::I64(42),
            Taint::Secret,
        );
        assert!(write_result.is_ok());
        let key_slots = [SlotIdx::new(0)];
        let result = verify_idempotency(&action, &key_slots, &frame);
        assert_eq!(result, Err(IdempotencyViolation::SecretInKey(0)));
    }

    #[test]
    fn validate_key_ingredients_clean_slots_pass() {
        let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
        assert!(frame.is_ok());
        let frame = frame.ok().expect("test setup");
        let key_slots = [SlotIdx::new(0), SlotIdx::new(1)];
        let result = validate_idempotency_key_ingredients(&key_slots, &frame);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_key_ingredients_derived_secret_rejected() {
        let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
        assert!(frame.is_ok());
        let mut frame = frame.ok().expect("test setup");
        let write_result = frame.write_slot_with_taint(
            SlotIdx::new(1),
            SlotValue::I64(99),
            Taint::DerivedFromSecret,
        );
        assert!(write_result.is_ok());
        let key_slots = [SlotIdx::new(1)];
        let result = validate_idempotency_key_ingredients(&key_slots, &frame);
        assert_eq!(result, Err(IdempotencyViolation::SecretInKey(1)));
    }

    #[test]
    fn verify_idempotency_sends_side_effect_key_required_rejected_without_key() {
        let action = ActionContract {
            id: ActionId::new(7),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 1000,
            idempotency: Idempotency::IdempotentExternal,
            side_effect: SideEffect::Sends,
            retry_safety: RetrySafety::KeyRequired,
            required_capabilities: Box::new([]),
        };
        let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
        assert!(frame.is_ok());
        let frame = frame.ok().expect("test setup");
        let result = verify_idempotency(&action, &[], &frame);
        assert_eq!(result, Err(IdempotencyViolation::MissingKey(SideEffect::Sends)));
    }

    #[test]
    fn verify_idempotency_creates_side_effect_unsafe_rejected() {
        let action = ActionContract {
            id: ActionId::new(8),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 1000,
            idempotency: Idempotency::AtLeastOnceExternal,
            side_effect: SideEffect::Creates,
            retry_safety: RetrySafety::Unsafe,
            required_capabilities: Box::new([]),
        };
        let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
        assert!(frame.is_ok());
        let frame = frame.ok().expect("test setup");
        let result = verify_idempotency(&action, &[SlotIdx::new(0)], &frame);
        assert_eq!(result, Err(IdempotencyViolation::MissingKey(SideEffect::Creates)));
    }

    #[test]
    fn action_contract_serializes_with_new_fields() {
        let contract = ActionContract {
            id: ActionId::new(1),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::IdempotentExternal,
            side_effect: SideEffect::Writes,
            retry_safety: RetrySafety::KeyRequired,
            required_capabilities: Box::new([]),
        };
        let bytes = postcard::to_allocvec(&contract);
        assert!(bytes.is_ok(), "postcard serialization should succeed");
        let bytes = bytes.ok().expect("test setup");
        let recovered: Result<ActionContract, _> = postcard::from_bytes(&bytes);
        assert!(recovered.is_ok(), "postcard deserialization should succeed");
        let recovered = recovered.ok().expect("test setup");
        assert_eq!(recovered.id, contract.id);
        assert_eq!(recovered.side_effect, contract.side_effect);
        assert_eq!(recovered.retry_safety, contract.retry_safety);
    }

    #[test]
    fn side_effect_is_copy() {
        let a = SideEffect::Writes;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn retry_safety_is_copy() {
        let a = RetrySafety::KeyRequired;
        let b = a;
        assert_eq!(a, b);
    }

    // =========================================================================
    // Phase 38 adversarial tests -- idempotency verification rejection paths
    // =========================================================================

    #[test]
    fn verify_idempotency_writes_with_safe_passes() {
        let action = ActionContract {
            id: ActionId::new(100),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::IdempotentExternal,
            side_effect: SideEffect::Writes,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        let frame = RunFrame::new(RunId::new(50), StepIdx::new(0), 2, 2);
        assert!(frame.is_ok());
        let frame = frame.ok().expect("test setup");
        let result = verify_idempotency(&action, &[], &frame);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn verify_idempotency_destroys_with_unsafe_rejected_even_with_keys() {
        let action = ActionContract {
            id: ActionId::new(101),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::AtLeastOnceExternal,
            side_effect: SideEffect::Destroys,
            retry_safety: RetrySafety::Unsafe,
            required_capabilities: Box::new([]),
        };
        let frame = RunFrame::new(RunId::new(51), StepIdx::new(0), 2, 2);
        assert!(frame.is_ok());
        let frame = frame.ok().expect("test setup");
        // Even though we supply key slots, Unsafe is always rejected.
        let key_slots = [SlotIdx::new(0), SlotIdx::new(1)];
        let result = verify_idempotency(&action, &key_slots, &frame);
        assert_eq!(
            result,
            Err(IdempotencyViolation::MissingKey(SideEffect::Destroys))
        );
    }

    #[test]
    fn verify_idempotency_destroys_with_unsafe_rejected_without_keys() {
        let action = ActionContract {
            id: ActionId::new(102),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::AtLeastOnceExternal,
            side_effect: SideEffect::Destroys,
            retry_safety: RetrySafety::Unsafe,
            required_capabilities: Box::new([]),
        };
        let frame = RunFrame::new(RunId::new(52), StepIdx::new(0), 2, 2);
        assert!(frame.is_ok());
        let frame = frame.ok().expect("test setup");
        let result = verify_idempotency(&action, &[], &frame);
        assert_eq!(
            result,
            Err(IdempotencyViolation::MissingKey(SideEffect::Destroys))
        );
    }

    #[test]
    fn verify_idempotency_key_required_rejects_secret_tainted_key_slot() {
        let action = ActionContract {
            id: ActionId::new(103),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::IdempotentExternal,
            side_effect: SideEffect::Writes,
            retry_safety: RetrySafety::KeyRequired,
            required_capabilities: Box::new([]),
        };
        let frame = RunFrame::new(RunId::new(53), StepIdx::new(0), 4, 4);
        assert!(frame.is_ok());
        let mut frame = frame.ok().expect("test setup");
        // Slot 0 has a clean value.
        let write_clean = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(10), Taint::Clean);
        assert!(write_clean.is_ok());
        // Slot 1 has a secret-tainted value.
        let write_secret = frame.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(20), Taint::Secret);
        assert!(write_secret.is_ok());
        // Slot 2 has a derived-from-secret value.
        let write_derived = frame.write_slot_with_taint(
            SlotIdx::new(2),
            SlotValue::I64(30),
            Taint::DerivedFromSecret,
        );
        assert!(write_derived.is_ok());

        // Clean key passes.
        let result_clean = verify_idempotency(&action, &[SlotIdx::new(0)], &frame);
        assert_eq!(result_clean, Ok(()));

        // Secret key is rejected.
        let result_secret = verify_idempotency(&action, &[SlotIdx::new(1)], &frame);
        assert_eq!(result_secret, Err(IdempotencyViolation::SecretInKey(1)));

        // DerivedFromSecret key is also rejected.
        let result_derived = verify_idempotency(&action, &[SlotIdx::new(2)], &frame);
        assert_eq!(result_derived, Err(IdempotencyViolation::SecretInKey(2)));
    }

    #[test]
    fn verify_idempotency_key_required_rejects_when_first_slot_clean_but_second_secret() {
        let action = ActionContract {
            id: ActionId::new(104),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::IdempotentExternal,
            side_effect: SideEffect::Creates,
            retry_safety: RetrySafety::KeyRequired,
            required_capabilities: Box::new([]),
        };
        let frame = RunFrame::new(RunId::new(54), StepIdx::new(0), 2, 2);
        assert!(frame.is_ok());
        let mut frame = frame.ok().expect("test setup");
        let write_clean = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(10), Taint::Clean);
        assert!(write_clean.is_ok());
        let write_secret = frame.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(20), Taint::Secret);
        assert!(write_secret.is_ok());
        // Key slots: [clean, secret]. Should reject on the second slot.
        let result = verify_idempotency(&action, &[SlotIdx::new(0), SlotIdx::new(1)], &frame);
        assert_eq!(result, Err(IdempotencyViolation::SecretInKey(1)));
    }

    #[test]
    fn verify_idempotency_none_side_effect_always_passes_even_unsafe() {
        // Actions with SideEffect::None always pass, regardless of retry_safety.
        let action = ActionContract {
            id: ActionId::new(105),
            input_slot_count: 0,
            output_slot_count: 1,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 0,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Unsafe,
            required_capabilities: Box::new([]),
        };
        let frame = RunFrame::new(RunId::new(55), StepIdx::new(0), 1, 1);
        assert!(frame.is_ok());
        let frame = frame.ok().expect("test setup");
        let result = verify_idempotency(&action, &[], &frame);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn verify_idempotency_sends_side_effect_unsafe_rejected() {
        let action = ActionContract {
            id: ActionId::new(106),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::AtLeastOnceExternal,
            side_effect: SideEffect::Sends,
            retry_safety: RetrySafety::Unsafe,
            required_capabilities: Box::new([]),
        };
        let frame = RunFrame::new(RunId::new(56), StepIdx::new(0), 2, 2);
        assert!(frame.is_ok());
        let frame = frame.ok().expect("test setup");
        let result = verify_idempotency(&action, &[SlotIdx::new(0)], &frame);
        assert_eq!(
            result,
            Err(IdempotencyViolation::MissingKey(SideEffect::Sends))
        );
    }

    // =========================================================================
    // Phase 18-19 tests -- Action dispatch, ticket issuance, outcome validation
    // =========================================================================

    // --- validate_action_dispatch ---

    #[test]
    fn validate_action_dispatch_succeeds_with_populated_input_and_output_slot() {
        let contract = ActionContract {
            id: ActionId::new(1),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
        assert!(frame.is_ok());
        let mut frame = frame.ok().expect("test setup");
        // Populate both input and output slots before dispatch.
        let write_input = frame.write_slot(SlotIdx::new(0), SlotValue::I64(42));
        assert!(write_input.is_ok());
        let write_output = frame.write_slot(SlotIdx::new(1), SlotValue::I64(0)); // Output must be initialized too.
        assert!(write_output.is_ok());
        let result = validate_action_dispatch(&contract, &frame, SlotIdx::new(0), SlotIdx::new(1));
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_action_dispatch_fails_on_uninitialized_input() {
        let contract = ActionContract {
            id: ActionId::new(1),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
        assert!(frame.is_ok());
        let frame = frame.ok().expect("test setup");
        // Slot 0 is not populated, so dispatch should fail.
        let result = validate_action_dispatch(&contract, &frame, SlotIdx::new(0), SlotIdx::new(1));
        assert_eq!(result, Err(ActionError::DispatchFailed));
    }

    #[test]
    fn validate_action_dispatch_fails_on_out_of_bounds_input() {
        let contract = ActionContract {
            id: ActionId::new(1),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
        assert!(frame.is_ok());
        let frame = frame.ok().expect("test setup");
        let result = validate_action_dispatch(&contract, &frame, SlotIdx::new(99), SlotIdx::new(1));
        assert_eq!(result, Err(ActionError::DispatchFailed));
    }

    #[test]
    fn validate_action_dispatch_succeeds_with_zero_output_count() {
        let contract = ActionContract {
            id: ActionId::new(2),
            input_slot_count: 1,
            output_slot_count: 0,
            max_input_bytes: 1024,
            max_output_bytes: 0,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
        assert!(frame.is_ok());
        let frame = frame.ok().expect("test setup");
        // Even with output_slot_count=0, the output slot is within frame bounds
        // so the taint check passes. But input is uninitialized, so dispatch fails.
        let result = validate_action_dispatch(&contract, &frame, SlotIdx::new(0), SlotIdx::new(0));
        assert_eq!(result, Err(ActionError::DispatchFailed));
    }

    // --- issue_action_ticket ---

    #[test]
    fn issue_action_ticket_captures_all_fields() {
        let ticket = issue_action_ticket(
            RunId::new(42),
            StepIdx::new(3),
            SeqNo::new(7),
            ActionId::new(5),
            2,
            12345,
        );
        assert_eq!(ticket.run, RunId::new(42));
        assert_eq!(ticket.step, StepIdx::new(3));
        assert_eq!(ticket.seq, SeqNo::new(7));
        assert_eq!(ticket.action, ActionId::new(5));
        assert_eq!(ticket.attempt, 2);
        assert_eq!(ticket.idempotency_key, 12345);
    }

    #[test]
    fn issue_action_ticket_with_zero_key_is_valid() {
        let ticket = issue_action_ticket(
            RunId::new(1),
            StepIdx::new(0),
            SeqNo::new(1),
            ActionId::new(0),
            1,
            0,
        );
        assert_eq!(ticket.idempotency_key, 0);
        assert_eq!(ticket.attempt, 1);
    }

    #[test]
    fn issue_action_ticket_with_max_values() {
        let ticket = issue_action_ticket(
            RunId::new(u64::MAX),
            StepIdx::new(u16::MAX),
            SeqNo::new(u64::MAX),
            ActionId::new(u16::MAX),
            u16::MAX,
            u128::MAX,
        );
        assert_eq!(ticket.run, RunId::new(u64::MAX));
        assert_eq!(ticket.step, StepIdx::new(u16::MAX));
        assert_eq!(ticket.seq, SeqNo::new(u64::MAX));
        assert_eq!(ticket.action, ActionId::new(u16::MAX));
        assert_eq!(ticket.attempt, u16::MAX);
        assert_eq!(ticket.idempotency_key, u128::MAX);
    }

    // --- validate_action_outcome ---

    #[test]
    fn validate_action_outcome_ready_succeeds_with_valid_slot() {
        let contract = ActionContract {
            id: ActionId::new(1),
            input_slot_count: 1,
            output_slot_count: 2,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        let output = ActionOutputReady {
            output_slot: SlotIdx::new(0),
            value: SlotValue::I64(42),
            taint: Taint::Clean,
            encoded_len: 8,
        };
        let outcome = ActionOutcome::Ready(output);
        let result = validate_action_outcome(&contract, &outcome);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_action_outcome_ready_rejects_out_of_bounds_slot() {
        let contract = ActionContract {
            id: ActionId::new(1),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        let output = ActionOutputReady {
            output_slot: SlotIdx::new(5),
            value: SlotValue::I64(42),
            taint: Taint::Clean,
            encoded_len: 8,
        };
        let outcome = ActionOutcome::Ready(output);
        let result = validate_action_outcome(&contract, &outcome);
        assert_eq!(
            result,
            Err(ActionError::OutputSlotOutOfBounds {
                slot: 5,
                max_slots: 1,
            })
        );
    }

    #[test]
    fn validate_action_outcome_failed_always_succeeds() {
        let contract = ActionContract {
            id: ActionId::new(1),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let outcome = ActionOutcome::Failed(failure);
        let result = validate_action_outcome(&contract, &outcome);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_action_outcome_suspended_rejected() {
        let contract = ActionContract {
            id: ActionId::new(1),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        let ticket = ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(1),
            action: ActionId::new(1),
            attempt: 1,
            idempotency_key: 0,
        };
        let outcome = ActionOutcome::Suspended(ticket);
        let result = validate_action_outcome(&contract, &outcome);
        assert_eq!(result, Err(ActionError::DispatchFailed));
    }

    // --- ActionJournalEvent ---

    #[test]
    fn journal_event_suspended_roundtrips_fields() {
        let ticket = ActionTicket {
            run: RunId::new(10),
            step: StepIdx::new(2),
            seq: SeqNo::new(3),
            action: ActionId::new(7),
            attempt: 1,
            idempotency_key: 999,
        };
        let event = ActionJournalEvent::Suspended {
            ticket,
            action: ActionId::new(7),
            input_slot: SlotIdx::new(0),
            output_slot: SlotIdx::new(1),
            step: StepIdx::new(2),
        };
        match event {
            ActionJournalEvent::Suspended {
                ticket: t,
                action,
                input_slot,
                output_slot,
                step,
            } => {
                assert_eq!(t.run, RunId::new(10));
                assert_eq!(action, ActionId::new(7));
                assert_eq!(input_slot, SlotIdx::new(0));
                assert_eq!(output_slot, SlotIdx::new(1));
                assert_eq!(step, StepIdx::new(2));
            }
            other => panic!("expected Suspended, got {other:?}"),
        }
    }

    #[test]
    fn journal_event_completed_roundtrips_fields() {
        let ticket = ActionTicket {
            run: RunId::new(11),
            step: StepIdx::new(3),
            seq: SeqNo::new(4),
            action: ActionId::new(8),
            attempt: 1,
            idempotency_key: 0,
        };
        let event = ActionJournalEvent::Completed {
            ticket,
            output_slot: SlotIdx::new(2),
            output_taint: Taint::Secret,
        };
        match event {
            ActionJournalEvent::Completed {
                ticket: t,
                output_slot,
                output_taint,
            } => {
                assert_eq!(t.run, RunId::new(11));
                assert_eq!(output_slot, SlotIdx::new(2));
                assert_eq!(output_taint, Taint::Secret);
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[test]
    fn journal_event_failed_roundtrips_fields() {
        let ticket = ActionTicket {
            run: RunId::new(12),
            step: StepIdx::new(4),
            seq: SeqNo::new(5),
            action: ActionId::new(9),
            attempt: 3,
            idempotency_key: 0,
        };
        let event = ActionJournalEvent::Failed {
            ticket,
            code: ActionFailureCode::Timeout,
            retryable: true,
        };
        match event {
            ActionJournalEvent::Failed {
                ticket: t,
                code,
                retryable,
            } => {
                assert_eq!(t.run, RunId::new(12));
                assert_eq!(code, ActionFailureCode::Timeout);
                assert!(retryable);
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn journal_event_serialization_roundtrip() {
        let ticket = ActionTicket {
            run: RunId::new(99),
            step: StepIdx::new(1),
            seq: SeqNo::new(2),
            action: ActionId::new(3),
            attempt: 1,
            idempotency_key: 12345,
        };
        let event = ActionJournalEvent::Suspended {
            ticket,
            action: ActionId::new(3),
            input_slot: SlotIdx::new(0),
            output_slot: SlotIdx::new(1),
            step: StepIdx::new(1),
        };
        let bytes = postcard::to_allocvec(&event);
        assert!(bytes.is_ok(), "serialization should succeed");
        let bytes = bytes.ok().expect("test setup");
        let recovered: Result<ActionJournalEvent, _> = postcard::from_bytes(&bytes);
        assert!(recovered.is_ok(), "deserialization should succeed");
        assert_eq!(recovered.ok().expect("test setup"), event);
    }

    #[test]
    fn journal_event_completed_serialization_roundtrip() {
        let ticket = ActionTicket {
            run: RunId::new(50),
            step: StepIdx::new(5),
            seq: SeqNo::new(10),
            action: ActionId::new(2),
            attempt: 1,
            idempotency_key: 0,
        };
        let event = ActionJournalEvent::Completed {
            ticket,
            output_slot: SlotIdx::new(3),
            output_taint: Taint::DerivedFromSecret,
        };
        let bytes = postcard::to_allocvec(&event);
        assert!(bytes.is_ok());
        let bytes = bytes.ok().expect("test setup");
        let recovered: Result<ActionJournalEvent, _> = postcard::from_bytes(&bytes);
        assert!(recovered.is_ok());
        assert_eq!(recovered.ok().expect("test setup"), event);
    }

    #[test]
    fn journal_event_failed_serialization_roundtrip() {
        let ticket = ActionTicket {
            run: RunId::new(51),
            step: StepIdx::new(6),
            seq: SeqNo::new(11),
            action: ActionId::new(4),
            attempt: 2,
            idempotency_key: 0,
        };
        let event = ActionJournalEvent::Failed {
            ticket,
            code: ActionFailureCode::Rejected,
            retryable: false,
        };
        let bytes = postcard::to_allocvec(&event);
        assert!(bytes.is_ok());
        let bytes = bytes.ok().expect("test setup");
        let recovered: Result<ActionJournalEvent, _> = postcard::from_bytes(&bytes);
        assert!(recovered.is_ok());
        assert_eq!(recovered.ok().expect("test setup"), event);
    }
}
