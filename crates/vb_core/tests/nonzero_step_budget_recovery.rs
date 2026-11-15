use vb_core::workflow::{ContractResource, WorkflowParts, node};
use vb_core::{
    CompiledNodeKind, CompiledWorkflow, ResourceContract, StepIdx, WorkflowDigest, WorkflowError,
};

fn parts(budget: u64) -> WorkflowParts {
    WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: "nonzero_step_budget".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::new([node(StepIdx::new(0), CompiledNodeKind::Nop, None, None)]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract {
            max_step_budget_per_tick: budget,
            ..ResourceContract::DEFAULT
        },
        step_names: Box::new([]),
    }
}

fn zero_error() -> WorkflowError {
    WorkflowError::ResourceContractExceeded {
        resource: ContractResource::StepBudgetPerTick,
    }
}

#[test]
fn legacy_checked_constructor_rejects_zero_step_budget() {
    assert_eq!(
        CompiledWorkflow::try_from_parts(parts(0)).map(|_| ()),
        Err(zero_error())
    );
}

#[test]
fn concurrent_checked_constructor_rejects_zero_step_budget() {
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(parts(0)).map(|_| ()),
        Err(zero_error())
    );
}

#[test]
fn checked_constructors_accept_one_and_hard_limit() {
    for budget in [1, vb_core::limits::MAX_STEP_BUDGET] {
        assert_eq!(
            CompiledWorkflow::try_from_parts(parts(budget)).map(|_| ()),
            Ok(())
        );
        assert_eq!(
            CompiledWorkflow::try_from_concurrent_parts(parts(budget)).map(|_| ()),
            Ok(())
        );
    }
}

#[test]
fn checked_constructors_preserve_over_hard_limit_error() -> Result<(), Box<dyn std::error::Error>> {
    let budget = vb_core::limits::MAX_STEP_BUDGET
        .checked_add(1)
        .ok_or("hard limit overflow")?;
    let expected = WorkflowError::ResourceContractTooLarge {
        resource: ContractResource::StepBudgetPerTick,
    };
    assert_eq!(
        CompiledWorkflow::try_from_parts(parts(budget)).map(|_| ()),
        Err(expected.clone())
    );
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(parts(budget)).map(|_| ()),
        Err(expected)
    );
    Ok(())
}

#[test]
fn actual_resource_usage_error_precedes_zero_step_budget() {
    let mut parts = parts(0);
    parts.resource_contract.max_steps = 0;
    assert_eq!(
        CompiledWorkflow::try_from_parts(parts).map(|_| ()),
        Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::Steps
        })
    );
}

#[test]
fn zero_step_budget_precedes_zero_transition_budget() {
    let mut parts = parts(0);
    parts.resource_contract.max_transitions_per_tick = 0;
    assert_eq!(
        CompiledWorkflow::try_from_parts(parts).map(|_| ()),
        Err(zero_error())
    );
}

#[test]
fn zero_transition_budget_retains_its_existing_error() {
    let mut parts = parts(1);
    parts.resource_contract.max_transitions_per_tick = 0;
    assert_eq!(
        CompiledWorkflow::try_from_parts(parts).map(|_| ()),
        Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::TransitionsPerTick
        })
    );
}

#[test]
fn bounds_only_engine_precheck_retains_zero_budget_acceptance() {
    assert_eq!(
        vb_core::engine::validate_resource_contract(&parts(0)),
        Ok(())
    );
}
