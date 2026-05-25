# Trusted Base Plan - vb-fq6u

## Trusted Components

- **Rust u64/u32 arithmetic**: Rust's `u64` and `u32` types have well-defined overflow semantics (wrap for debug, wrap for release by default; saturating arithmetic via `.saturating_add()` is explicitly requested in the fix).
- **`saturating_add` implementation**: The fix uses the standard library's `saturating_add` which is trusted to implement correct saturation semantics.
- **clippy::arithmetic_side_effects rule**: The lint correctly identifies bare `+` on integer types; fix is mechanical and semantically correct per contract.
- **moon ci toolchain**: Nightly Rust, clippy, cargo fmt are trusted for repository-level gates.

## Untrusted Inputs

- None. `SmallLinearMetrics::add` operates purely on its own struct fields.

## Model Bounds

- **MAX_STEPS**: `u64::MAX` - saturation at `u64::MAX`
- **MAX_ACTIONS**: `u32::MAX` - saturation at `u32::MAX`
- **MAX_TIMERS**: `u32::MAX` - saturation at `u32::MAX`
- **Node count**: `<= 1_000_000` per `BoundednessPolicy::absolute_max_steps_executable`

## Fail-Closed Requirements

- Overflow in `SmallLinearMetrics::add` must saturate at MAX, never wrap to zero.
- A wrap-to-zero would cause budget underestimation, potentially allowing unbounded workflow admission.
- Budget underestimation is a **fail-open** condition that could cause resource exhaustion.

## Residual Trusted-Base Risk

- **Formal proof scope**: Verus spec fns are auxiliary until proof-reviewer explicitly accepts their scope.
- **Kani model completeness**: Kani verification is bounded by harness design; the harness must cover all relevant paths.
- **Behavioral change acceptance**: The wrap→saturate semantic change is intentional. Downstream consumers that relied on wrap semantics will now see saturate behavior.
- **moon ci scope**: The fuzz crate (`fuzz/src/lib.rs:753`) is out of lint-src scope per contract.

## Assumptions

1. The fix uses `saturating_add` from the standard library (not `checked_add` with unwrap).
2. No other code path in `vb_core/budget.rs` relies on wrap semantics from `SmallLinearMetrics::add`.
3. The `BoundednessPolicy` hard limits are set correctly to prevent realistic overflow scenarios.
4. Workflows with node counts approaching `u64::MAX` are not realistic in production.
5. The `const fn` qualifier guarantees compile-time evaluation and eliminates runtime nondeterminism.

## Behavior-Affecting Change Notes

The change from `self.steps + other.steps` (wrap) to `self.steps.saturating_add(other.steps)` (saturate) is **behavior-affecting**:

| Scenario | Before (wrap) | After (saturate) |
|----------|---------------|------------------|
| `u64::MAX + 1` | `0` | `u64::MAX` |
| `u64::MAX + u64::MAX` | wraps to ~`u64::MAX*2 % 2^64` | `u64::MAX` |
| `0 + u64::MAX + 1` | wraps to `0` (chain) | `u64::MAX` |

This is the **intended** behavior change: budgets should clamp at maximum instead of silently wrapping, which would cause budget checks to pass when they should fail.

## Contract Alignment

Per `contract.md`:
- Postconditions explicitly require `saturating_add` semantics
- Invariant: "If overflow occurs, result is capped at `u64::MAX` (not wrapped)"
- Rationale: "If a workflow has more steps than `u64::MAX`, the budget is effectively 'infinite' for any bounded policy — capping at `MAX` is semantically accurate"