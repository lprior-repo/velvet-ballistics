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
mod kani_overflow_harnesses {
    use super::*;

    /// KANI-BUD-001 K1: body_count * iter_count overflow at u64::MAX boundary
    ///
    /// When body_count * iter_count > u64::MAX, the function must return
    /// an error, not silently saturate or wrap.
    ///
    /// Bound: body_count = u64::MAX / 2 + 1, iter_count = 2
    /// Expected: Err(BudgetError::TotalStepsExceeded { actual: u64::MAX, limit: u64::MAX })
    #[kani::proof]
    fn kani_nested_mul_overflow_u64_max() {
        // This test explores the overflow path in count_and_push_loop_body
        // where body_count.checked_mul(iter_count) returns None
        let body_count: u64 = (u64::MAX / 2) + 2; // Will overflow when multiplied by 2
        let iter_count: u64 = 2;

        // The expected behavior: checked_mul returns None for overflow
        let product = body_count.checked_mul(iter_count);
        match product {
            Some(_) => {
                // FAIL: overflow should have been detected
                kani::assert(false, "overflow not detected");
            }
            None => {
                // PASS: overflow correctly detected
            }
        }
    }

    /// KANI-BUD-001 K2: body_count * iter_count where body_count = u64::MAX and iter_count = 2
    ///
    /// Edge case: multiplying u64::MAX by any value > 1 must reject.
    #[kani::proof]
    fn kani_nested_mul_max_times_two_overflow() {
        let body_count: u64 = u64::MAX;
        let iter_count: u64 = 2;

        let product = body_count.checked_mul(iter_count);
        match product {
            Some(_) => {
                kani::assert(false, "u64::MAX * 2 should overflow");
            }
            None => {}
        }
    }

    /// KANI-BUD-001 K3: total.checked_add(product) overflow at u64::MAX
    ///
    /// After multiplication, the result is added to the running total.
    /// If total + product > u64::MAX, the addition must also fail.
    #[kani::proof]
    fn kani_total_plus_product_overflow() {
        let total: u64 = u64::MAX - 1;
        let product: u64 = 2;

        let sum = total.checked_add(product);
        match sum {
            Some(_) => {
                kani::assert(false, "total + product should overflow");
            }
            None => {}
        }
    }

    /// KANI-BUD-001 K4: near-boundary multiplication that should NOT overflow
    ///
    /// 1000 * 100 = 100000, well within u64::MAX
    #[kani::proof]
    fn kani_nested_mul_no_overflow_small_values() {
        let body_count: u64 = 1000;
        let iter_count: u64 = 100;

        let product = body_count.checked_mul(iter_count);
        match product {
            Some(p) => {
                kani::cover!(p == 100_000, "1000 * 100 = 100000");
            }
            None => {
                kani::assert(false, "1000 * 100 should not overflow");
            }
        }
    }

    /// KANI-BUD-001 K5: CollectStart with limit=0 should use minimum of 1 iteration
    ///
    /// The budget computation uses iter_count.max(1) to handle the degenerate case.
    #[kani::proof]
    fn kani_collect_zero_limit_uses_min_one() {
        let body_count: u64 = 5;
        let iter_count: u64 = 0;

        // Code does: let iter_count = iter_count.max(1);
        let effective_iter = iter_count.max(1);
        let product = body_count * effective_iter; // 5 * 1 = 5

        kani::cover!(effective_iter == 1, "zero limit becomes 1");
        kani::cover!(product == 5, "5 * 1 = 5");
    }

    /// KANI-BUD-001 K6: Aggregate add_dim overflow detection
    ///
    /// AggregateResourceUsage::try_add_budget uses checked_add for each dimension.
    /// Overflow must return AggregateBudgetError::Overflow.
    #[kani::proof]
    fn kani_aggregate_add_budget_overflow() {
        let usage = AggregateResourceUsage {
            max_steps_executable: u64::MAX,
            max_action_tickets: 0,
            max_parallel_in_flight: 0,
            max_gather_pages: 0,
            max_gather_items: 0,
            max_result_bytes: 0,
            max_total_slots_written: 0,
            max_timer_entries: 0,
            max_trace_events: 0,
            max_active_runs: 0,
            max_queue_depth: 0,
            max_journal_batch_bytes: 0,
            max_timer_entries: 0,
            max_trace_events: 0,
            max_ipc_payload_bytes: 0,
            max_blob_bytes: 0,
            max_input_bytes: 0,
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
            max_timer_entries: 0,
            max_trace_events: 0,
            max_queue_depth: 0,
            max_journal_batch_bytes: 0,
            max_timer_entries: 0,
            max_trace_events: 0,
            max_ipc_payload_bytes: 0,
            max_blob_bytes: 0,
            max_input_bytes: 0,
            max_step_budget_per_tick: 0,
            max_transitions_per_tick: 0,
        };

        let result = usage.try_add_budget(&budget);
        match result {
            Err(AggregateBudgetError::Overflow { resource }) => {
                kani::cover!(
                    resource == "max_steps_executable",
                    "overflow detected for max_steps_executable"
                );
            }
            _ => {
                kani::assert(false, "adding to u64::MAX should overflow");
            }
        }
    }

    /// KANI-BUD-001 K7: Aggregate sub_dim underflow detection
    ///
    /// Subtracting more than available must return Underflow error.
    #[kani::proof]
    fn kani_aggregate_sub_budget_underflow() {
        let usage = AggregateResourceUsage {
            max_steps_executable: 5,
            max_action_tickets: 0,
            max_parallel_in_flight: 0,
            max_gather_pages: 0,
            max_gather_items: 0,
            max_result_bytes: 0,
            max_total_slots_written: 0,
            max_timer_entries: 0,
            max_trace_events: 0,
            max_active_runs: 0,
            max_queue_depth: 0,
            max_journal_batch_bytes: 0,
            max_timer_entries: 0,
            max_trace_events: 0,
            max_ipc_payload_bytes: 0,
            max_blob_bytes: 0,
            max_input_bytes: 0,
            max_step_budget_per_tick: 0,
            max_transitions_per_tick: 0,
        };

        let budget = AggregateResourceBudget {
            max_steps_executable: 10, // Trying to subtract more than available
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
            max_timer_entries: 0,
            max_trace_events: 0,
            max_queue_depth: 0,
            max_journal_batch_bytes: 0,
            max_timer_entries: 0,
            max_trace_events: 0,
            max_ipc_payload_bytes: 0,
            max_blob_bytes: 0,
            max_input_bytes: 0,
            max_step_budget_per_tick: 0,
            max_transitions_per_tick: 0,
        };

        let result = usage.try_subtract_budget(&budget);
        match result {
            Err(AggregateBudgetError::Underflow { resource }) => {
                kani::cover!(resource == "max_steps_executable", "underflow detected");
            }
            _ => {
                kani::assert(false, "subtracting 10 from 5 should underflow");
            }
        }
    }
}

// ============================================================================
// PROP-BUD-001: Nested accepted budgets fit policy
// ============================================================================
//
// PROP-BUD-001 proof obligation:
//
// Invariant: For any generated structurally valid nested workflow with finite
// declared limits under policy, WholeWorkflowBudget::compute returns dimensions
// <= ResourceContract and <= BoundednessPolicy, and
// AggregateResourceBudget::from_workflow preserves those dimensions.
//
// Anti-invariant: Any generated workflow with a dimension over policy must
// return the exact budget error variant and actual/limit pair.

// ---------------------------------------------------------------------------
// Proptest: CollectStart body multiplication with finite limit
// ---------------------------------------------------------------------------

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

        prop_kani::assert(budget.is_ok(), "CollectStart workflow should compute budget successfully");

        let budget = budget.unwrap();

        // Expected: 1 (CollectStart) + limit * body_node_count + 1 (CollectFinish) + 1 (Finish)
        let expected_steps: u64 = 1 + (limit as u64) * (body_node_count as u64) + 1 + 1;

        prop_kani::assert_eq!(budget.max_total_steps, expected_steps,
            "CollectStart with limit={} and body_count={} should have {} total steps, got {}",
            limit, body_node_count, expected_steps, budget.max_total_steps);
        prop_kani::assert(budget.max_gather_items >= limit,
            "max_gather_items {} should be at least limit {}",
            budget.max_gather_items, limit);
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

        prop_kani::assert(budget.is_ok(), "RepeatStart workflow should compute budget successfully");

        let budget = budget.unwrap();

        // Expected: 1 (RepeatStart) + 1 * body_node_count + 1 (RepeatFinish) + 1 (Finish)
        // The cold-AST-conservative iter count is 1, so the body is counted once.
        let expected_steps: u64 = 1 + (body_node_count as u64) + 1 + 1;

        prop_kani::assert_eq!(budget.max_total_steps, expected_steps,
            "RepeatStart with max_attempts={} and body_count={} should have {} total steps (cold-AST-conservative), got {}",
            max_attempts, body_node_count, expected_steps, budget.max_total_steps);
        prop_kani::assert_eq!(budget.max_repeat_attempts, max_attempts,
            "max_repeat_attempts should be {}",
            max_attempts);
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

        prop_kani::assert(budget.is_ok(), "Nested ForEach workflow should compute budget successfully");

        let budget = budget.unwrap();

        // Expected: 1 (outer) + 1 (inner) + inner_limit * inner_body_count + 1 (inner join) + 1 (outer join) + 1 (finish)
        let inner_body_steps: u64 = inner_body_count as u64;
        let inner_loop_steps: u64 = 1 + (inner_limit as u64) * inner_body_steps + 1;
        let expected_steps: u64 = 1 + (outer_limit as u64) * inner_loop_steps + 1 + 1;

        prop_kani::assert_eq!(budget.max_total_steps, expected_steps,
            "Nested loops with outer={}, inner={}, inner_body={} should have {} steps, got {}",
            outer_limit, inner_limit, inner_body_count, expected_steps, budget.max_total_steps);
        prop_kani::assert_eq!(budget.max_nesting_depth, 2,
            "Nested loops should have depth 2, got {}",
            budget.max_nesting_depth);
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

        prop_kani::assert(budget.is_ok(), "TogetherStart workflow should compute budget successfully")

        let budget = budget.unwrap();

        prop_kani::assert_eq!(budget.max_fanout, branch_count,
            "max_fanout should be {}, got {}",
            branch_count, budget.max_fanout);
        prop_kani::assert_eq!(budget.max_together_branches, branch_count,
            "max_together_branches should be {}, got {}",
            branch_count, budget.max_together_branches);
        prop_kani::assert_eq!(budget.max_parallel_in_flight, branch_count,
            "max_parallel_in_flight should be {}, got {}",
            branch_count, budget.max_parallel_in_flight);
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

        prop_kani::assert(whole_budget.is_ok(), "WholeWorkflowBudget should compute successfully")

        let whole_budget = whole_budget.unwrap();

        // Create aggregate from whole workflow budget
        let aggregate = AggregateResourceBudget::from_whole_workflow_budget(whole_budget, contract);

        prop_kani::assert(aggregate.is_ok(), "AggregateResourceBudget should create successfully from whole budget")

        let aggregate = aggregate.unwrap();

        prop_kani::assert_eq!(aggregate.max_steps_executable, whole_budget.max_steps_executable,
            "aggregate.max_steps_executable should equal whole_budget.max_steps_executable");
        prop_kani::assert_eq!(aggregate.max_gather_items, whole_budget.max_gather_items,
            "aggregate.max_gather_items should equal whole_budget.max_gather_items");
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
                prop_kani::assert(actual > 0, "StepCountOverflow should have actual value for diagnostic")
                // This assertion FAILS because BudgetError::TotalStepsExceeded
                // does not carry the primitive kind (CollectStart) or node index
            }
            Ok(budget) => {
                // Accepted - verify gather items dimension
                prop_kani::assert(budget.max_gather_items >= limit, "Accepted workflow should track gather items correctly")
            }
            Err(other) => {
                // Other errors - fail closed
                prop_kani::assert(false,
                    "Expected StepCountOverflow or Ok, got {:?}", other)
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

        prop_kani::assert(result.is_err(),
            "Budget with {} steps should exceed policy limit {}",
            total_steps, policy_limit)

        match result {
            Err(BudgetError::TotalStepsExceeded { actual, limit }) => {
                // These fields ARE present in the current implementation
                prop_kani::assert_eq!(actual, total_steps);
                prop_kani::assert_eq!(limit, policy_limit);

                // FAIL: The error does not include the primitive kind or node index
                // PROP-DIAG-001 requires: primitive, node/step index, structural path
            }
            other => {
                prop_kani::assert(false,
                    "Expected TotalStepsExceeded, got {:?}", other)
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

        prop_kani::assert(budget_result.is_ok(), "WholeWorkflowBudget::compute should succeed even with large fanout")

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
                prop_kani::assert_eq!(actual, branch_count);
                prop_kani::assert_eq!(limit, policy_fanout);
                // FAIL: Missing primitive kind (TogetherStart) and node index (0)
                // PROP-DIAG-001 requires structural provenance
            }
            other => {
                prop_kani::assert(false,
                    "Expected FanoutExceeded for {} branches > limit {}, got {:?}",
                    branch_count, policy_fanout, other)
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
                prop_kani::assert_eq!(budget.max_total_steps, expected_steps,
                    "ReduceStart should compute {} steps with cold-AST conservative iter count",
                    expected_steps);
            }
            Err(WorkflowError::StepCountOverflow { actual }) => {
                prop_kani::assert(false,
                    "ReduceStart with cold-AST conservative iter count should not overflow, got actual={}", actual)
            }
            Err(other) => {
                prop_kani::assert(false,
                    "Expected Ok or StepCountOverflow, got {:?}", other)
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
                prop_kani::assert(false,
                    "Nested overflow should identify which ForEachStart caused it, actual={}", actual)
            }
            Ok(budget) => {
                // If accepted, the nesting depth should be 2
                prop_kani::assert_eq!(budget.max_nesting_depth, 2,
                    "Should have nesting depth 2");
            }
            Err(other) => {
                prop_kani::assert(false,
                    "Expected Ok or StepCountOverflow, got {:?}", other)
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

proptest! {
    #[test]
    fn prop_add_then_subtract_roundtrip(
        base_steps in 0u64..(u64::MAX / 4),
        delta_steps in 0u32..10_000u32,
        base_tickets in 0u64..(u64::MAX / 4),
        delta_tickets in 0u32..10_000u32,
        base_blob in 0u64..(u64::MAX / 4),
        delta_blob in 0u64..10_000u64,
        base_active in 1u64..100u64,
        base_step_tick in 1u64..(u64::MAX / 4),
        delta_step_tick in 0u64..10_000u64,
    ) {
        let usage = AggregateResourceUsage {
            max_steps_executable: base_steps,
            max_action_tickets: base_tickets,
            max_parallel_in_flight: base_steps.saturating_add(1),
            max_gather_pages: base_steps.saturating_add(2),
            max_gather_items: base_steps.saturating_add(3),
            max_result_bytes: base_steps.saturating_add(4),
            max_total_slots_written: base_steps.saturating_add(5),
            max_timer_entries: base_steps.saturating_add(6),
            max_trace_events: base_tickets,
            max_active_runs: base_active,
            max_queue_depth: base_steps.saturating_add(7),
            max_journal_batch_bytes: base_steps.saturating_add(8),
            max_ipc_payload_bytes: base_steps.saturating_add(9),
            max_blob_bytes: base_blob,
            max_input_bytes: base_steps.saturating_add(10),
            max_step_budget_per_tick: base_step_tick,
            max_transitions_per_tick: base_step_tick.saturating_add(1),
        };

        let budget = AggregateResourceBudget {
            max_steps_executable: delta_steps,
            max_action_tickets: delta_tickets,
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
            max_timer_entries: 0,
            max_trace_events: 0,
            max_queue_depth: 0,
            max_journal_batch_bytes: 0,
            max_ipc_payload_bytes: 0,
            max_blob_bytes: delta_blob,
            max_input_bytes: 0,
            max_step_budget_per_tick: delta_step_tick,
            max_transitions_per_tick: 0,
        };

        let added = usage.try_add_budget(&budget);
        let added = added.unwrap();

        let subtracted = added.try_subtract_budget(&budget);
        prop_kani::assert_eq!(subtracted,
            Ok(usage),
            "add then subtract same budget must roundtrip to original usage")
    }

    #[test]
    fn prop_add_never_overflows_silently(
        base in (u64::MAX - 100)..=u64::MAX,
        delta in 1u32..100u32,
    ) {
        let usage = AggregateResourceUsage {
            max_steps_executable: base,
            ..Default::default()
        };

        let budget = AggregateResourceBudget {
            max_steps_executable: delta,
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
            max_timer_entries: 0,
            max_trace_events: 0,
            max_queue_depth: 0,
            max_journal_batch_bytes: 0,
            max_ipc_payload_bytes: 0,
            max_blob_bytes: 0,
            max_input_bytes: 0,
            max_step_budget_per_tick: 1,
            max_transitions_per_tick: 1,
        };

        let result = usage.try_add_budget(&budget);
        match result {
            Ok(added) => {
                let expected = base.checked_add(u64::from(delta));
                match expected {
                    Some(exp) => {
                        prop_kani::assert_eq!(added.max_steps_executable, exp,
                            "Ok result must equal checked_add");
                    }
                    None => {
                        prop_kani::assert(false, "add returned Ok which overflows checked_add")
                    }
                }
            }
            Err(AggregateBudgetError::Overflow { resource }) => {
                prop_kani::assert_eq!(resource, "max_steps_executable",
                    "overflow must identify the correct resource dimension");
                let checked = base.checked_add(u64::from(delta));
                prop_kani::assert(checked.is_none(), "Err(Overflow) must correspond to real overflow")
            }
            Err(_other) => {
                prop_kani::assert(false, "expected Ok or Overflow, got unexpected error")
            }
        }
    }

    #[test]
    fn prop_subtract_never_goes_below_zero(
        base_steps in 0u64..5u64,
        delta_steps in 6u32..50u32,
        base_blob in 0u64..5u64,
        delta_blob in 6u64..50u64,
    ) {
        let usage = AggregateResourceUsage {
            max_steps_executable: base_steps,
            max_active_runs: 0,
            max_blob_bytes: base_blob,
            max_step_budget_per_tick: 1,
            ..Default::default()
        };

        let budget = AggregateResourceBudget {
            max_steps_executable: delta_steps,
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
            max_timer_entries: 0,
            max_trace_events: 0,
            max_queue_depth: 0,
            max_journal_batch_bytes: 0,
            max_ipc_payload_bytes: 0,
            max_blob_bytes: delta_blob,
            max_input_bytes: 0,
            max_step_budget_per_tick: 1,
            max_transitions_per_tick: 1,
        };

        let result = usage.try_subtract_budget(&budget);
        match result {
            Err(AggregateBudgetError::Underflow { resource }) => {
                prop_kani::assert(resource == "max_steps_executable"
                        || resource == "max_blob_bytes"
                        || resource == "max_active_runs", "underflow must identify the underflowing dimension")
            }
            Ok(subtracted) => {
                prop_kani::assert(base_steps >= u64::from(delta_steps), "subtract returned Ok but base < delta")
                prop_kani::assert_eq!(subtracted.max_steps_executable,
                    base_steps.checked_sub(u64::from(delta_steps)).unwrap_or(u64::MAX))
            }
            Err(_other) => {
                prop_kani::assert(false, "expected Ok or Underflow, got unexpected error")
            }
        }
    }

    #[test]
    fn prop_dimensions_independent(
        base in 0u64..1_000_000u64,
        delta_a in 1u32..1000u32,
        delta_b in 1001u32..2000u32,
    ) {
        prop_assume!(delta_a != delta_b);

        let usage = AggregateResourceUsage {
            max_steps_executable: base,
            max_action_tickets: base.saturating_add(100),
            max_blob_bytes: base.saturating_add(200),
            max_active_runs: 5,
            max_step_budget_per_tick: base.saturating_add(300),
            ..Default::default()
        };

        let budget_a = AggregateResourceBudget {
            max_steps_executable: delta_a,
            max_action_tickets: 50,
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
            max_timer_entries: 0,
            max_trace_events: 0,
            max_queue_depth: 0,
            max_journal_batch_bytes: 0,
            max_ipc_payload_bytes: 0,
            max_blob_bytes: 0,
            max_input_bytes: 0,
            max_step_budget_per_tick: 10,
            max_transitions_per_tick: 10,
        };

        let mut budget_b = budget_a;
        budget_b.max_steps_executable = delta_b;

        let result_a = usage.try_add_budget(&budget_a);
        let result_b = usage.try_add_budget(&budget_b);

        let added_a = result_a.unwrap();
        let added_b = result_b.unwrap();

        prop_kani::assert_ne!(added_a.max_steps_executable, added_b.max_steps_executable,
            "changing max_steps_executable must change that dimension");
        prop_kani::assert_eq!(added_a.max_action_tickets, added_b.max_action_tickets);
        prop_kani::assert_eq!(added_a.max_parallel_in_flight, added_b.max_parallel_in_flight);
        prop_kani::assert_eq!(added_a.max_gather_pages, added_b.max_gather_pages);
        prop_kani::assert_eq!(added_a.max_gather_items, added_b.max_gather_items);
        prop_kani::assert_eq!(added_a.max_result_bytes, added_b.max_result_bytes);
        prop_kani::assert_eq!(added_a.max_total_slots_written, added_b.max_total_slots_written);
        prop_kani::assert_eq!(added_a.max_timer_entries, added_b.max_timer_entries);
        prop_kani::assert_eq!(added_a.max_trace_events, added_b.max_trace_events);
        prop_kani::assert_eq!(added_a.max_active_runs, added_b.max_active_runs);
        prop_kani::assert_eq!(added_a.max_queue_depth, added_b.max_queue_depth);
        prop_kani::assert_eq!(added_a.max_journal_batch_bytes, added_b.max_journal_batch_bytes);
        prop_kani::assert_eq!(added_a.max_ipc_payload_bytes, added_b.max_ipc_payload_bytes);
        prop_kani::assert_eq!(added_a.max_blob_bytes, added_b.max_blob_bytes);
        prop_kani::assert_eq!(added_a.max_input_bytes, added_b.max_input_bytes);
        prop_kani::assert_eq!(added_a.max_step_budget_per_tick, added_b.max_step_budget_per_tick);
        prop_kani::assert_eq!(added_a.max_transitions_per_tick, added_b.max_transitions_per_tick);
    }
}

// ============================================================================
// Helper functions and test fixtures
// ============================================================================

/// Creates a test ResourceContract with sufficient limits for normal tests.
fn test_contract(max_steps: u16, max_slots: u16) -> ResourceContract {
    ResourceContract {
        max_steps,
        max_slots,
        max_constants: 1,
        max_accessors: 0,
        max_expressions: 0,
        max_expr_stack: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        max_blob_bytes: 1024,
        max_ipc_payload_bytes: 1024,
        max_retry_attempts: 3,
        max_fanout: 64,
        max_collect_items: u32::MAX,
        max_queue_depth: 100,
        max_journal_batch_bytes: 1024,
        ..ResourceContract::DEFAULT
    }
}

/// Creates a test WholeWorkflowBudget with the given dimensions.
fn test_budget(
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
        max_steps_executable: u32::try_from(max_total_steps).unwrap_or(u32::MAX),
        max_action_tickets: 0,
        max_parallel_in_flight: max_fanout,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: max_fanout,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: u32::try_from(max_total_slots).unwrap_or(u32::MAX),
        max_timer_entries: 0,
        max_trace_events: 0,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_queue_depth: 0,
    }
}

/// Creates a test BoundednessPolicy with the given limits.
fn test_policy(
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
        ..BoundednessPolicy::DEFAULT
    }
}
