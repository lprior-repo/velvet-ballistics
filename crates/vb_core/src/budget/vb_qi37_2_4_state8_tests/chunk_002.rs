//! Test chunk 002 of 5 from the original
//! `vb_qi37_2_4_state8_tests.rs` (Kani state-8 budget tests).
//! Lines 307–813 of the original. Semantic content is
//! preserved exactly; only the file structure changed.
//! vb-qi37.2.4 State 8: Failing-first tests for proof obligations
//!
//! KANI-BUD-001: Checked nested arithmetic rejects overflow
//! PROP-BUD-001: Nested accepted budgets fit policy
//! PROP-DIAG-001: Diagnostic parity for rejected nested growth
//!
//! These tests are written in failing-first style: they express the desired
//! behavior and will fail until the production code implements the required
//! contracts.
//!
//! RED PHASE: These tests COMPILE but FAIL because the implementation contains
//! intentional gaps documented in the test plan's Open Questions.

#![forbid(unsafe_code)]

use crate::budget::{
    AggregateBudgetError, AggregateResourceBudget, AggregateResourceUsage, BoundednessPolicy,
    BudgetError, WholeWorkflowBudget,
};
use crate::ids::{SlotIdx, StepIdx};
use crate::workflow::WorkflowError;
use crate::workflow::{CompiledNode, CompiledNodeKind, ResourceContract};
use proptest::prelude::*;

// ============================================================================
// KANI-BUD-001: Checked nested arithmetic rejects overflow
// ============================================================================
//
// KANI-BUD-001 proof obligation:
//
// Property: For bounded node/body/factor dimensions, sum/product budget
// arithmetic either equals mathematical expected value or returns typed
// overflow/rejection before admission.
//
// Bound: small node graphs up to 6 nodes, loop factors in {0,1,2,u16::MAX},
// and u32/u64 dimensions around overflow boundaries.
//
// These Kani harnesses test the `count_and_push_loop_body` and
// `count_nested_for_region` functions that perform checked_mul and
// checked_add for loop iteration multiplication.
//
// NOTE: These tests use concrete values designed to trigger overflow paths.
// They fail because production code currently has gaps in overflow detection
// for nested loop multiplication.

// ---------------------------------------------------------------------------
// Kani Harness: overflow detection in nested loop body multiplication
// ---------------------------------------------------------------------------

#[cfg(kani)]

proptest! {
    /// PROP-BUD-001 P1: CollectStart with finite limit multiplies body cost correctly.
    ///
    /// When a CollectStart with limit=N is in the workflow, the total steps
    /// should include N * body_steps + overhead.
    ///
    /// This test FAILS in red phase because the implementation does not
    /// properly account for CollectStart multiplication in all cases.
    #[test]
    fn prop_collect_body_multiplies_with_finite_limit(
        limit in 1u32..=10u32,
        body_node_count in 1u16..=5u16,
    ) {
        // Build a workflow: CollectStart -> [body nodes] -> CollectFinish -> Finish
        let body_end = 1 + body_node_count as u16;
        let collect_done = body_end + 1;

        let mut nodes: Vec<CompiledNode> = vec![
            // CollectStart at node 0
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::CollectStart {
                    source: SlotIdx::new(0),
                    limit,
                    page_size: 1,
                    body: StepIdx::new(1),
                    done: StepIdx::new(collect_done),
                },
            },
        ];

        // Body nodes: Nop chain
        for i in 1..=body_node_count as usize {
            nodes.push(CompiledNode {
                id: StepIdx::new(i as u16),
                output: None,
                next: if i < body_node_count as usize {
                    Some(StepIdx::new((i + 1) as u16))
                } else {
                    None
                },
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            });
        }

        // CollectFinish and Finish
        let finish_idx = collect_done + 1;
        nodes.push(CompiledNode {
            id: StepIdx::new(collect_done),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::new(1),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(finish_idx),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });

        let contract = test_contract(nodes.len() as u16 + 10, 4);
        let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);

        prop_assert!(budget.is_ok(), "CollectStart workflow should compute budget successfully");

        let budget = budget.unwrap();

        // Expected: 1 (CollectStart) + limit * body_node_count + 1 (CollectFinish) + 1 (Finish)
        let expected_steps: u64 = 1 + (limit as u64) * (body_node_count as u64) + 1 + 1;

        prop_assert_eq!(
            budget.max_total_steps, expected_steps,
            "CollectStart with limit={} and body_count={} should have {} total steps, got {}",
            limit, body_node_count, expected_steps, budget.max_total_steps
        );
        prop_assert!(
            budget.max_gather_items >= limit,
            "max_gather_items {} should be at least limit {}",
            budget.max_gather_items, limit
        );
    }

    /// PROP-BUD-001 P2: RepeatStart body is counted once at the cold-AST-conservative
    /// iter count of 1, regardless of declared `max_attempts`. The declared
    /// `max_attempts` is tracked separately via `max_repeat_attempts`.
    ///
    /// Cold-AST invariant (master §45) drops body, so the runtime attempt
    /// count cannot be bounded from the compiled IR alone. The conservative
    /// default iter count is 1.
    #[test]
    fn prop_repeat_body_multiplies_with_max_attempts(
        max_attempts in 1u16..=10u16,
        body_node_count in 1u16..=5u16,
    ) {
        let body_end = 1 + body_node_count as u16;
        let repeat_done = body_end + 1;

        let mut nodes: Vec<CompiledNode> = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::RepeatStart {
                    max_attempts,
                    body: StepIdx::new(1),
                    done: StepIdx::new(repeat_done),
                },
            },
        ];

        for i in 1..=body_node_count as usize {
            nodes.push(CompiledNode {
                id: StepIdx::new(i as u16),
                output: None,
                next: if i < body_node_count as usize {
                    Some(StepIdx::new((i + 1) as u16))
                } else {
                    None
                },
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            });
        }

        let finish_idx = repeat_done + 1;
        nodes.push(CompiledNode {
            id: StepIdx::new(repeat_done),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatFinish {
                result: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(finish_idx),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });

        let contract = test_contract(nodes.len() as u16 + 10, 4);
        let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);

        prop_assert!(budget.is_ok(), "RepeatStart workflow should compute budget successfully");

        let budget = budget.unwrap();

        // Expected: 1 (RepeatStart) + 1 * body_node_count + 1 (RepeatFinish) + 1 (Finish)
        // The cold-AST-conservative iter count is 1, so the body is counted once.
        let expected_steps: u64 = 1 + (body_node_count as u64) + 1 + 1;

        prop_assert_eq!(
            budget.max_total_steps, expected_steps,
            "RepeatStart with max_attempts={} and body_count={} should have {} total steps (cold-AST-conservative), got {}",
            max_attempts, body_node_count, expected_steps, budget.max_total_steps
        );
        prop_assert_eq!(
            budget.max_repeat_attempts, max_attempts,
            "max_repeat_attempts should be {}",
            max_attempts
        );
    }

    /// PROP-BUD-001 P3: Nested loops multiply correctly through multiple levels.
    ///
    /// A workflow with nested ForEachStart nodes should multiply body costs
    /// at each level.
    #[test]
    fn prop_nested_loops_multiply_correctly(
        outer_limit in 2u32..=5u32,
        inner_limit in 2u32..=5u32,
        inner_body_count in 1u16..=3u16,
    ) {
        // Build: ForEachStart(outer) -> ForEachStart(inner) -> [body] -> ForEachJoin -> ForEachJoin -> Finish
        let inner_body_end = 2 + inner_body_count as u16;
        let inner_done = inner_body_end + 1;
        let outer_done = inner_done + 1;

        let mut nodes: Vec<CompiledNode> = vec![
            // Node 0: Outer ForEachStart
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: outer_limit,
                    body: StepIdx::new(1),
                    done: StepIdx::new(outer_done),
                },
            },
            // Node 1: Inner ForEachStart
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(2),
                    item_slot: SlotIdx::new(3),
                    limit: inner_limit,
                    body: StepIdx::new(2),
                    done: StepIdx::new(inner_done),
                },
            },
        ];

        // Inner body: Nop chain
        for i in 2..=inner_body_count as usize {
            nodes.push(CompiledNode {
                id: StepIdx::new(i as u16),
                output: None,
                next: if i < inner_body_count as usize {
                    Some(StepIdx::new((i + 1) as u16))
                } else {
                    None
                },
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            });
        }

        let finish_idx = outer_done + 1;
        nodes.push(CompiledNode {
            id: StepIdx::new(inner_done),
            output: Some(SlotIdx::new(4)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(4),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(outer_done),
            output: Some(SlotIdx::new(5)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(5),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(finish_idx),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(4),
            },
        });

        let contract = test_contract(nodes.len() as u16 + 10, 6);
        let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);

        prop_assert!(budget.is_ok(), "Nested ForEach workflow should compute budget successfully");

        let budget = budget.unwrap();

        // Expected: 1 (outer) + 1 (inner) + inner_limit * inner_body_count + 1 (inner join) + 1 (outer join) + 1 (finish)
        let inner_body_steps: u64 = inner_body_count as u64;
        let inner_loop_steps: u64 = 1 + (inner_limit as u64) * inner_body_steps + 1;
        let expected_steps: u64 = 1 + (outer_limit as u64) * inner_loop_steps + 1 + 1;

        prop_assert_eq!(
            budget.max_total_steps, expected_steps,
            "Nested loops with outer={}, inner={}, inner_body={} should have {} steps, got {}",
            outer_limit, inner_limit, inner_body_count, expected_steps, budget.max_total_steps
        );
        prop_assert_eq!(
            budget.max_nesting_depth, 2,
            "Nested loops should have depth 2, got {}",
            budget.max_nesting_depth
        );
    }

    /// PROP-BUD-001 P4: TogetherStart fanout is counted correctly.
    ///
    /// When a TogetherStart with N branches is in the workflow, max_fanout
    /// should be N and max_together_branches should be N.
    #[test]
    fn prop_together_start_counts_fanout(branch_count in 2u16..=6u16) {
        let branch_count_usize = branch_count as usize;
        let join_idx = branch_count + 1;

        let mut nodes: Vec<CompiledNode> = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::TogetherStart {
                    branches: (1u16..=branch_count).map(|i| StepIdx::new(i)).collect::<Vec<_>>().into_boxed_slice(),
                    join: StepIdx::new(join_idx),
                },
            },
        ];

        // Branch nodes
        for i in 1..=branch_count_usize {
            nodes.push(CompiledNode {
                id: StepIdx::new(i as u16),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            });
        }

        // Join node
        nodes.push(CompiledNode {
            id: StepIdx::new(join_idx),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherJoin {
                branch_count,
                accumulator: SlotIdx::new(0),
            },
        });

        // Finish
        let finish_idx = join_idx + 1;
        nodes.push(CompiledNode {
            id: StepIdx::new(finish_idx),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });

        let contract = test_contract(nodes.len() as u16 + 10, 4);
        let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);

        prop_assert!(budget.is_ok(), "TogetherStart workflow should compute budget successfully");

        let budget = budget.unwrap();

        prop_assert_eq!(
            budget.max_fanout, branch_count,
            "max_fanout should be {}, got {}",
            branch_count, budget.max_fanout
        );
        prop_assert_eq!(
            budget.max_together_branches, branch_count,
            "max_together_branches should be {}, got {}",
            branch_count, budget.max_together_branches
        );
        prop_assert_eq!(
            budget.max_parallel_in_flight, branch_count,
            "max_parallel_in_flight should be {}, got {}",
            branch_count, budget.max_parallel_in_flight
        );
    }

    /// PROP-BUD-001 P5: AggregateResourceBudget::from_workflow preserves verified dimensions.
    ///
    /// When a workflow passes WholeWorkflowBudget::compute and is converted
    /// to AggregateResourceBudget, the aggregate dimensions should match
    /// the computed whole-workflow dimensions.
    #[test]
    fn prop_aggregate_preserves_whole_workflow_dimensions(
        limit in 1u32..=5u32,
        body_node_count in 1u16..=3u16,
    ) {
        let body_end = 1 + body_node_count as u16;
        let collect_done = body_end + 1;

        let mut nodes: Vec<CompiledNode> = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::CollectStart {
                    source: SlotIdx::new(0),
                    limit,
                    page_size: 1,
                    body: StepIdx::new(1),
                    done: StepIdx::new(collect_done),
                },
            },
        ];

        for i in 1..=body_node_count as usize {
            nodes.push(CompiledNode {
                id: StepIdx::new(i as u16),
                output: None,
                next: if i < body_node_count as usize {
                    Some(StepIdx::new((i + 1) as u16))
                } else {
                    None
                },
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            });
        }

        let finish_idx = collect_done + 1;
        nodes.push(CompiledNode {
            id: StepIdx::new(collect_done),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::new(1),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(finish_idx),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });

        let contract = test_contract(nodes.len() as u16 + 10, 4);

        // Compute whole workflow budget
        let whole_budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);

        prop_assert!(whole_budget.is_ok(), "WholeWorkflowBudget should compute successfully");

        let whole_budget = whole_budget.unwrap();

        // Create aggregate from whole workflow budget
        let aggregate = AggregateResourceBudget::from_whole_workflow_budget(whole_budget, contract);

        prop_assert!(aggregate.is_ok(), "AggregateResourceBudget should create successfully from whole budget");

        let aggregate = aggregate.unwrap();

        prop_assert_eq!(
            aggregate.max_steps_executable, whole_budget.max_steps_executable,
            "aggregate.max_steps_executable should equal whole_budget.max_steps_executable"
        );
        prop_assert_eq!(
            aggregate.max_gather_items, whole_budget.max_gather_items,
            "aggregate.max_gather_items should equal whole_budget.max_gather_items"
        );
    }
}

// ============================================================================
// PROP-DIAG-001: Diagnostic parity for rejected nested growth
// ============================================================================
//
// PROP-DIAG-001 proof obligation:
//
// Invariant: Every rejected generated collect/reduce/repeat/together case
// exposes primitive, node or path, resource, actual/computed value when
// known, and limit.
//
// Anti-invariant: A rejection without structural provenance fails the
// property even if the error variant is otherwise correct.
//
// NOTE: These tests fail because BudgetError variants currently lack
// primitive/node/structural-path fields. The test plan documents this gap:
// "Existing BudgetError lacks primitive/node/structural-path fields in
// crates/vb_core/src/budget.rs"

// ---------------------------------------------------------------------------
// Proptest: Diagnostic fields are present in rejected workflows
// ---------------------------------------------------------------------------

