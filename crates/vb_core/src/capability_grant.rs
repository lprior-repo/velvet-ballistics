#![forbid(unsafe_code)]

//! Capability grant lifecycle (MASTER §77.10, V1-MIN).
//!
//! Pure value types and evaluation: no I/O, no hashing, no clock reads.
//! Requirements are declared by artifacts; grants are issued by authority;
//! executors receive narrowed, short-lived tokens derived from a grant.
//!
//! A grant never mints authority on its own: admission checks the granted
//! set against the artifact's required set under the grant admission mode,
//! and dispatch re-checks the narrowed token against its grant, the
//! current revocation epoch, and the current time. A journaled grant is
//! historical evidence; replay validates its structure and never
//! authorizes a new live effect.
//!
//! Digests are supplied by callers (storage computes them); this crate
//! has no hash dependency in production code, so it never invents one.

use crate::capability::Capability;
use crate::ids::{ActionId, RunId, WorkflowDigest};
use crate::manual_gate::GateDigest;
use serde::{Deserialize, Serialize};

/// Maximum length in bytes for a grant identity or resource pattern.
pub const MAX_GRANT_TEXT_LEN: usize = 128;

/// Maximum number of capabilities in one grant scope.
pub const MAX_GRANT_CAPABILITIES: usize = 256;

/// Maximum number of resource patterns in one grant scope.
pub const MAX_GRANT_RESOURCES: usize = 256;

/// Maximum length in bytes for a grant signature.
pub const MAX_GRANT_SIGNATURE_LEN: usize = 1024;

/// Numeric capability-grant identifier.
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

/// Monotonic revocation epoch. A grant is live only while its epoch
/// equals the current epoch; any epoch advance revokes every grant
/// issued under an older epoch.
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

/// Opaque grant signature bytes, present only when the issuing profile
/// requires a signature. A grant with no signature is not invalid by
/// itself; a profile that requires signatures rejects it at its layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantSignature {
    /// Signature bytes; empty and over-long values are rejected by
    /// [`GrantSignature::new`].
    pub bytes: Box<[u8]>,
}

impl GrantSignature {
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

/// Grant admission mode (MASTER §77.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum GrantAdmissionMode {
    /// Normalized granted set must equal the required set. A missing
    /// grant or an overgrant rejects.
    StrictExact = 0,
    /// Granted set may be narrower only if the workflow can suspend at
    /// explicit missing-grant gates; otherwise reject.
    StrictNarrow = 1,
    /// Operator authority may be broader, but runtime dispatch passes
    /// only declared narrowed tokens to executors.
    ProdDefault = 2,
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
    pub signature: Option<GrantSignature>,
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

/// Replays a journaled grant: validates structure only. A replayed
/// grant is historical evidence; this function never authorizes a new
/// live effect, and dispatch must still re-check liveness, epoch, and
/// the narrowed token against the current state.
#[must_use]
pub fn replay_grant_is_structurally_valid(grant: &CapabilityGrant) -> bool {
    grant.is_valid()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(value: &str) -> GrantIdentity {
        GrantIdentity::new(value.to_string()).unwrap()
    }

    fn cap(name: &str, action: u16) -> Capability {
        Capability::new(name.into(), ActionId::new(action))
    }

    fn sample_grant() -> CapabilityGrant {
        CapabilityGrant {
            id: GrantId::new(1),
            issuer: identity("operator"),
            subject: identity("executor"),
            artifact_digest: WorkflowDigest::from_bytes([7; 32]),
            run: RunId::new(3),
            action: ActionId::new(9),
            capabilities: Box::new([cap("network.github", 9), cap("secrets.read", 9)]),
            resources: ResourceScope {
                patterns: Box::new(["repo-a".into(), "vault-a".into()]),
            },
            issued_at_ms: 100,
            expires_at_ms: 1_000,
            delegable: true,
            revocation_epoch: RevocationEpoch::new(2),
            break_glass: false,
            approval_receipt_digest: Some(GateDigest::from_bytes([4; 32])),
            signature: None,
        }
    }

    #[test]
    fn grant_id_and_epoch_round_trip() {
        assert_eq!(GrantId::new(42).get(), 42);
        assert_eq!(
            RevocationEpoch::new(2).next(),
            Some(RevocationEpoch::new(3))
        );
        assert_eq!(RevocationEpoch::new(u64::MAX).next(), None);
        assert_eq!(GrantDigest::from_bytes([9; 32]).as_bytes(), [9; 32]);
    }

    #[test]
    fn identity_validation_rejects_empty_and_overlong() {
        assert!(GrantIdentity::new(String::new()).is_none());
        assert!(GrantIdentity::new("x".repeat(MAX_GRANT_TEXT_LEN + 1)).is_none());
        assert!(identity("alice").is_valid());
    }

    #[test]
    fn signature_validation_rejects_empty_and_overlong() {
        assert!(GrantSignature::new(Box::new([])).is_none());
        assert!(
            GrantSignature::new(vec![1_u8; MAX_GRANT_SIGNATURE_LEN + 1].into_boxed_slice())
                .is_none()
        );
        assert!(GrantSignature::new(Box::new([1_u8])).unwrap().is_valid());
    }

    #[test]
    fn resource_scope_exact_match_only() {
        let scope = ResourceScope {
            patterns: Box::new(["repo-a".into()]),
        };
        assert!(scope.covers("repo-a"));
        assert!(!scope.covers("repo-ab"));
        assert!(!scope.covers("repo"));
        assert!(scope.is_valid());
    }

    #[test]
    fn grant_validity_requires_ordered_window_and_nonempty_scope() {
        let valid = sample_grant();
        assert!(valid.is_valid());
        let mut expired = sample_grant();
        expired.expires_at_ms = expired.issued_at_ms;
        assert!(!expired.is_valid());
        let mut empty = sample_grant();
        empty.capabilities = Box::new([]);
        assert!(!empty.is_valid());
        let mut bad_name = sample_grant();
        bad_name.capabilities = Box::new([cap("", 9)]);
        assert!(!bad_name.is_valid());
    }

    #[test]
    fn liveness_requires_unexpired_and_current_epoch() {
        let grant = sample_grant();
        assert!(grant.is_live(500, RevocationEpoch::new(2)));
        assert!(!grant.is_live(1_000, RevocationEpoch::new(2)));
        assert!(!grant.is_live(500, RevocationEpoch::new(3)));
    }

    #[test]
    fn strict_exact_accepts_equal_and_rejects_missing_or_overgrant() {
        let required = [cap("network.github", 9)];
        let granted = [cap("network.github", 9)];
        assert_eq!(
            evaluate_admission(GrantAdmissionMode::StrictExact, &required, &granted, false),
            AdmissionOutcome::Accepted
        );
        let missing = evaluate_admission(
            GrantAdmissionMode::StrictExact,
            &[cap("network.github", 9), cap("secrets.read", 9)],
            &granted,
            false,
        );
        assert!(matches!(missing, AdmissionOutcome::Rejected { .. }));
        let over = evaluate_admission(
            GrantAdmissionMode::StrictExact,
            &required,
            &[cap("network.github", 9), cap("secrets.read", 9)],
            false,
        );
        match over {
            AdmissionOutcome::Rejected {
                missing,
                overgranted,
            } => {
                assert!(missing.is_empty());
                assert_eq!(overgranted.len(), 1);
            }
            _ => panic!("expected rejection"),
        }
    }

    #[test]
    fn strict_narrow_suspends_only_when_suspendable_and_subset() {
        let required = [cap("network.github", 9), cap("secrets.read", 9)];
        let granted = [cap("network.github", 9)];
        match evaluate_admission(GrantAdmissionMode::StrictNarrow, &required, &granted, true) {
            AdmissionOutcome::SuspendAtMissingGrants { missing } => assert_eq!(missing.len(), 1),
            _ => panic!("expected suspension"),
        }
        assert!(matches!(
            evaluate_admission(GrantAdmissionMode::StrictNarrow, &required, &granted, false),
            AdmissionOutcome::Rejected { .. }
        ));
        assert!(matches!(
            evaluate_admission(
                GrantAdmissionMode::StrictNarrow,
                &[cap("network.github", 9)],
                &[cap("network.github", 9), cap("secrets.read", 9)],
                true,
            ),
            AdmissionOutcome::Rejected { .. }
        ));
    }

    #[test]
    fn prod_default_accepts_broad_grant_at_admission() {
        assert_eq!(
            evaluate_admission(
                GrantAdmissionMode::ProdDefault,
                &[cap("network.github", 9)],
                &[cap("network.github", 9), cap("secrets.read", 9)],
                false,
            ),
            AdmissionOutcome::Accepted
        );
    }

    #[test]
    fn narrow_tokens_cover_only_declared_and_deduplicate() {
        let grant = sample_grant();
        let tokens = narrow_tokens(
            &grant,
            &[
                cap("network.github", 9),
                cap("network.github", 9),
                cap("other", 9),
            ],
            100,
            50,
        )
        .unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].expires_at_ms, 150);
        assert_eq!(tokens[0].grant, GrantId::new(1));
        assert!(!tokens[0].break_glass);
    }

    #[test]
    fn narrow_tokens_cap_expiry_at_grant_expiry_and_reject_overflow() {
        let grant = sample_grant();
        let tokens = narrow_tokens(&grant, &[cap("network.github", 9)], 900, 500).unwrap();
        assert_eq!(tokens[0].expires_at_ms, 1_000);
        assert!(narrow_tokens(&grant, &[cap("network.github", 9)], u64::MAX, 1).is_none());
    }

    #[test]
    fn delegation_narrows_and_records_chain() {
        let grant = sample_grant();
        let delegated = grant
            .delegate(
                GrantId::new(2),
                identity("sub-executor"),
                Box::new([cap("network.github", 9)]),
                ResourceScope {
                    patterns: Box::new(["repo-a".into()]),
                },
                500,
            )
            .unwrap();
        assert_eq!(delegated.issuer.value(), "executor");
        assert_eq!(delegated.subject.value(), "sub-executor");
        assert_eq!(delegated.expires_at_ms, 500);
        assert!(delegated.signature.is_none());
        assert!(delegated.is_valid());
    }

    #[test]
    fn delegation_rejects_non_delegable_broadening_and_late_expiry() {
        let mut grant = sample_grant();
        grant.delegable = false;
        assert!(
            grant
                .delegate(
                    GrantId::new(2),
                    identity("sub"),
                    Box::new([cap("network.github", 9)]),
                    ResourceScope {
                        patterns: Box::new(["repo-a".into()]),
                    },
                    500,
                )
                .is_none()
        );
        let grant = sample_grant();
        assert!(
            grant
                .delegate(
                    GrantId::new(2),
                    identity("sub"),
                    Box::new([cap("other", 9)]),
                    ResourceScope {
                        patterns: Box::new(["repo-a".into()]),
                    },
                    500,
                )
                .is_none()
        );
        assert!(
            grant
                .delegate(
                    GrantId::new(2),
                    identity("sub"),
                    Box::new([cap("network.github", 9)]),
                    ResourceScope {
                        patterns: Box::new(["repo-a".into()]),
                    },
                    2_000,
                )
                .is_none()
        );
    }

    #[test]
    fn replay_validates_structure_only() {
        assert!(replay_grant_is_structurally_valid(&sample_grant()));
        let mut invalid = sample_grant();
        invalid.expires_at_ms = invalid.issued_at_ms;
        assert!(!replay_grant_is_structurally_valid(&invalid));
    }
}
