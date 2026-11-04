#![forbid(unsafe_code)]

//! Capability grant dispatch checks (MASTER §77.10, V1-MIN).
//!
//! Pure functions over the value types in [`vb_core::capability_grant`]:
//! grant digest computation, narrowed-token dispatch checks, and gate
//! receipt binding checks. Nothing here performs an external effect or
//! touches the journal directly; callers journal the grant as a
//! `JournalEvent::CapabilityGrantEvent`.
//!
//! Enforcement summary:
//! - A token authorizes dispatch only when it cites the grant it is
//!   checked against, names the grant's run and action, conveys a
//!   capability inside the grant's scope, and covers the requested
//!   resource.
//! - The grant and the token must both be unexpired at dispatch time,
//!   and the grant's revocation epoch must equal the current epoch.
//!   An epoch advance revokes the grant even before its expiry.
//! - A grant that cites a gate approval receipt is bound to that
//!   receipt's digest and run; the receipt authorizes the gate, never
//!   a broader capability, so receipt binding never widens a token.
//! - Replay validates structure only and never authorizes dispatch.

use vb_core::capability::Capability;
use vb_core::capability_grant::{CapabilityGrant, GrantDigest, NarrowedToken, RevocationEpoch};
use vb_core::manual_gate::ApprovalReceipt;

/// Typed capability-grant dispatch errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GrantDispatchError {
    /// Token cites a different grant than the one checked against.
    #[error(
        "token grant mismatch: token cites grant {token} but was checked against grant {expected}"
    )]
    TokenGrantMismatch {
        /// Grant the token cites.
        token: u64,
        /// Grant the token was checked against.
        expected: u64,
    },
    /// Token run does not match the grant run.
    #[error("token run mismatch: token run {token} does not match grant run {expected}")]
    TokenRunMismatch {
        /// Run the token names.
        token: u64,
        /// Run the grant names.
        expected: u64,
    },
    /// Token action does not match the grant action.
    #[error("token action mismatch: token action {token} does not match grant action {expected}")]
    TokenActionMismatch {
        /// Action the token names.
        token: u16,
        /// Action the grant names.
        expected: u16,
    },
    /// Token capability is outside the grant's capability scope.
    #[error("token capability is outside the grant scope")]
    TokenCapabilityOutsideScope,
    /// Requested resource is outside the grant's resource scope.
    #[error("requested resource is outside the grant resource scope")]
    ResourceOutsideScope,
    /// Token is expired at dispatch time.
    #[error("token expired at {now_ms}: token expiry {expires_at_ms}")]
    TokenExpired {
        /// Token expiry in milliseconds since the UNIX epoch.
        expires_at_ms: u64,
        /// Dispatch time in milliseconds since the UNIX epoch.
        now_ms: u64,
    },
    /// Grant is expired at dispatch time.
    #[error("grant expired at {now_ms}: grant expiry {expires_at_ms}")]
    GrantExpired {
        /// Grant expiry in milliseconds since the UNIX epoch.
        expires_at_ms: u64,
        /// Dispatch time in milliseconds since the UNIX epoch.
        now_ms: u64,
    },
    /// Grant revocation epoch does not equal the current epoch.
    #[error(
        "grant revoked: grant epoch {grant_epoch} does not equal current epoch {current_epoch}"
    )]
    GrantRevoked {
        /// Epoch the grant was issued under.
        grant_epoch: u64,
        /// Current revocation epoch.
        current_epoch: u64,
    },
    /// Grant fails structural validation.
    #[error("grant is structurally invalid")]
    InvalidGrant,
    /// Grant digest encoding failed (a length does not fit u64).
    #[error("grant digest encoding failed")]
    GrantDigestEncoding,
}

/// Computes the canonical digest of a grant, binding every field except
/// the signature, with domain separation. The same grant always
/// produces the same digest; any field change produces a different
/// digest. The signature is excluded because re-signing the same grant
/// must not change its identity.
pub fn compute_grant_digest(grant: &CapabilityGrant) -> Result<GrantDigest, GrantDispatchError> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"vb-capability-grant-v1");
    hasher.update(&grant.id.get().to_le_bytes());
    update_text(&mut hasher, grant.issuer.value())?;
    update_text(&mut hasher, grant.subject.value())?;
    hasher.update(&grant.artifact_digest.as_bytes());
    hasher.update(&grant.run.get().to_le_bytes());
    hasher.update(&grant.action.get().to_le_bytes());
    for capability in grant.capabilities.iter() {
        update_text(&mut hasher, capability.name())?;
        hasher.update(&capability.action_id().get().to_le_bytes());
    }
    for pattern in grant.resources.patterns.iter() {
        update_text(&mut hasher, pattern)?;
    }
    hasher.update(&grant.issued_at_ms.to_le_bytes());
    hasher.update(&grant.expires_at_ms.to_le_bytes());
    hasher.update(&[u8::from(grant.delegable)]);
    hasher.update(&grant.revocation_epoch.get().to_le_bytes());
    hasher.update(&[u8::from(grant.break_glass)]);
    match grant.approval_receipt_digest {
        Some(digest) => {
            hasher.update(&[1_u8]);
            hasher.update(&digest.as_bytes());
        }
        None => {
            hasher.update(&[0_u8]);
        }
    }
    Ok(GrantDigest::from_bytes(*hasher.finalize().as_bytes()))
}

fn update_text(hasher: &mut blake3::Hasher, text: &str) -> Result<(), GrantDispatchError> {
    let len = u64::try_from(text.len()).map_err(|_| GrantDispatchError::GrantDigestEncoding)?;
    hasher.update(&len.to_le_bytes());
    hasher.update(text.as_bytes());
    Ok(())
}

/// Returns true when the grant's cited approval receipt is exactly the
/// given receipt: same digest and same run. Receipt binding never
/// widens the grant; a grant with no cited receipt matches nothing.
#[must_use]
pub fn grant_receipt_matches(grant: &CapabilityGrant, receipt: &ApprovalReceipt) -> bool {
    grant.approval_receipt_digest == Some(receipt.digest()) && receipt.run == grant.run
}

/// Checks a narrowed token for live-effect dispatch against its grant
/// at `now_ms` under `current_epoch`, for the requested capability and
/// resource. Every failure is a distinct typed error; no failure mode
/// silently degrades to a narrower permission.
pub fn check_dispatch(
    grant: &CapabilityGrant,
    token: &NarrowedToken,
    capability: &Capability,
    resource: &str,
    now_ms: u64,
    current_epoch: RevocationEpoch,
) -> Result<(), GrantDispatchError> {
    if !grant.is_valid() {
        return Err(GrantDispatchError::InvalidGrant);
    }
    if token.grant != grant.id {
        return Err(GrantDispatchError::TokenGrantMismatch {
            token: token.grant.get(),
            expected: grant.id.get(),
        });
    }
    if token.run != grant.run {
        return Err(GrantDispatchError::TokenRunMismatch {
            token: token.run.get(),
            expected: grant.run.get(),
        });
    }
    if token.action != grant.action {
        return Err(GrantDispatchError::TokenActionMismatch {
            token: token.action.get(),
            expected: grant.action.get(),
        });
    }
    if token.capability != *capability || !grant.grants(&token.capability) {
        return Err(GrantDispatchError::TokenCapabilityOutsideScope);
    }
    if !grant.resources.covers(resource) {
        return Err(GrantDispatchError::ResourceOutsideScope);
    }
    if now_ms >= token.expires_at_ms {
        return Err(GrantDispatchError::TokenExpired {
            expires_at_ms: token.expires_at_ms,
            now_ms,
        });
    }
    if now_ms >= grant.expires_at_ms {
        return Err(GrantDispatchError::GrantExpired {
            expires_at_ms: grant.expires_at_ms,
            now_ms,
        });
    }
    if grant.revocation_epoch != current_epoch {
        return Err(GrantDispatchError::GrantRevoked {
            grant_epoch: grant.revocation_epoch.get(),
            current_epoch: current_epoch.get(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::capability_grant::{GrantId, GrantIdentity, ResourceScope, narrow_tokens};
    use vb_core::ids::{ActionId, RunId, WorkflowDigest};
    use vb_core::manual_gate::{ApproverId, GateDecision, GateDigest, GateId};

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

    fn sample_token(grant: &CapabilityGrant) -> NarrowedToken {
        narrow_tokens(grant, &[cap("network.github", 9)], 100, 50)
            .unwrap()
            .into_vec()
            .into_iter()
            .next()
            .unwrap()
    }

    fn receipt() -> ApprovalReceipt {
        ApprovalReceipt::bind(
            GateId::new(7),
            RunId::new(3),
            GateDecision::Approve,
            ApproverId::new("alice".to_string()).unwrap(),
            GateDigest::from_bytes([1; 32]),
            GateDigest::from_bytes([2; 32]),
            100,
            GateDigest::from_bytes([4; 32]),
        )
        .unwrap()
    }

    #[test]
    fn dispatch_accepts_valid_token() {
        let grant = sample_grant();
        let token = sample_token(&grant);
        assert_eq!(
            check_dispatch(
                &grant,
                &token,
                &cap("network.github", 9),
                "repo-a",
                120,
                RevocationEpoch::new(2),
            ),
            Ok(())
        );
    }

    #[test]
    fn dispatch_rejects_each_mismatch_distinctly() {
        let grant = sample_grant();
        let mut token = sample_token(&grant);
        token.grant = GrantId::new(99);
        assert!(matches!(
            check_dispatch(
                &grant,
                &token,
                &cap("network.github", 9),
                "repo-a",
                120,
                RevocationEpoch::new(2)
            ),
            Err(GrantDispatchError::TokenGrantMismatch { .. })
        ));
        let mut token = sample_token(&grant);
        token.run = RunId::new(99);
        assert!(matches!(
            check_dispatch(
                &grant,
                &token,
                &cap("network.github", 9),
                "repo-a",
                120,
                RevocationEpoch::new(2)
            ),
            Err(GrantDispatchError::TokenRunMismatch { .. })
        ));
        let mut token = sample_token(&grant);
        token.action = ActionId::new(99);
        assert!(matches!(
            check_dispatch(
                &grant,
                &token,
                &cap("network.github", 9),
                "repo-a",
                120,
                RevocationEpoch::new(2)
            ),
            Err(GrantDispatchError::TokenActionMismatch { .. })
        ));
        let token = sample_token(&grant);
        assert!(matches!(
            check_dispatch(
                &grant,
                &token,
                &cap("secrets.read", 9),
                "repo-a",
                120,
                RevocationEpoch::new(2)
            ),
            Err(GrantDispatchError::TokenCapabilityOutsideScope)
        ));
        let token = sample_token(&grant);
        assert!(matches!(
            check_dispatch(
                &grant,
                &token,
                &cap("network.github", 9),
                "repo-b",
                120,
                RevocationEpoch::new(2)
            ),
            Err(GrantDispatchError::ResourceOutsideScope)
        ));
    }

    #[test]
    fn dispatch_rejects_expired_token_grant_and_revoked_epoch() {
        let grant = sample_grant();
        let token = sample_token(&grant);
        assert!(matches!(
            check_dispatch(
                &grant,
                &token,
                &cap("network.github", 9),
                "repo-a",
                150,
                RevocationEpoch::new(2)
            ),
            Err(GrantDispatchError::TokenExpired { .. })
        ));
        let mut long_grant = sample_grant();
        long_grant.expires_at_ms = 200;
        let long_token = narrow_tokens(&long_grant, &[cap("network.github", 9)], 100, 500)
            .unwrap()
            .into_vec()
            .into_iter()
            .next()
            .unwrap();
        assert_eq!(long_token.expires_at_ms, 200);
        assert!(matches!(
            check_dispatch(
                &long_grant,
                &long_token,
                &cap("network.github", 9),
                "repo-a",
                200,
                RevocationEpoch::new(2)
            ),
            Err(GrantDispatchError::TokenExpired { .. })
        ));
        let token = sample_token(&grant);
        assert!(matches!(
            check_dispatch(
                &grant,
                &token,
                &cap("network.github", 9),
                "repo-a",
                120,
                RevocationEpoch::new(3)
            ),
            Err(GrantDispatchError::GrantRevoked { .. })
        ));
        let mut short_grant = sample_grant();
        short_grant.expires_at_ms = 200;
        let mut late_token = sample_token(&grant);
        late_token.expires_at_ms = 500;
        assert!(matches!(
            check_dispatch(
                &short_grant,
                &late_token,
                &cap("network.github", 9),
                "repo-a",
                250,
                RevocationEpoch::new(2)
            ),
            Err(GrantDispatchError::GrantExpired { .. })
        ));
        let mut invalid = sample_grant();
        invalid.expires_at_ms = invalid.issued_at_ms;
        let token = sample_token(&grant);
        assert!(matches!(
            check_dispatch(
                &invalid,
                &token,
                &cap("network.github", 9),
                "repo-a",
                120,
                RevocationEpoch::new(2)
            ),
            Err(GrantDispatchError::InvalidGrant)
        ));
    }

    #[test]
    fn grant_digest_is_stable_and_field_sensitive() {
        let grant = sample_grant();
        let first = compute_grant_digest(&grant).unwrap();
        let second = compute_grant_digest(&grant).unwrap();
        assert_eq!(first, second);
        let mut changed = sample_grant();
        changed.subject = identity("other");
        assert_ne!(first, compute_grant_digest(&changed).unwrap());
        let mut changed = sample_grant();
        changed.approval_receipt_digest = None;
        assert_ne!(first, compute_grant_digest(&changed).unwrap());
    }

    #[test]
    fn receipt_binding_matches_digest_and_run_only() {
        let grant = sample_grant();
        assert!(grant_receipt_matches(&grant, &receipt()));
        let mut other_run = receipt();
        other_run.run = RunId::new(99);
        assert!(!grant_receipt_matches(&grant, &other_run));
        let mut unbound = sample_grant();
        unbound.approval_receipt_digest = None;
        assert!(!grant_receipt_matches(&unbound, &receipt()));
    }
}
