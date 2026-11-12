#![forbid(unsafe_code)]
//! Scoped grant issuance and pre-dispatch verification (MASTER §77.10).

use super::{
    GrantAdmissionMode, GrantDispatchContext, GrantId, MAX_REVOKED_IDS, ScopedGrant,
    ScopedGrantParams,
};
use crate::action::action_ticket_has_valid_key;
use crate::manual_gate::GateDigest;
use crate::{ActionTicket, WorkflowDigest};

/// Typed grant lifecycle failures. Rejections happen before any effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum GrantError {
    /// Issuer identity is empty or over-long.
    #[error("grant issuer is invalid")]
    InvalidIssuer,
    /// Subject identity is empty or over-long.
    #[error("grant subject is invalid")]
    InvalidSubject,
    /// Issuer does not match the dispatcher expectation.
    #[error("grant issuer mismatch")]
    IssuerMismatch,
    /// Subject does not match the dispatcher expectation.
    #[error("grant subject mismatch")]
    SubjectMismatch,
    /// Capability name is empty.
    #[error("grant capability is empty")]
    EmptyCapability,
    /// Capability action does not match the bound ticket action.
    #[error("grant capability does not match ticket action")]
    CapabilityMismatch,
    /// Validity window is empty (`issued_at >= expires_at`).
    #[error("grant validity window is empty")]
    EmptyWindow,
    /// Bound ticket key or attempt bounds are invalid.
    #[error("grant ticket is invalid")]
    InvalidTicket,
    /// Artifact digest does not match the live invocation.
    #[error("grant artifact mismatch")]
    ArtifactMismatch,
    /// Ticket does not match the canonical pending ticket.
    #[error("grant ticket mismatch")]
    TicketMismatch,
    /// Scope does not equal the declared pack scope (overgrant).
    #[error("grant scope mismatch")]
    ScopeMismatch,
    /// Mode is not the wedge `StrictExact` mode.
    #[error("grant admission mode mismatch")]
    ModeMismatch,
    /// Approval receipt digest is missing (zero).
    #[error("grant approval receipt is missing")]
    MissingApprovalReceipt,
    /// Delegated issuance chain; this wedge allows direct issuance only.
    #[error("grant delegation violation")]
    DelegationViolation,
    /// Dispatch time precedes the validity window.
    #[error("grant is not yet valid")]
    NotYetValid,
    /// Dispatch time reaches or passes the exclusive expiry bound.
    #[error("grant is expired")]
    Expired,
    /// Grant epoch precedes the current revocation epoch.
    #[error("grant is superseded by revocation epoch")]
    StaleEpoch,
    /// Grant id is in the caller revocation set.
    #[error("grant is revoked")]
    Revoked,
    /// Revocation view exceeds the bounded scan capacity.
    #[error("revocation view is too large")]
    TooManyRevokedIds,
}

impl ScopedGrant {
    /// Trusted issuance: validates bindings and records lifecycle facts.
    pub fn issue(
        params: ScopedGrantParams,
        artifact: WorkflowDigest,
        ticket: ActionTicket,
    ) -> Result<Self, GrantError> {
        if !params.issuer.is_valid() {
            return Err(GrantError::InvalidIssuer);
        }
        if !params.subject.is_valid() {
            return Err(GrantError::InvalidSubject);
        }
        if params.capability.name().is_empty() {
            return Err(GrantError::EmptyCapability);
        }
        if params.capability.action_id() != ticket.action {
            return Err(GrantError::CapabilityMismatch);
        }
        if params.issued_at.get() >= params.expires_at.get() {
            return Err(GrantError::EmptyWindow);
        }
        if params.approval_receipt == GateDigest::from_bytes([0u8; 32]) {
            return Err(GrantError::MissingApprovalReceipt);
        }
        if params.delegated_from.is_some() {
            return Err(GrantError::DelegationViolation);
        }
        if !action_ticket_has_valid_key(ticket)
            || ticket.attempt == 0
            || ticket.attempt > ticket.capacity
        {
            return Err(GrantError::InvalidTicket);
        }
        Ok(Self {
            id: params.id,
            issuer: params.issuer,
            subject: params.subject,
            artifact,
            ticket,
            capability: params.capability,
            scope: params.scope,
            mode: GrantAdmissionMode::StrictExact,
            issued_at: params.issued_at,
            expires_at: params.expires_at,
            delegable: params.delegable,
            delegated_from: params.delegated_from,
            revocation_epoch: params.revocation_epoch,
            break_glass: params.break_glass,
            approval_receipt: params.approval_receipt,
            signature: params.signature,
        })
    }

    /// Identity bindings against the live invocation. No clock read.
    pub fn verify_identity(&self, ctx: &GrantDispatchContext) -> Result<(), GrantError> {
        if !matches!(self.mode, GrantAdmissionMode::StrictExact) {
            return Err(GrantError::ModeMismatch);
        }
        if !self.issuer.is_valid() {
            return Err(GrantError::InvalidIssuer);
        }
        if !self.subject.is_valid() {
            return Err(GrantError::InvalidSubject);
        }
        if self.issuer != ctx.expected_issuer {
            return Err(GrantError::IssuerMismatch);
        }
        if self.subject != ctx.expected_subject {
            return Err(GrantError::SubjectMismatch);
        }
        if self.capability.name().is_empty() {
            return Err(GrantError::EmptyCapability);
        }
        if self.capability.action_id() != self.ticket.action {
            return Err(GrantError::CapabilityMismatch);
        }
        if self.artifact != ctx.artifact {
            return Err(GrantError::ArtifactMismatch);
        }
        if self.ticket != ctx.ticket {
            return Err(GrantError::TicketMismatch);
        }
        if self.scope != ctx.declared_scope {
            return Err(GrantError::ScopeMismatch);
        }
        if self.approval_receipt == GateDigest::from_bytes([0u8; 32]) {
            return Err(GrantError::MissingApprovalReceipt);
        }
        if self.delegated_from.is_some() {
            return Err(GrantError::DelegationViolation);
        }
        Ok(())
    }

    /// Full pre-dispatch gate: identity plus typed time/revocation.
    /// Validity is `issued_at <= now < expires_at`.
    pub fn verify_for_execute(
        &self,
        ctx: &GrantDispatchContext,
        revoked_ids: &[GrantId],
    ) -> Result<(), GrantError> {
        self.verify_identity(ctx)?;
        let now = ctx.now_ms.get();
        if now < self.issued_at.get() {
            return Err(GrantError::NotYetValid);
        }
        if now >= self.expires_at.get() {
            return Err(GrantError::Expired);
        }
        if self.revocation_epoch.get() < ctx.min_epoch.get() {
            return Err(GrantError::StaleEpoch);
        }
        if revoked_ids.len() > MAX_REVOKED_IDS {
            return Err(GrantError::TooManyRevokedIds);
        }
        if revoked_ids.contains(&self.id) {
            return Err(GrantError::Revoked);
        }
        Ok(())
    }

    /// History gate: identity only. Expiry and revocation never block reads.
    pub fn verify_for_reconcile(&self, ctx: &GrantDispatchContext) -> Result<(), GrantError> {
        self.verify_identity(ctx)
    }
}
