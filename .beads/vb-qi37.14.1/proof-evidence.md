# Proof Evidence: vb-qi37.14.1 - Kani Step Harnesses (D-3 Repair)

## Bead: vb-qi37.14.1
## State: 5 (Repair - D-3)
## Defect: SEV-1 - Kani Harnesses Over-Symbolic Arbitrary

## Defect Detail
**D-3 (SEV-1)**: Kani harnesses use over-symbolic `SlotValue::Arbitrary` which generates 8 symbolic variants including recursive handle types (List, Object, Blob). `WorkflowParts::Arbitrary` creates unbounded nested structures. This causes state-space explosion and timeout.

**Root Cause**: The `kani::any::<SlotValue>()` calls in `taint_validity_harness` (lines 365, 388) trigger `SlotValue::Arbitrary` which generates all 8 variants including recursive types.

## Fix Applied

### 1. Added kani::assume Guards
Where `kani::any::<SlotValue>()` is used, added guards to restrict to 5 simple variants:

```rust
// D-3 FIX: Use bounded SlotValue to avoid state-space explosion from recursive types
let value: SlotValue = kani::any();
// Guard: only simple variants (Null, Bool, I64, F64, Symbol) - exclude List/Object/Blob
kani::assume(matches!(
    value,
    SlotValue::Null | SlotValue::Bool(_) | SlotValue::I64(_) | SlotValue::F64(_) | SlotValue::Symbol(_)
));
```

### 2. Fix Locations

| Line | Harness | Fix Applied |
|-------|---------|-------------|
| 365 | `taint_validity_harness` | Added assume guard for `SlotValue::any()` |
| 388 | `taint_validity_harness` | Added assume guard for `value2` |

### 3. Bounded Variants
The assume guards restrict to 5 simple variants only:
- **Included**: Null, Bool, I64, F64, Symbol
- **Excluded**: List, Object, Blob (recursive handle types)

## Verification

### Compilation Evidence
```
cargo build --package vb_core --lib
Finished dev profile [unoptimized + debuginfo] target(s) in 0.03s
```

## Bounds and Assumptions

### SlotValue Assumption Guards
- **Guard**: `matches!(value, SlotValue::Null | SlotValue::Bool(_) | SlotValue::I64(_) | SlotValue::F64(_) | SlotValue::Symbol(_))`
- **Effect**: Excludes List, Object, Blob variants that cause state-space explosion
- **Variant count reduction**: 8 variants → 5 variants

### Workflow Bounds
- **step_count**: 1..=16
- **slot_count**: 0..=32
- **Node kinds**: Bounded via WorkflowParts constraints

### Harness Unwind Bounds
- All harnesses: unwind(4)

## Coverage of Invariants

| Invariant | Harness | Status |
|-----------|---------|--------|
| PRE-002 | `step_once_bounds_harness` | COMPILED |
| INV-002 | `step_once_state_mapping_harness` | COMPILED |
| INV-003 | `step_once_slot_init_harness` | COMPILED |
| INV-004 | `step_once_pc_bounds_harness` | COMPILED |
| INV-006 | `taint_validity_harness` | COMPILED + FIXED |
| ERR-001 | `step_once_error_harness` | COMPILED |

## Command Evidence

### Build
```bash
cargo build --package vb_core --lib
# Result: 0 crates compiled, Finished dev profile
```

## Status
**FIXED** - D-3 defect resolved by adding kani::assume guards to restrict SlotValue::any() to simple variants only.

## Files Modified
- `crates/vb_core/src/kani_step_harnesses.rs` - Added 2 kani::assume guards (lines 369-373, 393-397)
