use super::*;
use crate::budget::{
    AggregateBudgetError, AggregateResourceBudget, WholeWorkflowBudget, validate_step_ceilings,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn workflow(count: u16) -> Result<CompiledWorkflow, WorkflowError> {
    let nodes = (0..count)
        .map(|step| {
            let next = step
                .checked_add(1)
                .filter(|next| *next < count)
                .map(StepIdx::new);
            crate::workflow::node(StepIdx::new(step), CompiledNodeKind::Nop, None, next)
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: "borrowed-budget".into(),
        digest: WorkflowDigest::from_bytes([19; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes,
        expressions: Box::default(),
        accessors: Box::default(),
        constants: Box::new([ConstValue::I64(23), ConstValue::Bool(true)]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: (0..count)
            .map(|step| format!("step{step}").into_boxed_str())
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        input_slots: Box::default(),
    })
}

// Exact former algorithm retained only as a comparison oracle.
fn cloned_parts_budget(
    workflow: &CompiledWorkflow,
) -> Result<AggregateResourceBudget, AggregateBudgetError> {
    let parts = workflow.to_parts();
    let budget = WholeWorkflowBudget::compute(
        &parts.nodes,
        workflow.entry(),
        &workflow.resource_contract(),
    )
    .map_err(AggregateBudgetError::WorkflowBudget)?;
    let aggregate =
        AggregateResourceBudget::from_whole_workflow_budget(budget, workflow.resource_contract())?;
    validate_step_ceilings(&aggregate)?;
    Ok(aggregate)
}

#[test]
fn node_slice_borrows_the_compiled_storage() -> TestResult {
    let workflow = workflow(2)?;
    assert_eq!(workflow.nodes().len(), usize::from(workflow.node_count()));
    let borrowed = workflow.nodes().first().ok_or("borrowed first node")?;
    let checked = workflow.node(StepIdx::ZERO).ok_or("checked first node")?;
    assert!(core::ptr::eq(borrowed, checked));
    Ok(())
}

#[test]
fn borrowed_budget_matches_cloned_parts_without_mutating_workflow() -> TestResult {
    for count in [1, 2] {
        let workflow = workflow(count)?;
        let before = workflow.to_parts();
        assert_eq!(
            AggregateResourceBudget::from_workflow(&workflow),
            cloned_parts_budget(&workflow)
        );
        assert_eq!(workflow.to_parts(), before);
    }
    Ok(())
}

#[test]
fn borrowed_budget_preserves_traversal_and_step_ceiling_errors() -> TestResult {
    for invalid in [0, 1] {
        let mut workflow = workflow(1)?;
        // Synthetic private invalid state to exercise old error paths, not admission.
        if invalid == 0 {
            workflow.entry = StepIdx::new(9);
        } else {
            workflow.resource_contract.max_step_budget_per_tick = 0;
        }
        let expected = cloned_parts_budget(&workflow);
        assert!(expected.is_err());
        assert_eq!(AggregateResourceBudget::from_workflow(&workflow), expected);
    }
    Ok(())
}
