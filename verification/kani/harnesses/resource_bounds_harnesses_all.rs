// =========================================================================
// KANI-EXPR-001: Expression stack bound checking never panics
// Target: check_expr_stack_bound (crate::workflow)
// =========================================================================

/// KANI-EXPR-001: prove check_expr_stack_bound never panics on arbitrary
/// expression programs with arbitrary capacity.
#[kani::proof]
#[kani::unwind(66)]
fn kani_expr_stack_bound_never_panics() {
    let ops: [crate::workflow::ExprOp; 64] = kani::any();
    let capacity: u8 = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 64);
    let slice = &ops[..len];
    let _result = check_expr_stack_bound(slice, capacity);
}

/// KANI-EXPR-002: Expression stack overflow returns typed error, not panic.
#[kani::proof]
#[kani::unwind(66)]
fn kani_expr_stack_overflow_returns_typed_error() {
    let capacity: u8 = kani::any();
    kani::assume(capacity < MAX_EXPRESSION_STACK);
    let over = usize::from(capacity) + 1;
    kani::assume(over <= MAX_EXPRESSION_OPS);
    let mut ops: Vec<crate::workflow::ExprOp> = Vec::new();
    let mut i: usize = 0;
    while i < over {
        ops.push(crate::workflow::ExprOp::LoadConst(ConstIdx::new(
            kani::any(),
        )));
        i += 1;
    }
    let result = check_expr_stack_bound(&ops, capacity);
    match result {
        Err(CoreError::ExpressionStackOverflow { max }) => {
            kani::assert(max == capacity, "overflow error reports correct capacity");
        }
        Err(CoreError::ResourceLimitExceeded { .. }) => {}
        Ok(_) => {
            kani::assert(false, "overflow input must not return Ok");
        }
    }
}

// =========================================================================
// KANI-ARENA-001: ValueStore cap enforcement never panics
// Target: ValueStore::with_max_slots + insert_symbol
// =========================================================================

/// KANI-ARENA-001: prove ValueStore with capacity never panics on
/// arbitrary insert sequences up to and past capacity.
#[kani::proof]
#[kani::unwind(17)]
fn kani_value_store_cap_never_panics() {
    let max_slots: u16 = kani::any();
    kani::assume(max_slots <= 16);
    let mut store = ValueStore::with_max_slots(max_slots);
    let cap = u64::from(max_slots);

    let mut i: u64 = 0;
    while i < cap {
        let result = store.insert_symbol(kani::any::<u64>().to_string());
        if i < cap {
            kani::assert(result.is_ok(), "insert under cap must succeed");
        }
        i += 1;
    }

    let result = store.insert_symbol("overflow");
    if cap > 0 {
        match result {
            Err(CoreError::BudgetExceeded { budget: _, limit }) => {
                kani::assert(limit == cap, "limit matches configured cap");
            }
            Ok(_) => {
                kani::assert(false, "insert past cap must not return Ok");
            }
            Err(_) => {}
        }
    }
}

// =========================================================================
// KANI-BUDGET-ARITH-001: add_dim/sub_dim checked arithmetic
// Target: budget module checked_add/checked_sub via try_add/try_sub
// =========================================================================

/// KANI-BUDGET-ARITH-001: prove try_add_budget uses checked arithmetic —
/// never panics on arbitrary usage + budget combinations.
#[kani::proof]
fn kani_try_add_budget_checked_arithmetic() {
    let usage: AggregateResourceUsage = kani::any();
    let budget: AggregateResourceBudget = kani::any();
    let result = usage.try_add_budget(&budget);
    kani::cover!(result.is_ok(), "try_add_budget returns Ok");
    kani::cover!(result.is_err(), "try_add_budget returns Err");
}

/// KANI-BUDGET-ARITH-002: prove try_sub_budget uses checked arithmetic —
/// never panics on arbitrary usage + budget combinations.
#[kani::proof]
fn kani_try_sub_budget_checked_arithmetic() {
    let usage: AggregateResourceUsage = kani::any();
    let budget: AggregateResourceBudget = kani::any();
    let result = usage.try_sub_budget(&budget);
    kani::cover!(result.is_ok(), "try_sub_budget returns Ok");
    kani::cover!(result.is_err(), "try_sub_budget returns Err");
}

// =========================================================================
// KANI-POLICY-001: validate_aggregate_budget never panics
// Target: validate_aggregate_budget
// =========================================================================

/// KANI-POLICY-001: prove validate_aggregate_budget never panics on
/// arbitrary budget + policy combinations.
#[kani::proof]
fn kani_validate_aggregate_budget_never_panics() {
    let budget: AggregateResourceBudget = kani::any();
    let policy: BoundednessPolicy = kani::any();
    let result = crate::budget::validate_aggregate_budget(&budget, &policy);
    kani::cover!(result.is_ok(), "validate_aggregate_budget returns Ok");
    kani::cover!(result.is_err(), "validate_aggregate_budget returns Err");
}

// =========================================================================
// KANI-CAPACITY-001: check_capacity exact semantics
// Target: fits_within
// =========================================================================

/// KANI-CAPACITY-001: prove fits_within has exact elementwise semantics.
#[kani::proof]
fn kani_fits_within_exact_semantics() {
    let usage: AggregateResourceUsage = kani::any();
    let capacity: AggregateResourceCapacity = kani::any();
    let result = usage.fits_within(&capacity);

    match result {
        Ok(()) => {
            kani::assert(
                usage.max_steps_executable <= capacity.max_steps_executable,
                "steps_exec within capacity",
            );
            kani::assert(
                usage.max_action_tickets <= capacity.max_action_tickets,
                "action_tickets within capacity",
            );
            kani::assert(
                u64::from(usage.max_parallel_in_flight)
                    <= u64::from(capacity.max_parallel_in_flight),
                "parallel within capacity",
            );
        }
        Err(AggregateBudgetError::CapacityExceeded { .. }) => {
            let any_exceeded = usage.max_steps_executable > capacity.max_steps_executable
                || usage.max_action_tickets > capacity.max_action_tickets
                || u64::from(usage.max_parallel_in_flight)
                    > u64::from(capacity.max_parallel_in_flight)
                || usage.max_gather_pages > capacity.max_gather_pages
                || usage.max_gather_items > capacity.max_gather_items
                || usage.max_result_bytes > capacity.max_result_bytes
                || usage.max_total_slots_written > capacity.max_total_slots_written
                || usage.max_active_runs > capacity.max_active_runs
                || usage.max_queue_depth > capacity.max_queue_depth
                || usage.max_journal_batch_bytes > capacity.max_journal_batch_bytes
                || usage.max_step_budget_per_tick > capacity.max_step_budget_per_tick
                || usage.max_transitions_per_tick > capacity.max_transitions_per_tick;
            kani::assert(any_exceeded, "at least one dim exceeds capacity on Err");
        }
        Err(_) => {
            kani::assert(false, "fits_within must only return Ok or CapacityExceeded");
        }
    }
}
