use super::support::{TestResult, at_cut};
use vb_core::root_bundle::{OwnedRootBundleError, validate_runtime_store};
use vb_core::value_store::ObjectField;
use vb_core::{ListId, SlotValue, SymbolId, Taint};

#[test]
fn runtime_validator_checks_unreferenced_cycle_without_input_reachability() -> TestResult {
    let fixture = at_cut(false)?;
    let mut checkpoint = fixture.state.store.checkpoint_v1();
    validate_runtime_store(&fixture.state.frame, &checkpoint, &fixture.state.workflow)?;
    let id = ListId::new(u32::try_from(checkpoint.lists.len())?);
    checkpoint.lists.push(Box::from([SlotValue::List(id)]));
    checkpoint.list_taints.push(Box::from([Taint::Clean]));
    assert!(matches!(
        validate_runtime_store(&fixture.state.frame, &checkpoint, &fixture.state.workflow),
        Err(OwnedRootBundleError::GraphCycle { .. })
    ));
    Ok(())
}

#[test]
fn runtime_validator_checks_unreferenced_duplicate_object_keys() -> TestResult {
    let fixture = at_cut(false)?;
    let mut checkpoint = fixture.state.store.checkpoint_v1();
    let field = ObjectField::with_taint(SymbolId::new(0), SlotValue::I64(1), Taint::Clean);
    checkpoint.objects.push(Box::from([field, field]));
    assert!(matches!(
        validate_runtime_store(&fixture.state.frame, &checkpoint, &fixture.state.workflow),
        Err(OwnedRootBundleError::GraphDuplicateObjectKey { .. })
    ));
    Ok(())
}

#[test]
fn runtime_validator_checks_retired_chain_depth() -> TestResult {
    let fixture = at_cut(false)?;
    let mut checkpoint = fixture.state.store.checkpoint_v1();
    let mut value = SlotValue::I64(1);
    for _ in 0..65 {
        let id = ListId::new(u32::try_from(checkpoint.lists.len())?);
        checkpoint.lists.push(Box::from([value]));
        checkpoint.list_taints.push(Box::from([Taint::Clean]));
        value = SlotValue::List(id);
    }
    assert!(matches!(
        validate_runtime_store(&fixture.state.frame, &checkpoint, &fixture.state.workflow),
        Err(OwnedRootBundleError::GraphDepth { depth: 65, .. })
    ));
    Ok(())
}
