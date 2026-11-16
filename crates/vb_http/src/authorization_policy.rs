//! Pure declaration checks, not authorization, dispatch permission or durable evidence.
//! All policy/time/revocation inputs are caller-supplied and remain untrusted here.

use crate::{RequestRejection, WorkflowCountAction};
use std::num::NonZeroU64;
use vb_core::action_pack::RequestDigest;
use vb_core::capability_grant::{GrantId, GrantTimeMs, RevocationEpoch, MAX_REVOKED_IDS};
use vb_core::manual_gate::ApproverId;
use vb_core::{ActionId, WorkflowDigest};

/// Untrusted versioned declaration. Constructing this value grants nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpDeclarationV1 {
    pub version: u16,
    pub id: GrantId,
    pub artifact: WorkflowDigest,
    pub action: ActionId,
    pub request: RequestDigest,
    pub issuer: ApproverId,
    pub subject: ApproverId,
    pub issued_at: GrantTimeMs,
    pub expires_at: GrantTimeMs,
    pub revocation_epoch: RevocationEpoch,
}

/// Explicit expected values; not an issuer or proof that these values are authorized.
#[derive(Debug, Clone)]
pub struct HttpRequestPolicy {
    pub artifact: WorkflowDigest,
    pub action: ActionId,
    pub request: RequestDigest,
    pub expected_issuer: ApproverId,
    pub expected_subject: ApproverId,
    pub now: GrantTimeMs,
    pub min_epoch: RevocationEpoch,
    pub max_lifetime_ms: NonZeroU64,
}

/// A declaration matched one supplied policy snapshot. Never an execution token.
/// No signature, issuer authority, durable intent or persistence was verified.
#[derive(Debug)]
pub struct ValidatedHttpDeclaration {
    declaration: HttpDeclarationV1,
    checked_at: GrantTimeMs,
}

impl ValidatedHttpDeclaration {
    pub fn check(
        declaration: HttpDeclarationV1,
        policy: &HttpRequestPolicy,
        action: &WorkflowCountAction,
        revoked_ids: &[GrantId],
    ) -> Result<Self, DeclarationRejection> {
        check_identity(&declaration, policy)?;
        check_liveness(&declaration, policy, revoked_ids)?;
        if action.request_digest()? != declaration.request {
            return Err(DeclarationRejection::Request);
        }
        Ok(Self { declaration, checked_at: policy.now })
    }

    pub const fn declaration(&self) -> &HttpDeclarationV1 {
        &self.declaration
    }

    pub const fn checked_at(&self) -> GrantTimeMs {
        self.checked_at
    }
}

fn check_identity(d: &HttpDeclarationV1, p: &HttpRequestPolicy) -> Result<(), DeclarationRejection> {
    if d.version != 1 {
        return Err(DeclarationRejection::Version);
    }
    if !d.issuer.is_valid() || !d.subject.is_valid()
        || !p.expected_issuer.is_valid() || !p.expected_subject.is_valid()
    {
        return Err(DeclarationRejection::Identity);
    }
    if d.artifact != p.artifact || d.action != p.action {
        return Err(DeclarationRejection::Invocation);
    }
    if d.issuer != p.expected_issuer || d.subject != p.expected_subject {
        return Err(DeclarationRejection::Identity);
    }
    if d.request != p.request {
        return Err(DeclarationRejection::Request);
    }
    Ok(())
}

fn check_liveness(
    d: &HttpDeclarationV1,
    p: &HttpRequestPolicy,
    revoked: &[GrantId],
) -> Result<(), DeclarationRejection> {
    match d.expires_at.get().checked_sub(d.issued_at.get()) {
        Some(duration) if duration > 0 && duration <= p.max_lifetime_ms.get() => {}
        _ => return Err(DeclarationRejection::Window),
    }
    if p.now < d.issued_at || p.now >= d.expires_at {
        return Err(DeclarationRejection::Time);
    }
    if d.revocation_epoch < p.min_epoch {
        return Err(DeclarationRejection::Epoch);
    }
    if revoked.len() > MAX_REVOKED_IDS {
        return Err(DeclarationRejection::RevocationLimit);
    }
    if revoked.contains(&d.id) {
        return Err(DeclarationRejection::Revoked);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeclarationRejection {
    #[error("unsupported HTTP declaration version")]
    Version,
    #[error("HTTP declaration invocation mismatch")]
    Invocation,
    #[error("HTTP declaration identity invalid or mismatched")]
    Identity,
    #[error("HTTP declaration request mismatch")]
    Request,
    #[error("HTTP declaration validity window invalid or over limit")]
    Window,
    #[error("HTTP declaration is not current at supplied time")]
    Time,
    #[error("HTTP declaration revocation epoch is stale")]
    Epoch,
    #[error("HTTP declaration revocation view exceeds bound")]
    RevocationLimit,
    #[error("HTTP declaration is revoked")]
    Revoked,
    #[error(transparent)]
    RequestEncoding(#[from] RequestRejection),
}

#[cfg(test)]
#[path = "authorization_policy_tests.rs"]
mod tests;
