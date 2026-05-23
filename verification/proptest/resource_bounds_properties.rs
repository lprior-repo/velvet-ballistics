#![forbid(unsafe_code)]

//! Proptest-based property tests for resource bounds and budget enforcement.
//!
//! These properties verify behavioral correctness of budget arithmetic,
//! capacity checking, and resource limit enforcement using randomized inputs.
//!
//! Maps to obligations: OBL-BUDGET-001, OBL-BUDGET-002, OBL-STEP-001,
//! OBL-STEP-002, OBL-ARENA-001, OBL-ARENA-002, OBL-EXPR-001, OBL-EXPR-002

use proptest::prelude::*;

use crate::budget::{
    AggregateBudgetError, AggregateResourceBudget, AggregateResourceCapacity,
    AggregateResourceUsage, BoundednessPolicy, BudgetError, WholeWorkflowBudget,
};
use crate::errors::CoreError;
use crate::expressions::{check_expr_stack_bound, ExprOp, ExprProgram};
use crate::limits::{MAX_EXPRESSION_OPS, MAX_EXPRESSION_STACK};
use crate::value::SlotValue;
use crate::value_store::ValueStore;
use bytes::Bytes;

// =========================================================================
// Strategy generators
// =========================================================================

fn arb_u64_bounded(max: u64) -> BoxedStrategy<u64> {
    (0u64..=max).prop_map(|v| v).boxed()
}

fn arb_u32_bounded(max: u32) -> BoxedStrategy<u32> {
    (0u32..=max).prop_map(|v| v).boxed()
}

fn arb_u16_bounded(max: u16) -> BoxedStrategy<u16> {
    (0u16..=max).prop_map(|v| v).boxed()
}

fn arb_whole_workflow_budget() -> BoxedStrategy<WholeWorkflowBudget> {
    (
        any::<u64>(),
        any::<u64>(),
        any::<u16>(),
        any::<u16>(),
        any::<u32>(),
        any::<u32>(),
        any::<u16>(),
        any::<u16>(),
        any::<u32>(),
        any::<u32>(),
        any::<u32>(),
        any::<u16>(),
        any::<u16>(),
        any::<u64>(),
        any::<u32>(),
        any::<u32>(),
    )
        .prop_map(
            |(
                max_total_steps,
                max_total_slots,
                max_fanout,
                max_nesting_depth,
                max_steps_executable,
                max_action_tickets,
                max_parallel_in_flight,
                max_retries_per_action,
                max_gather_pages,
                max_gather_items,
                max_for_each_iterations,
                max_together_branches,
                max_repeat_attempts,
                max_run_time_seconds,
                max_result_bytes,
                max_total_slots_written,
            )| {
                WholeWorkflowBudget {
                    max_total_steps,
                    max_total_slots,
                    max_fanout,
                    max_nesting_depth,
                    max_steps_executable,
                    max_action_tickets,
                    max_parallel_in_flight,
                    max_retries_per_action,
                    max_gather_pages,
                    max_gather_items,
                    max_for_each_iterations,
                    max_together_branches,
                    max_repeat_attempts,
                    max_run_time_seconds,
                    max_result_bytes,
                    max_total_slots_written,
                }
            },
        )
        .boxed()
}

fn arb_aggregate_budget() -> BoxedStrategy<AggregateResourceBudget> {
    (
        any::<u32>(),
        any::<u32>(),
        any::<u16>(),
        any::<u16>(),
        any::<u32>(),
        any::<u32>(),
        any::<u32>(),
        any::<u16>(),
        any::<u16>(),
        any::<u64>(),
        any::<u32>(),
        any::<u32>(),
        any::<u32>(),
        any::<u64>(),
        any::<u64>(),
    )
        .prop_map(
            |(
                max_steps_executable,
                max_action_tickets,
                max_parallel_in_flight,
                max_retries_per_action,
                max_gather_pages,
                max_gather_items,
                max_for_each_iterations,
                max_together_branches,
                max_repeat_attempts,
                max_run_time_seconds,
                max_result_bytes,
                max_total_slots_written,
                max_queue_depth,
                max_journal_batch_bytes,
                max_step_budget_per_tick,
                max_transitions_per_tick,
            )| {
                AggregateResourceBudget {
                    max_steps_executable,
                    max_action_tickets,
                    max_parallel_in_flight,
                    max_retries_per_action,
                    max_gather_pages,
                    max_gather_items,
                    max_for_each_iterations,
                    max_together_branches,
                    max_repeat_attempts,
                    max_run_time_seconds,
                    max_result_bytes,
                    max_total_slots_written,
                    max_queue_depth,
                    max_journal_batch_bytes,
                    max_step_budget_per_tick,
                    max_transitions_per_tick,
                }
            },
        )
        .boxed()
}

fn arb_aggregate_usage() -> BoxedStrategy<AggregateResourceUsage> {
    (
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
    )
        .prop_map(
            |(
                max_steps_executable,
                max_action_tickets,
                max_parallel_in_flight,
                max_gather_pages,
                max_gather_items,
                max_result_bytes,
                max_total_slots_written,
                max_active_runs,
                max_queue_depth,
                max_journal_batch_bytes,
                max_step_budget_per_tick,
                max_transitions_per_tick,
            )| {
                AggregateResourceUsage {
                    max_steps_executable,
                    max_action_tickets,
                    max_parallel_in_flight,
                    max_gather_pages,
                    max_gather_items,
                    max_result_bytes,
                    max_total_slots_written,
                    max_active_runs,
                    max_queue_depth,
                    max_journal_batch_bytes,
                    max_step_budget_per_tick,
                    max_transitions_per_tick,
                }
            },
        )
        .boxed()
}

fn arb_aggregate_capacity() -> BoxedStrategy<AggregateResourceCapacity> {
    (
        any::<u64>(),
        any::<u64>(),
        any::<u32>(),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
    )
        .prop_map(
            |(
                max_steps_executable,
                max_action_tickets,
                max_parallel_in_flight,
                max_gather_pages,
                max_gather_items,
                max_result_bytes,
                max_total_slots_written,
                max_active_runs,
                max_queue_depth,
                max_journal_batch_bytes,
                max_step_budget_per_tick,
                max_transitions_per_tick,
            )| {
                AggregateResourceCapacity {
                    max_steps_executable,
                    max_action_tickets,
                    max_parallel_in_flight,
                    max_gather_pages,
                    max_gather_items,
                    max_result_bytes,
                    max_total_slots_written,
                    max_active_runs,
                    max_queue_depth,
                    max_journal_batch_bytes,
                    max_step_budget_per_tick,
                    max_transitions_per_tick,
                }
            },
        )
        .boxed()
}

// =========================================================================
// Properties: Budget validation (OBL-BUDGET-001, OBL-BUDGET-002)
// =========================================================================

/// PROP-BUDGET-001: BoundednessPolicy::validate never panics on arbitrary inputs.
/// When budget is within policy, returns Ok(()).
proptest! {
    #[test]
    fn prop_budget_validate_within_policy_never_panics(
        max_total_steps in 0u64..=1_000_000,
        max_total_slots in 0u64..=65_535,
        max_fanout in 0u16..=64,
        max_nesting_depth in 0u16..=8,
        max_steps_executable in 0u32..=1_000_000,
        max_action_tickets in 0u32..=100_000,
        max_parallel_in_flight in 0u16..=256,
        max_run_time_seconds in 0u64..=2_592_000,
        max_result_bytes in 0u32..=262_144,
    ) {
        let policy = BoundednessPolicy::DEFAULT;
        let budget = WholeWorkflowBudget {
            max_total_steps,
            max_total_slots,
            max_fanout,
            max_nesting_depth,
            max_steps_executable,
            max_action_tickets,
            max_parallel_in_flight,
            max_retries_per_action: 3,
            max_gather_pages: 100,
            max_gather_items: 10_000,
            max_for_each_iterations: 1_000,
            max_together_branches: 16,
            max_repeat_attempts: 5,
            max_run_time_seconds,
            max_result_bytes,
            max_total_slots_written: 1_000,
        };
        let result = policy.validate(&budget);
        // If all within policy, must be Ok
        if max_total_steps <= policy.max_total_steps
            && max_total_slots <= policy.max_total_slots
            && max_fanout <= policy.max_fanout
            && max_nesting_depth <= policy.max_nesting_depth
            && max_action_tickets <= policy.absolute_max_action_tickets
            && max_parallel_in_flight <= policy.absolute_max_parallel
            && max_run_time_seconds <= policy.absolute_max_run_time_seconds
            && max_result_bytes <= policy.absolute_max_result_bytes
            && max_steps_executable <= policy.absolute_max_steps_executable
        {
            prop_assert!(result.is_ok(), "budget within policy must return Ok");
        }
    }
}

/// PROP-BUDGET-002: BoundednessPolicy::validate returns error when any bound exceeded.
proptest! {
    #[test]
    fn prop_budget_validate_exceeded_returns_error(
        excess in 1u64..=u64::MAX,
    ) {
        let policy = BoundednessPolicy::DEFAULT;
        // Exceed total_steps by adding excess to the limit
        let over_limit = policy.max_total_steps.saturating_add(excess);
        let budget = WholeWorkflowBudget {
            max_total_steps: over_limit,
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
        let result = policy.validate(&budget);
        prop_assert!(result.is_err(), "exceeding total_steps must return error");
        if let Err(BudgetError::TotalStepsExceeded { actual, limit }) = result {
            prop_assert_eq!(actual, over_limit);
            prop_assert_eq!(limit, policy.max_total_steps);
        } else {
            prop_assert!(false, "expected TotalStepsExceeded variant");
        }
    }
}

// =========================================================================
// Properties: Aggregate budget arithmetic (overflow prevention)
// =========================================================================

/// PROP-AGG-001: try_add_budget returns Overflow when arithmetic would overflow.
proptest! {
    #[test]
    fn prop_try_add_budget_overflow_returns_error(
        usage in arb_aggregate_usage(),
    ) {
        // Create a budget that will definitely overflow when added to usage
        let budget = AggregateResourceBudget {
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
            max_queue_depth: u32::MAX,
            max_journal_batch_bytes: u32::MAX,
            max_step_budget_per_tick: u64::MAX,
            max_transitions_per_tick: u64::MAX,
        };
        let result = usage.try_add_budget(&budget);
        // At least one dimension should overflow
        prop_assert!(
            result.is_err(),
            "adding MAX values to any usage should overflow"
        );
    }
}

/// PROP-AGG-002: try_add_budget with zero budget returns unchanged usage.
proptest! {
    #[test]
    fn prop_try_add_budget_zero_is_identity(
        usage in arb_aggregate_usage(),
    ) {
        let zero_budget = AggregateResourceBudget {
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
            max_transitions_per_tick: 0,
        };
        let result = usage.try_add_budget(&zero_budget);
        prop_assert!(result.is_ok(), "adding zero budget must succeed");
        let new_usage = result.expect("adding zero budget must succeed");
        prop_assert_eq!(new_usage, usage, "zero budget must not change usage");
    }
}

/// PROP-AGG-003: fits_within returns Ok when usage <= capacity elementwise.
proptest! {
    #[test]
    fn prop_fits_within_elementwise_ok(
        usage in arb_aggregate_usage(),
    ) {
        // Create capacity that is exactly equal to usage (should fit)
        let capacity = AggregateResourceCapacity {
            max_steps_executable: usage.max_steps_executable,
            max_action_tickets: usage.max_action_tickets,
            max_parallel_in_flight: usage.max_parallel_in_flight.try_into().expect("parallel must fit u32"),
            max_gather_pages: usage.max_gather_pages,
            max_gather_items: usage.max_gather_items,
            max_result_bytes: usage.max_result_bytes,
            max_total_slots_written: usage.max_total_slots_written,
            max_active_runs: usage.max_active_runs,
            max_queue_depth: usage.max_queue_depth,
            max_journal_batch_bytes: usage.max_journal_batch_bytes,
            max_step_budget_per_tick: usage.max_step_budget_per_tick,
            max_transitions_per_tick: usage.max_transitions_per_tick,
        };
        let result = usage.fits_within(&capacity);
        prop_assert!(result.is_ok(), "usage equal to capacity must fit");
    }
}

/// PROP-AGG-004: fits_within returns CapacityExceeded when any dimension exceeds.
proptest! {
    #[test]
    fn prop_fits_within_exceeded_returns_error(
        usage in arb_aggregate_usage(),
    ) {
        // Create capacity that is one less than usage on max_steps_executable
        let capacity = AggregateResourceCapacity {
            max_steps_executable: usage.max_steps_executable.saturating_sub(1),
            max_action_tickets: usage.max_action_tickets,
            max_parallel_in_flight: usage.max_parallel_in_flight.try_into().expect("parallel must fit u32"),
            max_gather_pages: usage.max_gather_pages,
            max_gather_items: usage.max_gather_items,
            max_result_bytes: usage.max_result_bytes,
            max_total_slots_written: usage.max_total_slots_written,
            max_active_runs: usage.max_active_runs,
            max_queue_depth: usage.max_queue_depth,
            max_journal_batch_bytes: usage.max_journal_batch_bytes,
            max_step_budget_per_tick: usage.max_step_budget_per_tick,
            max_transitions_per_tick: usage.max_transitions_per_tick,
        };
        let result = usage.fits_within(&capacity);
        if usage.max_steps_executable > 0 {
            prop_assert!(result.is_err(), "usage exceeding capacity must fail");
        }
    }
}

// =========================================================================
// Properties: Expression stack bounds (OBL-EXPR-001, OBL-EXPR-002)
// =========================================================================

/// PROP-EXPR-001: check_expr_stack_bound never panics on arbitrary valid ops.
proptest! {
    #[test]
    fn prop_expr_stack_bound_never_panics(
        ops in prop::collection::vec(
            prop_oneof![
                Just(ExprOp::LoadConst(crate::ids::ConstIdx::new(0))),
                Just(ExprOp::LoadSlot(crate::ids::SlotIdx::new(0))),
                Just(ExprOp::Add),
                Just(ExprOp::Sub),
                Just(ExprOp::Mul),
                Just(ExprOp::Eq),
                Just(ExprOp::And),
                Just(ExprOp::Or),
                Just(ExprOp::Not),
            ],
            0..=MAX_EXPRESSION_OPS,
        ),
    ) {
        let result = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK);
        // Should not panic — may return error for underflow/overflow
        // The property is that it never panics
        let _ = result;
    }
}

/// PROP-EXPR-002: Expression stack with balanced push/pop returns exact depth.
proptest! {
    #[test]
    fn prop_expr_stack_balanced_returns_depth_one(
        load_count in 1usize..=MAX_EXPRESSION_OPS / 2,
    ) {
        // Build a balanced expression: N loads, N-1 binary ops = depth 1
        let mut ops: Vec<ExprOp> = Vec::new();
        for i in 0..load_count {
            ops.push(ExprOp::LoadConst(crate::ids::ConstIdx::new(i as u16)));
        }
        // Add binary ops to consume pairs
        for _ in 0..(load_count - 1) {
            ops.push(ExprOp::Add);
        }
        let result = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK);
        if load_count <= MAX_EXPRESSION_STACK as usize {
            prop_assert!(result.is_ok(), "balanced expression should be valid");
            if let Ok(depth) = result {
                prop_assert_eq!(depth, 1, "balanced expression must leave depth 1");
            }
        }
    }
}

/// PROP-EXPR-003: Too many pushes without pops exceeds stack capacity.
proptest! {
    #[test]
    fn prop_expr_stack_overflow_on_excess_pushes(
        excess in 1usize..=128,
    ) {
        let over_limit = (MAX_EXPRESSION_STACK as usize).saturating_add(excess);
        let mut ops: Vec<ExprOp> = Vec::new();
        for i in 0..over_limit.min(MAX_EXPRESSION_OPS) {
            ops.push(ExprOp::LoadConst(crate::ids::ConstIdx::new(i as u16)));
        }
        if ops.len() > MAX_EXPRESSION_STACK as usize {
            let result = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK);
            prop_assert!(result.is_err(), "excess pushes must exceed stack capacity");
        }
    }
}

// =========================================================================
// Properties: ValueStore arena capacity (OBL-ARENA-001, OBL-ARENA-002)
// =========================================================================

/// PROP-ARENA-001: ValueStore with cap rejects inserts after capacity reached.
proptest! {
    #[test]
    fn prop_arena_cap_rejects_after_full(
        cap in 1u16..=100,
    ) {
        let mut store = ValueStore::with_max_slots(cap);
        let cap_u64 = u64::from(cap);

        // Fill to capacity with symbols
        let mut inserted = 0u64;
        while inserted < cap_u64 {
            let result = store.insert_symbol(format!("sym_{inserted}"));
            match result {
                Ok(_) => inserted += 1,
                Err(CoreError::BudgetExceeded { .. }) => break,
                Err(e) => {
                    prop_assert!(false, "unexpected error: {e:?}");
                    break;
                }
            }
        }

        prop_assert_eq!(inserted, cap_u64, "should fill to capacity");
        prop_assert_eq!(store.total_arena_count(), cap_u64);

        // Next insert must fail
        let result = store.insert_symbol("overflow");
        prop_assert!(
            matches!(result, Err(CoreError::BudgetExceeded { .. })),
            "insert after capacity must return BudgetExceeded"
        );
    }
}

/// PROP-ARENA-002: ValueStore accepts inserts while under capacity.
proptest! {
    #[test]
    fn prop_arena_accepts_under_cap(
        cap in 2u16..=100,
        num_inserts in 1usize..=50,
    ) {
        let mut store = ValueStore::with_max_slots(cap);
        let limit = num_inserts.min(cap as usize);

        for i in 0..limit {
            let result = store.insert_symbol(format!("item_{i}"));
            prop_assert!(result.is_ok(), "insert under capacity must succeed");
        }

        prop_assert_eq!(store.total_arena_count(), limit as u64);
    }
}

/// PROP-ARENA-003: Mixed insert types (symbol + list + blob) count toward same cap.
proptest! {
    #[test]
    fn prop_arena_mixed_types_share_cap(
        cap in 3u16..=30,
    ) {
        let mut store = ValueStore::with_max_slots(cap);
        let cap_u64 = u64::from(cap);

        // Insert 1 symbol, 1 list, 1 blob = 3 entries
        store.insert_symbol("a").expect("insert symbol must succeed");
        store.insert_list(vec![SlotValue::Null].into_boxed_slice()).expect("insert list must succeed");
        store.insert_blob(Bytes::new()).expect("insert blob must succeed");

        prop_assert_eq!(store.total_arena_count(), 3);

        if cap_u64 > 3 {
            // Should still have room
            let result = store.insert_symbol("b");
            prop_assert!(result.is_ok(), "should have room after 3 inserts");
        }
    }
}

// =========================================================================
// Properties: check_policy monotonicity
// =========================================================================

/// PROP-POLICY-001: validate_aggregate_budget is monotonic — if all budget
/// dimensions are within policy limits, always Ok.
proptest! {
    #[test]
    fn prop_policy_monotonic_within_limit(
        max_steps_executable in 0u32..=1_000_000,
        max_action_tickets in 0u32..=100_000,
        max_parallel_in_flight in 0u16..=256,
        max_run_time_seconds in 0u64..=2_592_000,
        max_result_bytes in 0u32..=262_144,
        max_total_slots_written in 0u32..=65_535,
    ) {
        let policy = BoundednessPolicy::DEFAULT;
        let budget = AggregateResourceBudget {
            max_steps_executable,
            max_action_tickets,
            max_parallel_in_flight,
            max_retries_per_action: 0,
            max_gather_pages: 0,
            max_gather_items: 0,
            max_for_each_iterations: 0,
            max_together_branches: 0,
            max_repeat_attempts: 0,
            max_run_time_seconds,
            max_result_bytes,
            max_total_slots_written,
            max_queue_depth: 0,
            max_journal_batch_bytes: 0,
            max_step_budget_per_tick: 1,
            max_transitions_per_tick: 1,
        };
        let result = crate::budget::validate_aggregate_budget(&budget, &policy);
        prop_assert!(result.is_ok(), "budget within policy must return Ok");
    }
}

/// PROP-POLICY-002: validate_aggregate_budget returns error when any bound exceeded.
proptest! {
    #[test]
    fn prop_policy_exceeded_returns_error(
        excess in 1u32..=u32::MAX,
    ) {
        let policy = BoundednessPolicy::DEFAULT;
        let over_limit = policy.absolute_max_steps_executable.saturating_add(excess);
        let budget = AggregateResourceBudget {
            max_steps_executable: over_limit,
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
            max_transitions_per_tick: 1,
        };
        let result = crate::budget::validate_aggregate_budget(&budget, &policy);
        prop_assert!(result.is_err(), "exceeding steps_executable must return error");
        if let Err(AggregateBudgetError::PolicyExceeded { resource, actual: a, limit: l }) = result {
            prop_assert_eq!(resource, "max_steps_executable");
            prop_assert_eq!(a, u64::from(over_limit));
            prop_assert_eq!(l, u64::from(policy.absolute_max_steps_executable));
        } else {
            prop_assert!(false, "expected PolicyExceeded variant");
        }
    }
}
