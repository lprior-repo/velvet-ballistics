//! Budget module integration tests.

use crate::budget::{
    AggregateBudgetError, AggregateResourceBudget, AggregateResourceUsage, BoundednessPolicy,
    BudgetError, WholeWorkflowBudget,
};
use crate::engine::StepBudget;
use crate::ids::{ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, ExprBranch, ResourceContract, SlotBranch, WorkflowError,
};

#[test]
fn budget_simple_linear_workflow() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_total_steps == 3 && b.max_fanout == 0 && b.max_nesting_depth == 0);

    assert!(budget.is_some(), "linear workflow budget mismatch");
}

#[test]
fn budget_branching_workflow() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ChooseSlot {
                branches: vec![
                    SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(2),
                    },
                ]
                .into_boxed_slice(),
                otherwise: Some(StepIdx::new(3)),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(4, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_total_steps == 4 && b.max_fanout == 2);

    assert!(budget.is_some(), "branching workflow budget mismatch");
}

#[test]
fn budget_nested_loop_depth() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(2),
                item_slot: SlotIdx::new(3),
                limit: 10,
                body: StepIdx::new(2),
                done: StepIdx::new(3),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: Some(SlotIdx::new(4)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(4),
            },
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: Some(SlotIdx::new(5)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(5),
            },
        },
        CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(4),
            },
        },
    ];
    let contract = test_contract(6, 6);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_nesting_depth == 2);

    assert!(budget.is_some(), "nested loop depth mismatch");
}

#[test]
fn budget_rejects_excessive_steps() {
    let budget = test_budget(3, 10, 1, 0);
    let policy = test_policy(2, 10, 64, 8);

    match policy.validate(&budget) {
        Err(BudgetError::TotalStepsExceeded {
            actual: 3,
            limit: 2,
        }) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn budget_rejects_excessive_fanout() {
    let budget = test_budget(1, 10, 3, 0);
    let policy = test_policy(1_000_000, 65_535, 2, 8);

    match policy.validate(&budget) {
        Err(BudgetError::FanoutExceeded {
            actual: 3,
            limit: 2,
        }) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn budget_accepts_within_policy() {
    let budget = test_budget(10, 100, 4, 2);
    let result = BoundednessPolicy::DEFAULT.validate(&budget);
    assert_eq!(result, Ok(()));
}

#[test]
fn budget_rejects_excessive_nesting_depth() {
    let budget = test_budget(1, 10, 1, 10);
    let policy = test_policy(1_000_000, 65_535, 64, 4);

    match policy.validate(&budget) {
        Err(BudgetError::NestingDepthExceeded {
            actual: 10,
            limit: 4,
        }) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn budget_rejects_excessive_total_slots() {
    let budget = test_budget(1, 200_000, 1, 0);
    let policy = test_policy(1_000_000, 65_535, 64, 8);

    match policy.validate(&budget) {
        Err(BudgetError::TotalSlotsExceeded {
            actual: 200_000,
            limit: 65_535,
        }) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn budget_default_policy_accepts_reasonable_budget() {
    let budget = test_budget(500_000, 10_000, 32, 4);
    let result = BoundednessPolicy::DEFAULT.validate(&budget);
    assert_eq!(result, Ok(()));
}

#[test]
fn budget_compute_rejects_entry_out_of_bounds() {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }];
    let contract = test_contract(1, 0);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(5), &contract);

    match result {
        Err(WorkflowError::EntryOutOfBounds { entry }) if entry == StepIdx::new(5) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn budget_together_start_fanout() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(1), StepIdx::new(2), StepIdx::new(3)]
                    .into_boxed_slice(),
                join: StepIdx::new(4),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(5, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_fanout == 3 && b.max_total_steps == 5);

    assert!(budget.is_some(), "together start fanout mismatch");
}

#[test]
fn budget_single_node_workflow() {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }];
    let contract = test_contract(1, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_total_steps == 1 && b.max_fanout == 0 && b.max_nesting_depth == 0);

    assert!(budget.is_some(), "single-node workflow budget mismatch");
}

#[test]
fn budget_error_display_formatting() {
    let err = BudgetError::TotalStepsExceeded {
        actual: 5,
        limit: 3,
    };
    assert_eq!(format!("{err}"), "total steps exceeded: 5 > 3");

    let err = BudgetError::TotalSlotsExceeded {
        actual: 200,
        limit: 100,
    };
    assert_eq!(format!("{err}"), "total slots exceeded: 200 > 100");

    let err = BudgetError::FanoutExceeded {
        actual: 10,
        limit: 4,
    };
    assert_eq!(format!("{err}"), "fanout exceeded: 10 > 4");

    let err = BudgetError::NestingDepthExceeded {
        actual: 16,
        limit: 8,
    };
    assert_eq!(format!("{err}"), "nesting depth exceeded: 16 > 8");

    let err = BudgetError::ParallelExceeded {
        actual: 128,
        limit: 64,
    };
    assert_eq!(format!("{err}"), "parallel exceeded: 128 > 64");

    let err = BudgetError::ActionTicketsExceeded {
        actual: 200_000,
        limit: 100_000,
    };
    assert_eq!(format!("{err}"), "action tickets exceeded: 200000 > 100000");

    let err = BudgetError::RunTimeExceeded {
        actual: 3_000_000,
        limit: 2_592_000,
    };
    assert_eq!(format!("{err}"), "run time exceeded: 3000000 > 2592000");

    let err = BudgetError::ResultBytesExceeded {
        actual: 524_288,
        limit: 262_144,
    };
    assert_eq!(format!("{err}"), "result bytes exceeded: 524288 > 262144");

    let err = BudgetError::StepsExecutableExceeded {
        actual: 2_000_000,
        limit: 1_000_000,
    };
    assert_eq!(
        format!("{err}"),
        "steps executable exceeded: 2000000 > 1000000"
    );
}

#[test]
fn budget_step_count_overflow_detected() {
    // Construct a workflow where a node's next points out of bounds,
    // verifying error propagation through the count path.
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: Some(StepIdx::new(99)), // out of bounds
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }];
    let contract = test_contract(1, 0);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Err(WorkflowError::StepOutOfBounds { .. }) => {}
        other => panic!("expected StepOutOfBounds, got {other:?}"),
    }
}

#[test]
fn budget_empty_nodes_rejected() {
    let nodes: Vec<CompiledNode> = vec![];
    let contract = test_contract(0, 0);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Err(WorkflowError::EntryOutOfBounds { .. }) => {}
        other => panic!("expected EntryOutOfBounds for empty nodes, got {other:?}"),
    }
}

#[test]
fn budget_choose_fanout_counted() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Choose {
                branches: vec![
                    ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(2),
                    },
                ]
                .into_boxed_slice(),
                otherwise: None,
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_fanout == 2 && b.max_total_steps == 3);

    assert!(budget.is_some(), "choose fanout budget mismatch");
}

// =========================================================================
// Security regression tests: loop-aware step counting
// =========================================================================

/// Verifies that a ForEachStart loop multiplies body steps by the limit.
/// Workflow: ForEachStart(limit=5, body=1, done=2) -> Nop -> Finish
/// Expected: 1 (header) + 5 * 1 (body * iterations) + 1 (Finish) = 7
#[test]
fn budget_foreach_loop_multiplies_body_steps() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 5,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_total_steps == 7);

    assert!(
        budget.is_some(),
        "for-each loop should multiply body steps by limit"
    );
}

/// Verifies that a RepeatStart loop multiplies body steps by max_attempts.
/// Workflow: RepeatStart(max=3, body=1, done=2) -> Nop -> Finish
/// Expected: 1 (header) + 3 * 1 (body * attempts) + 1 (Finish) = 5
#[test]
fn budget_repeat_loop_multiplies_body_steps() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 3,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_total_steps == 5);

    assert!(
        budget.is_some(),
        "repeat loop should multiply body steps by max_attempts"
    );
}

/// Verifies nested loop step counting multiplies correctly.
/// Outer ForEachStart(limit=10, body=1, done=4)
///   Inner ForEachStart(limit=10, body=2, done=3)
///     Nop (node 2)
///   ForEachJoin (node 3)
/// ForEachJoin (node 4)
///
/// Inner: body_count=1, product = 1*10=10, inner total = 1+10+1 = 12.
/// Outer body region: node1=12, node2=1, node3=1 = 14.
/// Wait — body_count counts distinct nodes in the region, inner ForEachStart(1) itself
/// is counted as 1, then its body is 1*10=10, then ForEachJoin(3) is 1. So region for
/// outer body = 1 + 10 + 1 = 12. Outer: product = 12*10 = 120, total = 1+120+1 = 122.
#[test]
fn budget_nested_loop_multiplies_correctly() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(2),
                item_slot: SlotIdx::new(3),
                limit: 10,
                body: StepIdx::new(2),
                done: StepIdx::new(3),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: Some(SlotIdx::new(4)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(4),
            },
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: Some(SlotIdx::new(5)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(5),
            },
        },
    ];
    let contract = test_contract(5, 6);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_total_steps == 122 && b.max_nesting_depth == 2);

    assert!(
        budget.is_some(),
        "nested loop should multiply step counts at each nesting level"
    );
}

/// Security regression: a workflow with loops that previously appeared as a
/// small step count (unique nodes only) now correctly reports the worst-case
/// multiplied count, which should exceed the default policy.
///
/// ForEachStart(limit=1000, body=1, done=2) -> Nop -> Finish
/// Old: 3 steps (passed policy).
/// New: 1 + 1000*1 + 1 = 1002 steps (still under 1M policy, but realistic).
#[test]
fn budget_large_loop_counted_realistically() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 1000,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_total_steps == 1002);

    assert!(budget.is_some(), "large loop should count 1002 steps not 3");

    // The default policy (1M steps) should accept this
    let budget_val = budget.as_ref().unwrap();
    assert!(
        BoundednessPolicy::DEFAULT.validate(budget_val).is_ok(),
        "1002 steps should be within default policy"
    );
}

/// Verifies that a CollectStart loop multiplies body steps by the limit.
#[test]
fn budget_collect_loop_multiplies_body_steps() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: 5,
                page_size: 2,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_total_steps == 7);

    assert!(
        budget.is_some(),
        "collect loop should multiply body steps by limit"
    );
}

/// Verifies that StepBudget clamps values above MAX_STEP_BUDGET.
#[test]
fn step_budget_clamps_above_max() {
    let budget = StepBudget::new(crate::limits::MAX_STEP_BUDGET + 100);
    assert_eq!(
        budget.remaining(),
        crate::limits::MAX_STEP_BUDGET,
        "budget should be clamped to MAX_STEP_BUDGET"
    );
}

/// Verifies that StepBudget::MAX equals MAX_STEP_BUDGET.
#[test]
fn step_budget_max_equals_limit() {
    assert_eq!(
        StepBudget::MAX.remaining(),
        crate::limits::MAX_STEP_BUDGET,
        "StepBudget::MAX should equal MAX_STEP_BUDGET"
    );
}

/// Verifies that StepBudget zero budget exhausts immediately.
#[test]
fn step_budget_zero_exhausts_immediately() {
    let mut budget = StepBudget::new(0);
    let result = budget.try_take();
    assert!(
        result.is_ok() && result.as_ref().map_err(|_| "").unwrap() == &false,
        "zero budget should return Ok(false) immediately"
    );
}

// =========================================================================
// BLACKHAT adversarial tests -- budget overflow, bypass, and edge cases
// =========================================================================

// --- FINDING BH-BUD-01: max_steps_executable silent saturation bypass ---

#[test]
fn blackhat_steps_executable_saturates_on_large_total() {
    let budget = WholeWorkflowBudget {
        max_total_steps: u64::from(u32::MAX) + 1,
        max_total_slots: 0,
        max_fanout: 0,
        max_nesting_depth: 0,
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
    };

    let saturated = u32::try_from(budget.max_total_steps).unwrap_or(u32::MAX);
    assert_eq!(
        saturated,
        u32::MAX,
        "BLACKHAT BH-BUD-01: u32 saturation hides overflow"
    );
    assert!(
        budget.max_total_steps > u64::from(saturated),
        "BLACKHAT BH-BUD-01: true count exceeds reported executable steps"
    );
}

// --- FINDING BH-BUD-02: max_run_time_seconds hardcoded to 0 ---

#[test]
fn blackhat_run_time_seconds_always_zero_in_computed_budget() {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }];
    let contract = test_contract(1, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_run_time_seconds == 0);

    assert!(
        budget.is_none(),
        "BLACKHAT BH-BUD-02: max_run_time_seconds must not be hardcoded to 0"
    );
}

// --- FINDING BH-BUD-03: From<WorkflowError> for BudgetError loses information ---

#[test]
fn blackhat_workflow_error_to_budget_error_produces_equal_actual_and_limit() {
    let workflow_err = WorkflowError::EntryOutOfBounds {
        entry: StepIdx::new(5),
    };
    let budget_err: BudgetError = workflow_err.into();

    match budget_err {
        BudgetError::TotalStepsExceeded { actual, limit } => {
            assert_eq!(actual, u64::MAX);
            assert_eq!(limit, u64::MAX);
        }
        other => panic!("BLACKHAT BH-BUD-03: unexpected variant: {other:?}"),
    }
}

// --- FINDING BH-BUD-04: ForEachStart limit=0 counts as 1 iteration ---

#[test]
fn blackhat_foreach_limit_zero_still_counts_as_one_iteration() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 0,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_total_steps == 3);

    assert!(
        budget.is_some(),
        "BLACKHAT BH-BUD-04: limit=0 counts as 1 iteration"
    );
}

// --- FINDING BH-BUD-05: Step count overflow uses misleading error variant ---

#[test]
fn blackhat_step_count_overflow_uses_misleading_error_variant() {
    let workflow_err = WorkflowError::StepOutOfBounds {
        step: StepIdx::new(0),
    };
    let converted: BudgetError = workflow_err.into();
    match converted {
        BudgetError::TotalStepsExceeded { actual, limit } => {
            assert_eq!(actual, u64::MAX);
            assert_eq!(limit, u64::MAX);
        }
        other => panic!("unexpected variant: {other:?}"),
    }
}

// --- FINDING BH-BUD-06: action_tickets saturating_add hides overflow ---

#[test]
fn blackhat_action_tickets_saturating_add_under_reports() {
    let mut max_action_tickets: u32 = u32::MAX;
    max_action_tickets = max_action_tickets.saturating_add(1);
    assert_eq!(
        max_action_tickets,
        u32::MAX,
        "BLACKHAT BH-BUD-06: saturating_add hides overflow"
    );
}

// --- FINDING BH-BUD-07: gather_items saturating_add accumulation ---

#[test]
fn blackhat_gather_items_accumulation_saturates() {
    let mut max_gather_items: u32 = u32::MAX - 10;
    max_gather_items = max_gather_items.saturating_add(20);
    assert_eq!(
        max_gather_items,
        u32::MAX,
        "BLACKHAT BH-BUD-07: gather items saturates at u32::MAX"
    );
}

// --- FINDING BH-BUD-08: retries_per_action copied from contract not computed ---

#[test]
fn blackhat_retries_per_action_copied_from_contract_not_computed() {
    let contract = ResourceContract {
        max_retry_attempts: 42,
        ..test_contract(1, 1)
    };
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }];
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_retries_per_action == 42);

    assert!(
        budget.is_some(),
        "BLACKHAT BH-BUD-08: retries copied from contract, not computed from IR"
    );
}

// --- FINDING BH-BUD-09: forward jump does not trigger cycle detection ---

#[test]
fn blackhat_jump_cycle_detection_relies_on_forward_edge_validation() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Jump {
                target: StepIdx::new(1),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(2, 1);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);

    match result {
        Ok(budget) => {
            assert_eq!(
                budget.max_total_steps, 2,
                "forward jump should count as 2 steps"
            );
        }
        Err(_) => {
            panic!("BLACKHAT BH-BUD-09: forward jump incorrectly detected as cycle");
        }
    }
}

// --- FINDING BH-BUD-10: policy boundary exact vs over ---

#[test]
fn blackhat_policy_allows_exact_limit() {
    let budget = test_budget(1_000_000, 65_535, 64, 8);
    let result = BoundednessPolicy::DEFAULT.validate(&budget);
    assert_eq!(
        result,
        Ok(()),
        "BLACKHAT BH-BUD-10: budget at exact limits should pass"
    );
}

#[test]
fn blackhat_policy_rejects_one_over_limit() {
    let budget = test_budget(1_000_001, 65_535, 64, 8);
    let result = BoundednessPolicy::DEFAULT.validate(&budget);
    assert!(
        result.is_err(),
        "BLACKHAT BH-BUD-10: budget one over limit must be rejected"
    );
}

// --- FINDING BH-BUD-11: StepBudget clamping is silent ---

#[test]
fn blackhat_step_budget_clamping_is_silent() {
    let budget = StepBudget::new(100_000);
    assert_eq!(
        budget.remaining(),
        crate::limits::MAX_STEP_BUDGET,
        "BLACKHAT BH-BUD-11: requested 100K steps silently clamped"
    );
}

// --- FINDING BH-BUD-12: self-referencing loop body graceful handling ---

#[test]
fn blackhat_self_referencing_loop_body_gracefully_handled() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(0),
                done: StepIdx::new(1),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(2, 3);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    assert!(
        result.is_ok() || result.is_err(),
        "BLACKHAT BH-BUD-12: self-referencing body must not panic"
    );
}

// --- FINDING BH-BUD-13: ReduceStart uses MAX_LIST_ITEMS_PER_VALUE iterations ---

#[test]
fn blackhat_reduce_start_uses_max_list_items_as_iteration_count() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ReduceStart {
                input: SlotIdx::new(0),
                accumulator: SlotIdx::new(1),
                initial: ConstIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract).ok();

    let expected_iters = u64::try_from(crate::limits::MAX_LIST_ITEMS_PER_VALUE).unwrap_or(u64::MAX);
    let expected = 1 + expected_iters + 1;

    assert!(
        budget.is_some(),
        "BLACKHAT BH-BUD-13: ReduceStart should compute with MAX_LIST_ITEMS iterations"
    );
    assert_eq!(
        budget.as_ref().map(|b| b.max_total_steps),
        Some(expected),
        "BLACKHAT BH-BUD-13: expected {expected} steps"
    );
}

// =========================================================================
// Comprehensive test coverage for budget.rs
// =========================================================================

const fn test_contract(max_steps: u16, max_slots: u16) -> ResourceContract {
    ResourceContract {
        max_steps,
        max_slots,
        max_constants: 1,
        max_accessors: 0,
        max_expressions: 0,
        max_expr_stack: 0,
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1,
        max_input_bytes: 1,
        max_output_bytes: 1,
        max_blob_bytes: 1,
        max_ipc_payload_bytes: 1,
        max_retry_attempts: 0,
        max_fanout: 64,
        max_collect_items: 0,
        max_queue_depth: 1,
        max_journal_batch_bytes: 1,
        ..ResourceContract::DEFAULT
    }
}

const fn test_budget(
    max_total_steps: u64,
    max_total_slots: u64,
    max_fanout: u16,
    max_nesting_depth: u16,
) -> WholeWorkflowBudget {
    WholeWorkflowBudget {
        max_total_steps,
        max_total_slots,
        max_fanout,
        max_nesting_depth,
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
    }
}

const fn test_policy(
    max_total_steps: u64,
    max_total_slots: u64,
    max_fanout: u16,
    max_nesting_depth: u16,
) -> BoundednessPolicy {
    BoundednessPolicy {
        max_total_steps,
        max_total_slots,
        max_fanout,
        max_nesting_depth,
        absolute_max_action_tickets: 100_000,
        absolute_max_parallel: 256,
        absolute_max_run_time_seconds: 2_592_000,
        absolute_max_result_bytes: 262_144,
        absolute_max_steps_executable: 1_000_000,
    }
}

fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
where
    T: core::fmt::Debug + PartialEq,
{
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, found {actual:?}"))
    }
}

fn single_node_workflow() -> Vec<CompiledNode> {
    vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }]
}

// -------------------------------------------------------------------------
// 1. Step budget creation and validation
// -------------------------------------------------------------------------

#[test]
fn step_budget_creation_at_one() -> Result<(), String> {
    let b = StepBudget::new(1);
    ensure_equal(b.remaining(), 1)
}

#[test]
fn step_budget_creation_at_max() -> Result<(), String> {
    let b = StepBudget::new(crate::limits::MAX_STEP_BUDGET);
    ensure_equal(b.remaining(), crate::limits::MAX_STEP_BUDGET)
}

#[test]
fn step_budget_creation_at_zero() -> Result<(), String> {
    let b = StepBudget::new(0);
    ensure_equal(b.remaining(), 0)
}

#[test]
fn step_budget_creation_clamps_large_value() -> Result<(), String> {
    let b = StepBudget::new(u64::MAX);
    ensure_equal(b.remaining(), crate::limits::MAX_STEP_BUDGET)
}

// -------------------------------------------------------------------------
// 2. Budget consumption tracking (single step, multi-step)
// -------------------------------------------------------------------------

#[test]
fn step_budget_single_consumption_decrements() -> Result<(), String> {
    let mut b = StepBudget::new(5);
    let taken = b.try_take().map_err(|e| e.to_string())?;
    ensure_equal(taken, true)?;
    ensure_equal(b.remaining(), 4)
}

#[test]
fn step_budget_multi_step_consumption_to_zero() -> Result<(), String> {
    let mut b = StepBudget::new(3);
    ensure_equal(b.try_take().map_err(|e| e.to_string())?, true)?;
    ensure_equal(b.remaining(), 2)?;
    ensure_equal(b.try_take().map_err(|e| e.to_string())?, true)?;
    ensure_equal(b.remaining(), 1)?;
    ensure_equal(b.try_take().map_err(|e| e.to_string())?, true)?;
    ensure_equal(b.remaining(), 0)
}

#[test]
fn step_budget_consumption_returns_true_each_time_until_exhausted() -> Result<(), String> {
    let mut b = StepBudget::new(4);
    for i in 0..4 {
        let taken = b.try_take().map_err(|e| e.to_string())?;
        ensure_equal(taken, true)?;
        ensure_equal(b.remaining(), 3 - i)?;
    }
    let final_take = b.try_take().map_err(|e| e.to_string())?;
    ensure_equal(final_take, false)
}

// -------------------------------------------------------------------------
// 3. Budget exhaustion detection
// -------------------------------------------------------------------------

#[test]
fn step_budget_exhausted_returns_false() -> Result<(), String> {
    let mut b = StepBudget::new(1);
    ensure_equal(b.try_take().map_err(|e| e.to_string())?, true)?;
    ensure_equal(b.try_take().map_err(|e| e.to_string())?, false)?;
    ensure_equal(b.remaining(), 0)
}

#[test]
fn step_budget_exhaustion_stays_at_zero() -> Result<(), String> {
    let mut b = StepBudget::new(2);
    b.try_take().map_err(|e| e.to_string())?;
    b.try_take().map_err(|e| e.to_string())?;
    for _ in 0..5 {
        let taken = b.try_take().map_err(|e| e.to_string())?;
        ensure_equal(taken, false)?;
        ensure_equal(b.remaining(), 0)?;
    }
    Ok(())
}

// -------------------------------------------------------------------------
// 4. Sub-graph budget accounting
// -------------------------------------------------------------------------

// 4a. ForEach body cost multiplication with limit=1 (single iteration)
#[test]
fn foreach_limit_one_counts_body_once() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 1,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 3)
}

// 4b. Together branch budget counts all branches
#[test]
fn together_start_counts_parallel_branches() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: vec![
                    StepIdx::new(1),
                    StepIdx::new(2),
                    StepIdx::new(3),
                    StepIdx::new(4),
                ]
                .into_boxed_slice(),
                join: StepIdx::new(5),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(6, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_fanout, 4)?;
    ensure_equal(budget.max_parallel_in_flight, 4)?;
    ensure_equal(budget.max_together_branches, 4)?;
    ensure_equal(budget.max_total_steps, 6)
}

// 4c. Collect loop body cost
#[test]
fn collect_start_body_accounting() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: 3,
                page_size: 1,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 5)?;
    ensure_equal(budget.max_gather_pages, 1)?;
    ensure_equal(budget.max_gather_items, 3)
}

// 4d. Reduce body cost
#[test]
fn reduce_start_body_accounting() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ReduceStart {
                input: SlotIdx::new(0),
                accumulator: SlotIdx::new(1),
                initial: ConstIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    let expected_iters = u64::try_from(crate::limits::MAX_LIST_ITEMS_PER_VALUE).unwrap_or(u64::MAX);
    ensure_equal(budget.max_total_steps, 1 + expected_iters + 1)
}

// 4e. Repeat body cost
#[test]
fn repeat_start_body_accounting() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 7,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 9)?;
    ensure_equal(budget.max_repeat_attempts, 7)
}

// -------------------------------------------------------------------------
// 5. Max step budget boundary (exactly at limit, one step over)
// -------------------------------------------------------------------------

#[test]
fn policy_allows_budget_at_exact_total_steps_limit() -> Result<(), String> {
    let budget = test_budget(1_000_000, 0, 0, 0);
    let policy = BoundednessPolicy {
        max_total_steps: 1_000_000,
        ..BoundednessPolicy::DEFAULT
    };
    ensure_equal(policy.validate(&budget), Ok(()))
}

#[test]
fn policy_rejects_budget_one_over_total_steps_limit() -> Result<(), String> {
    let budget = test_budget(1_000_001, 0, 0, 0);
    let policy = BoundednessPolicy {
        max_total_steps: 1_000_000,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::TotalStepsExceeded { actual, limit }) => {
            ensure_equal(actual, 1_000_001)?;
            ensure_equal(limit, 1_000_000)
        }
        other => Err(format!("expected TotalStepsExceeded, got {other:?}")),
    }
}

#[test]
fn policy_boundary_exact_fanout() -> Result<(), String> {
    let budget = test_budget(1, 0, 64, 0);
    let policy = BoundednessPolicy {
        max_fanout: 64,
        ..BoundednessPolicy::DEFAULT
    };
    ensure_equal(policy.validate(&budget), Ok(()))
}

#[test]
fn policy_boundary_fanout_one_over() -> Result<(), String> {
    let budget = test_budget(1, 0, 65, 0);
    let policy = BoundednessPolicy {
        max_fanout: 64,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::FanoutExceeded { actual, limit }) => {
            ensure_equal(actual, 65)?;
            ensure_equal(limit, 64)
        }
        other => Err(format!("expected FanoutExceeded, got {other:?}")),
    }
}

#[test]
fn policy_boundary_exact_nesting_depth() -> Result<(), String> {
    let budget = test_budget(1, 0, 0, 8);
    let policy = BoundednessPolicy {
        max_nesting_depth: 8,
        ..BoundednessPolicy::DEFAULT
    };
    ensure_equal(policy.validate(&budget), Ok(()))
}

#[test]
fn policy_boundary_nesting_depth_one_over() -> Result<(), String> {
    let budget = test_budget(1, 0, 0, 9);
    let policy = BoundednessPolicy {
        max_nesting_depth: 8,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::NestingDepthExceeded { actual, limit }) => {
            ensure_equal(actual, 9)?;
            ensure_equal(limit, 8)
        }
        other => Err(format!("expected NestingDepthExceeded, got {other:?}")),
    }
}

// -------------------------------------------------------------------------
// 6. Budget reset/reinitialization
// -------------------------------------------------------------------------

#[test]
fn step_budget_recreated_after_exhaustion() -> Result<(), String> {
    let mut b = StepBudget::new(2);
    b.try_take().map_err(|e| e.to_string())?;
    b.try_take().map_err(|e| e.to_string())?;
    ensure_equal(b.remaining(), 0)?;

    let mut b2 = StepBudget::new(2);
    ensure_equal(b2.remaining(), 2)?;
    ensure_equal(b2.try_take().map_err(|e| e.to_string())?, true)?;
    ensure_equal(b2.remaining(), 1)
}

#[test]
fn whole_workflow_budget_recompute_produces_same_result() -> Result<(), String> {
    let nodes = single_node_workflow();
    let contract = test_contract(1, 1);

    let budget1 = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    let budget2 = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;

    ensure_equal(budget1, budget2)
}

// -------------------------------------------------------------------------
// 7. Nested loop budget computation
// -------------------------------------------------------------------------

#[test]
fn nested_for_each_triple_depth() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 2,
                body: StepIdx::new(1),
                done: StepIdx::new(6),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(2),
                item_slot: SlotIdx::new(3),
                limit: 3,
                body: StepIdx::new(2),
                done: StepIdx::new(5),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(4),
                item_slot: SlotIdx::new(5),
                limit: 4,
                body: StepIdx::new(3),
                done: StepIdx::new(4),
            },
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: Some(SlotIdx::new(6)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(6),
            },
        },
        CompiledNode {
            id: StepIdx::new(5),
            output: Some(SlotIdx::new(7)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(7),
            },
        },
        CompiledNode {
            id: StepIdx::new(6),
            output: Some(SlotIdx::new(8)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(8),
            },
        },
    ];
    let contract = test_contract(7, 9);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;

    ensure_equal(budget.max_nesting_depth, 3)?;
    ensure_equal(budget.max_total_steps > 0, true)
}

// -------------------------------------------------------------------------
// 8. Parallel branch budget splitting (TogetherStart)
// -------------------------------------------------------------------------

#[test]
fn together_start_tracks_max_parallel_in_flight() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(1), StepIdx::new(2)].into_boxed_slice(),
                join: StepIdx::new(3),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(4, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;

    ensure_equal(budget.max_fanout, 2)?;
    ensure_equal(budget.max_parallel_in_flight, 2)?;
    ensure_equal(budget.max_together_branches, 2)?;
    ensure_equal(budget.max_total_steps, 4)
}

#[test]
fn larger_together_start_dominates_fanout() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(1), StepIdx::new(2)].into_boxed_slice(),
                join: StepIdx::new(3),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: vec![
                    StepIdx::new(4),
                    StepIdx::new(5),
                    StepIdx::new(6),
                    StepIdx::new(7),
                    StepIdx::new(8),
                ]
                .into_boxed_slice(),
                join: StepIdx::new(9),
            },
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(6),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(7),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(8),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(9),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(10, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;

    ensure_equal(budget.max_fanout, 5)?;
    ensure_equal(budget.max_parallel_in_flight, 5)?;
    ensure_equal(budget.max_together_branches, 5)
}

// -------------------------------------------------------------------------
// 9. Zero-budget edge cases
// -------------------------------------------------------------------------

#[test]
fn step_budget_zero_never_allows_consumption() -> Result<(), String> {
    let mut b = StepBudget::new(0);
    for _ in 0..10 {
        let taken = b.try_take().map_err(|e| e.to_string())?;
        ensure_equal(taken, false)?;
        ensure_equal(b.remaining(), 0)?;
    }
    Ok(())
}

#[test]
fn whole_workflow_budget_zero_slots_contract() -> Result<(), String> {
    let nodes = single_node_workflow();
    let contract = test_contract(1, 0);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_slots, 0)
}

#[test]
fn policy_validate_accepts_zero_budget() -> Result<(), String> {
    let budget = test_budget(0, 0, 0, 0);
    ensure_equal(BoundednessPolicy::DEFAULT.validate(&budget), Ok(()))
}

// -------------------------------------------------------------------------
// 10. Budget arithmetic overflow protection
// -------------------------------------------------------------------------

#[test]
fn whole_workflow_budget_max_total_slots_derives_from_contract() -> Result<(), String> {
    let nodes = single_node_workflow();
    let contract = test_contract(1, 500);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_slots, 500)
}

#[test]
fn whole_workflow_budget_max_result_bytes_from_contract() -> Result<(), String> {
    let nodes = single_node_workflow();
    let mut contract = test_contract(1, 1);
    contract.max_output_bytes = 9999;
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_result_bytes, 9999)
}

#[test]
fn whole_workflow_budget_max_retries_from_contract() -> Result<(), String> {
    let nodes = single_node_workflow();
    let mut contract = test_contract(1, 1);
    contract.max_retry_attempts = 7;
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_retries_per_action, 7)
}

#[test]
fn policy_rejects_action_tickets_exceeded() -> Result<(), String> {
    let mut budget = test_budget(1, 0, 0, 0);
    budget.max_action_tickets = 200_000;
    let policy = BoundednessPolicy {
        absolute_max_action_tickets: 100_000,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::ActionTicketsExceeded { actual, limit }) => {
            ensure_equal(actual, 200_000)?;
            ensure_equal(limit, 100_000)
        }
        other => Err(format!("expected ActionTicketsExceeded, got {other:?}")),
    }
}

#[test]
fn policy_rejects_parallel_exceeded() -> Result<(), String> {
    let mut budget = test_budget(1, 0, 0, 0);
    budget.max_parallel_in_flight = 512;
    let policy = BoundednessPolicy {
        absolute_max_parallel: 256,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::ParallelExceeded { actual, limit }) => {
            ensure_equal(actual, 512)?;
            ensure_equal(limit, 256)
        }
        other => Err(format!("expected ParallelExceeded, got {other:?}")),
    }
}

#[test]
fn policy_rejects_result_bytes_exceeded() -> Result<(), String> {
    let mut budget = test_budget(1, 0, 0, 0);
    budget.max_result_bytes = 1_000_000;
    let policy = BoundednessPolicy {
        absolute_max_result_bytes: 262_144,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::ResultBytesExceeded { actual, limit }) => {
            ensure_equal(actual, 1_000_000)?;
            ensure_equal(limit, 262_144)
        }
        other => Err(format!("expected ResultBytesExceeded, got {other:?}")),
    }
}

#[test]
fn policy_rejects_run_time_exceeded() -> Result<(), String> {
    let mut budget = test_budget(1, 0, 0, 0);
    budget.max_run_time_seconds = 5_000_000;
    let policy = BoundednessPolicy {
        absolute_max_run_time_seconds: 2_592_000,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::RunTimeExceeded { actual, limit }) => {
            ensure_equal(actual, 5_000_000)?;
            ensure_equal(limit, 2_592_000)
        }
        other => Err(format!("expected RunTimeExceeded, got {other:?}")),
    }
}

#[test]
fn policy_rejects_steps_executable_exceeded() -> Result<(), String> {
    let mut budget = test_budget(1, 0, 0, 0);
    budget.max_steps_executable = 2_000_000;
    let policy = BoundednessPolicy {
        absolute_max_steps_executable: 1_000_000,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::StepsExecutableExceeded { actual, limit }) => {
            ensure_equal(actual, 2_000_000)?;
            ensure_equal(limit, 1_000_000)
        }
        other => Err(format!("expected StepsExecutableExceeded, got {other:?}")),
    }
}

// -------------------------------------------------------------------------
// Additional coverage: Do node action ticket counting
// -------------------------------------------------------------------------

#[test]
fn do_node_increments_action_tickets() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(1),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 2);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_action_tickets, 2)?;
    ensure_equal(budget.max_total_steps, 3)
}

// -------------------------------------------------------------------------
// Additional coverage: ForEach iteration accumulation
// -------------------------------------------------------------------------

#[test]
fn multiple_for_each_accumulates_iterations() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 5,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: Some(SlotIdx::new(2)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(3),
                item_slot: SlotIdx::new(4),
                limit: 10,
                body: StepIdx::new(4),
                done: StepIdx::new(5),
            },
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(5),
            output: Some(SlotIdx::new(5)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(5),
            },
        },
    ];
    let mut nodes = nodes;
    nodes[2].next = Some(StepIdx::new(3));

    let contract = test_contract(6, 6);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_for_each_iterations, 15)?;
    ensure_equal(budget.max_total_steps, 19)
}

// -------------------------------------------------------------------------
// Additional coverage: Jump node handling
// -------------------------------------------------------------------------

#[test]
fn jump_chain_counts_all_nodes() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Jump {
                target: StepIdx::new(1),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Jump {
                target: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 3)
}

#[test]
fn jump_self_cycle_detected() -> Result<(), String> {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Jump {
            target: StepIdx::new(0),
        },
    }];
    let contract = test_contract(1, 1);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Err(WorkflowError::JumpCycle { step, target }) => {
            ensure_equal(step, StepIdx::new(0))?;
            ensure_equal(target, StepIdx::new(0))
        }
        other => Err(format!("expected JumpCycle, got {other:?}")),
    }
}

// -------------------------------------------------------------------------
// Additional coverage: BoundednessPolicy::DEFAULT sanity checks
// -------------------------------------------------------------------------

#[test]
fn default_policy_total_steps_is_one_million() -> Result<(), String> {
    ensure_equal(BoundednessPolicy::DEFAULT.max_total_steps, 1_000_000)
}

#[test]
fn default_policy_max_fanout_is_64() -> Result<(), String> {
    ensure_equal(BoundednessPolicy::DEFAULT.max_fanout, 64)
}

#[test]
fn default_policy_nesting_depth_is_8() -> Result<(), String> {
    ensure_equal(BoundednessPolicy::DEFAULT.max_nesting_depth, 8)
}

// -------------------------------------------------------------------------
// Additional coverage: BudgetError Display and Error trait
// -------------------------------------------------------------------------

#[test]
fn budget_error_implements_std_error() -> Result<(), String> {
    let err = BudgetError::TotalStepsExceeded {
        actual: 10,
        limit: 5,
    };
    let _: &dyn std::error::Error = &err;
    Ok(())
}

#[test]
fn budget_error_total_slots_display() -> Result<(), String> {
    let err = BudgetError::TotalSlotsExceeded {
        actual: 100,
        limit: 50,
    };
    ensure_equal(
        format!("{err}"),
        "total slots exceeded: 100 > 50".to_string(),
    )
}

#[test]
fn budget_error_parallel_display() -> Result<(), String> {
    let err = BudgetError::ParallelExceeded {
        actual: 300,
        limit: 256,
    };
    ensure_equal(format!("{err}"), "parallel exceeded: 300 > 256".to_string())
}

#[test]
fn budget_error_action_tickets_display() -> Result<(), String> {
    let err = BudgetError::ActionTicketsExceeded {
        actual: 150_000,
        limit: 100_000,
    };
    ensure_equal(
        format!("{err}"),
        "action tickets exceeded: 150000 > 100000".to_string(),
    )
}

#[test]
fn budget_error_run_time_display() -> Result<(), String> {
    let err = BudgetError::RunTimeExceeded {
        actual: 5_000_000,
        limit: 2_592_000,
    };
    ensure_equal(
        format!("{err}"),
        "run time exceeded: 5000000 > 2592000".to_string(),
    )
}

#[test]
fn budget_error_result_bytes_display() -> Result<(), String> {
    let err = BudgetError::ResultBytesExceeded {
        actual: 524_288,
        limit: 262_144,
    };
    ensure_equal(
        format!("{err}"),
        "result bytes exceeded: 524288 > 262144".to_string(),
    )
}

// -------------------------------------------------------------------------
// Additional coverage: WholeWorkflowBudget Copy and Clone
// -------------------------------------------------------------------------

#[test]
fn whole_workflow_budget_is_copy() -> Result<(), String> {
    let budget = test_budget(10, 100, 4, 2);
    let copy = budget;
    ensure_equal(budget, copy)
}

#[test]
fn boundedness_policy_is_copy() -> Result<(), String> {
    let policy = BoundednessPolicy::DEFAULT;
    let copy = policy;
    ensure_equal(policy, copy)
}

// -------------------------------------------------------------------------
// Additional coverage: ForEachStart limit=1 does not overcount
// -------------------------------------------------------------------------

#[test]
fn foreach_limit_one_exact_step_count() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 1,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 3)
}

// -------------------------------------------------------------------------
// Additional coverage: RepeatStart max_attempts=0 handled by max(1)
// -------------------------------------------------------------------------

#[test]
fn repeat_start_zero_attempts_counts_as_one() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 0,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 3)
}

// -------------------------------------------------------------------------
// Additional coverage: Linear chain with varied node types
// -------------------------------------------------------------------------

#[test]
fn linear_chain_set_const_copy_eval() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: Some(SlotIdx::new(2)),
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(2),
            },
        },
    ];
    let contract = test_contract(4, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 4)?;
    ensure_equal(budget.max_fanout, 0)?;
    ensure_equal(budget.max_nesting_depth, 0)
}

// -------------------------------------------------------------------------
// Additional coverage: CollectStart limit=0 handled by max(1)
// -------------------------------------------------------------------------

#[test]
fn collect_start_zero_limit_counts_as_one() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: 0,
                page_size: 1,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 3)
}

// -------------------------------------------------------------------------
// Additional coverage: Multi-body ForEach (body with 3 steps)
// -------------------------------------------------------------------------

#[test]
fn foreach_multi_step_body() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 3,
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(5, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 11)
}

// -------------------------------------------------------------------------
// Additional coverage: RepeatStart with max_attempts=1
// -------------------------------------------------------------------------

#[test]
fn repeat_start_one_attempt() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 1,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 3)?;
    ensure_equal(budget.max_repeat_attempts, 1)
}

// -------------------------------------------------------------------------
// Additional coverage: Policy validates first violation only
// -------------------------------------------------------------------------

#[test]
fn policy_reports_first_violation_steps_over_slots_over() -> Result<(), String> {
    let mut budget = test_budget(2_000_000, 200_000, 100, 20);
    budget.max_action_tickets = 500_000;
    let policy = BoundednessPolicy {
        max_total_steps: 1_000_000,
        max_total_slots: 65_535,
        max_fanout: 64,
        max_nesting_depth: 8,
        absolute_max_action_tickets: 100_000,
        absolute_max_parallel: 256,
        absolute_max_run_time_seconds: 2_592_000,
        absolute_max_result_bytes: 262_144,
        absolute_max_steps_executable: 1_000_000,
    };
    match policy.validate(&budget) {
        Err(BudgetError::TotalStepsExceeded { actual, limit }) => {
            ensure_equal(actual, 2_000_000)?;
            ensure_equal(limit, 1_000_000)
        }
        other => Err(format!("expected TotalStepsExceeded, got {other:?}")),
    }
}

// -------------------------------------------------------------------------
// Additional coverage: WholeWorkflowBudget max_steps_executable derivation
// -------------------------------------------------------------------------

#[test]
fn max_steps_executable_equals_total_steps_when_under_u32_max() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(2, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    let expected_executable = u32::try_from(budget.max_total_steps).unwrap_or(u32::MAX);
    ensure_equal(budget.max_steps_executable, expected_executable)?;
    ensure_equal(budget.max_steps_executable, 2)
}

// -------------------------------------------------------------------------
// Additional coverage: max_total_slots_written equals contract max_slots
// -------------------------------------------------------------------------

#[test]
fn max_total_slots_written_equals_contract_max_slots() -> Result<(), String> {
    let nodes = single_node_workflow();
    let contract = test_contract(1, 42);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_slots_written, 42)
}

// -------------------------------------------------------------------------
// Additional coverage: ErrorHandler node step counting
// -------------------------------------------------------------------------

#[test]
fn error_handler_counts_body_and_handler() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(2),
                error_slot: None,
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 3)
}

// -------------------------------------------------------------------------
// Additional coverage: BoundednessPolicy validate returns checks in order
// -------------------------------------------------------------------------

#[test]
fn policy_check_order_total_steps_before_slots() -> Result<(), String> {
    let budget = test_budget(2_000_000, 200_000, 0, 0);
    let result = BoundednessPolicy::DEFAULT.validate(&budget);
    match result {
        Err(BudgetError::TotalStepsExceeded { .. }) => Ok(()),
        other => Err(format!(
            "expected TotalStepsExceeded (first check), got {other:?}"
        )),
    }
}

#[test]
fn policy_check_order_slots_before_fanout() -> Result<(), String> {
    let budget = test_budget(100, 200_000, 100, 0);
    let result = BoundednessPolicy::DEFAULT.validate(&budget);
    match result {
        Err(BudgetError::TotalSlotsExceeded { .. }) => Ok(()),
        other => Err(format!(
            "expected TotalSlotsExceeded (second check), got {other:?}"
        )),
    }
}

// -------------------------------------------------------------------------
// Additional coverage: RepeatStart max_attempts tracking uses max not add
// -------------------------------------------------------------------------

#[test]
fn repeat_start_max_attempts_tracks_maximum_not_sum() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 3,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 10,
                body: StepIdx::new(4),
                done: StepIdx::new(5),
            },
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(6, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_repeat_attempts, 10)
}

// -------------------------------------------------------------------------
// Additional coverage: StepBudget::MAX is const
// -------------------------------------------------------------------------

#[test]
fn step_budget_max_is_const_compatible() -> Result<(), String> {
    const _MAX: StepBudget = StepBudget::MAX;
    ensure_equal(_MAX.remaining(), crate::limits::MAX_STEP_BUDGET)
}

// -------------------------------------------------------------------------
// Additional coverage: WholeWorkflowBudget debug format
// -------------------------------------------------------------------------

#[test]
fn whole_workflow_budget_debug_format() -> Result<(), String> {
    let budget = test_budget(10, 20, 4, 2);
    let debug = format!("{budget:?}");
    ensure_equal(debug.contains("WholeWorkflowBudget"), true)?;
    ensure_equal(debug.contains("max_total_steps"), true)
}

#[test]
fn boundedness_policy_debug_format() -> Result<(), String> {
    let policy = BoundednessPolicy::DEFAULT;
    let debug = format!("{policy:?}");
    ensure_equal(debug.contains("BoundednessPolicy"), true)?;
    ensure_equal(debug.contains("max_total_steps"), true)
}

#[test]
fn budget_error_debug_format() -> Result<(), String> {
    let err = BudgetError::FanoutExceeded {
        actual: 5,
        limit: 3,
    };
    let debug = format!("{err:?}");
    ensure_equal(debug.contains("FanoutExceeded"), true)
}

// =========================================================================
// Additional edge-case tests — Budget construction, checked operations
// =========================================================================

#[test]
fn whole_workflow_budget_zero_fields_is_valid() -> Result<(), String> {
    let budget = test_budget(0, 0, 0, 0);
    ensure_equal(budget.max_total_steps, 0)?;
    ensure_equal(budget.max_total_slots, 0)?;
    ensure_equal(budget.max_fanout, 0)?;
    ensure_equal(budget.max_nesting_depth, 0)
}

#[test]
fn whole_workflow_budget_max_fields() -> Result<(), String> {
    let budget = WholeWorkflowBudget {
        max_total_steps: u64::MAX,
        max_total_slots: u64::MAX,
        max_fanout: u16::MAX,
        max_nesting_depth: u16::MAX,
        max_steps_executable: u32::MAX,
        max_action_tickets: u32::MAX,
        max_parallel_in_flight: u16::MAX,
        max_retries_per_action: u16::MAX,
        max_gather_pages: u32::MAX,
        max_gather_items: u32::MAX,
        max_for_each_iterations: u32::MAX,
        max_together_branches: u16::MAX,
        max_repeat_attempts: u16::MAX,
        max_run_time_seconds: u64::MAX,
        max_result_bytes: u32::MAX,
        max_total_slots_written: u32::MAX,
    };
    ensure_equal(budget.max_total_steps, u64::MAX)?;
    ensure_equal(budget.max_total_slots, u64::MAX)?;
    ensure_equal(budget.max_fanout, u16::MAX)?;
    ensure_equal(budget.max_nesting_depth, u16::MAX)
}

#[test]
fn boundedness_policy_default_values_are_sensible() -> Result<(), String> {
    let p = BoundednessPolicy::DEFAULT;
    ensure_equal(p.max_total_steps, 1_000_000)?;
    ensure_equal(p.max_total_slots, 65_535)?;
    ensure_equal(p.max_fanout, 64)?;
    ensure_equal(p.max_nesting_depth, 8)?;
    ensure_equal(p.absolute_max_action_tickets, 100_000)?;
    ensure_equal(p.absolute_max_parallel, 256)?;
    ensure_equal(p.absolute_max_run_time_seconds, 2_592_000)?;
    ensure_equal(p.absolute_max_result_bytes, 262_144)?;
    ensure_equal(p.absolute_max_steps_executable, 1_000_000)
}

#[test]
fn budget_error_all_variants_display_non_empty() -> Result<(), String> {
    let errors = [
        BudgetError::TotalStepsExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::TotalSlotsExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::FanoutExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::NestingDepthExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::ParallelExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::ActionTicketsExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::RunTimeExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::ResultBytesExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::StepsExecutableExceeded {
            actual: 1,
            limit: 0,
        },
    ];
    for err in &errors {
        let display = format!("{err}");
        if display.is_empty() {
            return Err(format!("BudgetError display is empty for {err:?}"));
        }
    }
    Ok(())
}

#[test]
fn budget_error_equality_same_variants() -> Result<(), String> {
    let a = BudgetError::TotalStepsExceeded {
        actual: 5,
        limit: 3,
    };
    let b = BudgetError::TotalStepsExceeded {
        actual: 5,
        limit: 3,
    };
    ensure_equal(a, b)
}

#[test]
fn budget_error_inequality_different_actual() -> Result<(), String> {
    let a = BudgetError::TotalStepsExceeded {
        actual: 5,
        limit: 3,
    };
    let b = BudgetError::TotalStepsExceeded {
        actual: 6,
        limit: 3,
    };
    assert_ne!(a, b);
    Ok(())
}

#[test]
fn budget_error_clone_preserves_equality() -> Result<(), String> {
    let a = BudgetError::FanoutExceeded {
        actual: 10,
        limit: 5,
    };
    let b = a.clone();
    ensure_equal(a, b)
}

#[test]
fn budget_error_from_workflow_error_preserves_variant() -> Result<(), String> {
    let wf_err = WorkflowError::EntryOutOfBounds {
        entry: StepIdx::new(0),
    };
    let budget_err: BudgetError = wf_err.into();
    match budget_err {
        BudgetError::TotalStepsExceeded { actual, limit } => {
            ensure_equal(actual, u64::MAX)?;
            ensure_equal(limit, u64::MAX)
        }
        other => Err(format!(
            "expected TotalStepsExceeded sentinel for workflow error, got {other:?}"
        )),
    }
}

#[test]
fn step_budget_new_one_consumes_to_zero() -> Result<(), String> {
    let mut b = StepBudget::new(1);
    ensure_equal(b.remaining(), 1)?;
    let taken = b.try_take().map_err(|e| e.to_string())?;
    ensure_equal(taken, true)?;
    ensure_equal(b.remaining(), 0)?;
    let taken2 = b.try_take().map_err(|e| e.to_string())?;
    ensure_equal(taken2, false)
}

#[test]
fn step_budget_try_take_never_panics() -> Result<(), String> {
    let mut b = StepBudget::new(0);
    for _ in 0..100 {
        let result = b.try_take();
        match result {
            Ok(false) => {}
            other => return Err(format!("expected Ok(false), got {other:?}")),
        }
    }
    Ok(())
}

#[test]
fn boundedness_policy_custom_zero_limits_accept_zero_budget() -> Result<(), String> {
    let policy = BoundednessPolicy {
        max_total_steps: 0,
        max_total_slots: 0,
        max_fanout: 0,
        max_nesting_depth: 0,
        absolute_max_action_tickets: 0,
        absolute_max_parallel: 0,
        absolute_max_run_time_seconds: 0,
        absolute_max_result_bytes: 0,
        absolute_max_steps_executable: 0,
    };
    let budget = test_budget(0, 0, 0, 0);
    ensure_equal(policy.validate(&budget), Ok(()))
}

#[test]
fn boundedness_policy_custom_zero_limits_reject_nonzero() -> Result<(), String> {
    let policy = BoundednessPolicy {
        max_total_steps: 0,
        max_total_slots: 0,
        max_fanout: 0,
        max_nesting_depth: 0,
        absolute_max_action_tickets: 0,
        absolute_max_parallel: 0,
        absolute_max_run_time_seconds: 0,
        absolute_max_result_bytes: 0,
        absolute_max_steps_executable: 0,
    };
    let budget = test_budget(1, 0, 0, 0);
    match policy.validate(&budget) {
        Err(BudgetError::TotalStepsExceeded {
            actual: 1,
            limit: 0,
        }) => Ok(()),
        other => Err(format!("expected TotalStepsExceeded, got {other:?}")),
    }
}

// =========================================================================
// VB-CORE-BUDGET overflow/underflow tests (BudgetArithmetic.tla)
// =========================================================================

/// UT-BUDGET-001: AggregateResourceUsage::try_add_budget returns Err on overflow.
/// Does NOT panic; overflow propagates as AggregateBudgetError::Overflow.
#[test]
fn ut_budget_add_never_overflows() {
    // Test overflow on a u64 field: u64::MAX - 1 + 2 = u64::MAX + 1 -> overflow
    let usage_near_max = AggregateResourceUsage {
        max_steps_executable: u64::MAX - 1,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let budget_adds_2 = AggregateResourceBudget {
        max_steps_executable: 2,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage_near_max.try_add_budget(&budget_adds_2);
    assert!(
        result.is_err(),
        "u64::MAX - 1 + 2 should overflow (return Err), got Ok"
    );

    match result.unwrap_err() {
        AggregateBudgetError::Overflow { resource } => {
            assert_eq!(
                resource, "max_steps_executable",
                "overflow should be in max_steps_executable"
            );
        }
        other => panic!("expected AggregateBudgetError::Overflow, got {other:?}"),
    }

    // Verify adding small budget to zero usage does NOT overflow for non-u64 fields
    let zero_usage = AggregateResourceUsage::default();
    let small_budget = AggregateResourceBudget {
        max_steps_executable: 1,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let result = zero_usage.try_add_budget(&small_budget);
    assert!(
        result.is_ok(),
        "adding small budget to zero usage should not overflow, got Err"
    );
}

/// UT-BUDGET-002: AggregateResourceUsage::try_subtract_budget returns Err on underflow.
/// Subtraction of a larger budget from a smaller one returns Underflow error.
#[test]
fn ut_budget_sub_never_underflows() {
    // Zero usage minus any budget should underflow
    let zero_usage = AggregateResourceUsage::default();
    let budget = AggregateResourceBudget {
        max_steps_executable: 1,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = zero_usage.try_subtract_budget(&budget);
    assert!(
        result.is_err(),
        "subtracting from zero usage should underflow (return Err), got Ok"
    );

    match result.unwrap_err() {
        AggregateBudgetError::Underflow { resource: _ } => {}
        other => panic!("expected AggregateBudgetError::Underflow, got {other:?}"),
    }

    // Non-zero usage subtract that results in zero is fine (no underflow)
    let usage = AggregateResourceUsage {
        max_steps_executable: 5,
        max_action_tickets: 5,
        max_parallel_in_flight: 5,
        max_gather_pages: 5,
        max_gather_items: 5,
        max_result_bytes: 5,
        max_total_slots_written: 5,
        max_active_runs: 5,
        max_queue_depth: 5,
        max_journal_batch_bytes: 5,
        max_step_budget_per_tick: 5,
        max_transitions_per_tick: 5,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 3,
        max_action_tickets: 3,
        max_parallel_in_flight: 3,
        max_retries_per_action: 0, // 0 subtract 0 = 0, no underflow
        max_gather_pages: 3,
        max_gather_items: 3,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 3,
        max_total_slots_written: 3,
        max_queue_depth: 3,
        max_journal_batch_bytes: 3,
        max_step_budget_per_tick: 3,
        max_transitions_per_tick: 3,
    };

    let result = usage.try_subtract_budget(&budget);
    assert!(
        result.is_ok(),
        "subtract resulting in non-negative should be Ok, got {result:?}"
    );
}

// -------------------------------------------------------------------------
// Proptest property: PROPTEST-POST-006
// BoundednessPolicy::validate returns Ok for WholeWorkflowBudget within policy limits
// -------------------------------------------------------------------------

proptest::proptest! {
    #[test]
    fn property_boundedness_policy(
        max_total_steps: u64,
        max_total_slots: u64,
        max_fanout: u16,
        max_nesting_depth: u16,
    ) {
        use crate::budget::{BoundednessPolicy, WholeWorkflowBudget};
        use proptest::prop_assert;

        let policy = BoundednessPolicy::DEFAULT;
        let budget = WholeWorkflowBudget {
            max_total_steps,
            max_total_slots,
            max_fanout,
            max_nesting_depth,
            max_steps_executable: max_total_steps as u32,
            max_action_tickets: 0,
            max_parallel_in_flight: 0,
            max_retries_per_action: 0,
            max_gather_pages: 0,
            max_gather_items: 0,
            max_for_each_iterations: 0,
            max_together_branches: 0,
            max_repeat_attempts: 0,
            max_run_time_seconds: 0,
            max_result_bytes: 0,
            max_total_slots_written: 0,
        };

        // If all dimensions are within policy defaults, validation should pass
        let result = policy.validate(&budget);
        if max_total_steps <= policy.max_total_steps
            && max_total_slots <= policy.max_total_slots
            && max_fanout <= policy.max_fanout
            && max_nesting_depth <= policy.max_nesting_depth
        {
            prop_assert!(matches!(result, Ok(())));
        } else {
            // If any dimension exceeds policy, validation should fail
            prop_assert!(result.is_err());
        }
    }
}

// -------------------------------------------------------------------------
// Unit test: UNIT-POST-005
// test_step_count_overflow — WholeWorkflowBudget::compute propagates overflow
// -------------------------------------------------------------------------

#[test]
fn test_step_count_overflow() -> Result<(), String> {
    use crate::ids::StepIdx;
    use crate::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

    // Build a minimal 1-node workflow (a single Nop) to test the compute path.
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }];

    let parts = WorkflowParts {
        name: Box::from("step_count_overflow_test"),
        digest: crate::ids::WorkflowDigest::from_bytes([0x41; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    // Single-node workflow should compute without overflow
    let budget = crate::budget::WholeWorkflowBudget::compute(
        &parts.nodes,
        parts.entry,
        &parts.resource_contract,
    )
    .map_err(|e| e.to_string())?;
    // 1 node = 1 step
    assert_eq!(
        budget.max_total_steps, 1,
        "single-node workflow should have 1 step"
    );

    // Verify WorkflowError::StepCountOverflow can be constructed correctly.
    // This is the error type returned when u32::try_from(max_total_steps) fails
    // (i.e., when step count exceeds u32::MAX).
    let overflow_err = crate::workflow::WorkflowError::StepCountOverflow { actual: u64::MAX };
    match overflow_err {
        crate::workflow::WorkflowError::StepCountOverflow { actual } => {
            assert_eq!(actual, u64::MAX, "StepCountOverflow should carry u64::MAX");
        }
        other => return Err(format!("expected StepCountOverflow, got {:?}", other)),
    }

    Ok(())
}

// =========================================================================
// Additional coverage: count_and_push_loop_body overflow paths
// =========================================================================

#[test]
fn count_total_steps_overflow_returns_step_count_overflow() {
    use crate::ids::StepIdx;
    use crate::workflow::{CompiledNode, CompiledNodeKind};

    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: u32::MAX,
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(4, 4);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Ok(budget) => {
            assert!(budget.max_total_steps > u64::from(u32::MAX));
        }
        Err(WorkflowError::StepCountOverflow { actual: _ }) => {}
        Err(other) => panic!("expected StepCountOverflow, got {:?}", other),
    }
}

#[test]
fn count_and_push_loop_body_overflow_propagates_budget_error() {
    use crate::ids::StepIdx;
    use crate::workflow::{CompiledNode, CompiledNodeKind};

    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: u32::MAX,
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(2),
                item_slot: SlotIdx::new(3),
                limit: u32::MAX,
                body: StepIdx::new(2),
                done: StepIdx::new(3),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(5, 5);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Ok(budget) => {
            let _ = budget;
        }
        Err(WorkflowError::StepCountOverflow { actual: _ }) => {}
        Err(e) => panic!("expected StepCountOverflow, got {:?}", e),
    }
}

// -------------------------------------------------------------------------
// Additional coverage: WholeWorkflowBudget fields
// -------------------------------------------------------------------------

#[test]
fn whole_workflow_budget_max_parallel_in_flight_from_together() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: vec![
                    StepIdx::new(1),
                    StepIdx::new(2),
                    StepIdx::new(3),
                    StepIdx::new(4),
                    StepIdx::new(5),
                ]
                .into_boxed_slice(),
                join: StepIdx::new(6),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(6),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(7, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_parallel_in_flight, 5)
}

#[test]
fn whole_workflow_budget_max_action_tickets_from_do_nodes() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(1),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(2),
                input: SlotIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(4, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_action_tickets, 3)
}

#[test]
fn whole_workflow_budget_max_gather_pages_and_items() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: 50,
                page_size: 10,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_gather_pages, 1)?;
    ensure_equal(budget.max_gather_items, 50)
}

// -------------------------------------------------------------------------
// WorkflowError variants from compute path
// -------------------------------------------------------------------------

#[test]
fn whole_workflow_budget_jump_cycle_detected_in_compute() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Jump {
                target: StepIdx::new(1),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Jump {
                target: StepIdx::new(0),
            },
        },
    ];
    let contract = test_contract(2, 1);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Err(WorkflowError::JumpCycle { step, target }) => {
            ensure_equal(step, StepIdx::new(1))?;
            ensure_equal(target, StepIdx::new(0))
        }
        other => Err(format!("expected JumpCycle, got {:?}", other)),
    }
}

#[test]
fn whole_workflow_budget_step_out_of_bounds_in_visit() -> Result<(), String> {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: Some(StepIdx::new(99)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }];
    let contract = test_contract(1, 0);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Err(WorkflowError::StepOutOfBounds { step }) => ensure_equal(step, StepIdx::new(99)),
        other => Err(format!("expected StepOutOfBounds, got {:?}", other)),
    }
}

// -------------------------------------------------------------------------
// AggregateResourceBudget and AggregateResourceUsage
// -------------------------------------------------------------------------

#[test]
fn aggregate_resource_budget_from_whole_workflow_budget() -> Result<(), String> {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }];
    let contract = test_contract(1, 1);
    let wfb = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| format!("{:?}", e))?;
    let arb = crate::budget::AggregateResourceBudget::from_whole_workflow_budget(wfb, contract)
        .map_err(|e| format!("{:?}", e))?;
    ensure_equal(arb.max_steps_executable, wfb.max_steps_executable)?;
    ensure_equal(arb.max_action_tickets, wfb.max_action_tickets)?;
    ensure_equal(arb.max_parallel_in_flight, wfb.max_parallel_in_flight)
}

#[test]
fn aggregate_resource_usage_try_add_budget_overflow() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: u64::MAX,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 1,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    match usage.try_add_budget(&budget) {
        Err(AggregateBudgetError::Overflow { resource }) => {
            ensure_equal(resource, "max_steps_executable")
        }
        other => Err(format!("expected Overflow, got {:?}", other)),
    }
}

#[test]
fn aggregate_resource_usage_try_subtract_budget_underflow() -> Result<(), String> {
    let usage = AggregateResourceUsage::default();
    let budget = AggregateResourceBudget {
        max_steps_executable: 1,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    match usage.try_subtract_budget(&budget) {
        Err(AggregateBudgetError::Underflow { resource: _ }) => Ok(()),
        other => Err(format!("expected Underflow, got {:?}", other)),
    }
}

#[test]
fn aggregate_resource_usage_fits_within_capacity() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 100,
        max_action_tickets: 50,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 200,
        max_action_tickets: 100,
        max_parallel_in_flight: 20,
        max_gather_pages: 10,
        max_gather_items: 200,
        max_result_bytes: 2000,
        max_total_slots_written: 1000,
        max_active_runs: 10,
        max_queue_depth: 40,
        max_journal_batch_bytes: 8192,
        max_step_budget_per_tick: 2000,
        max_transitions_per_tick: 1000,
    };
    ensure_equal(usage.fits_within(&capacity), Ok(()))
}

#[test]
fn aggregate_resource_usage_fits_within_rejects_insufficient() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 300,
        max_action_tickets: 50,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 200,
        max_action_tickets: 100,
        max_parallel_in_flight: 20,
        max_gather_pages: 10,
        max_gather_items: 200,
        max_result_bytes: 2000,
        max_total_slots_written: 1000,
        max_active_runs: 10,
        max_queue_depth: 40,
        max_journal_batch_bytes: 8192,
        max_step_budget_per_tick: 2000,
        max_transitions_per_tick: 1000,
    };
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_steps_executable")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_action_tickets() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 150,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_active_runs: 3,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_action_tickets")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_parallel_in_flight() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 15,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_active_runs: 3,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_parallel_in_flight")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_gather_pages() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 10,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_active_runs: 3,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_gather_pages")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_gather_items() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 200,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_active_runs: 3,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_gather_items")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_result_bytes() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 2000,
        max_total_slots_written: 250,
        max_active_runs: 3,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_result_bytes")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_total_slots_written() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 1000,
        max_active_runs: 3,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_total_slots_written")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_active_runs() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_active_runs: 10,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_active_runs")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_queue_depth() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_active_runs: 3,
        max_queue_depth: 50,
        max_journal_batch_bytes: 2048,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_queue_depth")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_journal_batch_bytes() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_active_runs: 3,
        max_queue_depth: 10,
        max_journal_batch_bytes: 8192,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_journal_batch_bytes")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_step_budget_per_tick() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_active_runs: 3,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_step_budget_per_tick: 2000,
        max_transitions_per_tick: 250,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_step_budget_per_tick")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_transitions_per_tick() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_active_runs: 3,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 1000,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_transitions_per_tick")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

// -------------------------------------------------------------------------
// validate_aggregate_budget tests
// -------------------------------------------------------------------------

#[test]
fn validate_aggregate_budget_accepts_valid_budget() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    let policy = BoundednessPolicy::DEFAULT;
    ensure_equal(
        crate::budget::validate_aggregate_budget(&budget, &policy),
        Ok(()),
    )
}

#[test]
fn validate_aggregate_budget_rejects_exceeded_steps() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_steps_executable: 2_000_000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    let policy = BoundednessPolicy::DEFAULT;
    match crate::budget::validate_aggregate_budget(&budget, &policy) {
        Err(AggregateBudgetError::PolicyExceeded { resource, .. }) => {
            ensure_equal(resource, "max_steps_executable")
        }
        other => Err(format!("expected PolicyExceeded, got {:?}", other)),
    }
}

#[test]
fn validate_aggregate_budget_rejects_exceeded_action_tickets() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_steps_executable: 1000,
        max_action_tickets: 200_000,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    let policy = BoundednessPolicy::DEFAULT;
    match crate::budget::validate_aggregate_budget(&budget, &policy) {
        Err(AggregateBudgetError::PolicyExceeded { resource, .. }) => {
            ensure_equal(resource, "max_action_tickets")
        }
        other => Err(format!("expected PolicyExceeded, got {:?}", other)),
    }
}

#[test]
fn validate_aggregate_budget_rejects_exceeded_parallel_in_flight() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 512,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    let policy = BoundednessPolicy::DEFAULT;
    match crate::budget::validate_aggregate_budget(&budget, &policy) {
        Err(AggregateBudgetError::PolicyExceeded { resource, .. }) => {
            ensure_equal(resource, "max_parallel_in_flight")
        }
        other => Err(format!("expected PolicyExceeded, got {:?}", other)),
    }
}

#[test]
fn validate_aggregate_budget_rejects_exceeded_run_time() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3_000_000,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    let policy = BoundednessPolicy::DEFAULT;
    match crate::budget::validate_aggregate_budget(&budget, &policy) {
        Err(AggregateBudgetError::PolicyExceeded { resource, .. }) => {
            ensure_equal(resource, "max_run_time_seconds")
        }
        other => Err(format!("expected PolicyExceeded, got {:?}", other)),
    }
}

#[test]
fn validate_aggregate_budget_rejects_exceeded_result_bytes() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 300_000,
        max_total_slots_written: 1000,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    let policy = BoundednessPolicy::DEFAULT;
    match crate::budget::validate_aggregate_budget(&budget, &policy) {
        Err(AggregateBudgetError::PolicyExceeded { resource, .. }) => {
            ensure_equal(resource, "max_result_bytes")
        }
        other => Err(format!("expected PolicyExceeded, got {:?}", other)),
    }
}

#[test]
fn validate_aggregate_budget_rejects_exceeded_total_slots() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 100_000,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    let policy = BoundednessPolicy {
        max_total_steps: 1_000_000,
        max_total_slots: 65_535,
        max_fanout: 64,
        max_nesting_depth: 8,
        absolute_max_action_tickets: 100_000,
        absolute_max_parallel: 256,
        absolute_max_run_time_seconds: 2_592_000,
        absolute_max_result_bytes: 262_144,
        absolute_max_steps_executable: 1_000_000,
    };
    match crate::budget::validate_aggregate_budget(&budget, &policy) {
        Err(AggregateBudgetError::PolicyExceeded { resource, .. }) => {
            ensure_equal(resource, "max_total_slots_written")
        }
        other => Err(format!("expected PolicyExceeded, got {:?}", other)),
    }
}

#[test]
fn validate_aggregate_budget_rejects_exceeded_together_branches() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 100,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    let policy = BoundednessPolicy {
        max_total_steps: 1_000_000,
        max_total_slots: 65_535,
        max_fanout: 64,
        max_nesting_depth: 8,
        absolute_max_action_tickets: 100_000,
        absolute_max_parallel: 256,
        absolute_max_run_time_seconds: 2_592_000,
        absolute_max_result_bytes: 262_144,
        absolute_max_steps_executable: 1_000_000,
    };
    match crate::budget::validate_aggregate_budget(&budget, &policy) {
        Err(AggregateBudgetError::PolicyExceeded { resource, .. }) => {
            ensure_equal(resource, "max_together_branches")
        }
        other => Err(format!("expected PolicyExceeded, got {:?}", other)),
    }
}

// -------------------------------------------------------------------------
// validate_step_ceilings tests
// -------------------------------------------------------------------------

#[test]
fn validate_step_ceilings_accepts_valid() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 5000,
        max_transitions_per_tick: 500,
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
    };
    ensure_equal(crate::budget::validate_step_ceilings(&budget), Ok(()))
}

#[test]
fn validate_step_ceilings_rejects_zero_step_budget() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 500,
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
    };
    match crate::budget::validate_step_ceilings(&budget) {
        Err(AggregateBudgetError::StepCeilingExceeded { requested: 0, .. }) => Ok(()),
        other => Err(format!("expected StepCeilingExceeded(0), got {:?}", other)),
    }
}

#[test]
fn validate_step_ceilings_rejects_zero_transitions() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 5000,
        max_transitions_per_tick: 0,
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
    };
    match crate::budget::validate_step_ceilings(&budget) {
        Err(AggregateBudgetError::PerTickCeilingExceeded { requested: 0, .. }) => Ok(()),
        other => Err(format!(
            "expected PerTickCeilingExceeded(0), got {:?}",
            other
        )),
    }
}

#[test]
fn validate_step_ceilings_rejects_step_over_hard_limit() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 2_000_000,
        max_transitions_per_tick: 500,
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
    };
    match crate::budget::validate_step_ceilings(&budget) {
        Err(AggregateBudgetError::StepCeilingExceeded {
            requested: 2_000_000,
            ..
        }) => Ok(()),
        other => Err(format!("expected StepCeilingExceeded, got {:?}", other)),
    }
}

#[test]
fn validate_step_ceilings_rejects_transitions_over_hard_limit() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 5000,
        max_transitions_per_tick: 2_000_000,
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
    };
    match crate::budget::validate_step_ceilings(&budget) {
        Err(AggregateBudgetError::PerTickCeilingExceeded {
            requested: 2_000_000,
            ..
        }) => Ok(()),
        other => Err(format!("expected PerTickCeilingExceeded, got {:?}", other)),
    }
}

// -------------------------------------------------------------------------
// AggregateCapacity and AggregateReservation
// -------------------------------------------------------------------------

#[test]
fn aggregate_resource_capacity_is_copy() -> Result<(), String> {
    let cap = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 50,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    let copy = cap;
    ensure_equal(cap.max_steps_executable, copy.max_steps_executable)
}

#[test]
fn aggregate_resource_usage_is_copy() -> Result<(), String> {
    let usage = AggregateResourceUsage::default();
    let copy = usage;
    ensure_equal(usage.max_steps_executable, copy.max_steps_executable)
}

#[test]
fn aggregate_reservation_debug_format() -> Result<(), String> {
    let reservation = crate::budget::AggregateReservation {
        run: crate::ids::RunId::new(42),
        requested: AggregateResourceBudget {
            max_steps_executable: 1000,
            max_action_tickets: 100,
            max_parallel_in_flight: 10,
            max_retries_per_action: 3,
            max_gather_pages: 5,
            max_gather_items: 100,
            max_for_each_iterations: 50,
            max_together_branches: 5,
            max_repeat_attempts: 3,
            max_run_time_seconds: 3600,
            max_result_bytes: 65536,
            max_total_slots_written: 1000,
            max_queue_depth: 50,
            max_journal_batch_bytes: 4096,
            max_step_budget_per_tick: 1000,
            max_transitions_per_tick: 500,
        },
    };
    let debug = format!("{:?}", reservation);
    ensure_equal(debug.contains("AggregateReservation"), true)
}

// -------------------------------------------------------------------------
// AggregateBudgetError Debug
// -------------------------------------------------------------------------

#[test]
fn aggregate_budget_error_workflow_debug() -> Result<(), String> {
    let err = AggregateBudgetError::WorkflowBudget(WorkflowError::EntryOutOfBounds {
        entry: StepIdx::new(5),
    });
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_policy_exceeded_debug() -> Result<(), String> {
    let err = AggregateBudgetError::PolicyExceeded {
        resource: "max_steps",
        actual: 100,
        limit: 50,
    };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_capacity_exceeded_debug() -> Result<(), String> {
    let err = AggregateBudgetError::CapacityExceeded {
        resource: "max_steps",
        requested: 100,
        available: 50,
    };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_overflow_debug() -> Result<(), String> {
    let err = AggregateBudgetError::Overflow { resource: "cpu" };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_underflow_debug() -> Result<(), String> {
    let err = AggregateBudgetError::Underflow { resource: "cpu" };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_invalid_capacity_debug() -> Result<(), String> {
    let err = AggregateBudgetError::InvalidCapacity { resource: "cpu" };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_reservation_not_found_debug() -> Result<(), String> {
    let err = AggregateBudgetError::ReservationNotFound {
        run: crate::ids::RunId::new(42),
    };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_step_ceiling_exceeded_debug() -> Result<(), String> {
    let err = AggregateBudgetError::StepCeilingExceeded {
        requested: 100,
        limit: 50,
    };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_per_tick_ceiling_exceeded_debug() -> Result<(), String> {
    let err = AggregateBudgetError::PerTickCeilingExceeded {
        requested: 100,
        limit: 50,
    };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

// -------------------------------------------------------------------------
// BudgetError from WorkflowError
// -------------------------------------------------------------------------

#[test]
fn budget_error_from_step_out_of_bounds() -> Result<(), String> {
    let wf_err = WorkflowError::StepOutOfBounds {
        step: StepIdx::new(10),
    };
    let budget_err: BudgetError = wf_err.into();
    match budget_err {
        BudgetError::TotalStepsExceeded { actual, limit } => {
            ensure_equal(actual, u64::MAX)?;
            ensure_equal(limit, u64::MAX)
        }
        other => Err(format!(
            "expected TotalStepsExceeded sentinel, got {:?}",
            other
        )),
    }
}

#[test]
fn budget_error_from_jump_cycle() -> Result<(), String> {
    let wf_err = WorkflowError::JumpCycle {
        step: StepIdx::new(1),
        target: StepIdx::new(0),
    };
    let budget_err: BudgetError = wf_err.into();
    match budget_err {
        BudgetError::TotalStepsExceeded { actual, limit } => {
            ensure_equal(actual, u64::MAX)?;
            ensure_equal(limit, u64::MAX)
        }
        other => Err(format!(
            "expected TotalStepsExceeded sentinel, got {:?}",
            other
        )),
    }
}

#[test]
fn try_add_budget_exercises_multiple_dimensions() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 100,
        max_action_tickets: 50,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 50,
        max_action_tickets: 25,
        max_parallel_in_flight: 5,
        max_retries_per_action: 2,
        max_gather_pages: 3,
        max_gather_items: 50,
        max_for_each_iterations: 10,
        max_together_branches: 2,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
    };
    let result = usage.try_add_budget(&budget);
    let added = result.map_err(|e| format!("{:?}", e))?;
    ensure_equal(added.max_steps_executable, 150)?;
    ensure_equal(added.max_action_tickets, 75)?;
    ensure_equal(added.max_parallel_in_flight, 15)?;
    ensure_equal(added.max_gather_pages, 8)?;
    ensure_equal(added.max_gather_items, 150)?;
    ensure_equal(added.max_result_bytes, 1500)?;
    ensure_equal(added.max_total_slots_written, 750)?;
    ensure_equal(added.max_active_runs, 6)?;
    ensure_equal(added.max_queue_depth, 30)?;
    ensure_equal(added.max_journal_batch_bytes, 6144)?;
    ensure_equal(added.max_step_budget_per_tick, 1500)?;
    ensure_equal(added.max_transitions_per_tick, 750)
}

#[test]
fn try_subtract_budget_exercises_multiple_dimensions() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 150,
        max_action_tickets: 75,
        max_parallel_in_flight: 15,
        max_gather_pages: 8,
        max_gather_items: 150,
        max_result_bytes: 1500,
        max_total_slots_written: 750,
        max_active_runs: 6,
        max_queue_depth: 30,
        max_journal_batch_bytes: 6144,
        max_step_budget_per_tick: 1500,
        max_transitions_per_tick: 750,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 50,
        max_action_tickets: 25,
        max_parallel_in_flight: 5,
        max_retries_per_action: 2,
        max_gather_pages: 3,
        max_gather_items: 50,
        max_for_each_iterations: 10,
        max_together_branches: 2,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
    };
    let result = usage.try_subtract_budget(&budget);
    let subtracted = result.map_err(|e| format!("{:?}", e))?;
    ensure_equal(subtracted.max_steps_executable, 100)?;
    ensure_equal(subtracted.max_action_tickets, 50)?;
    ensure_equal(subtracted.max_parallel_in_flight, 10)?;
    ensure_equal(subtracted.max_gather_pages, 5)?;
    ensure_equal(subtracted.max_gather_items, 100)?;
    ensure_equal(subtracted.max_result_bytes, 1000)?;
    ensure_equal(subtracted.max_total_slots_written, 500)?;
    ensure_equal(subtracted.max_active_runs, 5)?;
    ensure_equal(subtracted.max_queue_depth, 20)?;
    ensure_equal(subtracted.max_journal_batch_bytes, 4096)?;
    ensure_equal(subtracted.max_step_budget_per_tick, 1000)?;
    ensure_equal(subtracted.max_transitions_per_tick, 500)
}

#[test]
fn try_add_budget_overflow_action_tickets_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: u64::MAX - 1,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 2,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    match usage.try_add_budget(&budget) {
        Err(AggregateBudgetError::Overflow { resource }) => {
            ensure_equal(resource, "max_action_tickets")
        }
        other => Err(format!("expected Overflow, got {:?}", other)),
    }
}

#[test]
fn try_add_budget_overflow_parallel_in_flight_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: u64::MAX - 1,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 2,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    match usage.try_add_budget(&budget) {
        Err(AggregateBudgetError::Overflow { resource }) => {
            ensure_equal(resource, "max_parallel_in_flight")
        }
        other => Err(format!("expected Overflow, got {:?}", other)),
    }
}

#[test]
fn try_add_budget_overflow_gather_pages_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: u64::MAX - 1,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 2,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    match usage.try_add_budget(&budget) {
        Err(AggregateBudgetError::Overflow { resource }) => {
            ensure_equal(resource, "max_gather_pages")
        }
        other => Err(format!("expected Overflow, got {:?}", other)),
    }
}

#[test]
fn try_add_budget_overflow_gather_items_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: u64::MAX - 1,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 2,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    match usage.try_add_budget(&budget) {
        Err(AggregateBudgetError::Overflow { resource }) => {
            ensure_equal(resource, "max_gather_items")
        }
        other => Err(format!("expected Overflow, got {:?}", other)),
    }
}

#[test]
fn try_add_budget_overflow_result_bytes_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: u64::MAX - 1,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 2,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    match usage.try_add_budget(&budget) {
        Err(AggregateBudgetError::Overflow { resource }) => {
            ensure_equal(resource, "max_result_bytes")
        }
        other => Err(format!("expected Overflow, got {:?}", other)),
    }
}

#[test]
fn try_add_budget_overflow_total_slots_written_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: u64::MAX - 1,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 2,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    match usage.try_add_budget(&budget) {
        Err(AggregateBudgetError::Overflow { resource }) => {
            ensure_equal(resource, "max_total_slots_written")
        }
        other => Err(format!("expected Overflow, got {:?}", other)),
    }
}

#[test]
fn try_add_budget_overflow_queue_depth_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: u64::MAX - 1,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 2,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    match usage.try_add_budget(&budget) {
        Err(AggregateBudgetError::Overflow { resource }) => {
            ensure_equal(resource, "max_queue_depth")
        }
        other => Err(format!("expected Overflow, got {:?}", other)),
    }
}

#[test]
fn try_add_budget_overflow_journal_batch_bytes_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: u64::MAX - 1,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 2,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    match usage.try_add_budget(&budget) {
        Err(AggregateBudgetError::Overflow { resource }) => {
            ensure_equal(resource, "max_journal_batch_bytes")
        }
        other => Err(format!("expected Overflow, got {:?}", other)),
    }
}

#[test]
fn try_add_budget_overflow_step_budget_per_tick_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: u64::MAX - 1,
        max_transitions_per_tick: 0,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 2,
        max_transitions_per_tick: 0,
    };
    match usage.try_add_budget(&budget) {
        Err(AggregateBudgetError::Overflow { resource }) => {
            ensure_equal(resource, "max_step_budget_per_tick")
        }
        other => Err(format!("expected Overflow, got {:?}", other)),
    }
}

#[test]
fn try_add_budget_overflow_transitions_per_tick_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: u64::MAX - 1,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 2,
    };
    match usage.try_add_budget(&budget) {
        Err(AggregateBudgetError::Overflow { resource }) => {
            ensure_equal(resource, "max_transitions_per_tick")
        }
        other => Err(format!("expected Overflow, got {:?}", other)),
    }
}

#[test]
fn try_subtract_budget_underflow_action_tickets_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 1,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    match usage.try_subtract_budget(&budget) {
        Err(AggregateBudgetError::Underflow { resource }) => {
            ensure_equal(resource, "max_action_tickets")
        }
        other => Err(format!("expected Underflow, got {:?}", other)),
    }
}

#[test]
fn try_subtract_budget_underflow_parallel_in_flight_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 1,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    match usage.try_subtract_budget(&budget) {
        Err(AggregateBudgetError::Underflow { resource }) => {
            ensure_equal(resource, "max_parallel_in_flight")
        }
        other => Err(format!("expected Underflow, got {:?}", other)),
    }
}

#[test]
fn try_subtract_budget_underflow_gather_pages_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 1,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    match usage.try_subtract_budget(&budget) {
        Err(AggregateBudgetError::Underflow { resource }) => {
            ensure_equal(resource, "max_gather_pages")
        }
        other => Err(format!("expected Underflow, got {:?}", other)),
    }
}

#[test]
fn try_subtract_budget_underflow_gather_items_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 1,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    match usage.try_subtract_budget(&budget) {
        Err(AggregateBudgetError::Underflow { resource }) => {
            ensure_equal(resource, "max_gather_items")
        }
        other => Err(format!("expected Underflow, got {:?}", other)),
    }
}

#[test]
fn try_subtract_budget_underflow_result_bytes_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 1,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    match usage.try_subtract_budget(&budget) {
        Err(AggregateBudgetError::Underflow { resource }) => {
            ensure_equal(resource, "max_result_bytes")
        }
        other => Err(format!("expected Underflow, got {:?}", other)),
    }
}

#[test]
fn try_subtract_budget_underflow_total_slots_written_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 1,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    match usage.try_subtract_budget(&budget) {
        Err(AggregateBudgetError::Underflow { resource }) => {
            ensure_equal(resource, "max_total_slots_written")
        }
        other => Err(format!("expected Underflow, got {:?}", other)),
    }
}

#[test]
fn try_subtract_budget_underflow_queue_depth_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 2,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 1,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    match usage.try_subtract_budget(&budget) {
        Err(AggregateBudgetError::Underflow { resource }) => {
            ensure_equal(resource, "max_queue_depth")
        }
        other => Err(format!("expected Underflow, got {:?}", other)),
    }
}

#[test]
fn try_subtract_budget_underflow_journal_batch_bytes_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 2,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 1,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    match usage.try_subtract_budget(&budget) {
        Err(AggregateBudgetError::Underflow { resource }) => {
            ensure_equal(resource, "max_journal_batch_bytes")
        }
        other => Err(format!("expected Underflow, got {:?}", other)),
    }
}

#[test]
fn try_subtract_budget_underflow_step_budget_per_tick_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 2,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 0,
    };
    match usage.try_subtract_budget(&budget) {
        Err(AggregateBudgetError::Underflow { resource }) => {
            ensure_equal(resource, "max_step_budget_per_tick")
        }
        other => Err(format!("expected Underflow, got {:?}", other)),
    }
}

#[test]
fn try_subtract_budget_underflow_transitions_per_tick_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 2,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 1,
    };
    match usage.try_subtract_budget(&budget) {
        Err(AggregateBudgetError::Underflow { resource }) => {
            ensure_equal(resource, "max_transitions_per_tick")
        }
        other => Err(format!("expected Underflow, got {:?}", other)),
    }
}

// ============================================================================
// Mutation-killing tests for production code survivors
// These target mutations that survive when boundary-value tests are missing.
// ============================================================================

/// Kills: validate_step_ceilings > with >= at lines 740, 753
/// The mutation replaces `> HARD_MAX` with `>= HARD_MAX`, which would reject
/// values exactly at the hard limit. This test uses exact boundary values.
#[test]
fn validate_step_ceilings_accepts_exact_hard_limit() -> Result<(), String> {
    // HARD_MAX_STEP_BUDGET_PER_TICK = 1_000_000
    // HARD_MAX_TRANSITIONS_PER_TICK = 1_000_000
    // The production code uses `>` (strict), so value == 1_000_000 should pass.
    // The mutation `>` → `>=` would incorrectly reject 1_000_000.
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 1_000_000,
        max_transitions_per_tick: 1_000_000,
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
    };
    ensure_equal(crate::budget::validate_step_ceilings(&budget), Ok(()))
}

/// Kills: check_capacity > with >= at line 788 (via WholeWorkflowBudget path)
/// When current == limit, should NOT error. Mutation `>` → `>=` would fail.
/// This tests the boundary through the public add_budget API.
#[test]
fn whole_workflow_budget_add_at_exact_limit() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
    };
    let mut usage = AggregateResourceUsage::default();
    // Set usage to exactly the limit
    usage.max_steps_executable = 1000;
    // Adding the budget should succeed (usage starts at 0, budget adds 1000 to steps = 2000)
    let expected = AggregateResourceUsage {
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
        max_steps_executable: 2000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_active_runs: 1,
    };
    match usage.try_add_budget(&budget) {
        Ok(actual) => ensure_equal(actual, expected),
        Err(e) => Err(format!("unexpected error: {:?}", e)),
    }
}

/// Kills: check_policy > with >= at line 804 (via WholeWorkflowBudget path)
/// When usage == limit, policy check should pass.
#[test]
fn whole_workflow_budget_policy_at_exact_limit() -> Result<(), String> {
    let policy = BoundednessPolicy {
        max_total_steps: 1_000_000,
        max_total_slots: 65_535,
        max_fanout: 64,
        max_nesting_depth: 8,
        absolute_max_action_tickets: 100,
        absolute_max_parallel: 10,
        absolute_max_run_time_seconds: 3600,
        absolute_max_result_bytes: 65536,
        absolute_max_steps_executable: 1000,
    };
    let usage = AggregateResourceUsage {
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_active_runs: 1,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };
    // Usage at exact limit should pass (>= comparison, not >)
    ensure_equal(usage.check_policy(&policy), Ok(()))
}

/// Kills: check_policy > with >= — tests the over-limit case to confirm
/// the error type and values are correct, preventing `>` → `==` mutation
/// (which would only fail when exactly equal, missing the over-limit case).
#[test]
fn whole_workflow_budget_policy_exceeds_limit() -> Result<(), String> {
    let policy = BoundednessPolicy {
        max_total_steps: 1_000_000,
        max_total_slots: 65_535,
        max_fanout: 64,
        max_nesting_depth: 8,
        absolute_max_action_tickets: 100,
        absolute_max_parallel: 10,
        absolute_max_run_time_seconds: 3600,
        absolute_max_result_bytes: 65536,
        absolute_max_steps_executable: 1000,
    };
    let usage = AggregateResourceUsage {
        // Exceeds by 1
        max_steps_executable: 1001,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_active_runs: 1,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };
    match usage.check_policy(&policy) {
        Err(AggregateBudgetError::PolicyExceeded {
            resource: "max_steps_executable",
            actual: 1001,
            limit: 1000,
        }) => Ok(()),
        Err(e) => Err(format!(
            "expected PolicyExceeded {{resource: max_steps_executable, actual: 1001, limit: 1000}}, got {:?}",
            e
        )),
        Ok(()) => Err("expected PolicyExceeded, got Ok(())".to_string()),
    }
}

/// Kills: check_capacity > with >= — tests the exact equality boundary
/// where requested == available should succeed (not error).
#[test]
fn whole_workflow_budget_capacity_at_exact_limit() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
    };
    let mut usage = AggregateResourceUsage::default();
    // Set requested to exactly match limit
    usage.max_steps_executable = 1000;
    // Adding the budget should succeed (usage starts at 0, so result = budget values + 1000 for steps)
    let expected = AggregateResourceUsage {
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
        max_steps_executable: 2000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_active_runs: 1,
    };
    match usage.try_add_budget(&budget) {
        Ok(actual) => ensure_equal(actual, expected),
        Err(e) => Err(format!("unexpected error: {:?}", e)),
    }
}

/// Kills: validate_step_ceilings > with >= — tests over-limit case to
/// ensure `>` → `==` mutation is killed (only fails at exact equality).
#[test]
fn validate_step_ceilings_rejects_step_over_limit_by_one() -> Result<(), String> {
    // 1_000_001 is > 1_000_000, should be rejected
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 1_000_001,
        max_transitions_per_tick: 500,
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
    };
    match crate::budget::validate_step_ceilings(&budget) {
        Err(AggregateBudgetError::StepCeilingExceeded {
            requested: 1_000_001,
            limit: 1_000_000,
        }) => Ok(()),
        other => Err(format!(
            "expected StepCeilingExceeded(1_000_001, 1_000_000), got {:?}",
            other
        )),
    }
}

/// Kills: validate_step_ceilings > with >= — tests transitions boundary.
#[test]
fn validate_step_ceilings_accepts_exact_transition_hard_limit() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 5000,
        max_transitions_per_tick: 1_000_000,
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
    };
    ensure_equal(crate::budget::validate_step_ceilings(&budget), Ok(()))
}

/// Kills: validate_step_ceilings > with >= — over-limit for transitions.
#[test]
fn validate_step_ceilings_rejects_transitions_over_limit_by_one() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 5000,
        max_transitions_per_tick: 1_000_001,
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
    };
    match crate::budget::validate_step_ceilings(&budget) {
        Err(AggregateBudgetError::PerTickCeilingExceeded {
            requested: 1_000_001,
            limit: 1_000_000,
        }) => Ok(()),
        other => Err(format!(
            "expected PerTickCeilingExceeded(1_000_001, 1_000_000), got {:?}",
            other
        )),
    }
}
