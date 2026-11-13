use vb_core::budget::{BoundednessPolicy, BudgetError, WholeWorkflowBudget};
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind as K, ResourceContract, WorkflowError, node,
};

fn n(id: u16, kind: K, next: Option<u16>) -> CompiledNode {
    node(StepIdx::new(id), kind, None, next.map(StepIdx::new))
}

fn tail() -> Vec<CompiledNode> {
    vec![
        n(
            2,
            K::ConcurrentLaneEnd {
                owner: StepIdx::new(0),
                result: None,
            },
            Some(3),
        ),
        n(
            3,
            K::ConcurrentGroupJoin {
                owner: StepIdx::new(0),
            },
            Some(4),
        ),
        n(
            4,
            K::Finish {
                result: SlotIdx::new(0),
            },
            None,
        ),
    ]
}

fn group(limit: Option<u32>, cap: u16) -> Vec<CompiledNode> {
    let start = match limit {
        Some(limit) => fixtures::foreach(0, limit, cap, (1, 3)),
        None => n(
            0,
            K::TogetherConcurrentStart {
                branches: Box::new([StepIdx::new(1)]),
                join: StepIdx::new(3),
            },
            None,
        ),
    };
    let mut nodes = vec![start, fixtures::action(1, 2)];
    nodes.extend(tail());
    nodes
}

fn compute(nodes: &[CompiledNode]) -> Result<WholeWorkflowBudget, WorkflowError> {
    WholeWorkflowBudget::compute(nodes, StepIdx::new(0), &ResourceContract::DEFAULT)
}

#[test]
fn five_node_regression() -> Result<(), WorkflowError> {
    let nodes = group(None, 2);
    let b = compute(&nodes)?;
    assert!(
        !(b.max_total_steps != 5 || b.max_action_tickets != 1 || b.max_parallel_in_flight != 1)
    );
    let mut alt = nodes.clone();
    if let Some(first) = alt.first_mut() {
        first.kind = K::TogetherStart {
            branches: Box::new([StepIdx::new(1)]),
            join: StepIdx::new(3),
        };
    } else {
        return Err(WorkflowError::EmptyNodes);
    }
    let b2 = compute(&alt)?;
    assert!(
        !(b2.max_total_steps != 5 || b2.max_action_tickets != 1 || b2.max_parallel_in_flight != 1)
    );
    Ok(())
}

#[test]
fn foreach_limit3_cap2() -> Result<(), WorkflowError> {
    let nodes = group(Some(3), 2);
    let b = compute(&nodes)?;
    assert!(
        !(b.max_total_steps != 9
            || b.max_action_tickets != 3
            || b.max_parallel_in_flight != 2
            || b.max_fanout != 2
            || b.max_for_each_iterations != 3)
    );
    Ok(())
}

#[test]
fn zero_limit_cap2() -> Result<(), WorkflowError> {
    let nodes = group(Some(0), 2);
    let b = compute(&nodes)?;
    assert!(
        !(b.max_total_steps != 3
            || b.max_action_tickets != 0
            || b.max_parallel_in_flight != 0
            || b.max_fanout != 2)
    );
    Ok(())
}

#[test]
fn zero_limit_cap65_fails_validate() -> Result<(), WorkflowError> {
    let nodes = group(Some(0), 65);
    let b = compute(&nodes)?;
    assert_eq!(
        BoundednessPolicy::DEFAULT.validate(&b),
        Err(BudgetError::FanoutExceeded {
            actual: 65,
            limit: 64
        })
    );
    Ok(())
}

#[test]
fn first_success_variant() -> Result<(), WorkflowError> {
    let mut nodes = group(None, 2);
    if let Some(first) = nodes.first_mut() {
        first.kind = K::FirstSuccessStart {
            branches: Box::new([StepIdx::new(1)]),
            join: StepIdx::new(3),
        };
    } else {
        return Err(WorkflowError::EmptyNodes);
    }
    let b = compute(&nodes)?;
    assert!(
        !(b.max_total_steps != 5 || b.max_action_tickets != 1 || b.max_parallel_in_flight != 1)
    );
    Ok(())
}

#[test]
fn derived_dimensions_foreach3_cap2() -> Result<(), WorkflowError> {
    let nodes = group(Some(3), 2);
    let b = compute(&nodes)?;
    assert!(
        !(b.max_trace_events != 9
            || b.max_incident_emissions != 9
            || b.max_run_time_seconds != 9
            || b.max_transitions != 36
            || b.max_cost_units != 12
            || b.max_journal_bytes_total != 9437184)
    );
    let contract = ResourceContract::DEFAULT;
    let expected_value = 9u64
        .checked_add(u64::from(contract.max_slots))
        .and_then(|v| v.checked_add(u64::from(contract.max_constants)));
    assert!(!(Some(b.max_value_store_entries) != expected_value));
    assert!(!(b.max_steps_executable != WholeWorkflowBudget::MAX_STEPS_EXECUTABLE));
    copied_dimensions(&b, &contract);
    Ok(())
}

#[path = "shipping_concurrent_budget/cases.rs"]
mod cases;
#[path = "shipping_concurrent_budget/fixtures.rs"]
mod fixtures;

fn copied_dimensions(b: &WholeWorkflowBudget, contract: &ResourceContract) {
    assert!(
        !(b.max_input_bytes != contract.max_input_bytes
            || b.max_blob_bytes != contract.max_blob_bytes
            || b.max_queue_depth != contract.max_queue_depth
            || b.max_result_bytes != contract.max_output_bytes
            || b.max_total_slots != u64::from(contract.max_slots)
            || b.max_journal_batch_bytes != contract.max_journal_batch_bytes)
    );
}

#[path = "shipping_concurrent_budget/composition.rs"]
mod composition;
#[path = "shipping_concurrent_budget/legacy_fixture.rs"]
mod legacy_fixture;

#[path = "shipping_concurrent_budget/heterogeneous.rs"]
mod heterogeneous;
#[path = "shipping_concurrent_budget/zero.rs"]
mod zero;

#[path = "shipping_concurrent_budget/dimensions.rs"]
mod dimensions;

#[path = "shipping_concurrent_budget/legacy_families.rs"]
mod legacy_families;

#[path = "shipping_concurrent_budget/construction.rs"]
mod construction;
