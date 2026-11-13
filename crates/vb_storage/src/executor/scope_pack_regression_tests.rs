//! Direct adapter regression for changed-only scope and changed-only pack.

use super::test_support::scratch_grant;
use super::{ExecutorError, FjallRegister, RegisterGrant};
use vb_core::WorkflowDigest;
use vb_core::action_pack::{ActionPackDigest, CapabilityScopeSpec};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn changed_scope_only_conflicts_without_extra_effect() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut adapter = FjallRegister::open(dir.path(), 8)?;
    let grant = scratch_grant();
    let first = adapter.execute(&grant, grant.ticket, 42)?;
    assert_eq!(adapter.effect_count()?, 1);
    let changed_scope = RegisterGrant {
        scope: CapabilityScopeSpec {
            namespace: 1,
            key: 2,
        },
        ..grant.clone()
    };
    assert!(matches!(
        adapter.execute(&changed_scope, grant.ticket, 42),
        Err(ExecutorError::ReceiptConflict)
    ));
    assert_eq!(adapter.effect_count()?, 1);
    assert_eq!(adapter.value(grant.scope)?, Some(42));
    assert_eq!(adapter.value(changed_scope.scope)?, None);
    assert_eq!(adapter.reconcile(&grant, grant.ticket, 42)?, first);
    Ok(())
}

#[test]
fn changed_pack_only_conflicts_without_extra_effect() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut adapter = FjallRegister::open(dir.path(), 8)?;
    let grant = scratch_grant();
    let first = adapter.execute(&grant, grant.ticket, 42)?;
    assert_eq!(adapter.effect_count()?, 1);
    let changed_pack = RegisterGrant {
        pack: ActionPackDigest::from_digest(WorkflowDigest::from_bytes([2; 32])),
        ..grant.clone()
    };
    assert!(matches!(
        adapter.execute(&changed_pack, grant.ticket, 42),
        Err(ExecutorError::ReceiptConflict)
    ));
    assert_eq!(adapter.effect_count()?, 1);
    assert_eq!(adapter.value(grant.scope)?, Some(42));
    assert_eq!(adapter.reconcile(&grant, grant.ticket, 42)?, first);
    Ok(())
}
