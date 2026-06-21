# RP-006: `compute_delay` redundant `if current_attempt > 0` guard

- **Severity**: Info
- **Category**: simplification
- **Location**: `crates/vb_runtime/src/primitives/retry.rs:50-54`
- **Confidence**: confirmed

## Description

The exponent computation guards `current_attempt > 0` before calling `saturating_sub(1)`. `saturating_sub` already returns 0 for `0u16.saturating_sub(1)`, so the branch is dead.

## Evidence

`crates/vb_runtime/src/primitives/retry.rs:50-54`:

```rust
let exponent = if current_attempt > 0 {
    u32::from(current_attempt.saturating_sub(1))
} else {
    0
};
```

Both arms produce `0` when `current_attempt == 0`. The `if` adds one branch and one constant for no behavioral effect.

## Adversarial Check

`u16::saturating_sub` is total and returns 0 for underflow. There is no scenario in which the two arms diverge.

## Suggested Fix

```rust
let exponent = u32::from(current_attempt.saturating_sub(1));
```

Folded into the `checked_pow` rewrite proposed in RP-005.
