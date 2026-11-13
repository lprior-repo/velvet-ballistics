#![forbid(unsafe_code)]
#[allow(dead_code)]
#[path = "artifact_evidence_support/mod.rs"]
mod artifact_evidence_support;

use artifact_evidence_support::*;
use vb_core::action_pack::{CapabilityScopeSpec, RecoveryClass};
use vb_core::value::ConstValue;
use vb_core::workflow::{ResourceContract, node};
use vb_core::{
    ActionContract, ActionId, ActionTicket, Capability, CompiledNodeKind, CompiledWorkflow,
    ConstIdx, RunId, RuntimePolicy, SeqNo, SlotIdx, StepIdx, WorkflowDigest, WorkflowParts,
};
use vb_storage::executor::{ExecutorCatalog, ExecutorError, FjallRegister};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn scope(namespace: u64, key: u64) -> CapabilityScopeSpec {
    CapabilityScopeSpec { namespace, key }
}

fn external_contract(action: ActionId) -> Result<ActionContract, Box<dyn std::error::Error>> {
    let mut contract = contract()?;
    contract.id = action;
    contract.required_capabilities =
        Box::new([Capability::new("velvet.register.write".into(), action)]);
    Ok(contract)
}

fn ticket(action: ActionId, step: u16, seq: u64) -> ActionTicket {
    let run = RunId::new(100);
    let seq = SeqNo::new(seq);
    ActionTicket {
        run,
        step: StepIdx::new(step),
        seq,
        action,
        run_attempt: 1,
        attempt: 1,
        idempotency_key: vb_core::action::compute_action_idempotency_key(run, 1, seq, action),
        capacity: 2,
        fencing_token: 0,
    }
}

fn two_action_workflow() -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::from("two_action_executor"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::new([
            node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Do {
                    action: ActionId::new(0),
                    input: SlotIdx::new(0),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(2)),
            ),
            node(
                StepIdx::new(2),
                CompiledNodeKind::Do {
                    action: ActionId::new(1),
                    input: SlotIdx::new(1),
                },
                Some(SlotIdx::new(2)),
                Some(StepIdx::new(3)),
            ),
            node(
                StepIdx::new(3),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
                None,
                None,
            ),
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(42)]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    canonicalize(parts).map_err(Into::into)
}

#[test]
fn wrong_register_scope_is_unauthorized_without_effect() -> TestResult {
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let contract = external_contract(ActionId::new(0))?;
    let workflow = action_workflow()?;
    let pack = catalog.register_pack(&contract, scope(1, 1), RecoveryClass::ReconcileReceipt)?;
    let artifact =
        catalog.prepare_artifact(&workflow, RuntimePolicy::Strict, &[contract], &[pack])?;
    let ticket = ticket(ActionId::new(0), 1, 1);
    let valid = catalog.issue_register_grant(&artifact, ticket, scope(1, 1))?;
    let dir = tempfile::tempdir()?;
    let mut adapter = FjallRegister::open(dir.path().join("register").as_path(), 2)?;
    adapter.execute(&valid, ticket, 42)?;
    assert_eq!(adapter.effect_count()?, 1);
    let result = catalog.issue_register_grant(&artifact, ticket, scope(1, 2));
    assert!(matches!(result, Err(ExecutorError::Unauthorized)));
    assert_eq!(adapter.effect_count()?, 1);
    assert_eq!(adapter.value(scope(1, 1))?, Some(42));
    assert_eq!(adapter.value(scope(1, 2))?, None);
    Ok(())
}

#[test]
fn two_pack_dispatch_selects_action_one_pack_and_rejects_bad_order() -> TestResult {
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let contract0 = external_contract(ActionId::new(0))?;
    let contract1 = external_contract(ActionId::new(1))?;
    let pack0 = catalog.register_pack(&contract0, scope(1, 1), RecoveryClass::ReconcileReceipt)?;
    let pack1 = catalog.register_pack(&contract1, scope(1, 2), RecoveryClass::ReconcileReceipt)?;
    let workflow = two_action_workflow()?;
    let artifact = catalog.prepare_artifact(
        &workflow,
        RuntimePolicy::Strict,
        &[contract0.clone(), contract1.clone()],
        &[pack0.clone(), pack1.clone()],
    )?;
    assert_eq!(
        catalog.declared_scope(&artifact, ActionId::new(0))?,
        scope(1, 1)
    );
    assert_eq!(
        catalog.declared_scope(&artifact, ActionId::new(1))?,
        scope(1, 2)
    );
    let action1 = ticket(ActionId::new(1), 2, 1);
    let grant = catalog.issue_register_grant(&artifact, action1, scope(1, 2))?;
    let dir = tempfile::tempdir()?;
    let mut adapter = FjallRegister::open(dir.path().join("register").as_path(), 8)?;
    let receipt = adapter.execute(&grant, action1, 42)?;
    assert_eq!(receipt.value(), 42);
    assert_eq!(adapter.effect_count()?, 1);
    assert_eq!(adapter.value(scope(1, 1))?, None);
    assert_eq!(adapter.value(scope(1, 2))?, Some(42));
    let duplicate = adapter.execute(&grant, action1, 42)?;
    assert_eq!(duplicate, receipt);
    assert_eq!(adapter.effect_count()?, 1);
    let reversed = [pack1.clone(), pack0.clone()];
    let duplicated = [pack0.clone(), pack0.clone()];
    let extra = [pack0.clone(), pack1.clone(), pack1.clone()];
    for packs in [reversed.as_slice(), duplicated.as_slice(), extra.as_slice()] {
        assert!(matches!(
            catalog.prepare_artifact(
                &workflow,
                RuntimePolicy::Strict,
                &[contract0.clone(), contract1.clone()],
                packs,
            ),
            Err(ExecutorError::PackMismatch)
        ));
    }
    Ok(())
}

#[test]
fn capacity_and_attempt_bounds_reject_without_effect() -> TestResult {
    let dir = tempfile::tempdir()?;
    assert!(matches!(
        FjallRegister::open(dir.path().join("zero").as_path(), 0),
        Err(ExecutorError::Capacity)
    ));
    let max = FjallRegister::open(dir.path().join("max").as_path(), 4096)?;
    drop(max);
    assert!(matches!(
        FjallRegister::open(dir.path().join("max").as_path(), 4097),
        Err(ExecutorError::Capacity)
    ));
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let contract = external_contract(ActionId::new(0))?;
    let workflow = action_workflow()?;
    let pack = catalog.register_pack(&contract, scope(1, 1), RecoveryClass::ReconcileReceipt)?;
    let artifact =
        catalog.prepare_artifact(&workflow, RuntimePolicy::Strict, &[contract], &[pack])?;
    let ticket = ticket(ActionId::new(0), 1, 1);
    let valid = catalog.issue_register_grant(&artifact, ticket, scope(1, 1))?;
    let mut adapter = FjallRegister::open(dir.path().join("attempt").as_path(), 2)?;
    adapter.execute(&valid, ticket, 42)?;
    assert_eq!(adapter.effect_count()?, 1);
    for attempt in [0_u16, 3_u16] {
        let mut bad = ticket;
        bad.attempt = attempt;
        assert!(matches!(
            catalog.issue_register_grant(&artifact, bad, scope(1, 1)),
            Err(ExecutorError::InvalidTicket)
        ));
    }
    assert_eq!(adapter.effect_count()?, 1);
    assert_eq!(adapter.value(scope(1, 1))?, Some(42));
    Ok(())
}
