# Proof-to-Implementation Input - vb-fq6u

Maps proof claims to Rust source, test, and harness obligations.

## Source References

| Artifact | Location | Target Function |
|----------|----------|-----------------|
| crates/vb_core/src/budget.rs | Line 315-321 | `SmallLinearMetrics::add` |

### Current Code (needs fix)
```rust
const fn add(self, other: Self) -> Self {
    Self {
        steps: self.steps + other.steps,      // ← clippy: arithmetic_side_effects
        actions: self.actions + other.actions,  // ← clippy: arithmetic_side_effects
        timers: self.timers + other.timers,    // ← clippy: arithmetic_side_effects
    }
}
```

### Required Fix
```rust
const fn add(self, other: Self) -> Self {
    Self {
        steps: self.steps.saturating_add(other.steps),
        actions: self.actions.saturating_add(other.actions),
        timers: self.timers.saturating_add(other.timers),
    }
}
```

## Verification Obligations

### Kani Harness Requirement

**File**: `verification/kani/harnesses/vb_core_budget_overflow.rs` (create)

**Harness**: `small_linear_metrics_overflow`

**Must prove**:
- `u64::MAX.saturating_add(n) == u64::MAX` for any `n > 0`
- `u32::MAX.saturating_add(n) == u32::MAX` for any `n > 0`
- `SmallLinearMetrics::add` never produces `steps == 0` when both inputs have `steps > 0`
- No panic paths in `SmallLinearMetrics::add`

**Harness structure**:
```rust
#[kani::proof]
fn small_linear_metrics_overflow() {
    // Bounded enumeration of overflow-edge cases
    // MAX, MAX-1, MAX-2, 0, 1
    // Verify saturating behavior
}
```

### Verus Spec Requirement

**File**: `crates/vb_core/src/budget.rs` (annotate)

**Spec fn** for `SmallLinearMetrics::add`:
```verus
spec fn add_spec(a: SmallLinearMetrics, b: SmallLinearMetrics) -> SmallLinearMetrics {
    SmallLinearMetrics {
        steps: a.steps.saturating_add(b.steps),
        actions: a.actions.saturating_add(b.actions),
        timers: a.timers.saturating_add(b.timers),
    }
}
```

**Requires/Ensures contract**:
```verus
exec fn add(a: Self, b: Self) -> Self
    ensures result.steps >= a.steps && result.steps >= b.steps
    ensures result.actions >= a.actions && result.actions >= b.actions
    ensures result.timers >= a.timers && result.timers >= b.timers
    ensures result.steps == 0 ==> a.steps == 0 && b.steps == 0
    ensures result.actions == 0 ==> a.actions == 0 && b.actions == 0
    ensures result.timers == 0 ==> a.timers == 0 && b.timers == 0
```

### proptest Requirement

**File**: `crates/vb_core/src/budget.rs` (existing or add tests)

**Properties to test**:
1. `add(a, b).steps >= a.steps` and `add(a, b).steps >= b.steps`
2. `add(a, b).actions >= a.actions` and `add(a, b).actions >= b.actions`
3. `add(a, b).timers >= a.timers` and `add(a, b).timers >= b.timers`
4. `add(a, b)` is deterministic: `add(a, b) == add(a, b)`
5. Edge cases: `MAX + 1` saturates, not wraps

## Implementation Instructions

1. **Fix the source** in `crates/vb_core/src/budget.rs:315-321`:
   - Change `self.steps + other.steps` → `self.steps.saturating_add(other.steps)`
   - Change `self.actions + other.actions` → `self.actions.saturating_add(other.actions)`
   - Change `self.timers + other.timers` → `self.timers.saturating_add(other.timers)`

2. **Add Verus spec/contract** to `SmallLinearMetrics::add`

3. **Create Kani harness** at `verification/kani/harnesses/vb_core_budget_overflow.rs`

4. **Run moon ci** to verify all gates pass:
   ```bash
   moon ci
   ```

## Evidence Commands

| Evidence | Command |
|----------|---------|
| Kani | `cargo kani --harness small_linear_metrics_overflow --no-unwind` |
| Verus | `verus crates/vb_core/src/budget.rs` |
| proptest | `cargo test --package vb_core --lib budget::small_linear --no-fail-fast` |
| clippy | `cargo clippy --package vb_core -- -D clippy::arithmetic_side_effects` |
| fmt | `cargo fmt --check` |
| moon ci | `moon ci` |

## Mapping Status

| Obligation | Status |
|------------|--------|
| PO-001 (Kani) | planned |
| PO-002 (proptest) | planned |
| PO-003 (clippy) | planned |
| PO-004 (fmt) | planned |
| PO-005 (Verus) | planned |