#![forbid(unsafe_code)]
//! Scoped capability grant lifecycle (MASTER §77.10, `strict_exact` wedge).
//!
//! A [`ScopedGrant`] is a trusted authority-bound value checked at the
//! admission/dispatch boundary before any live effect. It binds grant id,
//! issuer, subject, artifact, ticket (run/action), capability and resource
//! scopes, issue/expiry typed time, delegability, revocation epoch,
//! break-glass marker, and approval receipt digest. Signatures are recorded
//! but verified only by an opted-in profile (follow-up batch).
//!
//! Time and revocation arrive as explicit typed dispatch inputs; this module
//! never reads a clock or the network. [`ScopedGrant::verify_for_execute`]
//! enforces identity plus liveness before live dispatch, while
//! [`ScopedGrant::verify_for_reconcile`] enforces identity only so
//! historical evidence stays readable after expiry or revocation.

use crate::action::action_ticket_has_valid_key;
use crate::action_pack::CapabilityScopeSpec;
use crate::capability::Capability;
use crate::manual_gate::{ApproverId, GateDigest};
use crate::{ActionTicket, WorkflowDigest};
use serde::{Deserialize, Serialize};

/// Maximum revoked grant ids accepted in one dispatch view.
pub const MAX_REVOKED_IDS: usize = 1024;

/// Numeric grant identifier, in the style of `GateId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct GrantId(u64);

impl GrantId {
    /// Creates a grant identifier from a validated integer.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw identifier value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Typed milliseconds since the UNIX epoch for grant windows.
/// Comparisons only, never arithmetic, so no overflow is possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct GrantTimeMs(u64);

impl GrantTimeMs {
    /// Creates a typed timestamp from validated milliseconds.
    #[must_use]
    pub const fn new(millis: u64) -> Self {
        Self(millis)
    }

    /// Returns the raw millisecond value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Monotonic revocation epoch observed at issuance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct RevocationEpoch(u64);

impl RevocationEpoch {
    /// Creates a revocation epoch from a validated integer.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw epoch value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Opaque approval signature bytes. Recorded, never verified in this wedge.
/// Stored as two halves because serde derives cover arrays only to 32.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GrantSignature {
    high: [u8; 32],
    low: [u8; 32],
}

impl GrantSignature {
    /// Wraps verified signature bytes; this does not establish authority.
    #[must_use]
    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        let mut high = [0u8; 32];
        let mut low = [0u8; 32];
        let mut source = bytes.into_iter();
        for slot in high.iter_mut().chain(low.iter_mut()) {
            if let Some(byte) = source.next() {
                *slot = byte;
            }
        }
        Self { high, low }
    }

    /// Returns the raw signature bytes.
    #[must_use]
    pub fn as_bytes(self) -> [u8; 64] {
        let mut out = [0u8; 64];
        let mut source = [self.high, self.low].into_iter().flatten();
        for slot in out.iter_mut() {
            if let Some(byte) = source.next() {
                *slot = byte;
            }
        }
        out
    }
}

/// Admission mode. This wedge implements `StrictExact` only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum GrantAdmissionMode {
    /// Granted scope must equal the declared scope; overgrant rejects.
    StrictExact = 0,
}

/// Trusted scoped grant binding authority, invocation, scope, and time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopedGrant {
    /// Grant identifier.
    pub id: GrantId,
    /// Authority that issued the grant.
    pub issuer: ApproverId,
    /// Party authorized to use the grant.
    pub subject: ApproverId,
    /// Artifact digest the grant is bound to.
    pub artifact: WorkflowDigest,
    /// Exact ticket (run/step/seq/action/attempts/key) the grant is bound to.
    pub ticket: ActionTicket,
    /// Capability (name and action) the grant authorizes.
    pub capability: Capability,
    /// Exact resource scope the grant authorizes.
    pub scope: CapabilityScopeSpec,
    /// Admission mode the grant was issued under.
    pub mode: GrantAdmissionMode,
    /// Start of the validity window, inclusive.
    pub issued_at: GrantTimeMs,
    /// End of the validity window, exclusive.
    pub expires_at: GrantTimeMs,
    /// Whether the grantee may delegate; chains are rejected in this wedge.
    pub delegable: bool,
    /// Parent grant for delegated issuance; always `None` in this wedge.
    pub delegated_from: Option<GrantId>,
    /// Revocation epoch observed at issuance.
    pub revocation_epoch: RevocationEpoch,
    /// Emergency break-glass marker; receipt still required.
    pub break_glass: bool,
    /// Digest of the approval receipt authorizing this grant.
    pub approval_receipt: GateDigest,
    /// Opted-in profile signature; recorded only in this wedge.
    pub signature: Option<GrantSignature>,
}

/// Single constructor bundle for issuance (never N primitive arguments).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopedGrantParams {
    /// Grant identifier.
    pub id: GrantId,
    /// Authority that issues the grant.
    pub issuer: ApproverId,
    /// Party authorized to use the grant.
    pub subject: ApproverId,
    /// Capability (name and action) the grant authorizes.
    pub capability: Capability,
    /// Exact resource scope the grant authorizes.
    pub scope: CapabilityScopeSpec,
    /// Start of the validity window, inclusive.
    pub issued_at: GrantTimeMs,
    /// End of the validity window, exclusive.
    pub expires_at: GrantTimeMs,
    /// Whether the grantee may delegate; chains are rejected in this wedge.
    pub delegable: bool,
    /// Parent grant for delegated issuance; always `None` in this wedge.
    pub delegated_from: Option<GrantId>,
    /// Revocation epoch observed at issuance.
    pub revocation_epoch: RevocationEpoch,
    /// Emergency break-glass marker; receipt still required.
    pub break_glass: bool,
    /// Digest of the approval receipt authorizing this grant.
    pub approval_receipt: GateDigest,
    /// Opted-in profile signature; recorded only in this wedge.
    pub signature: Option<GrantSignature>,
}

/// Single dispatch bundle: expected bindings plus typed time/revocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantDispatchContext {
    /// Artifact digest the live invocation was admitted under.
    pub artifact: WorkflowDigest,
    /// Canonical pending ticket the live invocation carries.
    pub ticket: ActionTicket,
    /// Declared pack scope the executor may receive.
    pub declared_scope: CapabilityScopeSpec,
    /// Issuer the dispatcher expects.
    pub expected_issuer: ApproverId,
    /// Subject the dispatcher expects.
    pub expected_subject: ApproverId,
    /// Dispatch time in milliseconds; caller-owned, never clocked here.
    pub now_ms: GrantTimeMs,
    /// Minimum live revocation epoch; caller-owned revocation view.
    pub min_epoch: RevocationEpoch,
}

/// Borrowed presentation for one live execution: the scoped grant, the
/// expected dispatch view, and the caller-owned revocation list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopedDispatch<'a> {
    /// Scoped grant presented for this dispatch.
    pub scoped: &'a ScopedGrant,
    /// Expected bindings plus typed time/revocation.
    pub dispatch: &'a GrantDispatchContext,
    /// Caller-owned revocation list, bounded by [`MAX_REVOKED_IDS`].
    pub revoked_ids: &'a [GrantId],
}

impl<'a> ScopedDispatch<'a> {
    /// Bundles one presentation for live dispatch.
    #[must_use]
    pub const fn new(
        scoped: &'a ScopedGrant,
        dispatch: &'a GrantDispatchContext,
        revoked_ids: &'a [GrantId],
    ) -> Self {
        Self {
            scoped,
            dispatch,
            revoked_ids,
        }
    }
}

impl GrantDispatchContext {
    /// Bundles the expected bindings with caller-owned time/revocation.
    #[must_use]
    pub fn new(
        artifact: WorkflowDigest,
        ticket: ActionTicket,
        declared_scope: CapabilityScopeSpec,
        expected_issuer: ApproverId,
        expected_subject: ApproverId,
        now_ms: GrantTimeMs,
        min_epoch: RevocationEpoch,
    ) -> Self {
        Self {
            artifact,
            ticket,
            declared_scope,
            expected_issuer,
            expected_subject,
            now_ms,
            min_epoch,
        }
    }
}

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

#[cfg(test)]
#[path = "capability_grant_tests.rs"]
mod tests;

pub use historical::{
    CapabilityGrant, GrantDigest, GrantIdentity, HistoricalGrantSignature, MAX_GRANT_CAPABILITIES,
    MAX_GRANT_RESOURCES, MAX_GRANT_SIGNATURE_LEN, MAX_GRANT_TEXT_LEN, NarrowedToken, ResourceScope,
};
mod historical;
