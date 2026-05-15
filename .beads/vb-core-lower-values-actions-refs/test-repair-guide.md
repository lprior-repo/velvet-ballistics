# Test Repair Guide — vb-core-lower-values-actions-refs

**Bead**: `vb-core-lower-values-actions-refs`
**Workspace**: `/tmp/vb-ws/vb-core-lower-values-actions-refs`
**Reviewer**: test-reviewer (state 9)
**Date**: 2026-05-15

---

## BLOCK-1: Kani harnesses not integrated into vb_compile crate

**Severity**: BLOCK_LOCAL  
**Classification**: integration defect (test-writer owns)

### Problem

`cargo kani --package vb_compile` finds only 1 harness (from `kani_idempotency_parity`). The 5 harness files in `crates/vb_compile/src/kani/` are not integrated into the module tree:

- `vb_compile_slot.rs` — 2 `#[kani::proof]` functions
- `vb_compile_bytecode.rs` — 1 `#[kani::proof]` function
- `vb_compile_accessor.rs` — 1 `#[kani::proof]` function
- `vb_compile_constant.rs` — 1 `#[kani::proof]` function
- `vb_compile_node_dedup.rs` — 1 `#[kani::proof]` function

These files exist in the correct directory (`crates/vb_compile/src/kani/`) but are not declared as submodules.

### Evidence

```bash
$ cargo kani --package vb_compile 2>&1 | grep "Total.*of.*harnesses"
Manual Harness Summary:
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

Only `kani_idempotency_parity` is found because only it is declared in `lib.rs:37`:
```rust
#[cfg(kani)]
pub mod kani_idempotency_parity;  // ✅ declared
// vb_compile_slot, vb_compile_bytecode, vb_compile_accessor,
// vb_compile_constant, vb_compile_node_dedup are NOT declared
```

### Required Repair

**Step 1**: Create `crates/vb_compile/src/kani/mod.rs`:

```rust
//! Kani proof harnesses for vb_compile verification obligations.
//!
//! Bead: vb-core-lower-values-actions-refs
//! Workspace: /tmp/vb-ws/vb-core-lower-values-actions-refs

#![forbid(unsafe_code)]

pub mod vb_compile_slot;
pub mod vb_compile_bytecode;
pub mod vb_compile_accessor;
pub mod vb_compile_constant;
pub mod vb_compile_node_dedup;
```

**Step 2**: Add to `crates/vb_compile/src/lib.rs` after the existing `#[cfg(kani)]` module declarations:

```rust
// Kani harnesses for idempotency gate parity verification (State 5 proof-writer).
#[cfg(kani)]
pub mod kani_idempotency_parity;

// Kani proof harnesses for KANI-EXPR-BYTECODE-001, KANI-ACCESSOR-REF-001,
// KANI-SLOT-REF-001, KANI-CONSTANT-POOL-001, INV-007-NODEDUP-001
#[cfg(kani)]
pub mod kani;
```

**Step 3**: Verify integration:

```bash
cargo kani --package vb_compile 2>&1 | grep "Total.*of.*harnesses"
# Expected: 6 successfully verified harnesses
```

### Verification Commands

After repair, running `cargo kani --package vb_compile` should show:

```
Manual Harness Summary:
Complete - 6 successfully verified harnesses, 0 failures, 6 total.
```

Each harness corresponds to:
1. `lower_slot_reference_valid` (KANI-SLOT-REF-001)
2. `lower_slot_reference_with_path_creates_accessor` (KANI-SLOT-REF-001)
3. `compile_expr_to_bytecode_overflow` (KANI-EXPR-BYTECODE-001)
4. `lower_accessor_reference_numeric` (KANI-ACCESSOR-REF-001)
5. `push_constant_overflow` (KANI-CONSTANT-POOL-001)
6. (node_dedup harness name from vb_compile_node_dedup.rs)
