use super::fixtures::nested;
use super::{WorkflowError, compute, group};
use vb_core::budget::{BoundednessPolicy, BudgetError, compute_max_parallel_in_flight};
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::{CompiledNodeKind, Deadline};
#[test]
fn nested_weighted_reservations() -> Result<(), WorkflowError> {
    let nodes = nested(8, 8, 4, 4);
    let budget = compute(&nodes)?;
    assert_eq!(budget.max_total_steps, 91);
    assert_eq!(budget.max_action_tickets, 32);
    assert_eq!(budget.max_parallel_in_flight, 40);
    assert_eq!(budget.max_fanout, 8);
    assert_eq!(budget.max_nesting_depth, 2);

    let budgeted = compute_max_parallel_in_flight(nodes.iter())?;
    assert_eq!(budgeted, 40);

    Ok(())
}

#[test]
fn nested_different_limits() -> Result<(), WorkflowError> {
    let nodes = nested(3, 2, 5, 4);
    let budget = compute(&nodes)?;
    assert_eq!(budget.max_total_steps, 42);
    assert_eq!(budget.max_action_tickets, 15);
    assert_eq!(budget.max_parallel_in_flight, 10);
    assert_eq!(budget.max_fanout, 4);
    assert_eq!(budget.max_nesting_depth, 2);
    Ok(())
}

#[test]
fn zero_outer_keeps_encoded_cap() -> Result<(), WorkflowError> {
    let nodes = nested(0, 2, 100, 65);
    let budget = compute(&nodes)?;
    assert_eq!(budget.max_total_steps, 3);
    assert_eq!(budget.max_action_tickets, 0);
    assert_eq!(budget.max_parallel_in_flight, 0);
    assert_eq!(budget.max_fanout, 65);

    let policy = BoundednessPolicy::DEFAULT;
    let err = policy.validate(&budget);
    assert!(matches!(
        err,
        Err(BudgetError::FanoutExceeded {
            actual: 65,
            limit: 64
        })
    ));
    Ok(())
}

#[test]
fn nested_run_overflow_is_typed() {
    let nodes = nested(u32::MAX, 1, u32::MAX, 1);
    let err = compute(&nodes);
    assert!(matches!(err, Err(WorkflowError::StepCountOverflow { .. })));
}

#[test]
fn total_u32_overflow_is_typed() {
    let nodes = group(Some(u32::MAX), 1);
    let err = compute(&nodes);
    assert!(matches!(err, Err(WorkflowError::StepCountOverflow { .. })));
}

#[test]
fn active_capacity_is_not_limit() -> Result<(), WorkflowError> {
    let nodes = group(Some(100), 2);
    let budget = compute(&nodes)?;
    assert_eq!(budget.max_total_steps, 203);
    assert_eq!(budget.max_action_tickets, 100);
    assert_eq!(budget.max_parallel_in_flight, 2);
    assert_eq!(budget.max_fanout, 2);
    Ok(())
}

#[test]
fn foreign_lane_end_rejected() -> Result<(), WorkflowError> {
    let mut nodes = group(None, 1);
    let node = nodes.get_mut(2).ok_or(WorkflowError::EmptyNodes)?;
    node.kind = CompiledNodeKind::ConcurrentLaneEnd {
        owner: StepIdx::new(99),
        result: None,
    };
    assert_eq!(
        compute(&nodes),
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(2)
        })
    );
    Ok(())
}

#[test]
fn unowned_jump_backedge_rejected() -> Result<(), WorkflowError> {
    let mut nodes = group(Some(3), 2);
    if let Some(n) = nodes.get_mut(1) {
        n.kind = CompiledNodeKind::Jump {
            target: StepIdx::new(1),
        };
    }
    let err = compute(&nodes);
    assert!(matches!(err, Err(WorkflowError::JumpCycle { .. })));
    Ok(())
}

#[test]
fn parallel_policy_independent() -> Result<(), WorkflowError> {
    let nodes = nested(16, 16, 16, 16);
    let budget = compute(&nodes)?;
    assert_eq!(budget.max_parallel_in_flight, 272);
    assert_eq!(budget.max_fanout, 16);
    assert_eq!(budget.max_action_tickets, 256);

    let policy = BoundednessPolicy::DEFAULT;
    let err = policy.validate(&budget);
    assert!(matches!(
        err,
        Err(BudgetError::ParallelExceeded {
            actual: 272,
            limit: 256
        })
    ));
    Ok(())
}

#[test]
fn foreach_ask_reservations() -> Result<(), WorkflowError> {
    let mut nodes = group(Some(100), 2);
    if let Some(n) = nodes.get_mut(1) {
        n.kind = CompiledNodeKind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: Deadline::Unset,
        };
    }
    let budget = compute(&nodes)?;
    assert_eq!(budget.max_total_steps, 203);
    assert_eq!(budget.max_action_tickets, 0);
    assert_eq!(budget.max_timer_entries, 2);
    assert_eq!(budget.max_open_gates, 2);
    assert_eq!(budget.max_signal_subscriptions, 0);
    Ok(())
}

#[test]
fn foreach_wait_event_reservations() -> Result<(), WorkflowError> {
    let mut nodes = group(Some(100), 2);
    if let Some(n) = nodes.get_mut(1) {
        n.kind = CompiledNodeKind::WaitEvent {
            event: SlotIdx::new(0),
            timeout_slot: Deadline::Unset,
        };
    }
    let budget = compute(&nodes)?;
    assert_eq!(budget.max_total_steps, 203);
    assert_eq!(budget.max_timer_entries, 2);
    assert_eq!(budget.max_signal_subscriptions, 2);
    assert_eq!(budget.max_open_gates, 0);
    Ok(())
}

#[test]
fn zero_at_once_rejected() {
    let nodes = group(Some(5), 0);
    assert_eq!(
        compute(&nodes),
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(0)
        })
    );
}
