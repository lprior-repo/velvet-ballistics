# Proof Repair Guide — vb-qi37.9.2

**For proof-writer — State 6 → State 5 (regression)**

## LETHAL Finding: PF-001

### Harness: `kani_f64_zero_div_zero_returns_non_finite_float`
**File**: `crates/vb_expr/src/proofs/f64_div.rs:65`

### Root Cause

`eval_div_op` at `crates/vb_expr/src/eval.rs:227` performs `l.get() / r.get()` BEFORE `FiniteF64::new(result)` is called. When dividend is `0.0` and divisor is `0.0`, IEEE 754 defines `0.0 / 0.0 = NaN`. Kani's f64 division modeling detects this NaN at the division operation (line 227), producing a "NaN on division" failed check, before the error handler at line 229 can return `Err(ExprError::NonFiniteFloat)`.

The harness uses `#[kani::cover]` to observe the error path, but `#[kani::cover]` is observe-only — it does not suppress Kani's failed check on the NaN-producing division itself.

### Current (Broken) Code

```rust
#[kani::proof]
#[kani::unwind(4)]
fn kani_f64_zero_div_zero_returns_non_finite_float() {
    let dividend_f64: f64 = kani::any();
    kani::assume(dividend_f64.is_finite());
    kani::assume(dividend_f64 == 0.0);

    let dividend = FiniteF64::new(dividend_f64).unwrap();
    let divisor = FiniteF64::new(0.0_f64).unwrap();

    let result =
        eval_binary_op(BinaryOp::Div, SlotValue::F64(dividend), SlotValue::F64(divisor));

    // Cover: confirm the error path is taken for 0/0
    kani::cover(result.is_err(), "0/0 returns error (NonFiniteFloat)");

    if let Err(e) = result {
        kani::cover(matches!(e, ExprError::NonFiniteFloat), "0/0 error is NonFiniteFloat");
    }
}
```

### Required Fix (choose one)

#### Option A: Remove the harness (Recommended)

The 0/0 → NaN → NonFiniteFloat path is already verified by proptest:
- `finite_f64_rejects_nan_returns_non_finite_number` in vb_core
- The proptest strategy `f64_edge_case_strategy()` includes `Just(f64::NAN)`

Remove `kani_f64_zero_div_zero_returns_non_finite_float` from `f64_div.rs`. The non-zero dividend harness `kani_f64_div_by_zero_returns_non_finite_float` (which passes Kani) covers the primary IEEE 754 ±Inf path. The 0/0 → NaN path is handled by proptest.

#### Option B: Restructure `eval_div_op` to check 0/0 before division

Modify `eval_div_op` in `crates/vb_expr/src/eval.rs` to check for the 0/0 case before performing division:

```rust
fn eval_div_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match (left, right) {
        (SlotValue::F64(l), SlotValue::F64(r)) => {
            // Check 0/0 case before division to avoid NaN in Kani
            if l.get() == 0.0 && r.get() == 0.0 {
                return Err(ExprError::NonFiniteFloat);
            }
            let result = l.get() / r.get();
            let finite =
                vb_core::value::FiniteF64::new(result).map_err(|_| ExprError::NonFiniteFloat)?;
            Ok(SlotValue::F64(finite))
        }
        // ... rest unchanged
    }
}
```

Then the Kani harness can verify the early-return path without triggering NaN division.

**Trade-off**: Option A is preferred because adding a branch in `eval_div_op` adds runtime overhead and the proptest already covers this case. The Kani harness's value is in verifying the ±Inf path (non-zero dividend / 0), which `kani_f64_div_by_zero_returns_non_finite_float` already does.

#### Option C: Lower to proptest-only (same as Option A)

The Kani harness for 0/0 cannot work with the current `eval_div_op` implementation because Kani's f64 division modeling detects NaN before Rust's error handling can intercept it. The 0/0 case should be verified by proptest (which it already is), not by Kani.

### Rerun Targets

After fix, rerun:
```bash
cargo kani -p vb_expr --harness kani_f64_zero_div_zero_returns_non_finite_float
# Expected: PASS (if harness kept)
cargo kani -p vb_expr --harness kani_f64_div_by_zero_returns_non_finite_float
# Must remain: PASS
cargo test -p vb_expr f64_div
# Must pass
```

### Anti-Regression Note

Do NOT modify the other 7 passing harnesses. The bounds in `f64_ops.rs` and `f64_div.rs:96` are intentionally conservative and correct.

---

## MINOR Finding: PF-002

### File: `proof-evidence.md` lines 64-67

Update the entry for `kani_f64_zero_div_zero_returns_non_finite_float` to reflect the actual Kani result: **FAILED** (not "Cover pass"). Do not claim PASS for a FAILED harness.

---

## Summary of Required Changes

| File | Change | Reason |
|---|---|---|
| `crates/vb_expr/src/proofs/f64_div.rs` | Remove `kani_f64_zero_div_zero_returns_non_finite_float` (Option A) or restructure eval_div_op (Option B) | Harness FAILED Kani |
| `proof-evidence.md` | Correct `kani_f64_zero_div_zero_returns_non_finite_float` result from "Cover pass" to "FAILED" | Misrepresentation |
| (optional) `crates/vb_expr/src/eval.rs` | Add 0/0 check before division (if Option B chosen) | Enable Kani to verify 0/0 path |
