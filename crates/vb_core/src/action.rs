#![forbid(unsafe_code)]

//! Action ABI contract for the do/retry/on_error primitives.

use crate::capability::Capability;
use crate::frame::RunFrame;
use crate::ids::{ActionId, BlobId, RunId, SeqNo, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Maximum length for an action name.
const MAX_ACTION_NAME_LENGTH: usize = 64;

/// Error type for invalid action names.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ActionNameError {
    /// Action name is empty or whitespace-only.
    #[error("action name is empty")]
    Empty,
    /// Action name exceeds maximum length of 64 characters.
    #[error("action name exceeds maximum length of {MAX_ACTION_NAME_LENGTH} characters")]
    TooLong,
    /// Action name contains whitespace.
    #[error("action name contains whitespace")]
    ContainsWhitespace,
}

/// A validated action name.
///
/// An action name is a non-empty string with no whitespace and at most 64 characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActionName(String);

impl ActionName {
    /// Creates a new validated action name.
    ///
    /// Returns `Err(ActionNameError)` if the name is empty, too long, or contains whitespace.
    pub fn new(s: impl Into<String>) -> Result<Self, ActionNameError> {
        let s = s.into();
        Self::validate(&s)?;
        Ok(Self(s))
    }
    /// Creates a new validated action name from a `&'static str` whose
    /// invariants are guaranteed by construction at the call site.
    ///
    /// Use this constructor ONLY when the input is a string literal (or other
    /// `&'static str`) known at compile time to be non-empty, free of
    /// whitespace, and within `MAX_ACTION_NAME_LENGTH`. Examples: test
    /// fixtures with hardcoded names, `const` tables of action names, or
    /// generated code emitting validated literals.
    ///
    /// The internal `.expect()` is bounded at construction: the caller
    /// guarantees the static-slice input satisfies the validation rules.
    /// The single panic path is reachable only via a programmer error at
    /// the call site (passing a literal that violates an invariant), which
    /// is the intended fail-fast behavior for a hardcoded-valid input.
    ///
    /// For runtime/derived input where the value is not statically known,
    /// use the fallible [`ActionName::new`] and propagate the error.
    pub fn from_static_infallible(s: &'static str) -> Self {
        Self::new(s).expect("ActionName::from_static_infallible caller must guarantee non-empty, no-whitespace, length<=64; programmer error otherwise")
    }

    /// Validates an action name string.
    fn validate(s: &str) -> Result<(), ActionNameError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(ActionNameError::Empty);
        }
        if trimmed.len() > MAX_ACTION_NAME_LENGTH {
            return Err(ActionNameError::TooLong);
        }
        if trimmed.chars().any(|c| c.is_whitespace()) {
            return Err(ActionNameError::ContainsWhitespace);
        }
        Ok(())
    }

    /// Returns the action name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.trim()
    }
}

impl std::fmt::Display for ActionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl AsRef<str> for ActionName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// Declares how an action behaves with respect to repeated execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
pub enum RetrySafety {
    /// Always safe to retry (pure/idempotent).
    Safe = 0,
    /// Safe to retry IF an idempotency key is present.
    KeyRequired = 1,
    /// Never safe to retry (destructive side-effect with no key).
    Unsafe = 2,
}

/// Policy for whether an action failure can be retried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum RetryPolicy {
    /// Failure can be retried.
    Retryable = 0,
    /// Failure cannot be retried.
    NonRetryable = 1,
}

/// Verification error when an action's idempotency contract is violated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[non_exhaustive]
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
    /// Maximum attempts allowed (capacity bound from retry policy).
    pub capacity: u16,
}

/// Computes the canonical deterministic idempotency key for an action ticket.
#[must_use]
pub fn compute_action_idempotency_key(run: RunId, seq: SeqNo, action: ActionId) -> u128 {
    let run_part = u128::from(run.get());
    let seq_part = u128::from(seq.get());
    let action_part = u128::from(action.get());
    run_part
        .wrapping_mul(0x6c62272e07bb0143_u128)
        .wrapping_add(seq_part)
        .wrapping_mul(0x3b4f1a5b6c2d8e7f_u128)
        .wrapping_add(action_part)
        .wrapping_mul(0x5bd1e9956c7b4d3a_u128)
}

/// Returns true when a ticket carries the canonical key for its run/seq/action.
#[must_use]
pub fn action_ticket_has_valid_key(ticket: ActionTicket) -> bool {
    ticket.idempotency_key == compute_action_idempotency_key(ticket.run, ticket.seq, ticket.action)
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
    pub retry_policy: RetryPolicy,
    /// Taint of the input that caused the failure.
    pub taint: Taint,
    /// Optional detail blob for diagnostics.
    pub detail: Option<BlobId>,
    /// Encoded byte length of the failure payload.
    pub encoded_len: u32,
}

impl From<ActionFailureCode> for ActionFailure {
    fn from(code: ActionFailureCode) -> Self {
        let retry_policy = match code {
            ActionFailureCode::Rejected => RetryPolicy::NonRetryable,
            ActionFailureCode::Timeout => RetryPolicy::Retryable,
            ActionFailureCode::RateLimited => RetryPolicy::Retryable,
            ActionFailureCode::ResourceExhausted => RetryPolicy::Retryable,
            ActionFailureCode::ExternalUnavailable => RetryPolicy::Retryable,
            ActionFailureCode::InvalidInput => RetryPolicy::NonRetryable,
            ActionFailureCode::PermissionDenied => RetryPolicy::NonRetryable,
            ActionFailureCode::Conflict => RetryPolicy::Retryable,
            ActionFailureCode::Unknown => RetryPolicy::NonRetryable,
        };
        Self {
            code,
            retry_policy,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        }
    }
}

/// Machine-readable action failure codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[non_exhaustive]
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
#[non_exhaustive]
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
            Taint::Secret | Taint::DerivedFromSecret | Taint::Random | Taint::TimeDependent => {
                Taint::DerivedFromSecret
            }
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
            Taint::Random => {
                return Err(IdempotencyViolation::RandomInKey(u32::from(slot.get())));
            }
            Taint::TimeDependent => {
                return Err(IdempotencyViolation::TimeInKey(u32::from(slot.get())));
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
    capacity: u16,
) -> ActionTicket {
    ActionTicket {
        run,
        step,
        seq,
        action,
        attempt,
        idempotency_key,
        capacity,
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
        ActionOutcome::Ready(output_ready) => validate_ready_outcome(contract, output_ready),
        ActionOutcome::Suspended(_) => validate_suspended_outcome(),
        ActionOutcome::Failed(_) => validate_failed_outcome(),
    }
}

/// Validates the output slot index for a Ready action outcome.
fn validate_ready_outcome(
    contract: &ActionContract,
    output_ready: &ActionOutputReady,
) -> Result<(), ActionError> {
    check_output_slot_in_bounds(output_ready.output_slot, contract.output_slot_count)?;
    check_output_size_in_bounds(output_ready.encoded_len, contract.max_output_bytes)?;
    Ok(())
}

fn check_output_size_in_bounds(actual_bytes: u32, max_bytes: u32) -> Result<(), ActionError> {
    if actual_bytes > max_bytes {
        return Err(ActionError::PayloadTooLarge {
            max_bytes,
            actual_bytes,
        });
    }
    Ok(())
}

/// Checks that the output slot index is within the contract's declared bounds.
fn check_output_slot_in_bounds(slot: SlotIdx, max_slots: u16) -> Result<(), ActionError> {
    let slot_raw = slot.get();
    if u32::from(slot_raw) >= u32::from(max_slots) && max_slots > 0 {
        return Err(ActionError::OutputSlotOutOfBounds {
            slot: slot_raw,
            max_slots,
        });
    }
    Ok(())
}

/// Suspension is not a terminal outcome; completing with a suspended outcome is invalid.
fn validate_suspended_outcome() -> Result<(), ActionError> {
    Err(ActionError::DispatchFailed)
}

/// Failure outcomes are always valid terminal completions.
fn validate_failed_outcome() -> Result<(), ActionError> {
    Ok(())
}

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

#[cfg(test)]
#[path = "action/tests.rs"]
mod tests;
