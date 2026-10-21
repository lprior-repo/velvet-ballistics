use super::ExecutorError;
use vb_core::action_pack::{ActionPackDigest, CapabilityScopeSpec};
use vb_core::{ActionTicket, WorkflowDigest};

/// Opaque exact operator-issued permission. Runtime rechecks the canonical live ticket.
/// This narrow local permission does not implement expiry/revocation/remote grant lifecycle.
#[derive(Debug, Clone)]
pub struct RegisterGrant {
    pub(super) artifact: WorkflowDigest,
    pub(super) pack: ActionPackDigest,
    pub(super) ticket: ActionTicket,
    pub(super) scope: CapabilityScopeSpec,
    pub(super) timeout_ms: u64,
}

impl RegisterGrant {
    pub(super) fn validate(&self, ticket: ActionTicket) -> Result<(), ExecutorError> {
        if self.ticket != ticket {
            return Err(ExecutorError::Unauthorized);
        }
        if !vb_core::action::action_ticket_has_valid_key(ticket)
            || ticket.attempt == 0
            || ticket.attempt > ticket.capacity
        {
            return Err(ExecutorError::InvalidTicket);
        }
        Ok(())
    }

    /// Exact artifact authorized by the local operator.
    pub const fn artifact_digest(&self) -> WorkflowDigest {
        self.artifact
    }
}
