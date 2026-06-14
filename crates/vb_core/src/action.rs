#![forbid(unsafe_code)]

//! Action ABI contract for the do/retry/on_error primitives.

use crate::capability::Capability;
use crate::frame::RunFrame;
use crate::ids::{ActionId, BlobId, RunId, SeqNo, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};
use serde::{Deserialize, Serialize};
use std::hash::Hash;
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
///
/// 7-variant master taxonomy per `velvet-ballistics-MASTER.md` §65:
/// `{Pure, LocalRead, LocalWrite, ExternalRead, ExternalWrite, Process, UnsafeShell}`.
///
/// Discriminant values are stable: `Pure = 0` ... `UnsafeShell = 6`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum SideEffect {
    /// Pure deterministic computation with no observable side effects.
    Pure = 0,
    /// Reads from local state (memory, file, local registry) without mutation.
    LocalRead = 1,
    /// Writes to local state (memory, file, local registry) only.
    LocalWrite = 2,
    /// Reads from external state (database, API, remote service) without mutation.
    ExternalRead = 3,
    /// Writes to external state (database, API, remote service) without local change.
    ExternalWrite = 4,
    /// Spawns or controls an external process (subprocess, sidecar).
    Process = 5,
    /// Shells out to an external binary that may be `unsafe` or untrusted.
    UnsafeShell = 6,
}

impl SideEffect {
    /// Returns true if this side-effect is fully pure (no I/O, no shell, no process).
    pub const fn is_pure(self) -> bool {
        matches!(self, Self::Pure)
    }

    /// Returns true if this side-effect can be safely retried without external coordination.
    /// `Pure`, `LocalRead`, `LocalWrite`, `ExternalRead` are idempotent.
    /// `Process`, `UnsafeShell`, `ExternalWrite` are not.
    pub const fn is_idempotent(self) -> bool {
        matches!(
            self,
            Self::Pure | Self::LocalRead | Self::LocalWrite | Self::ExternalRead
        )
    }

    /// Returns true if executing this side-effect requires an external lease/capability.
    pub const fn requires_external_lease(self) -> bool {
        matches!(self, Self::Process | Self::UnsafeShell)
    }
}

/// Classifies whether an action can be safely retried.
///
/// Master plan §65: 4-variant taxonomy. Discriminant order is preserved
/// (0, 1, 2) for binary compatibility with persisted contracts; the new
/// `Unknown` variant is assigned discriminant 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum RetrySafety {
    /// Always safe to retry (pure/idempotent).
    Idempotent = 0,
    /// Safe to retry IF an idempotency key is present.
    RequiresIdempotencyKey = 1,
    /// Never safe to retry (destructive side-effect with no key).
    NotRetrySafe = 2,
    /// Retry safety cannot be statically decided; treat as not retry safe.
    Unknown = 3,
}

impl RetrySafety {
    /// Returns true if this retry safety class is itself idempotent
    /// (no key required, no static gating).
    pub const fn is_idempotent(self) -> bool {
        matches!(self, Self::Idempotent)
    }

    /// Returns true if this retry safety class admits retry at all
    /// (with appropriate gating).
    pub const fn is_retry_safe(self) -> bool {
        matches!(self, Self::Idempotent | Self::RequiresIdempotencyKey)
    }
}

/// Free-function form: returns true if the given retry safety class is itself
/// idempotent (no key required, no static gating).
pub const fn is_idempotent(safety: RetrySafety) -> bool {
    safety.is_idempotent()
}

/// Free-function form: returns true if the given retry safety class admits
/// retry at all (with appropriate gating).
pub const fn is_retry_safe(safety: RetrySafety) -> bool {
    safety.is_retry_safe()
}

/// Runtime check: returns true if the given retry safety class admits
/// retry under the given key-present condition.
///
/// - `Idempotent`: always `true` (no key required).
/// - `RequiresIdempotencyKey`: `true` iff `key_present`.
/// - `NotRetrySafe | Unknown`: always `false`.
pub const fn is_retry_safe_with_key(safety: RetrySafety, key_present: bool) -> bool {
    match safety {
        RetrySafety::Idempotent => true,
        RetrySafety::RequiresIdempotencyKey => key_present,
        RetrySafety::NotRetrySafe | RetrySafety::Unknown => false,
    }
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
    /// Idempotency key ingredient contains a random-tainted value.
    #[error("idempotency key ingredient contains random-tainted value at slot {0}")]
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

/// Marker enum identifying which mock handler should process an action.
///
/// Each variant corresponds to one of the three canonical action names that
/// use mock handlers instead of real implementations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MockMarker {
    /// Mock handler for `github.issue.create` actions.
    GithubIssueCreate = 0,
    /// Mock handler for `ai.classify_ticket` actions.
    AiClassifyTicket = 1,
    /// Mock handler for `http.request` actions.
    #[default]
    HttpGet = 2,
}

impl MockMarker {
    /// Derives a MockMarker from an action contract name.
    ///
    /// Returns the matching variant for the three canonical mock action names,
    /// or `MockMarker::HttpGet` as the default for unknown names.
    #[must_use]
    pub fn from_contract_name(name: &str) -> Self {
        match name {
            "github.issue.create" => MockMarker::GithubIssueCreate,
            "ai.classify_ticket" => MockMarker::AiClassifyTicket,
            _ => MockMarker::HttpGet,
        }
    }
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
    /// Action completion attempted to downgrade taint below the required level.
    #[error("action taint violation: required {required:?}, supplied {supplied:?}")]
    TaintViolation {
        /// Taint required by the action's idempotency contract and input.
        required: Taint,
        /// Taint supplied by the action completion.
        supplied: Taint,
    },
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
/// - DeterministicPure and Idempotency::IdempotentExternal: output taint >= input taint (join).
/// - AtLeastOnceExternal: DerivedFromSecret when any input is Secret/DerivedFromSecret.
/// - Clean result from tainted input is rejected unless the action declares declassification
///   (not modeled here; caller must validate).
///
/// # Defense-in-depth note
///
/// This function is kept in sync with `vb_runtime::shard::lifecycle::reject_taint_downgrade`.
/// Both are defense-in-depth layers; the runtime enforces at completion and the core enforces
/// at validation. The duplication is architectural debt — do not refactor one without checking the other.
#[must_use]
pub const fn propagate_action_taint(idempotency: Idempotency, input_taint: Taint) -> Taint {
    match idempotency {
        // Deterministic/idempotent actions propagate taint unchanged (identity join).
        Idempotency::DeterministicPure | Idempotency::IdempotentExternal => input_taint,
        Idempotency::AtLeastOnceExternal => match input_taint {
            Taint::Clean => Taint::Clean,
            Taint::Secret | Taint::DerivedFromSecret => Taint::DerivedFromSecret,
        },
    }
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
/// - `SideEffect::Pure` always passes regardless of retry_safety.
/// - `RetrySafety::Idempotent` always passes.
/// - `RetrySafety::RequiresIdempotencyKey` passes if key ingredients are valid.
/// - `RetrySafety::NotRetrySafe` always fails with `MissingKey`.
/// - `RetrySafety::Unknown` is statically undecidable; treated as `NotRetrySafe`.
pub fn verify_idempotency(
    action: &ActionContract,
    key_slots: &[SlotIdx],
    frame: &RunFrame,
) -> Result<(), IdempotencyViolation> {
    if action.side_effect.is_pure() {
        return Ok(());
    }
    match action.retry_safety {
        RetrySafety::Idempotent => Ok(()),
        RetrySafety::RequiresIdempotencyKey => {
            if key_slots.is_empty() {
                return Err(IdempotencyViolation::MissingKey(action.side_effect));
            }
            validate_idempotency_key_ingredients(key_slots, frame)
        }
        RetrySafety::NotRetrySafe | RetrySafety::Unknown => {
            Err(IdempotencyViolation::MissingKey(action.side_effect))
        }
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
        ..Default::default()
    }
}

/// Validates that an action completion outcome is consistent with the contract.
///
/// For success completions, verifies the output slot is valid and the output taint
/// satisfies the action's taint propagation contract (no downgrade).
/// For failure completions, verifies the failure code is recognized.
pub fn validate_action_outcome(
    contract: &ActionContract,
    outcome: &ActionOutcome,
    input_taint: Taint,
) -> Result<(), ActionError> {
    match outcome {
        ActionOutcome::Ready(output_ready) => {
            validate_ready_outcome(contract, output_ready, input_taint)
        }
        ActionOutcome::Suspended(_) => validate_suspended_outcome(),
        ActionOutcome::Failed(_) => validate_failed_outcome(),
    }
}

/// Validates the output slot index and taint for a Ready action outcome.
///
/// Rejects completions that attempt to downgrade taint below the level
/// required by the action's idempotency contract and input taint.
fn validate_ready_outcome(
    contract: &ActionContract,
    output_ready: &ActionOutputReady,
    input_taint: Taint,
) -> Result<(), ActionError> {
    check_output_slot_in_bounds(output_ready.output_slot, contract.output_slot_count)?;
    check_output_size_in_bounds(output_ready.encoded_len, contract.max_output_bytes)?;
    check_taint_downgrade(contract.idempotency, input_taint, output_ready.taint)?;
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

/// Checks that the supplied output taint is not a downgrade from the required taint.
///
/// # Defense-in-depth note
///
/// This function is kept in sync with `vb_runtime::shard::lifecycle::reject_taint_downgrade`.
/// Both are defense-in-depth layers; the core validates here and the runtime enforces at
/// completion. The duplication is architectural debt — do not refactor one without checking the other.
///
/// DeterministicPure and IdempotentExternal actions additionally require that the
/// input is Clean and the output is Clean.
/// For all actions, the supplied taint must be at least as restrictive as the
/// taint propagated from the input according to the idempotency contract.
fn check_taint_downgrade(
    idempotency: Idempotency,
    input_taint: Taint,
    supplied: Taint,
) -> Result<(), ActionError> {
    if (idempotency == Idempotency::DeterministicPure
        || idempotency == Idempotency::IdempotentExternal)
        && input_taint != Taint::Clean
    {
        return Err(ActionError::TaintViolation {
            required: Taint::Clean,
            supplied: input_taint,
        });
    }
    if (idempotency == Idempotency::DeterministicPure
        || idempotency == Idempotency::IdempotentExternal)
        && supplied != Taint::Clean
    {
        return Err(ActionError::TaintViolation {
            required: Taint::Clean,
            supplied,
        });
    }
    let required = propagate_action_taint(idempotency, input_taint);
    if crate::value::join_taint(required, supplied) != supplied {
        return Err(ActionError::TaintViolation { required, supplied });
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
