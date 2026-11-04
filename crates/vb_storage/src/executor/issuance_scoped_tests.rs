//! Scoped-grant issuance tests: trusted binding of lifecycle facts to one
//! register grant (strict-exact narrowing, capability equality).

use super::issuance_test_support::*;
use crate::executor::ExecutorError;
use vb_core::action::compute_action_idempotency_key;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
use vb_core::manual_gate::{ApproverId, GateDigest};
use vb_core::{
    ActionTicket, GrantId, GrantTimeMs, RevocationEpoch, RuntimePolicy, ScopedGrantParams,
};

fn ticket() -> ActionTicket {
    let run = RunId::new(41);
    let seq = SeqNo::new(1);
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

fn params() -> Result<ScopedGrantParams, Box<dyn std::error::Error>> {
    let issuer = ApproverId::new("grant-issuer".into()).ok_or("bad issuer")?;
    let subject = ApproverId::new("run-subject".into()).ok_or("bad subject")?;
    Ok(ScopedGrantParams {
        id: GrantId::new(5),
        issuer,
        subject,
        capability: vb_core::Capability::new("velvet.register.write".into(), ActionId::new(0)),
        scope: register_scope(),
        issued_at: GrantTimeMs::new(1_000),
        expires_at: GrantTimeMs::new(2_000),
        delegable: false,
        delegated_from: None,
        revocation_epoch: RevocationEpoch::new(1),
        break_glass: false,
        approval_receipt: GateDigest::from_bytes([0x22u8; 32]),
        signature: None,
    })
}

/// Valid issuance binds artifact, ticket, capability, and declared scope.
#[test]
fn scoped_issuance_binds_exact_declarations() -> TestResult {
    let fixture = PackedFixture::new()?;
    let (_dir, journal) = open_test_journal()?;
    let artifact = fixture.submit(&journal, RuntimePolicy::Strict)?;
    let register = fixture
        .catalog
        .issue_register_grant(&artifact, ticket(), register_scope())?;
    let scoped = fixture
        .catalog
        .issue_scoped_grant(&artifact, &register, params()?)?;
    assert_eq!(scoped.artifact, artifact.digest);
    assert_eq!(scoped.ticket, ticket());
    assert_eq!(scoped.capability, params()?.capability);
    assert_eq!(scoped.scope, register_scope());
    assert_eq!(
        scoped.scope,
        fixture
            .catalog
            .declared_scope(&artifact, ActionId::new(0))?
    );
    Ok(())
}

/// Over-broad scope is rejected: params must equal register and pack scope.
#[test]
fn scoped_issuance_rejects_overgrant_scope() -> TestResult {
    let fixture = PackedFixture::new()?;
    let (_dir, journal) = open_test_journal()?;
    let artifact = fixture.submit(&journal, RuntimePolicy::Strict)?;
    let register = fixture
        .catalog
        .issue_register_grant(&artifact, ticket(), register_scope())?;
    let mut overgrant = params()?;
    overgrant.scope = vb_core::action_pack::CapabilityScopeSpec {
        namespace: 15,
        key: 30,
    };
    let result = fixture
        .catalog
        .issue_scoped_grant(&artifact, &register, overgrant);
    assert!(
        matches!(result, Err(ExecutorError::Unauthorized)),
        "expected Unauthorized for overgrant scope, got: {result:?}"
    );
    Ok(())
}

/// Capability outside the contract requirement is rejected.
#[test]
fn scoped_issuance_rejects_foreign_capability() -> TestResult {
    let fixture = PackedFixture::new()?;
    let (_dir, journal) = open_test_journal()?;
    let artifact = fixture.submit(&journal, RuntimePolicy::Strict)?;
    let register = fixture
        .catalog
        .issue_register_grant(&artifact, ticket(), register_scope())?;
    let mut foreign = params()?;
    foreign.capability = vb_core::Capability::new("velvet.other.write".into(), ActionId::new(0));
    let result = fixture
        .catalog
        .issue_scoped_grant(&artifact, &register, foreign);
    assert!(
        matches!(result, Err(ExecutorError::Unauthorized)),
        "expected Unauthorized for foreign capability, got: {result:?}"
    );
    Ok(())
}

/// Register grant bound to another artifact cannot back this issuance.
#[test]
fn scoped_issuance_rejects_foreign_register_grant() -> TestResult {
    let fixture = PackedFixture::new()?;
    let (_dir, journal) = open_test_journal()?;
    let first = fixture.submit(&journal, RuntimePolicy::Strict)?;
    let second = fixture.submit(&journal, RuntimePolicy::Strict)?;
    assert_ne!(first.digest, second.digest);
    let foreign = fixture
        .catalog
        .issue_register_grant(&second, ticket(), register_scope())?;
    let result = fixture
        .catalog
        .issue_scoped_grant(&first, &foreign, params()?);
    assert!(
        matches!(result, Err(ExecutorError::Unauthorized)),
        "expected Unauthorized for foreign register grant, got: {result:?}"
    );
    Ok(())
}
