use super::{ApproverId, CapabilityScopeSpec, GrantDispatchContext, GrantTimeMs, RevocationEpoch};
use crate::{ActionTicket, WorkflowDigest};

impl GrantDispatchContext {
    /// Bundles the expected bindings with caller-owned time/revocation.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        artifact: WorkflowDigest,
        ticket: ActionTicket,
        declared_scope: CapabilityScopeSpec,
        expected_issuer: ApproverId,
        expected_subject: ApproverId,
        now_ms: GrantTimeMs,
        min_epoch: RevocationEpoch,
        can_suspend_at_missing_grant_gates: bool,
    ) -> Self {
        Self {
            artifact,
            ticket,
            declared_scope,
            expected_issuer,
            expected_subject,
            now_ms,
            min_epoch,
            can_suspend_at_missing_grant_gates,
        }
    }
}
