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
use crate::ids::{ActionId, RunId};
use crate::manual_gate::{ApproverId, GateDigest};
use crate::{ActionTicket, WorkflowDigest};
use serde::{Deserialize, Serialize};

/// Maximum revoked grant ids accepted in one dispatch view.
pub const MAX_REVOKED_IDS: usize = 1024;

/// Maximum length in bytes for a grant identity or resource pattern.
pub const MAX_GRANT_TEXT_LEN: usize = 128;

/// Maximum number of capabilities in one grant scope.
pub const MAX_GRANT_CAPABILITIES: usize = 256;

/// Maximum number of resource patterns in one grant scope.
pub const MAX_GRANT_RESOURCES: usize = 256;

/// Maximum length in bytes for a grant signature.
pub const MAX_GRANT_SIGNATURE_LEN: usize = 1024;

/// Numeric grant identifier, in the style of `GateId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

    /// Returns the next epoch, or `None` when the addition overflows.
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(next) => Some(Self(next)),
            None => None,
        }
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

/// Opaque grant signature bytes, present only when the issuing profile
/// requires a signature. A grant with no signature is not invalid by
/// itself; a profile that requires signatures rejects it at its layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoricalGrantSignature {
    /// Signature bytes; empty and over-long values are rejected by
    /// [`HistoricalGrantSignature::new`].
    pub bytes: Box<[u8]>,
}

impl HistoricalGrantSignature {
    /// Creates a signature, or `None` when the bytes are empty or over-long.
    #[must_use]
    pub fn new(bytes: Box<[u8]>) -> Option<Self> {
        if bytes.is_empty() || bytes.len() > MAX_GRANT_SIGNATURE_LEN {
            None
        } else {
            Some(Self { bytes })
        }
    }

    /// Returns true when the bytes are non-empty and bounded.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.bytes.is_empty() && self.bytes.len() <= MAX_GRANT_SIGNATURE_LEN
    }
}

/// Admission mode. This wedge implements `StrictExact` only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum GrantAdmissionMode {
    /// Granted scope must equal the declared scope; overgrant rejects.
    StrictExact = 0,
    /// Granted set may be narrower only if the workflow can suspend at
    /// explicit missing-grant gates; otherwise reject.
    StrictNarrow = 1,
    /// Operator authority may be broader, but runtime dispatch passes
    /// only declared narrowed tokens to executors.
    ProdDefault = 2,
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

// Journaled grant value types (MASTER §77.10, V1-MIN): pure value types
// and evaluation recovered from the reviewed §77.10 batch. A journaled
// grant carries historical evidence only; replay validates structure
// and never authorizes a new live effect.

/// 32-byte capability-grant digest, in the style of
/// [`crate::manual_gate::GateDigest`]. Callers supply the actual digest
/// of the grant they computed; never fabricated or default-filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct GrantDigest([u8; 32]);

impl GrantDigest {
    /// Creates a digest from verified bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Identity of a grant issuer or subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantIdentity {
    /// Identity string; empty and over-long values are rejected by
    /// [`GrantIdentity::new`].
    pub value: String,
}

impl GrantIdentity {
    /// Creates an identity, or `None` when the value is empty or over-long.
    #[must_use]
    pub fn new(value: String) -> Option<Self> {
        if value.is_empty() || value.len() > MAX_GRANT_TEXT_LEN {
            None
        } else {
            Some(Self { value })
        }
    }

    /// Returns the identity string.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns true when the value is non-empty and bounded.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.value.is_empty() && self.value.len() <= MAX_GRANT_TEXT_LEN
    }
}

/// Resource scope of a grant: exact-match resource patterns only.
/// Patterns are matched exactly, never expanded as globs, so a grant
/// cannot silently cover a resource it did not name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceScope {
    /// Resource patterns covered by the grant.
    pub patterns: Box<[Box<str>]>,
}

impl ResourceScope {
    /// Returns true when the scope covers the resource by exact match.
    #[must_use]
    pub fn covers(&self, resource: &str) -> bool {
        self.patterns
            .iter()
            .any(|pattern| pattern.as_ref() == resource)
    }

    /// Returns true when every pattern of `other` is covered by this scope.
    #[must_use]
    pub fn covers_scope(&self, other: &ResourceScope) -> bool {
        other.patterns.iter().all(|pattern| self.covers(pattern))
    }

    /// Returns true when every pattern is non-empty and bounded and the
    /// pattern count is bounded.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.patterns.len() <= MAX_GRANT_RESOURCES
            && self
                .patterns
                .iter()
                .all(|pattern| !pattern.is_empty() && pattern.len() <= MAX_GRANT_TEXT_LEN)
    }
}

/// Capability grant (MASTER §77.10 required fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityGrant {
    /// Grant identifier.
    pub id: GrantId,
    /// Issuing authority.
    pub issuer: GrantIdentity,
    /// Subject the grant is issued to.
    pub subject: GrantIdentity,
    /// Digest of the artifact the grant is issued for.
    pub artifact_digest: WorkflowDigest,
    /// Run the grant is issued for.
    pub run: RunId,
    /// Action the grant covers.
    pub action: ActionId,
    /// Capability scope: capabilities granted, non-empty and bounded.
    pub capabilities: Box<[Capability]>,
    /// Resource scope covered by the grant.
    pub resources: ResourceScope,
    /// Issue time in milliseconds since the UNIX epoch.
    pub issued_at_ms: u64,
    /// Expiry time in milliseconds since the UNIX epoch; must exceed
    /// the issue time.
    pub expires_at_ms: u64,
    /// Whether the subject may delegate a narrowed grant onward.
    pub delegable: bool,
    /// Revocation epoch at issuance; the grant is live only while this
    /// equals the current epoch.
    pub revocation_epoch: RevocationEpoch,
    /// Whether the grant was issued under break-glass authority.
    pub break_glass: bool,
    /// Digest of the gate approval receipt backing this grant, when the
    /// grant cites a gate approval. `None` only for grants issued
    /// without a gate approval; a grant that claims gate backing must
    /// carry `Some`.
    pub approval_receipt_digest: Option<GateDigest>,
    /// Signature, present only when the issuing profile requires one.
    pub signature: Option<HistoricalGrantSignature>,
}

impl CapabilityGrant {
    /// Returns true when the grant's capability scope covers the
    /// capability by exact name and action.
    #[must_use]
    pub fn grants(&self, capability: &Capability) -> bool {
        self.capabilities.iter().any(|granted| {
            granted.name() == capability.name() && granted.action_id() == capability.action_id()
        })
    }

    /// Returns true when the grant's capability scope covers every
    /// capability in `required`.
    #[must_use]
    pub fn covers_all(&self, required: &[Capability]) -> bool {
        required.iter().all(|capability| self.grants(capability))
    }

    /// Returns true when the grant is live at `now_ms` under
    /// `current_epoch`: unexpired and unrevoked. Liveness alone does
    /// not authorize dispatch; dispatch also checks the narrowed token.
    #[must_use]
    pub fn is_live(&self, now_ms: u64, current_epoch: RevocationEpoch) -> bool {
        now_ms < self.expires_at_ms && self.revocation_epoch == current_epoch
    }

    /// Checks the structural invariants of the grant: valid identities,
    /// non-empty bounded capability and resource scopes, valid
    /// signature when present, and expiry strictly after issue.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issuer.is_valid()
            && self.subject.is_valid()
            && !self.capabilities.is_empty()
            && self.capabilities.len() <= MAX_GRANT_CAPABILITIES
            && self
                .capabilities
                .iter()
                .all(|capability| !capability.name().is_empty())
            && self.resources.is_valid()
            && self
                .signature
                .as_ref()
                .is_none_or(|signature| signature.is_valid())
            && self.issued_at_ms < self.expires_at_ms
    }

    /// Derives a delegated grant for a new subject, or `None` when this
    /// grant is not delegable, the narrowed capabilities or resources
    /// exceed this grant's scopes, or the new expiry exceeds this
    /// grant's expiry. The delegation chain is recorded by issuer: the
    /// delegating subject becomes the new grant's issuer. A delegated
    /// grant carries no signature; a profile that requires signatures
    /// must re-sign it before it is usable there.
    #[must_use]
    pub fn delegate(
        &self,
        new_id: GrantId,
        new_subject: GrantIdentity,
        new_capabilities: Box<[Capability]>,
        new_resources: ResourceScope,
        new_expires_at_ms: u64,
    ) -> Option<Self> {
        if !self.delegable {
            return None;
        }
        if new_capabilities.is_empty()
            || !new_capabilities
                .iter()
                .all(|capability| self.grants(capability))
        {
            return None;
        }
        if !self.resources.covers_scope(&new_resources) {
            return None;
        }
        if new_expires_at_ms <= self.issued_at_ms || new_expires_at_ms > self.expires_at_ms {
            return None;
        }
        if !new_subject.is_valid() || !new_resources.is_valid() {
            return None;
        }
        Some(Self {
            id: new_id,
            issuer: self.subject.clone(),
            subject: new_subject,
            artifact_digest: self.artifact_digest,
            run: self.run,
            action: self.action,
            capabilities: new_capabilities,
            resources: new_resources,
            issued_at_ms: self.issued_at_ms,
            expires_at_ms: new_expires_at_ms,
            delegable: self.delegable,
            revocation_epoch: self.revocation_epoch,
            break_glass: self.break_glass,
            approval_receipt_digest: self.approval_receipt_digest,
            signature: None,
        })
    }
}

/// Outcome of a grant admission check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionOutcome {
    /// The granted set satisfies the required set under the mode.
    Accepted,
    /// Under `strict_narrow`, the granted set is a subset of the
    /// required set and the workflow can suspend at explicit
    /// missing-grant gates until the missing grants arrive.
    SuspendAtMissingGrants {
        /// Required capabilities with no covering grant.
        missing: Box<[Capability]>,
    },
    /// The granted set does not satisfy the required set under the mode.
    Rejected {
        /// Required capabilities with no covering grant.
        missing: Box<[Capability]>,
        /// Granted capabilities no requirement declares.
        overgranted: Box<[Capability]>,
    },
}

fn capability_covered(capability: &Capability, set: &[Capability]) -> bool {
    set.iter().any(|granted| {
        granted.name() == capability.name() && granted.action_id() == capability.action_id()
    })
}

fn missing_capabilities(required: &[Capability], granted: &[Capability]) -> Box<[Capability]> {
    required
        .iter()
        .filter(|capability| !capability_covered(capability, granted))
        .cloned()
        .collect::<Vec<Capability>>()
        .into_boxed_slice()
}

fn overgranted_capabilities(required: &[Capability], granted: &[Capability]) -> Box<[Capability]> {
    let mut overgranted: Vec<Capability> = Vec::new();
    for capability in granted {
        if !capability_covered(capability, required)
            && !capability_covered(capability, &overgranted)
        {
            overgranted.push(capability.clone());
        }
    }
    overgranted.into_boxed_slice()
}

/// Checks a granted capability set against an artifact's required set
/// under the grant admission mode. Human approval of a gate decision
/// never enters this check: approval authorizes a gate, it does not
/// broaden a grant, so an overgrant rejects under the strict modes
/// regardless of any receipt the grant cites.
#[must_use]
pub fn evaluate_admission(
    mode: GrantAdmissionMode,
    required: &[Capability],
    granted: &[Capability],
    suspendable_at_missing_grants: bool,
) -> AdmissionOutcome {
    let missing = missing_capabilities(required, granted);
    let overgranted = overgranted_capabilities(required, granted);
    match mode {
        GrantAdmissionMode::StrictExact => {
            if missing.is_empty() && overgranted.is_empty() {
                AdmissionOutcome::Accepted
            } else {
                AdmissionOutcome::Rejected {
                    missing,
                    overgranted,
                }
            }
        }
        GrantAdmissionMode::StrictNarrow => {
            if overgranted.is_empty() && missing.is_empty() {
                AdmissionOutcome::Accepted
            } else if overgranted.is_empty() && suspendable_at_missing_grants {
                AdmissionOutcome::SuspendAtMissingGrants { missing }
            } else {
                AdmissionOutcome::Rejected {
                    missing,
                    overgranted,
                }
            }
        }
        GrantAdmissionMode::ProdDefault => AdmissionOutcome::Accepted,
    }
}

/// Narrowed, short-lived token handed to an executor. A token names
/// exactly one capability of one grant for one run and action; it
/// carries no resource scope beyond the grant it cites, and dispatch
/// re-validates it against that grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NarrowedToken {
    /// Grant the token narrows.
    pub grant: GrantId,
    /// Run the token is valid for.
    pub run: RunId,
    /// Action the token is valid for.
    pub action: ActionId,
    /// The single capability this token conveys.
    pub capability: Capability,
    /// Token expiry in milliseconds since the UNIX epoch; never later
    /// than the grant's expiry.
    pub expires_at_ms: u64,
    /// Whether the token was derived from a break-glass grant.
    pub break_glass: bool,
}

/// Narrows a grant to tokens for exactly the declared required
/// capabilities it covers, each expiring at the earlier of the grant
/// expiry and `now_ms + ttl_ms`, or `None` when that addition
/// overflows. Capabilities the grant does not cover produce no token:
/// under `prod_default` an operator-broad grant still hands executors
/// only these declared tokens.
#[must_use]
pub fn narrow_tokens(
    grant: &CapabilityGrant,
    required: &[Capability],
    now_ms: u64,
    ttl_ms: u64,
) -> Option<Box<[NarrowedToken]>> {
    let token_deadline = now_ms.checked_add(ttl_ms)?.min(grant.expires_at_ms);
    let mut tokens: Vec<NarrowedToken> = Vec::new();
    for capability in required {
        if grant.grants(capability) && !tokens.iter().any(|token| token.capability == *capability) {
            tokens.push(NarrowedToken {
                grant: grant.id,
                run: grant.run,
                action: grant.action,
                capability: capability.clone(),
                expires_at_ms: token_deadline,
                break_glass: grant.break_glass,
            });
        }
    }
    Some(tokens.into_boxed_slice())
}

#[cfg(test)]
#[path = "capability_grant_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "capability_grant_signature_tests.rs"]
mod signature_tests;
