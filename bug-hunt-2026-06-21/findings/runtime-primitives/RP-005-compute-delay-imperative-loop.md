# RP-005: `compute_delay` exponential backoff uses imperative multiply loop

- **Severity**: Low
- **Category**: simplification
- **Location**: `crates/vb_runtime/src/primitives/retry.rs:47-69`
- **Confidence**: confirmed

## Description

The exponential-backoff branch of `compute_delay` computes `delay_ms * multiplier^exponent` via an explicit `while` loop with manual `saturating_add(1)` on the induction variable. `u32::checked_pow` already expresses this exactly with the same saturation semantics in one expression.

## Evidence

`crates/vb_runtime/src/primitives/retry.rs:55-67`:

```rust
let mut delay = policy.delay_ms();
let multiplier = policy.backoff_multiplier();
let mut i: u32 = 0;
while i < exponent {
    delay = match delay.checked_mul(multiplier) {
        Some(d) => d,
        None => return u32::MAX,
    };
    i = i.saturating_add(1);
}
delay
```

`saturating_add(1)` on `i` is dead defensiveness — the loop condition guarantees `i < exponent <= u32::MAX`, so `i + 1` cannot overflow. The whole loop is the standard "pow then mul, saturating on overflow" pattern.

## Adversarial Check

`checked_pow` returns `None` on overflow; the current code saturates the *running product* on the first overflowing multiply. These are equivalent: if `multiplier^exponent` overflows, then some intermediate `delay * multiplier^k` must also overflow (delay ≥ 1 because `delay_ms == 0` short-circuits earlier in `compute_delay` for `Fixed`/`ExponentialBackoff` when combined with `multiplier` ≥ 1). So `checked_pow` faithfully preserves the saturation contract.

The simplification is purely cosmetic, hence Low / Info severity.

## Suggested Fix

```rust
DelayStrategy::ExponentialBackoff => {
    let exponent = u32::from(current_attempt.saturating_sub(1));
    let factor   = policy.backoff_multiplier().checked_pow(exponent).unwrap_or(u32::MAX);
    policy.delay_ms().checked_mul(factor).unwrap_or(u32::MAX)
}
```

Removes the loop, the mutable `i`, and the redundant `saturating_add`. Bonus: also resolves RP-006.
