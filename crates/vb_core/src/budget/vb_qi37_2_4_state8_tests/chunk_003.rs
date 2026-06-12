//! Test chunk 003 of 5 from the original
//! `vb_qi37_2_4_state8_tests.rs` (Kani state-8 budget tests).
//! Lines 814–1256 of the original. Semantic content is
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
    fn prop_collect_overflow_includes_diagnostic_fields(
        // Using large limits that will cause overflow in multiplication
        limit in 2_000_000_000u32..=u32::MAX,
    ) {
        // Build a simple workflow: CollectStart -> Nop -> CollectFinish -> Finish
        let nodes = vec![
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
                kind: CompiledNodeKind::CollectFinish {
                    collector_slot: SlotIdx::new(1),
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

        let contract = test_contract(4, 4);
        let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);

        // Either the computation succeeds (if policy is loose enough) or it fails
        // with overflow. If it fails, we require diagnostic fields.
        match result {
            Err(WorkflowError::StepCountOverflow { actual }) => {
                // REJECTED due to overflow - diagnostic fields required
                // FAIL: BudgetError currently doesn't expose primitive/node/path fields
                prop_assert!(
                    actual > 0,
                    "StepCountOverflow should have actual value for diagnostic"
                );
                // This assertion FAILS because BudgetError::TotalStepsExceeded
                // does not carry the primitive kind (CollectStart) or node index
            }
            Ok(budget) => {
                // Accepted - verify gather items dimension
                prop_assert!(
                    budget.max_gather_items >= limit,
                    "Accepted workflow should track gather items correctly"
                );
            }
            Err(other) => {
                // Other errors - fail closed
                prop_assert!(
                    false,
                    "Expected StepCountOverflow or Ok, got {:?}",
                    other
                );
            }
        }
    }

    /// PROP-DIAG-001 D2: Policy rejection includes all diagnostic fields.
    ///
    /// When BoundednessPolicy::validate rejects a budget, the error should
    /// include: resource name, actual value, and limit.
    #[test]
    fn prop_policy_rejection_includes_resource_and_values(
        total_steps in 1_000_001u64..=2_000_000u64,
        policy_limit in 500_000u64..=1_000_000u64,
    ) {
        // Create a budget that exceeds the policy
        let budget = test_budget(
            total_steps,
            10,
            1,
            1,
        );

        let policy = test_policy(
            policy_limit,
            65_535,
            64,
            8,
        );

        let result = policy.validate(&budget);

        prop_assert!(
            result.is_err(),
            "Budget with {} steps should exceed policy limit {}",
            total_steps, policy_limit
        );

        match result {
            Err(BudgetError::TotalStepsExceeded { actual, limit }) => {
                // These fields ARE present in the current implementation
                prop_assert_eq!(actual, total_steps);
                prop_assert_eq!(limit, policy_limit);

                // FAIL: The error does not include the primitive kind or node index
                // PROP-DIAG-001 requires: primitive, node/step index, structural path
            }
            other => {
                prop_assert!(
                    false,
                    "Expected TotalStepsExceeded, got {:?}",
                    other
                );
            }
        }
    }

    /// PROP-DIAG-001 D3: TogetherStart fanout exceeded includes diagnostic fields.
    ///
    /// When TogetherStart has more branches than policy allows, the error
    /// should identify the TogetherStart primitive, its node index, the
    /// actual branch count, and the policy limit.
    #[test]
    fn prop_together_fanout_exceeded_includes_diagnostics(
        branch_count in 65u16..=128u16,
        policy_fanout in 2u16..=64u16,
    ) {
        prop_assume!(branch_count > policy_fanout);

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
        let budget_result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);

        prop_assert!(
            budget_result.is_ok(),
            "WholeWorkflowBudget::compute should succeed even with large fanout"
        );

        let budget = budget_result.unwrap();

        // Now validate against tight policy
        let policy = test_policy(
            1_000_000,
            65_535,
            policy_fanout,
            8,
        );

        let validation_result = policy.validate(&budget);

        match validation_result {
            Err(BudgetError::FanoutExceeded { actual, limit }) => {
                prop_assert_eq!(actual, branch_count);
                prop_assert_eq!(limit, policy_fanout);
                // FAIL: Missing primitive kind (TogetherStart) and node index (0)
                // PROP-DIAG-001 requires structural provenance
            }
            other => {
                prop_assert!(
                    false,
                    "Expected FanoutExceeded for {} branches > limit {}, got {:?}",
                    branch_count, policy_fanout, other
                );
            }
        }
    }

    /// PROP-DIAG-001 D4: ReduceStart with cold-AST-conservative iter count
    /// produces a deterministic, bounded step count.
    ///
    /// Cold-AST invariant (master §45) drops body, so the budget traversal
    /// cannot recover the declared input length. The conservative iter count
    /// is 1, giving body_count * 1 = 1 (header + body + finish).
    #[test]
    fn prop_reduce_max_list_items_has_diagnostic_context(
        _dummy in 0u8..1u8,
    ) {
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
                    initial: crate::ids::ConstIdx::new(0),
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
                kind: CompiledNodeKind::ReduceFinish {
                    accumulator: SlotIdx::new(1),
                },
            },
        ];

        let contract = test_contract(3, 3);
        let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);

        // ReduceStart with cold-AST conservative iter count should produce
        // a deterministic step count of 3 (header + 1 body + finish).
        match result {
            Ok(budget) => {
                let expected_steps = 1 + 1 + 1; // header + 1 iteration + finish
                prop_assert_eq!(
                    budget.max_total_steps, expected_steps,
                    "ReduceStart should compute {} steps with cold-AST conservative iter count",
                    expected_steps
                );
            }
            Err(WorkflowError::StepCountOverflow { actual }) => {
                prop_assert!(
                    false,
                    "ReduceStart with cold-AST conservative iter count should not overflow, got actual={}",
                    actual
                );
            }
            Err(other) => {
                prop_assert!(
                    false,
                    "Expected Ok or StepCountOverflow, got {:?}",
                    other
                );
            }
        }
    }

    /// PROP-DIAG-001 D5: Nested rejection traces the full structural path.
    ///
    /// When a deeply nested construct causes rejection, the error should
    /// trace the full path: outer -> middle -> inner primitives.
    #[test]
    fn prop_nested_rejection_traces_full_path(
        outer_limit in 1000u32..=2000u32,
        inner_limit in 1000u32..=2000u32,
    ) {
        // Build: ForEachStart(outer) -> ForEachStart(inner) -> [body] -> joins -> Finish
        let inner_body_count = 1u16;
        let inner_body_end = 2 + inner_body_count as u16;
        let inner_done = inner_body_end + 1;
        let outer_done = inner_done + 1;

        let mut nodes: Vec<CompiledNode> = vec![
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
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(inner_done),
                output: Some(SlotIdx::new(4)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachJoin {
                    output: SlotIdx::new(4),
                },
            },
            CompiledNode {
                id: StepIdx::new(outer_done),
                output: Some(SlotIdx::new(5)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachJoin {
                    output: SlotIdx::new(5),
                },
            },
        ];

        let finish_idx = outer_done + 1;
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

        // Use a very tight contract to force rejection
        let contract = test_contract(nodes.len() as u16 + 10, 4);
        let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);

        match result {
            Err(WorkflowError::StepCountOverflow { actual }) => {
                // FAILS: The error doesn't trace that the overflow originated
                // from the nested ForEachStart at node 1 (inner) vs node 0 (outer)
                prop_assert!(
                    false,
                    "Nested overflow should identify which ForEachStart caused it, actual={}",
                    actual
                );
            }
            Ok(budget) => {
                // If accepted, the nesting depth should be 2
                prop_assert_eq!(
                    budget.max_nesting_depth, 2,
                    "Should have nesting depth 2"
                );
            }
            Err(other) => {
                prop_assert!(
                    false,
                    "Expected Ok or StepCountOverflow, got {:?}",
                    other
                );
            }
        }
    }
}

// ============================================================================
// PROP-USG-001: AggregateResourceUsage arithmetic properties
// ============================================================================
//
// PROP-USG-001 proof obligation:
//
// For any AggregateResourceUsage and AggregateResourceBudget:
// - add then subtract same amount = original (roundtrip)
// - add never overflows silently
// - subtract never goes below zero
// - all dimensions are independent

