# RS-107-life: Unrepresentable timer deadlines fire immediately

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/shard/transitions/continuation.rs:137`
- **Confidence**: confirmed

## Description

When a timer duration is too large for `Instant::checked_add`, the code falls back to `Instant::now()`. An overlarge wait or ask timeout therefore becomes an immediate timeout instead of being rejected or clamped.

## Evidence

Deadline construction treats overflow as “now”:

```rust
let deadline = std::time::Instant::now()
    .checked_add(std::time::Duration::from_millis(deadline_ms))
    .unwrap_or_else(std::time::Instant::now);
```

`compute_deadline_ms_from_slot` can produce very large values, including `u64::MAX` for large finite floats:

```rust
if ms > u64::MAX as f64 {
    u64::MAX
} else {
    ms as u64
}
```

## Adversarial Check

This is not the documented zero-duration behavior. The comment says unreadable or non-numeric slots yield a zero deadline that fires immediately; it does not say representational overflow should fire immediately. `checked_add` returning `None` is an overflow/representability failure, and the current fallback silently changes a very long delay into no delay.

## Suggested Fix

Return a typed runtime error for unrepresentable deadlines, or clamp to the maximum representable `Instant` duration. Prefer enforcing a resource-contract maximum before constructing the `Duration` so timer semantics stay bounded and explicit.
