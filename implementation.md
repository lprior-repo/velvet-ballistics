# vb-0sps: Fix dead code in parity.rs

## State
- **Bead**: vb-0sps
- **State**: 10 holzman-rust attempt 4
- **Status**: READY

## Fix Applied

**File**: `crates/vb_codegen/src/codegen/parity.rs:470-488`

**Problem**: Dead `let _ = i;` no-op - index `i` from `enumerate()` was never used.

**Solution**: Removed `enumerate()` and the dead `let _ = i;` assignment. The `ir.iter().zip(gen_run.iter())` already provides parallel iteration without needing an index.

## Diff

```rust
 // BEFORE (dead code highlighted)
 for (i, ((ir_slot, ir_taint), (gen_slot, gen_taint))) in
     ir.iter().zip(gen_run.iter()).enumerate()
 {
     if ir_slot != gen_slot { ... }
     if ir_taint != gen_taint { ... }
     let _ = i;  // <-- dead no-op
 }

 // AFTER
 for ((ir_slot, ir_taint), (gen_slot, gen_taint)) in
     ir.iter().zip(gen_run.iter())
 {
     if ir_slot != gen_slot { ... }
     if ir_taint != gen_taint { ... }
 }
```

## Verification Commands

| Command | Result |
|---------|--------|
| `cargo build` | SUCCESS |
| `cargo nextest run --workspace` | 11101 tests PASSED |

## Power of Ten Compliance

- **Rule 4 (Simple control flow)**: SATISFIED - removed dead code, control flow unchanged
- **Rule 10 (Warnings mandatory)**: SATISFIED - no new warnings introduced
- **zero_forbidden_constructs**: SATISFIED - no unsafe/unwrap/panic/todo in modified code

## Changed Files
- `crates/vb_codegen/src/codegen/parity.rs`

## Residual Risk
None - minimal one-line dead code removal with no behavioral change.
