# BLACK-HAT REVIEW: Section 38 Property Tests

**Reviewer**: black-hat-reviewer
**Date**: 2026-05-17
**Files Under Review**:
- `crates/vb_expr/src/property_tests/constant_folding.rs`
- `crates/vb_expr/src/property_tests/arithmetic_overflow.rs`
- `crates/vb_expr/src/property_tests/bound_enforcement.rs`
- `velvet-ballistics-MASTER.md` Section 38

---

## VERDICT: **REJECTED**

### Critical Gaps

---

## PHASE 1: Contract & Bead Parity

### ❌ FAIL — 9 of 11 Section 38 properties have ZERO test coverage

Section 38 mandates 11 properties. Only 3 are covered by the submitted files:

| # | Property | Test Coverage | Status |
|---|----------|---------------|--------|
| 1 | Constant folding | `constant_folding.rs` | ✓ Partial |
| 2 | Bytecode/AST parity | **NONE** | ✗ MISSING |
| 3 | Digest stability | **NONE** | ✗ MISSING |
| 4 | Layout stability | **NONE** | ✗ MISSING |
| 5 | Replay determinism | **NONE** | ✗ MISSING |
| 6 | Snapshot equivalence | **NONE** | ✗ MISSING |
| 7 | Ordering invariants | **NONE** | ✗ MISSING |
| 8 | Bound enforcement | `arithmetic_overflow.rs` + `bound_enforcement.rs` | ✗ Partial |
| 9 | State machine | **NONE** | ✗ MISSING |
| 10 | Taint safety | **NONE** | ✗ MISSING |
| 11 | IR/generated parity | **NONE** | ✗ MISSING |

**74% of required properties are unimplemented.**

### ❌ FAIL — CF coverage is incomplete (10 of 18 test cases missing)

`constant_folding.rs` claims coverage of CF-1..CF-18 per test-plan §1.1 but provides only:

- CF-1, CF-2, CF-4 (bool, i64, null literals) ✓
- CF-5, CF-6 (`not true`, `not false`) ✓
- CF-7 (negation) ✗ **MISSING** — No test for `fold_unary(Neg, I64(n))`
- CF-8..CF-11 (i64 add/sub/mul/div with overflow) ✗ **MISSING**
- CF-12..CF-15 (comparison, `and`, `or`) ✗ **MISSING**
- CF-16..CF-18 (non-constant expressions, mixed-type) ✗ **MISSING**

**Lines**: `constant_folding.rs:1-127` — The `cf_not_true_folds_to_false` test does NOT test CF-7 (negation of i64 literal). CF-7 requires `fold_unary(Neg, ExprAst::Literal(ExprLiteral::I64(n)))`.

### ❌ FAIL — AO coverage is incomplete (AO-7..AO-13 missing)

`arithmetic_overflow.rs` covers AO-1..AO-6 only. Per test-plan §1.8:

- AO-7 (f64 addition with result > f64::MAX) ✗ MISSING
- AO-8 (f64 subtraction with result < f64::MIN) ✗ MISSING
- AO-9 (f64 multiplication with result > f64::MAX) ✗ MISSING
- AO-10 (f64 `-0.0` negation) ✓ Covered at line 98-101
- AO-11 (`eval_helper_length` overflow) ✗ MISSING
- AO-12 (`eval_helper_count` overflow) ✗ MISSING
- AO-13 (`eval_helper_sum` accumulation overflow) ✗ MISSING

### ❌ FAIL — BE coverage is incomplete (BE-7..BE-11 missing)

`bound_enforcement.rs` covers only BE-1..BE-6. Per test-plan §1.5:

- BE-7 (`eval_neg_op` with f64 NaN) ✗ MISSING
- BE-8 (`eval_i64_values_` wrapper) ✗ MISSING
- BE-9 (`eval_helper_sum` checked_add) ✗ MISSING
- BE-10 (stack index OOB → `StackUnderflow`) ✗ MISSING
- BE-11 (program index overflow → `UnexpectedEof`) ✗ MISSING

---

## PHASE 2: Farley Engineering Rigor

### ⚠️ WARNING — `unwrap()` in proptest context

`arithmetic_overflow.rs:99-100`:
```rust
let result = eval_unary_op(UnaryOp::Neg, SlotValue::F64(FiniteF64::new(-0.0).unwrap()));
prop_assert_eq!(result, Ok(SlotValue::F64(FiniteF64::new(0.0).unwrap())));
```

`FiniteF64::new()` returns `Result`, and `.unwrap()` is used in a proptest. If `FiniteF64::new(-0.0)` ever returned `Err` (which it shouldn't for `-0.0` but signals fragile test design), the test would panic. The assertion should be written to handle the `Result` properly or use `unwrap_unchecked` with a comment justifying why it's safe.

### ⚠️ WARNING — Duplication between `arithmetic_overflow.rs` and `bound_enforcement.rs`

Both files test identical behaviors:
- `arithmetic_overflow.rs:18-23` and `bound_enforcement.rs:18-23` — both test `eval_add_op` with `i64::MAX + 1`
- `arithmetic_overflow.rs:36-39` and `bound_enforcement.rs:26-31` — both test `eval_sub_op` with `i64::MIN - 1`
- `arithmetic_overflow.rs:47-52` and `bound_enforcement.rs:33-39` — both test `eval_mul_op` with overflow

This is wasted test effort. The two files should be consolidated, with `bound_enforcement.rs` focusing on genuine bound enforcement (stack limits, program length limits) and `arithmetic_overflow.rs` focusing on arithmetic error paths.

### ✓ PASS — Function length

All test functions are under 25 lines. No violations.

### ✓ PASS — Parameter count

All functions use 5 or fewer parameters. No violations.

---

## PHASE 3: Holzman Rust (The Big 6)

### ℹ️ INFO — Test code is not first-party runtime

The black-hat reviewer notes that property test files are test code, not production runtime code. However, the "no `unwrap()`" rule should still be followed in tests because:
1. Tests that panic hide real bugs
2. `FiniteF64::new()` returning `Err` for `-0.0` would indicate a bug in `FiniteF64`

### ℹ️ INFO — Type safety in tests

The tests correctly use typed `SlotValue` and `ConstValue` rather than raw primitives. No `String` or untyped values leak into test assertions.

---

## PHASE 4: Ruthless Simplicity & DDD (Scott Wlaschin)

### ℹ️ INFO — No `unwrap()` in production code under review

The 3 files under review are test files. The production functions being tested (`eval_binary_op`, `eval_unary_op`, `const_fold_expr`) do not use `unwrap()` or `panic!`.

### ⚠️ WARNING — Test organization doesn't match domain

The test file organization is confusing:
- `constant_folding.rs` tests `const_fold_expr` (correct)
- `arithmetic_overflow.rs` tests `eval_*_op` functions (misnamed — these are arithmetic operations, not overflow boundary tests)
- `bound_enforcement.rs` tests `eval_*_op` functions (completely misnamed — these test arithmetic overflow, not resource bounds)

**Per test-plan §1.5, `bound_enforcement` should test:**
- Stack index OOB → `StackUnderflow`
- Program index overflow → `UnexpectedEof`
- Retry attempts exceeding limit
- Collect exceeding page/item/time limits

**What `bound_enforcement.rs` actually tests:**
- i64 arithmetic overflow (duplicate of `arithmetic_overflow.rs`)

---

## PHASE 5: The Bitter Truth (Velocity & Legibility)

### ❌ FAIL — Misleading file names constitute YAGNI violation

`bound_enforcement.rs` does NOT test bound enforcement. Its name implies it tests resource limits (retry attempts, collect limits, stack bounds), but it only tests arithmetic overflow — identical to `arithmetic_overflow.rs`.

This is a **junior developer** mistake: creating files with names that describe what the author wishes the code did, not what it actually does.

### ❌ FAIL — 74% of required coverage is missing

Section 38 explicitly requires 11 property invariants. The submitted code delivers partial coverage of 3 properties (with gaps within each). 8 properties have zero coverage.

This is not a review finding — this is a **rejection criterion** per Phase 1 rules: "If code fails here, REJECT immediately without proceeding to aesthetics."

---

## MANDATORY FIXES

### Fix 1: Implement all 11 Section 38 properties

Create test files for the 8 missing properties:
- `crates/vb_expr/src/property_tests/bytecode_ast_parity.rs` (BP)
- `crates/vb_storage/src/property_tests/digest_stability.rs` (DS)
- `crates/vb_ui_model/src/property_tests/layout_stability.rs` (LS)
- `crates/vb_runtime/src/property_tests/replay_determinism.rs`
- `crates/vb_runtime/src/property_tests/snapshot_equivalence.rs`
- `crates/vb_runtime/src/property_tests/for_each_ordering.rs` (FE)
- `crates/vb_runtime/src/property_tests/resource_budget.rs` (RB)
- `crates/vb_validate/src/property_tests/taint_propagation.rs` (TP)
- `crates/vb_ipc/src/property_tests/concurrency_safety.rs` (CS)
- `crates/vb_runtime/src/property_tests/error_recovery.rs` (ER)
- `crates/vb_compile/src/property_tests/ir_generated_parity.rs`

### Fix 2: Complete CF coverage

Add tests for CF-7 through CF-18 in `constant_folding.rs`:
- `fold_unary(Neg, I64(n))` for arbitrary i64
- `fold_binary(Add, I64, I64)` for arbitrary pairs (including overflow)
- `fold_binary(Sub, Mul, Div)` similar coverage
- Comparison operators (Eq, NotEq, Lt, Lte, Gt, Gte)
- Boolean operators (And, Or)
- Non-constant expressions returning None

### Fix 3: Complete AO coverage

Add tests for AO-7 through AO-13 in `arithmetic_overflow.rs`:
- f64 addition/subtraction/multiplication overflow (AO-7, AO-8, AO-9)
- `eval_helper_length` error path (AO-11)
- `eval_helper_count` error path (AO-12)
- `eval_helper_sum` accumulation overflow (AO-13)

### Fix 4: Rename `bound_enforcement.rs`

This file tests arithmetic overflow, not bounds. Either:
1. Rename to `arithmetic_overflow.rs` and consolidate with existing `arithmetic_overflow.rs`, OR
2. Rewrite to actually test bound enforcement (stack limits, program length, retry limits)

### Fix 5: Remove `unwrap()` in test code

Replace `FiniteF64::new(-0.0).unwrap()` with proper `Result` handling or use `proptest::prop_assert!` with a custom assertion that explains why `-0.0` must succeed.

---

## CONCLUSION

**REJECTED.** This implementation covers only 3 of 11 required Section 38 properties, and each of those 3 is only partially covered. The remaining 8 properties (74%) have zero test files. Additionally, `bound_enforcement.rs` is misnamed — it tests arithmetic overflow, not resource bounds.

The bead tracking Section 38 property tests is not complete. The implementation does not satisfy the contract specified in `velvet-ballistics-MASTER.md`.

**Next Steps**: Implement all 11 properties per test-plan-property-tests.md before resubmission.