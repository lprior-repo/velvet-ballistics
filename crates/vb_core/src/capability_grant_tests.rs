//! Unit tests for the scoped grant lifecycle (`strict_exact` wedge).

use super::*;
use crate::action::compute_action_idempotency_key;
use crate::ids::{ActionId, RunId, SeqNo, StepIdx};

fn issuer() -> ApproverId {
    ApproverId::new("grant-issuer".into()).unwrap()
}

fn subject() -> ApproverId {
    ApproverId::new("run-subject".into()).unwrap()
}

fn artifact() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0x11u8; 32])
}

fn scope() -> CapabilityScopeSpec {
    CapabilityScopeSpec {
        namespace: 15,
        key: 29,
    }
}

fn capability() -> Capability {
    Capability::new("velvet.register.write".into(), ActionId::new(0))
}

fn receipt() -> GateDigest {
    GateDigest::from_bytes([0x22u8; 32])
}

fn ticket() -> ActionTicket {
    let run = RunId::new(7);
    let seq = SeqNo::new(3);
    let action = ActionId::new(0);
    ActionTicket {
        run,
        step: StepIdx::new(1),
        seq,
        action,
        run_attempt: 1,
        attempt: 1,
        idempotency_key: compute_action_idempotency_key(run, 1, seq, action),
        capacity: 2,
    }
}

fn params() -> ScopedGrantParams {
    ScopedGrantParams {
        id: GrantId::new(9),
        issuer: issuer(),
        subject: subject(),
        capability: capability(),
        scope: scope(),
        issued_at: GrantTimeMs::new(1_000),
        expires_at: GrantTimeMs::new(2_000),
        delegable: false,
        delegated_from: None,
        revocation_epoch: RevocationEpoch::new(1),
        break_glass: false,
        approval_receipt: receipt(),
        signature: None,
    }
}

fn grant() -> ScopedGrant {
    ScopedGrant::issue(params(), artifact(), ticket()).unwrap()
}

fn ctx() -> GrantDispatchContext {
    GrantDispatchContext::new(
        artifact(),
        ticket(),
        scope(),
        issuer(),
        subject(),
        GrantTimeMs::new(1_500),
        RevocationEpoch::new(1),
    )
}

#[test]
fn issue_valid_grant_records_bindings() {
    let grant = grant();
    assert_eq!(grant.id, GrantId::new(9));
    assert_eq!(grant.issuer, issuer());
    assert_eq!(grant.subject, subject());
    assert_eq!(grant.artifact, artifact());
    assert_eq!(grant.ticket, ticket());
    assert_eq!(grant.capability, capability());
    assert_eq!(grant.scope, scope());
    assert!(matches!(grant.mode, GrantAdmissionMode::StrictExact));
    assert_eq!(grant.revocation_epoch, RevocationEpoch::new(1));
    assert!(!grant.break_glass);
    assert_eq!(grant.approval_receipt, receipt());
}

#[test]
fn issue_rejects_empty_window() {
    let mut empty = params();
    empty.issued_at = GrantTimeMs::new(2_000);
    empty.expires_at = GrantTimeMs::new(2_000);
    assert_eq!(
        ScopedGrant::issue(empty, artifact(), ticket()),
        Err(GrantError::EmptyWindow)
    );

    let mut inverted = params();
    inverted.issued_at = GrantTimeMs::new(3_000);
    inverted.expires_at = GrantTimeMs::new(2_000);
    assert_eq!(
        ScopedGrant::issue(inverted, artifact(), ticket()),
        Err(GrantError::EmptyWindow)
    );
}

#[test]
fn issue_rejects_missing_receipt() {
    let mut missing = params();
    missing.approval_receipt = GateDigest::from_bytes([0u8; 32]);
    assert_eq!(
        ScopedGrant::issue(missing, artifact(), ticket()),
        Err(GrantError::MissingApprovalReceipt)
    );
}

#[test]
fn issue_rejects_chained_issuance() {
    let mut chained = params();
    chained.delegated_from = Some(GrantId::new(1));
    assert_eq!(
        ScopedGrant::issue(chained, artifact(), ticket()),
        Err(GrantError::DelegationViolation)
    );
}

#[test]
fn issue_rejects_capability_action_mismatch() {
    let mut mismatched = params();
    mismatched.capability = Capability::new("velvet.register.write".into(), ActionId::new(4));
    assert_eq!(
        ScopedGrant::issue(mismatched, artifact(), ticket()),
        Err(GrantError::CapabilityMismatch)
    );
}

#[test]
fn issue_rejects_bad_ticket() {
    let mut bad_key = ticket();
    bad_key.idempotency_key = 0;
    assert_eq!(
        ScopedGrant::issue(params(), artifact(), bad_key),
        Err(GrantError::InvalidTicket)
    );

    let mut no_attempt = ticket();
    no_attempt.attempt = 0;
    assert_eq!(
        ScopedGrant::issue(params(), artifact(), no_attempt),
        Err(GrantError::InvalidTicket)
    );
}

#[test]
fn execute_ok_inside_window_and_at_issue_bound() {
    let grant = grant();
    assert!(grant.verify_for_execute(&ctx(), &[]).is_ok());

    let mut at_issue = ctx();
    at_issue.now_ms = GrantTimeMs::new(1_000);
    assert!(grant.verify_for_execute(&at_issue, &[]).is_ok());
}

#[test]
fn execute_rejects_expiry_bound_and_outside_window() {
    let grant = grant();

    let mut at_expiry = ctx();
    at_expiry.now_ms = GrantTimeMs::new(2_000);
    assert_eq!(
        grant.verify_for_execute(&at_expiry, &[]),
        Err(GrantError::Expired)
    );

    let mut early = ctx();
    early.now_ms = GrantTimeMs::new(999);
    assert_eq!(
        grant.verify_for_execute(&early, &[]),
        Err(GrantError::NotYetValid)
    );
}

#[test]
fn execute_rejects_each_binding_mismatch() {
    let grant = grant();

    let mut wrong_issuer = ctx();
    wrong_issuer.expected_issuer = ApproverId::new("other-issuer".into()).unwrap();
    assert_eq!(
        grant.verify_for_execute(&wrong_issuer, &[]),
        Err(GrantError::IssuerMismatch)
    );

    let mut wrong_subject = ctx();
    wrong_subject.expected_subject = ApproverId::new("other-subject".into()).unwrap();
    assert_eq!(
        grant.verify_for_execute(&wrong_subject, &[]),
        Err(GrantError::SubjectMismatch)
    );

    let mut wrong_artifact = ctx();
    wrong_artifact.artifact = WorkflowDigest::from_bytes([0x33u8; 32]);
    assert_eq!(
        grant.verify_for_execute(&wrong_artifact, &[]),
        Err(GrantError::ArtifactMismatch)
    );

    let mut wrong_ticket = ctx();
    let mut other = ticket();
    other.seq = SeqNo::new(4);
    wrong_ticket.ticket = other;
    assert_eq!(
        grant.verify_for_execute(&wrong_ticket, &[]),
        Err(GrantError::TicketMismatch)
    );

    // Approval for scope A cannot broaden to scope B.
    let mut broad_scope = ctx();
    broad_scope.declared_scope = CapabilityScopeSpec {
        namespace: 15,
        key: 30,
    };
    assert_eq!(
        grant.verify_for_execute(&broad_scope, &[]),
        Err(GrantError::ScopeMismatch)
    );
}

#[test]
fn execute_rejects_revocation_epoch_and_revoked_id() {
    let grant = grant();

    let mut future_epoch = ctx();
    future_epoch.min_epoch = RevocationEpoch::new(2);
    assert_eq!(
        grant.verify_for_execute(&future_epoch, &[]),
        Err(GrantError::StaleEpoch)
    );

    assert_eq!(
        grant.verify_for_execute(&ctx(), &[GrantId::new(9)]),
        Err(GrantError::Revoked)
    );
    assert!(grant.verify_for_execute(&ctx(), &[GrantId::new(8)]).is_ok());
}

#[test]
fn execute_rejects_oversized_revocation_view() {
    let grant = grant();
    let oversized = vec![GrantId::new(0); MAX_REVOKED_IDS + 1];
    assert_eq!(
        grant.verify_for_execute(&ctx(), &oversized),
        Err(GrantError::TooManyRevokedIds)
    );
}

#[test]
fn reconcile_ignores_expiry_and_revocation() {
    let grant = grant();

    let mut expired = ctx();
    expired.now_ms = GrantTimeMs::new(9_999);
    expired.min_epoch = RevocationEpoch::new(99);
    assert!(grant.verify_for_reconcile(&expired).is_ok());
    assert_eq!(
        grant.verify_for_execute(&expired, &[GrantId::new(9)]),
        Err(GrantError::Expired)
    );
}

#[test]
fn grant_postcard_roundtrip_is_stable() {
    let grant = grant();
    let bytes = postcard::to_allocvec(&grant).unwrap();
    let decoded: ScopedGrant = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(decoded, grant);
}
