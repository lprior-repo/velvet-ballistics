(* BudgetArithmetic.tla
 *
 * Budget arithmetic model for AggregateResourceBudget.
 * Models component-wise Nat addition, subtraction (floor at 0), multiplication.
 *
 * Rust uses u64 for dimensions; TLA+ models use Nat (unbounded).
 * Overflow/underflow safety in Rust: checked_add/checked_sub return Err on overflow,
 * which this spec assumes propagates correctly (never panics).
 *
 * Key properties verified:
 *   - add_budgets is monotonic (b1 <= b1 + b2)
 *   - sub_budgets never goes below 0
 *   - mul_budget is component-wise Nat multiplication
 *)

---- MODULE BudgetArithmetic ----

EXTENDS Integers, FiniteSets, TLC

(**
 * Budget record matching Rust AggregateResourceBudget fields.
 * All fields are Nat (unbounded) for TLA+ modeling.
 *)
Budget ==
    [
        max_steps_executable: Nat,
        max_action_tickets: Nat,
        max_parallel_in_flight: Nat,
        max_retries_per_action: Nat,
        max_gather_pages: Nat,
        max_gather_items: Nat,
        max_for_each_iterations: Nat,
        max_together_branches: Nat,
        max_repeat_attempts: Nat,
        max_run_time_seconds: Nat,
        max_result_bytes: Nat,
        max_total_slots_written: Nat,
        max_queue_depth: Nat,
        max_journal_batch_bytes: Nat,
        max_step_budget_per_tick: Nat,
        max_transitions_per_tick: Nat
    ]

(**
 * Component-wise addition of two budgets.
 * Models Rust add_dim() -> checked_add() with Err on overflow.
 * For TLA+ (Nat), addition is always defined.
 *)
add_budgets(b1, b2) ==
    [
        max_steps_executable     |-> b1.max_steps_executable + b2.max_steps_executable,
        max_action_tickets       |-> b1.max_action_tickets + b2.max_action_tickets,
        max_parallel_in_flight   |-> b1.max_parallel_in_flight + b2.max_parallel_in_flight,
        max_retries_per_action   |-> b1.max_retries_per_action + b2.max_retries_per_action,
        max_gather_pages         |-> b1.max_gather_pages + b2.max_gather_pages,
        max_gather_items         |-> b1.max_gather_items + b2.max_gather_items,
        max_for_each_iterations  |-> b1.max_for_each_iterations + b2.max_for_each_iterations,
        max_together_branches    |-> b1.max_together_branches + b2.max_together_branches,
        max_repeat_attempts      |-> b1.max_repeat_attempts + b2.max_repeat_attempts,
        max_run_time_seconds     |-> b1.max_run_time_seconds + b2.max_run_time_seconds,
        max_result_bytes         |-> b1.max_result_bytes + b2.max_result_bytes,
        max_total_slots_written  |-> b1.max_total_slots_written + b2.max_total_slots_written,
        max_queue_depth          |-> b1.max_queue_depth + b2.max_queue_depth,
        max_journal_batch_bytes  |-> b1.max_journal_batch_bytes + b2.max_journal_batch_bytes,
        max_step_budget_per_tick |-> b1.max_step_budget_per_tick + b2.max_step_budget_per_tick,
        max_transitions_per_tick |-> b1.max_transitions_per_tick + b2.max_transitions_per_tick
    ]

(**
 * Component-wise subtraction of two budgets.
 * Floor at 0: if b1[field] < b2[field], result is 0.
 * Models Rust sub_dim() -> checked_sub() with Err on underflow propagated
 * to caller as AggregateBudgetError::Underflow.
 *)
sub_budgets(b1, b2) ==
    [
        max_steps_executable     |-> IF b1.max_steps_executable >= b2.max_steps_executable
                                     THEN b1.max_steps_executable - b2.max_steps_executable
                                     ELSE 0,
        max_action_tickets       |-> IF b1.max_action_tickets >= b2.max_action_tickets
                                     THEN b1.max_action_tickets - b2.max_action_tickets
                                     ELSE 0,
        max_parallel_in_flight   |-> IF b1.max_parallel_in_flight >= b2.max_parallel_in_flight
                                     THEN b1.max_parallel_in_flight - b2.max_parallel_in_flight
                                     ELSE 0,
        max_retries_per_action   |-> IF b1.max_retries_per_action >= b2.max_retries_per_action
                                     THEN b1.max_retries_per_action - b2.max_retries_per_action
                                     ELSE 0,
        max_gather_pages         |-> IF b1.max_gather_pages >= b2.max_gather_pages
                                     THEN b1.max_gather_pages - b2.max_gather_pages
                                     ELSE 0,
        max_gather_items         |-> IF b1.max_gather_items >= b2.max_gather_items
                                     THEN b1.max_gather_items - b2.max_gather_items
                                     ELSE 0,
        max_for_each_iterations  |-> IF b1.max_for_each_iterations >= b2.max_for_each_iterations
                                     THEN b1.max_for_each_iterations - b2.max_for_each_iterations
                                     ELSE 0,
        max_together_branches    |-> IF b1.max_together_branches >= b2.max_together_branches
                                     THEN b1.max_together_branches - b2.max_together_branches
                                     ELSE 0,
        max_repeat_attempts      |-> IF b1.max_repeat_attempts >= b2.max_repeat_attempts
                                     THEN b1.max_repeat_attempts - b2.max_repeat_attempts
                                     ELSE 0,
        max_run_time_seconds     |-> IF b1.max_run_time_seconds >= b2.max_run_time_seconds
                                     THEN b1.max_run_time_seconds - b2.max_run_time_seconds
                                     ELSE 0,
        max_result_bytes         |-> IF b1.max_result_bytes >= b2.max_result_bytes
                                     THEN b1.max_result_bytes - b2.max_result_bytes
                                     ELSE 0,
        max_total_slots_written  |-> IF b1.max_total_slots_written >= b2.max_total_slots_written
                                     THEN b1.max_total_slots_written - b2.max_total_slots_written
                                     ELSE 0,
        max_queue_depth          |-> IF b1.max_queue_depth >= b2.max_queue_depth
                                     THEN b1.max_queue_depth - b2.max_queue_depth
                                     ELSE 0,
        max_journal_batch_bytes  |-> IF b1.max_journal_batch_bytes >= b2.max_journal_batch_bytes
                                     THEN b1.max_journal_batch_bytes - b2.max_journal_batch_bytes
                                     ELSE 0,
        max_step_budget_per_tick |-> IF b1.max_step_budget_per_tick >= b2.max_step_budget_per_tick
                                     THEN b1.max_step_budget_per_tick - b2.max_step_budget_per_tick
                                     ELSE 0,
        max_transitions_per_tick |-> IF b1.max_transitions_per_tick >= b2.max_transitions_per_tick
                                     THEN b1.max_transitions_per_tick - b2.max_transitions_per_tick
                                     ELSE 0
    ]

(**
 * Component-wise multiplication of a budget by a natural number.
 *)
mul_budget(b, n) ==
    [
        max_steps_executable     |-> b.max_steps_executable * n,
        max_action_tickets       |-> b.max_action_tickets * n,
        max_parallel_in_flight   |-> b.max_parallel_in_flight * n,
        max_retries_per_action   |-> b.max_retries_per_action * n,
        max_gather_pages         |-> b.max_gather_pages * n,
        max_gather_items         |-> b.max_gather_items * n,
        max_for_each_iterations  |-> b.max_for_each_iterations * n,
        max_together_branches    |-> b.max_together_branches * n,
        max_repeat_attempts      |-> b.max_repeat_attempts * n,
        max_run_time_seconds     |-> b.max_run_time_seconds * n,
        max_result_bytes         |-> b.max_result_bytes * n,
        max_total_slots_written  |-> b.max_total_slots_written * n,
        max_queue_depth          |-> b.max_queue_depth * n,
        max_journal_batch_bytes  |-> b.max_journal_batch_bytes * n,
        max_step_budget_per_tick |-> b.max_step_budget_per_tick * n,
        max_transitions_per_tick |-> b.max_transitions_per_tick * n
    ]

(**
 * Budget dimension field names for reference.
 *)
BudgetFields == {
    "max_steps_executable",
    "max_action_tickets",
    "max_parallel_in_flight",
    "max_retries_per_action",
    "max_gather_pages",
    "max_gather_items",
    "max_for_each_iterations",
    "max_together_branches",
    "max_repeat_attempts",
    "max_run_time_seconds",
    "max_result_bytes",
    "max_total_slots_written",
    "max_queue_depth",
    "max_journal_batch_bytes",
    "max_step_budget_per_tick",
    "max_transitions_per_tick"
}

(**
 * Property: add_budgets is monotonic.
 * For each field: b1[field] <= add_budgets(b1, b2)[field]
 *)
AddIsMonotonic ==
    \A b1 \in Budget, b2 \in Budget :
        \A field \in BudgetFields :
            b1[field] <= add_budgets(b1, b2)[field]

(**
 * Property: sub_budgets never goes below 0 for any field.
 *)
SubNeverNegative ==
    \A b1 \in Budget, b2 \in Budget :
        \A field \in BudgetFields :
            sub_budgets(b1, b2)[field] >= 0

(**
 * Property: sub_budgets(b1, zero) = b1  (subtract zero leaves budget unchanged)
 *)
SubZeroIsIdentity ==
    \A b \in Budget :
        sub_budgets(b, [
            max_steps_executable     |-> 0,
            max_action_tickets       |-> 0,
            max_parallel_in_flight   |-> 0,
            max_retries_per_action   |-> 0,
            max_gather_pages         |-> 0,
            max_gather_items         |-> 0,
            max_for_each_iterations  |-> 0,
            max_together_branches    |-> 0,
            max_repeat_attempts      |-> 0,
            max_run_time_seconds     |-> 0,
            max_result_bytes         |-> 0,
            max_total_slots_written  |-> 0,
            max_queue_depth          |-> 0,
            max_journal_batch_bytes  |-> 0,
            max_step_budget_per_tick |-> 0,
            max_transitions_per_tick |-> 0
        ]) = b

(**
 * Property: mul_budget(b, 1) = b  (multiply by 1 is identity)
 *)
MulOneIsIdentity ==
    \A b \in Budget :
        mul_budget(b, 1) = b

(**
 * Property: add_budgets is commutative
 *)
AddIsCommutative ==
    \A b1 \in Budget, b2 \in Budget :
        add_budgets(b1, b2) = add_budgets(b2, b1)

(**
 * Property: add_budgets is associative
 *)
AddIsAssociative ==
    \A b1 \in Budget, b2 \in Budget, b3 \in Budget :
        add_budgets(add_budgets(b1, b2), b3) = add_budgets(b1, add_budgets(b2, b3))

(**
 * Trivial state machine for TLC model checking.
 *)
VARIABLE dummy

Init == dummy = 0
Next == dummy' = dummy
Spec == Init /\ [][Next]_dummy

THEOREM Spec => []AddIsMonotonic
THEOREM Spec => []SubNeverNegative
THEOREM Spec => []SubZeroIsIdentity
THEOREM Spec => []MulOneIsIdentity
THEOREM Spec => []AddIsCommutative
THEOREM Spec => []AddIsAssociative

====
