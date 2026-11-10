use super::{CompiledNodeKind, WorkflowError, WorkflowParts};
use serde::{Deserialize, Serialize};

/// Fixed retry policy attached exclusively to one action node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledRetryPolicy {
    /// Total attempts, including the initial action execution.
    pub max_attempts: u16,
    /// Fixed milliseconds between failed and subsequent attempts.
    pub delay_ms: u64,
}

impl CompiledRetryPolicy {
    /// Largest supported fixed delay, in milliseconds.
    pub const MAX_DELAY_MS: u64 = 4_294_967_295;
}

pub(super) fn validate(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    for node in &parts.nodes {
        if let Some(policy) = node.retry
            && (!matches!(node.kind, CompiledNodeKind::Do { .. })
                || policy.max_attempts == 0
                || policy.max_attempts > crate::limits::MAX_RETRY_ATTEMPTS
                || policy.max_attempts > parts.resource_contract.max_retry_attempts
                || policy.delay_ms > CompiledRetryPolicy::MAX_DELAY_MS)
        {
            return Err(WorkflowError::RetryPolicy { step: node.id });
        }
    }
    Ok(())
}
