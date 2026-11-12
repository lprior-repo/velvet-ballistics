//! Shared scoped-grant issuance for dispatch tests (bead vb-hk1j8).
//!
//! Issues a strict-exact [`ScopedGrant`] plus its [`GrantDispatchContext`]
//! for an existing register grant. Test authority is fixed, the validity
//! window is wide open, and the revocation epoch is zero, so legacy tests
//! exercise dispatch wiring without per-file lifecycle fixtures.

use vb_core::action_pack::CapabilityScopeSpec;
use vb_core::manual_gate::{ApproverId, GateDigest};
use vb_core::{
    ActionTicket, Capability, GrantDispatchContext, GrantId, GrantTimeMs, RevocationEpoch,
    ScopedGrant, ScopedGrantParams,
};
use vb_storage::AcceptedArtifact;
use vb_storage::executor::{ExecutorCatalog, RegisterGrant};

/// Issues the scoped grant and dispatch view for one register grant.
pub(crate) fn issue_scoped(
    catalog: &ExecutorCatalog,
    artifact: &AcceptedArtifact,
    grant: &RegisterGrant,
    ticket: ActionTicket,
    scope: CapabilityScopeSpec,
    grant_id: u64,
) -> Result<(ScopedGrant, GrantDispatchContext), Box<dyn std::error::Error>> {
    let issuer = ApproverId::new("test-issuer".into()).ok_or("bad test issuer")?;
    let subject = ApproverId::new("test-subject".into()).ok_or("bad test subject")?;
    let params = ScopedGrantParams {
        id: GrantId::new(grant_id),
        issuer: issuer.clone(),
        subject: subject.clone(),
        capability: Capability::new("velvet.register.write".into(), ticket.action),
        scope,
        issued_at: GrantTimeMs::new(0),
        expires_at: GrantTimeMs::new(u64::MAX),
        delegable: false,
        delegated_from: None,
        revocation_epoch: RevocationEpoch::new(0),
        break_glass: false,
        approval_receipt: GateDigest::from_bytes([0x77u8; 32]),
        signature: None,
    };
    let scoped = catalog.issue_scoped_grant(artifact, grant, params)?;
    let dispatch = GrantDispatchContext::new(
        artifact.digest,
        ticket,
        scope,
        issuer,
        subject,
        GrantTimeMs::new(1_000_000),
        RevocationEpoch::new(0),
    );
    Ok((scoped, dispatch))
}
