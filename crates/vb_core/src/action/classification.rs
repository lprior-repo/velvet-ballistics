//! Classification enums for action semantics: idempotency, side effects, retry behavior.

use serde::{Deserialize, Serialize};

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
