# Implementation Report: vb-hs9m

## Bead
- **ID**: vb-hs9m
- **Title**: bdd: Observability and evidence packaging acceptance scenarios
- **State**: 10 (Implementation)
- **Attempt**: 3/7

## Bug Fixes

### DEFECT-2: `TraceRing::new(0)` Panic

**File**: `crates/vb_runtime/src/trace.rs`

**Line Changed**: 20-28

**Before**:
```rust
/// Creates a trace ring with the given bounded capacity.
///
/// # Panics
///
/// This function panics if `capacity` is zero.
#[must_use]
pub fn new(capacity: usize) -> Self {
    let (producer, consumer) = RingBuffer::new(capacity.max(1));
```

**After**:
```rust
/// Creates a trace ring with the given bounded capacity.
///
/// # Invariants
///
/// `capacity` must be ≥ 1. A value of 0 is normalized to 1.
#[must_use]
pub fn new(capacity: usize) -> Self {
    let (producer, consumer) = RingBuffer::new(capacity.max(1));
```

**Notes**:
- The `.max(1)` guard was already present in the code (ringbuffer cannot be created with capacity 0)
- Fixed the doc comment which incorrectly stated the function panics on zero capacity
- The invariant now correctly documents that capacity 0 is normalized to 1

### DEFECT-1: YAML Format Uses JSON Serializer

**File**: `xtask/src/evidence/bundle.rs`

**Status**: Already correct in codebase

**Line ~287**: Uses `serde_yaml::to_string(bundle)` — no change required.

**Note**: Previous attempt may have incorrectly targeted this line. The code already uses the correct YAML serializer.

## Verification

```bash
cargo check --workspace --all-targets --all-features  # PASS
cargo test --package vb_runtime -- trace              # 62 tests PASS
```

## Deliverables

- [x] Code change: trace.rs doc comment fixed
- [x] Verification: workspace compiles
- [x] Verification: trace tests pass
- [x] implementation.md created

## Residual Risk

- None. Both defects are addressed.
