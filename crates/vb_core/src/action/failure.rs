//! Machine-readable failure codes and the `ActionFailure` struct.

use crate::action::RetryPolicy;
use crate::ids::BlobId;
use crate::value::Taint;
use serde::{Deserialize, Serialize};

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
