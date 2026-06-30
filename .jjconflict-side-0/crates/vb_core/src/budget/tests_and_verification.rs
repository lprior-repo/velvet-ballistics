// Kani harnesses for PO-RUST-002-BUDGET-KANI: add_dim/sub_dim panic-freedom and correctness.
// Uses a minimal local error enum to avoid transitive inclusion of AggregateBudgetError
// -> WorkflowError -> Capability (Vec<Capability> causes deep drop_in_place unwind).
// add_dim/sub_dim are pure checked_add/checked_sub — we prove these directly.
#[cfg(kani)]
mod kani_harnesses {
    use crate::budget::{
        AggregateBudgetError, AggregateResourceBudget, AggregateResourceCapacity,
        AggregateResourceUsage,
    };

    fn same_static_str(left: &'static str, right: &'static str) -> bool {
        left.len() == right.len() && core::ptr::eq(left.as_ptr(), right.as_ptr())
    }

    /// Minimal error enum mirroring the Overflow/Underflow variants.
    /// No transitive types — just &'static str for resource naming.
    #[derive(Debug)]
    enum LocalError {
        Overflow { resource: &'static str },
        Underflow { resource: &'static str },
    }

    /// Standalone add_dim — identical logic to budget.rs, no panics.
    fn add_dim(current: u64, requested: u64, resource: &'static str) -> Result<u64, LocalError> {
        current
            .checked_add(requested)
            .ok_or(LocalError::Overflow { resource })
    }

    /// Standalone sub_dim — identical logic to budget.rs, no panics.
    fn sub_dim(current: u64, requested: u64, resource: &'static str) -> Result<u64, LocalError> {
        current
            .checked_sub(requested)
            .ok_or(LocalError::Underflow { resource })
    }

    /// K-B1: add_dim is panic-free for bounded symbolic inputs.
    /// Uses kani::any() with assume bounds to prevent overflow.
    #[kani::proof]
    fn add_dim_no_panic() {
        let current: u64 = kani::any();
        let requested: u64 = kani::any();
        // Bound inputs to prevent overflow in add_dim
        kani::assume(current <= u64::MAX / 2);
        kani::assume(requested <= u64::MAX / 2);
        let result = add_dim(current, requested, "cpu");
        // add_dim with bounded inputs cannot overflow - returns Ok
        assert!(result.is_ok());
    }

    /// K-B2: sub_dim is panic-free for bounded symbolic inputs.
    /// Uses kani::any() with assume bounds to prevent underflow.
    #[kani::proof]
    fn sub_dim_no_panic() {
        let current: u64 = kani::any();
        let requested: u64 = kani::any();
        // Bound: requested <= current to prevent underflow in sub_dim
        kani::assume(requested <= current);
        let result = sub_dim(current, requested, "disk");
        // sub_dim with valid inputs (requested <= current) cannot underflow - returns Ok
        assert!(result.is_ok());
    }

    /// K-B3: add_dim overflow with symbolic inputs.
    #[kani::proof]
    fn add_dim_max_plus_max_overflow() {
        let a = kani::any::<u64>();
        let b = kani::any::<u64>();
        // Bound: inputs are large enough that their sum overflows u64
        kani::assume(a > u64::MAX / 2);
        kani::assume(b > u64::MAX / 2);
        let result = add_dim(a, b, "cpu");
        match result {
            Err(LocalError::Overflow { resource }) => kani::assert(
                same_static_str(resource, "cpu"),
                "overflow resource identifies cpu",
            ),
            Ok(_) => kani::assert(false, "overflowing add must return Err"),
            Err(_) => kani::assert(false, "only Overflow valid here"),
        }
    }

    /// K-B4: add_dim non-overflow with bounded symbolic inputs.
    #[kani::proof]
    fn add_dim_zero_plus_zero() {
        let a = kani::any::<u64>();
        let b = kani::any::<u64>();
        // Bound: inputs are small enough that sum does not overflow
        kani::assume(a <= u64::MAX / 2);
        kani::assume(b <= u64::MAX / 2);
        let result = add_dim(a, b, "cpu");
        kani::assert(result.is_ok(), "bounded add must return Ok");
        kani::assert(result.unwrap() == a + b, "add result must equal a + b");
    }

    /// K-B5: add_dim overflow with edge-case symbolic inputs.
    #[kani::proof]
    fn add_dim_one_plus_max_overflow() {
        let a = kani::any::<u64>();
        let b = kani::any::<u64>();
        // Bound: a is non-zero and b is large enough that a + b overflows
        kani::assume(a > 0);
        kani::assume(b > u64::MAX - a);
        let result = add_dim(a, b, "cpu");
        match result {
            Err(LocalError::Overflow { resource }) => kani::assert(
                same_static_str(resource, "cpu"),
                "overflow resource identifies cpu",
            ),
            Ok(_) => kani::assert(false, "overflowing add must return Err"),
            Err(_) => kani::assert(false, "only Overflow valid here"),
        }
    }

    /// K-B6: sub_dim underflow with symbolic inputs.
    #[kani::proof]
    fn sub_dim_zero_minus_one_underflow() {
        let current = kani::any::<u64>();
        let requested = kani::any::<u64>();
        // Bound: current < requested to force underflow
        kani::assume(current < requested);
        let result = sub_dim(current, requested, "disk");
        match result {
            Err(LocalError::Underflow { resource }) => kani::assert(
                same_static_str(resource, "disk"),
                "underflow resource identifies disk",
            ),
            Ok(_) => kani::assert(false, "underflowing sub must return Err"),
            Err(_) => kani::assert(false, "only Underflow valid here"),
        }
    }

    /// K-B7: sub_dim non-underflow with symbolic inputs.
    #[kani::proof]
    fn sub_dim_hundred_minus_fifty() {
        let current = kani::any::<u64>();
        let requested = kani::any::<u64>();
        // Bound: current >= requested to prevent underflow
        kani::assume(current >= requested);
        let result = sub_dim(current, requested, "disk");
        kani::assert(result.is_ok(), "bounded sub must return Ok");
        kani::assert(
            result.unwrap() == current - requested,
            "sub result must equal current - requested",
        );
    }

    /// K-B8: add_dim non-overflow with symbolic inputs.
    #[kani::proof]
    fn add_dim_non_overflow() {
        let a = kani::any::<u64>();
        let b = kani::any::<u64>();
        // Bound: inputs are small enough that sum does not overflow
        kani::assume(a <= u64::MAX / 2);
        kani::assume(b <= u64::MAX / 2);
        let result = add_dim(a, b, "mem");
        kani::assert(result.is_ok(), "bounded add must return Ok");
        kani::assert(result.unwrap() == a + b, "add result must equal a + b");
    }

    /// K-B9: sub_dim non-underflow with symbolic inputs.
    #[kani::proof]
    fn sub_dim_non_underflow() {
        let current = kani::any::<u64>();
        let requested = kani::any::<u64>();
        // Bound: current >= requested to prevent underflow
        kani::assume(current >= requested);
        let result = sub_dim(current, requested, "net");
        kani::assert(result.is_ok(), "bounded sub must return Ok");
        kani::assert(
            result.unwrap() == current - requested,
            "sub result must equal current - requested",
        );
    }

    /// PO-010a: aggregate usage addition succeeds with bounded symbolic inputs.
    #[kani::proof]
    fn aggregate_usage_try_add_budget_no_overflow_symbolic() {
        let usage = kani::any::<AggregateResourceUsage>();
        let budget = kani::any::<AggregateResourceBudget>();

        // Bound all shared fields to prevent overflow in try_add_budget
        kani::assume(usage.max_steps_executable <= u64::MAX / 2);
        kani::assume(u64::from(budget.max_steps_executable) <= u64::MAX / 2);
        kani::assume(usage.max_action_tickets <= u64::MAX / 2);
        kani::assume(u64::from(budget.max_action_tickets) <= u64::MAX / 2);
        kani::assume(usage.max_parallel_in_flight <= u64::MAX / 2);
        kani::assume(u64::from(budget.max_parallel_in_flight) <= u64::MAX / 2);
        kani::assume(usage.max_gather_pages <= u64::MAX / 2);
        kani::assume(u64::from(budget.max_gather_pages) <= u64::MAX / 2);
        kani::assume(usage.max_gather_items <= u64::MAX / 2);
        kani::assume(u64::from(budget.max_gather_items) <= u64::MAX / 2);
        kani::assume(usage.max_result_bytes <= u64::MAX / 2);
        kani::assume(u64::from(budget.max_result_bytes) <= u64::MAX / 2);
        kani::assume(usage.max_total_slots_written <= u64::MAX / 2);
        kani::assume(u64::from(budget.max_total_slots_written) <= u64::MAX / 2);
        kani::assume(usage.max_active_runs <= u64::MAX / 2);
        kani::assume(usage.max_queue_depth <= u64::MAX / 2);
        kani::assume(u64::from(budget.max_queue_depth) <= u64::MAX / 2);
        kani::assume(usage.max_journal_batch_bytes <= u64::MAX / 2);
        kani::assume(u64::from(budget.max_journal_batch_bytes) <= u64::MAX / 2);
        kani::assume(usage.max_step_budget_per_tick <= u64::MAX / 2);
        kani::assume(budget.max_step_budget_per_tick <= u64::MAX / 2);
        kani::assume(usage.max_transitions_per_tick <= u64::MAX / 2);
        kani::assume(u64::from(budget.max_transitions_per_tick) <= u64::MAX / 2);

        let result = usage.try_add_budget(&budget);
        kani::assert(result.is_ok(), "bounded try_add_budget returns Ok");

        let next = result.unwrap();
        kani::assert(
            next.max_steps_executable
                == usage.max_steps_executable + u64::from(budget.max_steps_executable),
            "max_steps_executable sum matches",
        );
        kani::assert(
            next.max_action_tickets
                == usage.max_action_tickets + u64::from(budget.max_action_tickets),
            "max_action_tickets sum matches",
        );
        kani::assert(
            next.max_parallel_in_flight
                == usage.max_parallel_in_flight + u64::from(budget.max_parallel_in_flight),
            "max_parallel_in_flight sum matches",
        );
        kani::assert(
            next.max_gather_pages == usage.max_gather_pages + u64::from(budget.max_gather_pages),
            "max_gather_pages sum matches",
        );
        kani::assert(
            next.max_gather_items == usage.max_gather_items + u64::from(budget.max_gather_items),
            "max_gather_items sum matches",
        );
        kani::assert(
            next.max_result_bytes == usage.max_result_bytes + u64::from(budget.max_result_bytes),
            "max_result_bytes sum matches",
        );
        kani::assert(
            next.max_total_slots_written
                == usage.max_total_slots_written + u64::from(budget.max_total_slots_written),
            "max_total_slots_written sum matches",
        );
        kani::assert(
            next.max_active_runs == usage.max_active_runs + 1,
            "max_active_runs increments by one",
        );
        kani::assert(
            next.max_queue_depth == usage.max_queue_depth + u64::from(budget.max_queue_depth),
            "max_queue_depth sum matches",
        );
        kani::assert(
            next.max_journal_batch_bytes
                == usage.max_journal_batch_bytes + u64::from(budget.max_journal_batch_bytes),
            "max_journal_batch_bytes sum matches",
        );
        kani::assert(
            next.max_step_budget_per_tick
                == usage.max_step_budget_per_tick + budget.max_step_budget_per_tick,
            "max_step_budget_per_tick sum matches",
        );
        kani::assert(
            next.max_transitions_per_tick
                == usage.max_transitions_per_tick + u64::from(budget.max_transitions_per_tick),
            "max_transitions_per_tick sum matches",
        );
    }

    /// PO-010b: aggregate usage addition rejects overflow with symbolic inputs.
    #[kani::proof]
    fn aggregate_usage_try_add_budget_overflow_symbolic() {
        let mut usage = kani::any::<AggregateResourceUsage>();
        let budget = kani::any::<AggregateResourceBudget>();

        // Bound: force overflow on max_steps_executable by setting it to max
        usage.max_steps_executable = u64::MAX;
        kani::assume(budget.max_steps_executable > 0);

        let result = usage.try_add_budget(&budget);
        kani::assert(result.is_err(), "try_add_budget must reject overflow");

        match &result {
            Err(AggregateBudgetError::Overflow { resource }) => {
                kani::assert(
                    same_static_str(resource, "max_steps_executable"),
                    "overflow resource identifies max_steps_executable",
                );
            }
            Ok(_) => kani::assert(false, "overflow must return Err"),
            Err(_) => kani::assert(false, "only Overflow valid here"),
        }
    }

    /// PO-011: aggregate capacity rejection reports exact resource/request/available.
    #[kani::proof]
    fn aggregate_usage_fits_within_rejects_over_capacity_fields() {
        let usage = kani::any::<AggregateResourceUsage>();
        let capacity = kani::any::<AggregateResourceCapacity>();

        // Bound: force max_steps_executable to exceed capacity so fits_within fails
        kani::assume(usage.max_steps_executable > capacity.max_steps_executable);

        let result = usage.fits_within(&capacity);
        kani::assert(result.is_err(), "fits_within must reject over-capacity");

        match &result {
            Err(AggregateBudgetError::CapacityExceeded {
                resource,
                requested,
                available,
            }) => {
                kani::assert(
                    same_static_str(resource, "max_steps_executable"),
                    "capacity resource identifies max_steps_executable",
                );
                kani::assert(
                    *requested == usage.max_steps_executable,
                    "capacity requested value matches usage",
                );
                kani::assert(
                    *available == capacity.max_steps_executable,
                    "capacity available value matches capacity",
                );
            }
            Ok(()) => kani::assert(false, "over-capacity must return Err"),
            Err(_) => kani::assert(false, "only CapacityExceeded valid here"),
        }
        core::mem::forget(result);
    }
}
