# CF-001: `RunFrame::max_parallel_in_flight` is documented as a ceiling but behaves as a high-water mark

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_core/src/frame/parallel.rs:19`
- **Confidence**: confirmed

## Description

`add_parallel_in_flight` does not enforce any configured ceiling. When the
new `parallel_in_flight` exceeds `max_parallel_in_flight`, the function
*ratchets `max_parallel_in_flight` upward* instead of returning an error.
The accessor docstring (`accessors.rs:38-40`) says the field is the
"Maximum allowed parallel in-flight branches for this workflow" — i.e. a
configured ceiling — but no caller ever enforces it. The result is that a
workflow can spawn arbitrarily many concurrent branches without any
admission rejection.

## Evidence

```rust
pub fn add_parallel_in_flight(&mut self, count: u16) -> CoreResult<()> {
    self.parallel_in_flight = self.parallel_in_flight.checked_add(count).ok_or(
        CoreError::InternalInvariantViolation {
            reason: "parallel_in_flight overflow",
        },
    )?;
    if self.parallel_in_flight > self.max_parallel_in_flight {
        self.max_parallel_in_flight = self.parallel_in_flight;   // <-- ratchet, not enforce
    }
    Ok(())
}
```

(`crates/vb_core/src/frame/parallel.rs:19-29`)

The existing test `parallel_in_flight_overflow_returns_error`
(`frame/tests_and_verification.rs:730-739`) sets
`set_max_parallel_in_flight(u16::MAX - 1)` and then successfully calls
`add_parallel_in_flight(u16::MAX)` — confirming the ceiling is not
enforced, only the u16 arithmetic overflow is.

## Adversarial Check

A defender might say "the configured ceiling lives in the budget module's
`WholeWorkflowBudget.max_parallel_in_flight` and is enforced separately;
the RunFrame field is purely observability." But the RunFrame field is
*set* by `set_max_parallel_in_flight(limit)` (line 13), which strongly
suggests configuration, not observation. And the docstring on the accessor
uses the word "allowed" — a normative ceiling term. Either the docstring is
wrong or the enforcement is missing; in either case the public API is
misleading. If two engineers wire `set_max_parallel_in_flight(10)` and
then `add_parallel_in_flight(20)` expecting an error, they will get
silent success and a `max_parallel_in_flight == 20` high-water mark.

## Suggested Fix

Either:
(a) rename to `peak_parallel_in_flight` and remove `set_max_parallel_in_flight`,
documenting the field as observability only; or
(b) keep the name, change `add_parallel_in_flight` to return
`CoreError::BudgetExceeded` when `parallel_in_flight + count > max_parallel_in_flight`,
and stop ratcheting.
