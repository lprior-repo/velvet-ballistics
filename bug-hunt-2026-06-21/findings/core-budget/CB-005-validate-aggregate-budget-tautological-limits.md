# CB-005: `validate_aggregate_budget` uses type-MAX as policy limit for several dimensions

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/budget/validation.rs:33` (and 38, 43, 48, 58)
- **Confidence**: confirmed

## Description

`validate_aggregate_budget` invokes `check_policy` against `u64::from(u16::MAX)`
for `max_retries_per_action` and `max_repeat_attempts`, and against
`u64::from(u32::MAX)` for `max_gather_pages`, `max_gather_items`, and
`max_for_each_iterations`. Because the budget field is itself bounded by
that same type, these checks can *never* trip, so the policy does not
constrain these dimensions at all.

## Evidence

```rust
check_policy(
    "max_retries_per_action",
    u64::from(budget.max_retries_per_action),
    u64::from(u16::MAX),             // <-- budget.max_retries_per_action is u16
)?;
check_policy(
    "max_gather_pages",
    u64::from(budget.max_gather_pages),
    u64::from(u32::MAX),             // <-- budget.max_gather_pages is u32
)?;
```

(`crates/vb_core/src/budget/validation.rs:30-58`)

The same pattern applies to `max_gather_items`, `max_for_each_iterations`,
and `max_repeat_attempts`.

## Adversarial Check

One might say "these dimensions are inherently bounded by their types, so the
check is structural and intentionally a no-op." But then the check should be
encoded as a static type-level assertion or omitted, not presented as a
policy gate. As written, an operator reading the policy file thinks these
dimensions are policed; an engineer wiring telemetry sees zero
`PolicyExceeded` events for them; and a security reviewer cannot tell
whether the absence of failures is real or a tautology. The master spec
§65 line 3241 enumerates a small, finite set of `absolute_max_*` policy
fields; padding the validator with tautological `u16::MAX / u32::MAX` calls
obscures which limits are actually configured.

## Suggested Fix

Either remove the tautological checks, or extend `BoundednessPolicy` with
real `absolute_max_*` fields (with concrete defaults like 1024 retries, 1M
gather items) so the policy actually constrains these dimensions.
