use super::fixtures::{action, end, finish, foreach, join};
use super::legacy_fixture::{inner_legacy, outer_legacy};
use super::{K, WorkflowError, compute, group, n};
use vb_core::budget::compute_max_parallel_in_flight;
use vb_core::ids::{SlotIdx, StepIdx};

#[test]
fn sequential_groups_release_reservations() -> Result<(), WorkflowError> {
    let mut nodes = group(None, 1);
    nodes.truncate(4);
    nodes.extend([
        foreach(4, 5, 4, (5, 7)),
        action(5, 6),
        end(6, 4, 7),
        join(7, 4, 8),
        finish(8),
    ]);
    let budget = compute(&nodes)?;
    assert_eq!(
        (
            budget.max_total_steps,
            budget.max_action_tickets,
            budget.max_parallel_in_flight
        ),
        (17, 6, 4)
    );
    assert_eq!(compute_max_parallel_in_flight(nodes.iter())?, 4);
    Ok(())
}

#[test]
fn first_success_budgets_every_branch() -> Result<(), WorkflowError> {
    let nodes = first_success_nodes();
    let budget = compute(&nodes)?;
    assert_eq!(
        (
            budget.max_total_steps,
            budget.max_action_tickets,
            budget.max_parallel_in_flight
        ),
        (7, 2, 2)
    );
    Ok(())
}

#[test]
fn legacy_outer_loop_multiplies_new_group() -> Result<(), WorkflowError> {
    let budget = compute(&outer_legacy()?)?;
    assert_eq!(
        (
            budget.max_total_steps,
            budget.max_action_tickets,
            budget.max_parallel_in_flight
        ),
        (42, 15, 4)
    );
    assert_eq!(budget.max_for_each_iterations, 18);
    assert_eq!(budget.max_nesting_depth, 2);
    Ok(())
}

#[test]
fn new_group_multiplies_legacy_inner_loop() -> Result<(), WorkflowError> {
    let budget = compute(&inner_legacy()?)?;
    assert_eq!(
        (
            budget.max_total_steps,
            budget.max_action_tickets,
            budget.max_parallel_in_flight
        ),
        (42, 15, 2)
    );
    assert_eq!(budget.max_for_each_iterations, 18);
    assert_eq!(budget.max_nesting_depth, 2);
    Ok(())
}

#[test]
fn legacy_latch_requires_matching_state_slot() -> Result<(), WorkflowError> {
    let mut nodes = inner_legacy()?;
    nodes.get_mut(3).ok_or(WorkflowError::EmptyNodes)?.kind = K::ForEachNext {
        iterator_slot: SlotIdx::new(99),
        body: StepIdx::new(2),
        done: StepIdx::new(4),
    };
    assert!(matches!(
        compute(&nodes),
        Err(WorkflowError::JumpCycle { .. })
    ));
    Ok(())
}

fn first_success_nodes() -> Vec<vb_core::workflow::CompiledNode> {
    vec![
        n(
            0,
            K::FirstSuccessStart {
                branches: Box::new([StepIdx::new(1), StepIdx::new(3)]),
                join: StepIdx::new(5),
            },
            None,
        ),
        action(1, 2),
        end(2, 0, 5),
        action(3, 4),
        end(4, 0, 5),
        join(5, 0, 6),
        finish(6),
    ]
}

#[test]
fn heterogeneous_lanes_weight_only_the_nested_branch() -> Result<(), WorkflowError> {
    let nodes = super::heterogeneous::graph();
    let budget = compute(&nodes)?;
    assert_eq!(
        (
            budget.max_total_steps,
            budget.max_action_tickets,
            budget.max_parallel_in_flight
        ),
        (16, 5, 6)
    );
    assert_eq!(compute_max_parallel_in_flight(nodes.iter())?, 6);
    Ok(())
}

#[test]
fn concurrent_foreach_scales_collect_metrics() -> Result<(), WorkflowError> {
    let budget = compute(&super::legacy_families::collect()?)?;
    assert_eq!(
        (
            budget.max_total_steps,
            budget.max_action_tickets,
            budget.max_parallel_in_flight
        ),
        (42, 15, 2)
    );
    assert_eq!((budget.max_gather_pages, budget.max_gather_items), (9, 15));
    Ok(())
}
#[test]
fn concurrent_foreach_scales_reduce_work() -> Result<(), WorkflowError> {
    let budget = compute(&super::legacy_families::reduce()?)?;
    assert_eq!(
        (
            budget.max_total_steps,
            budget.max_action_tickets,
            budget.max_parallel_in_flight
        ),
        (42, 15, 2)
    );
    Ok(())
}
#[test]
fn concurrent_foreach_scales_repeat_and_timer_reservations() -> Result<(), WorkflowError> {
    let budget = compute(&super::legacy_families::repeat()?)?;
    assert_eq!(
        (
            budget.max_total_steps,
            budget.max_action_tickets,
            budget.max_parallel_in_flight
        ),
        (42, 15, 2)
    );
    assert_eq!(
        (budget.max_repeat_attempts, budget.max_timer_entries),
        (5, 2)
    );
    Ok(())
}

#[test]
fn parallel_api_rejects_rootless_new_graph() -> Result<(), WorkflowError> {
    let mut nodes = group(None, 1);
    nodes
        .first_mut()
        .ok_or(WorkflowError::EmptyNodes)?
        .routing
        .next = Some(StepIdx::new(0)).into();
    assert!(matches!(
        compute_max_parallel_in_flight(nodes.iter()),
        Err(WorkflowError::JumpCycle { .. })
    ));
    Ok(())
}
