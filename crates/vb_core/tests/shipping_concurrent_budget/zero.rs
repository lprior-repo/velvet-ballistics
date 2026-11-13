use super::fixtures::{action, end, finish, foreach, join};
use super::{WorkflowError, compute};
use vb_core::workflow::CompiledNode;

fn graph() -> Vec<CompiledNode> {
    vec![
        foreach(0, 0, 1, (1, 9)),
        foreach(1, u32::MAX, 1, (2, 7)),
        foreach(2, u32::MAX, 1, (3, 5)),
        action(3, 4),
        end(4, 2, 5),
        join(5, 2, 6),
        end(6, 1, 7),
        join(7, 1, 8),
        end(8, 0, 9),
        join(9, 0, 10),
        finish(10),
    ]
}

#[test]
fn zero_items_do_not_execute_overflowing_descendants() -> Result<(), WorkflowError> {
    let budget = compute(&graph())?;
    assert_eq!(budget.max_total_steps, 3);
    assert_eq!(budget.max_action_tickets, 0);
    assert_eq!(budget.max_parallel_in_flight, 0);
    assert_eq!(budget.max_fanout, 1);
    assert_eq!(budget.max_nesting_depth, 3);
    Ok(())
}
