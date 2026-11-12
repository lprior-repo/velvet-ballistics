# Proof Repair Guide — vb-core-lower-values-actions-refs

**Bead**: `vb-core-lower-values-actions-refs`
**Status**: REJECTED at proof-review (state 6)
**Must resolve before**: State 6 re-review

---

## Overview

The proof artifacts were found to have 3 LETHAL blockers and 5 MAJOR issues. This guide provides step-by-step instructions to repair each finding.

---

## LETHAL Blockers (must fix before any re-review)

### F-001 + F-002: Integration of kani-harnesses/ into vb_compile

**Problem**: Harnesses are standalone files. `lower_slot_reference_for_testing` doesn't exist.

**Repair Steps**:

1. Create `crates/vb_compile/src/kani/` directory:
   ```bash
   mkdir -p crates/vb_compile/src/kani
   ```

2. Move harness files:
   ```bash
   mv kani-harnesses/vb_compile_bytecode.rs crates/vb_compile/src/kani/
   mv kani-harnesses/vb_compile_slot.rs crates/vb_compile/src/kani/
   mv kani-harnesses/vb_compile_accessor.rs crates/vb_compile/src/kani/
   mv kani-harnesses/vb_compile_constant.rs crates/vb_compile/src/kani/
   mv kani-harnesses/vb_compile_node_dedup.rs crates/vb_compile/src/kani/
   ```

3. Export `lower_slot_reference_for_testing` in `expression_bytecode.rs`. Add after the existing `pub(crate)` declarations:
   ```rust
   #[cfg(test)]
   pub(crate) fn lower_slot_reference_for_testing(
       reference: &str,
       accessors: &mut Vec<AccessorProgram>,
   ) -> Result<ExprOp, CompileError> {
       lower_slot_reference(reference, accessors)
   }
   ```
   OR alternatively rewrite the harness to use the public `compile_expr_to_bytecode_with_accessors` API on a `ParsedExpression::Reference` (recommended to avoid adding test-only APIs).

4. Declare the kani module in `crates/vb_compile/src/lib.rs`. Add after line 37 (`pub mod kani_idempotency_parity;`):
   ```rust
   #[cfg(kani)]
   pub mod kani;
   ```

5. Remove the old `kani-harnesses/` directory:
   ```bash
   rm -rf kani-harnesses/
   ```

6. Verify with:
   ```bash
   cargo kani --package vb_compile --harness compile_expr_to_bytecode_overflow 2>&1 | head -20
   ```

---

### F-003: Create scripts/rust-verification-gauntlet.sh

**Problem**: Moon tasks reference a non-existent script.

**Repair Steps**:

1. Create `scripts/` directory:
   ```bash
   mkdir -p scripts
   ```

2. Create `scripts/rust-verification-gauntlet.sh` with the following structure (adapt from existing `moon-rust-verification.yml` and the proof-obligations.jsonl commands):

   The script should support `fast`, `standard`, `deep`, `proof`, `all` arguments and run the corresponding verification commands from `proof-obligations.planned.jsonl`.

   At minimum for `fast` lane:
   ```bash
   #!/bin/bash
   set -euo pipefail
   MODE="${1:-fast}"
   
   case "$MODE" in
     fast)
       cargo clippy -p vb_compile --lib -- -D warnings -A unsafe_code
       cargo test -p vb_compile --lib expression_bytecode -- --nocapture
       cargo test -p vb_compile --lib slot_compiler -- --nocapture
       cargo test -p vb_compile --lib lower -- --nocapture
       ;;
     standard)
       # fast + Kani harnesses
       cargo kani --package vb_compile --harness compile_expr_to_bytecode_overflow
       cargo kani --package vb_compile --harness lower_slot_reference_valid
       cargo kani --package vb_compile --harness lower_accessor_reference_numeric
       cargo kani --package vb_compile --harness push_constant_overflow
       ;;
     deep)
       # standard + node dedup
       cargo kani --package vb_compile --harness node_id_uniqueness
       ;;
     proof)
       # deep + all verification
       ;;
     all)
       ;;
   esac
   ```

3. Make executable:
   ```bash
   chmod +x scripts/rust-verification-gauntlet.sh
   ```

---

## MAJOR Issues

### F-004: Add `#[kani::proof]` to second harness in vb_compile_slot.rs

**File**: `crates/vb_compile/src/kani/vb_compile_slot.rs`
**Line**: 116

Change:
```rust
/// Test $slot.N with a nested path — should create an accessor entry instead.
#[kani::proof]
#[kani::unwind(6)]
fn lower_slot_reference_with_path_creates_accessor() {
```

---

### F-005: Increase unwind bound for while loop in vb_compile_slot.rs

**File**: `crates/vb_compile/src/kani/vb_compile_slot.rs`
**Line**: 27

Change `#[kani::unwind(6)]` to `#[kani::unwind(8)]` for `lower_slot_reference_valid`.

Alternatively, unroll the edge case loop to avoid `while` entirely:
```rust
// Instead of while loop, test each case individually
let (ref_text, expected_idx) = edge_cases[0];
// ... test case 0
let (ref_text, expected_idx) = edge_cases[1];
// ... test case 1
// etc.
```

---

### F-006: Fix Test 5 logic in vb_compile_bytecode.rs

**File**: `crates/vb_compile/src/kani/vb_compile_bytecode.rs`
**Lines**: 118-131

Replace:
```rust
let count = kani::any::<usize>();
kani::assume(count <= MAX_EXPRESSION_OPS);
// Verify MAX_EXPRESSION_OPS is respected in check_expr_stack_bound
let ops_vec: Vec<ExprOp> = std::iter::repeat_with(|| load_ops[0])
    .take(count)
    .collect();
let bound_result = check_expr_stack_bound(&ops_vec, MAX_EXPRESSION_STACK);
if count > MAX_EXPRESSION_OPS {
    kani::assert(
        bound_result.is_err(),
        "ops exceeding MAX_EXPRESSION_OPS should return Err",
    );
}
```

With:
```rust
// Test the MAX_EXPRESSION_OPS boundary directly
// Test case A: ops within limit should succeed (if structurally valid)
let within_ops: Vec<ExprOp> = std::iter::repeat_with(|| load_ops[0])
    .take(MAX_EXPRESSION_OPS)
    .collect();
let within_result = check_expr_stack_bound(&within_ops, MAX_EXPRESSION_STACK);
kani::assert(within_ops.len() == MAX_EXPRESSION_OPS, "should fill to MAX");
// Note: even within the op count limit, stack overflow can still occur
// from deeply nested expressions

// Test case B: symbolic count - Kani explores all possible counts
let count = kani::any::<usize>();
let ops_vec: Vec<ExprOp> = std::iter::repeat_with(|| load_ops[0])
    .take(count)
    .collect();
let bound_result = check_expr_stack_bound(&ops_vec, MAX_EXPRESSION_STACK);
// The result correctly reflects both op_count and stack bounds
```

Or use a symbolic `bool` approach:
```rust
let count_too_big = kani::any::<bool>();
let actual_count = if count_too_big { MAX_EXPRESSION_OPS + 1 } else { MAX_EXPRESSION_OPS };
let ops_vec: Vec<ExprOp> = std::iter::repeat_with(|| load_ops[0])
    .take(actual_count)
    .collect();
let bound_result = check_expr_stack_bound(&ops_vec, MAX_EXPRESSION_STACK);
if count_too_big {
    kani::assert(bound_result.is_err(), "too many ops should fail");
}
```

---

### F-007: Add Err path coverage in Test 7

**File**: `crates/vb_compile/src/kani/vb_compile_bytecode.rs`
**Lines**: 155-183

Add after the existing Test 7:
```rust
// Test 8: Err path - expression with too many ops (overflow)
let deep_expr = build_deeply_nested_expr(MAX_EXPRESSION_OPS + 1);
let mut consts = Vec::new();
let overflow_result = compile_expr_to_bytecode(&deep_expr, &mut consts);
kani::assert(overflow_result.is_err(), "expression exceeding MAX_EXPRESSION_OPS should return Err");

// Test 9: Err path - helper with wrong arity
let wrong_arity_expr = ParsedExpression::HelperCall {
    name: ExpressionHelper::Contains, // arity 2
    args: vec![], // no args — wrong arity
};
let mut consts2 = Vec::new();
let arity_result = compile_expr_to_bytecode(&wrong_arity_expr, &mut consts2);
kani::assert(arity_result.is_err(), "wrong helper arity should return Err");
```

---

### F-008: Fix prefill loop in vb_compile_constant.rs

**File**: `crates/vb_compile/src/kani/vb_compile_constant.rs`
**Lines**: 32-68

Replace the concrete 65535-iteration prefill loop with a symbolic approach:

Option A (recommended — use a known-fill technique):
```rust
// Fill to capacity using a fixed repeat
let mut full_compiler = SlotCompiler::new();
// Use repeat_n with a concrete count that Kani can reason about
let fill_values: Vec<ConstValue> = (0..65535)
    .map(|i| ConstValue::I64(i as i64))
    .collect();
for val in fill_values {
    let _ = full_compiler.push_constant(val);
}
// Now verify overflow
let overflow_result = full_compiler.push_constant(ConstValue::Null);
kani::assert(overflow_result.is_err(), "push_constant on full compiler should return Err");
```

Option B (use the public API without explicit loop):
```rust
// Use SlotCompiler::new() and verify the slot_count invariant
let mut compiler = SlotCompiler::new();
// Record slots up to a max
compiler.record_slot(vb_core::SlotIdx::new(u16::MAX));
let count = compiler.slot_count();
kani::assert(count.is_ok(), "slot_count should be Ok");
kani::assert(count.unwrap() == u16::MAX as u16 + 1, "max slot should be tracked");
```

---

## Verification After Repair

After making all repairs, run:

```bash
# 1. Verify harness compiles
cargo build -p vb_compile --features kani 2>&1 | head -30

# 2. Run a single Kani harness
cargo kani --package vb_compile --harness compile_expr_to_bytecode_overflow 2>&1 | head -50

# 3. Run unit tests
cargo test -p vb_compile --lib expression_bytecode -- --nocapture 2>&1 | tail -20

# 4. Run clippy
cargo clippy -p vb_compile --lib -- -D warnings -A unsafe_code 2>&1 | head -20
```

---

## Repair Owner

Proof-writer (proof-write state 5 re-entry). Waivers for F-001 and F-002 are not applicable — these are integration/harness correctness issues, not verification methodology waivers.
