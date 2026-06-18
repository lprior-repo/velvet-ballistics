//! Typed errors from the action subsystem.

use crate::ids::ActionId;
use crate::value::Taint;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
