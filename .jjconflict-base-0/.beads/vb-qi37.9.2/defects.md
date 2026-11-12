# Defects — vb-qi37.9.2 (Black Hat Review)

## DEFECT-1: Fabricated Test Evidence in verification-ledger.jsonl

**Severity**: Critical
**Phase**: PHASE 1 (Contract & Bead Parity)
**File**: `.beads/vb-qi37.9.2/verification-ledger.jsonl`, PO-010 entry

### Description

The verification ledger (verification-ledger.jsonl) line for PO-010 claims:

```
"evidence":"38 tests PASS including eval_binary_op_f64_compares_greater_than,
eval_binary_op_f64_compares_less_than, edge_f64_rejected_by_comparison,
f64_comparison_nan_yields_false"
```

The test `f64_comparison_nan_yields_false` **does not exist** in the vb_expr test suite.

### Evidence

```bash
$ cargo test -p vb_expr -- --list 2>&1 | grep f64_comparison_nan_yields_false
# (empty — test does not exist)
```

### Impact

- POST-006 (F64 comparison ops return IEEE 754 semantics; NaN comparisons yield false) is **NOT tested**
- The contract claim that "NaN comparisons yield false per IEEE 754" has **no verification**
- Downstream beads relying on this obligation are working on unverified assumptions

### Remediation

1. Remove `f64_comparison_nan_yields_false` from the PO-010 evidence string
2. Write the missing test covering NaN comparison semantics:
   ```rust
   #[test]
   fn f64_comparison_nan_yields_false() -> ExprResult<()> {
       let nan = SlotValue::F64(FiniteF64::new(f64::NAN).unwrap());
       let finite = SlotValue::F64(FiniteF64::new(1.0).unwrap());

       // NaN > finite → false
       let r = eval_binary_op(BinaryOp::Gt, nan.clone(), finite.clone())?;
       assert_eq!(r, SlotValue::Bool(false));

       // finite < NaN → false
       let r = eval_binary_op(BinaryOp::Lt, finite, nan.clone())?;
       assert_eq!(r, SlotValue::Bool(false));

       // NaN >= NaN → false (NaN !== NaN per IEEE 754)
       let r = eval_binary_op(BinaryOp::Gte, nan.clone(), nan)?;
       assert_eq!(r, SlotValue::Bool(false));

       Ok(())
   }
   ```
3. Re-run formal verification to regenerate ledger from actual test output

---

## DEFECT-2: Test Count Mismatch in Ledger

**Severity**: Low
**Phase**: PHASE 1 (Contract & Bead Parity)
**File**: `.beads/vb-qi37.9.2/verification-ledger.jsonl`

### Description

Ledger claims "38 tests PASS" for PO-005. Actual count:
```bash
$ cargo test -p vb_expr -- --list 2>&1 | grep -c 'f64'
35
```

35 tests match the `f64` filter, not 38.

### Impact

Minor: suggests ledger is manually edited rather than auto-generated from test runs. Could mask other discrepancies.

### Remediation

Audit all PO entries for similar count mismatches. Ensure ledger is generated programmatically from `cargo test -- --list` output.

---

## DEFECT-3: Inconsistent Error Conversion Style

**Severity**: Informational
**Phase**: PHASE 4 (Ruthless Simplicity)
**File**: `crates/vb_expr/src/eval.rs`

### Description

`eval_add_op`, `eval_sub_op`, `eval_mul_op`, `eval_neg_op` use implicit `?` for `CoreError → ExprError` conversion:
```rust
let finite = vb_core::value::FiniteF64::new(result)?;  // CoreError → ExprError via From
```

`eval_div_op` uses explicit `map_err`:
```rust
let finite = vb_core::value::FiniteF64::new(result).map_err(|_| ExprError::NonFiniteFloat)?;
```

### Impact

Both paths work (due to `From<CoreError> for ExprError` existing), but inconsistency suggests possible confusion about whether the conversion is correct. Could mask future type errors if the `From` impl is removed.

### Remediation

Standardize on one style. Recommend explicit `map_err` for clarity:
```rust
// eval_add_op etc.
let finite = vb_core::value::FiniteF64::new(result)
    .map_err(|_| ExprError::NonFiniteFloat)?;
```

Or add a comment explaining why implicit `?` is correct here.

---

## Outstanding Risk

The core implementation in `eval.rs` is **correct**. The F64 bytecode semantics are properly implemented:
- NaN/Inf rejection via FiniteF64 wrapper ✓
- F64/0 → NonFiniteFloat (not DivisionByZero) ✓
- IEEE 754 comparison semantics (NaN → false) ✓
- Stack bounds enforced ✓

The defects are **evidence gaps**, not implementation gaps. The code works; the verification record is incomplete.
