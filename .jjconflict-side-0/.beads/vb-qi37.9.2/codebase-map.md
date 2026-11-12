# Codebase Map — vb-qi37.9.2

## Bead
- **ID**: vb-qi37.9.2
- **Title**: expr: Execute F64 bytecode semantics
- **State**: 2 (Explore and scope)
- **Workspace**: /home/lewis/src/vb-qi37-9-2

## Scope Summary
F64 bytecode compilation and evaluation in `vb_expr`. Covers lexer → parser → bytecode compiler → stack-based evaluator pipeline for floating-point literals and arithmetic.

---

## Primary Crate: vb_expr

**Path**: `crates/vb_expr/`
**Build status**: ✅ Compiles successfully

### Public API (lib.rs exports)

| Symbol | Type | File | Notes |
|--------|------|------|-------|
| `compile_expr` | fn | bytecode/mod.rs | Full compile: lex + parse + bytecode |
| `compile_expr_to_bytecode` | fn | bytecode/mod.rs | AST → ExprProgram |
| `compile_expr_with_pool` | fn | bytecode/mod.rs | AST → ExprProgram with external constant pool |
| `compile_expr_with_resolver` | fn | bytecode/mod.rs | With ReferenceResolver |
| `ReferenceResolver` | trait | bytecode/mod.rs | Slot resolution trait |
| `check_expr_stack_bound` | fn | bytecode/mod.rs | Validates stack depth |
| `eval_expr_program` | fn | eval.rs | Full eval with fresh ValueStore |
| `eval_expr_program_with_store` | fn | eval.rs | Eval with caller-supplied ValueStore |
| `eval_binary_op` | fn | eval.rs | Public binary op dispatcher |
| `eval_unary_op` | fn | eval.rs | Public unary op dispatcher |
| `eval_helper` | fn | eval.rs | Helper dispatch (no store) |
| `eval_helper_with_store` | fn | eval.rs | Helper dispatch (with store) |

### Key Internal Files

| File | Lines | Purpose |
|------|-------|---------|
| `src/eval.rs` | 1037 | Stack-based bytecode evaluator. F64 ops: `eval_add_op`, `eval_sub_op`, `eval_mul_op`, `eval_div_op`, `eval_neg_op` |
| `src/bytecode/mod.rs` | 296 | AST → postfix bytecode compiler. F64 literals via `literal_to_const` → `ConstValue::F64` |
| `src/bytecode/fold.rs` | 86 | Constant folding. **NOTE**: F64 arithmetic folding returns `None` (not implemented) |
| `src/lexer/mod.rs` | — | Lexer producing tokens for parser |
| `src/parser/mod.rs` | — | AST builder |
| `src/typecheck/mod.rs` | 228 | Static type inference |
| `src/lib.rs` | 114 | Error enum (`ExprError`), public re-exports |

### F64 Semantics in eval.rs

F64 arithmetic uses `vb_core::value::FiniteF64` wrapper:

```
SlotValue::F64(FiniteF64)
  └── FiniteF64::new(f64) → CoreResult<FiniteF64>
      └── rejects NaN, +Inf, -Inf
      └── accepts subnormals, signed zero, min/max finite
```

**F64 binary ops** (eval_add_op, eval_sub_op, eval_mul_op, eval_div_op):
- Pattern: `SlotValue::F64(l)` + `SlotValue::F64(r)` → raw f64 op → `FiniteF64::new(result)?`
- On non-finite result → `Err(ExprError::NonFiniteFloat)` (via `From<CoreError>` impl)
- Division by zero: I64 path uses `checked_div`; F64 path relies on f64 IEEE semantics (produces ±Inf, NOT an error)

**F64 comparison ops** (eval_gt_op, eval_gte_op, eval_lt_op, eval_lte_op):
- Raw f64 comparison via `l.get() > r.get()` etc.
- NaN comparisons yield `false` (IEEE semantics)

**F64 unary negation** (eval_neg_op):
- `SlotValue::F64(f)` → `-f.get()` → `FiniteF64::new(result)?`

### Error Mapping (eval.rs → ExprError)

| CoreError variant | ExprError variant | Trigger |
|-------------------|------------------|---------|
| `NonFiniteNumber` | `NonFiniteFloat` | F64 arithmetic overflow to ±Inf; NaN from 0/0 |
| `ExpressionStackOverflow { max }` | `StackOverflow { max }` | Stack exceeds 64 entries |
| `ExpressionStackUnderflow` | `StackUnderflow` | pop on empty stack |
| `ResourceLimitExceeded` | `BytecodeTooLong` | >256 ops |

---

## Supporting Crate: vb_core

**Path**: `crates/vb_core/`
**Build status**: ✅ Compiles successfully

### Key Files

| File | Purpose |
|------|---------|
| `src/expressions.rs` | `ExprProgram`, `ExprOp` enum (LoadSlot, LoadConst, Add, Sub, Mul, Div, etc.) |
| `src/value.rs` | `SlotValue`, `ConstValue`, `FiniteF64` — 1115 lines with exhaustive tests |
| `src/limits.rs` | Constants: `MAX_EXPRESSION_STACK = 64`, `MAX_EXPRESSION_OPS = 256` |
| `src/errors.rs` | `CoreError::NonFiniteNumber` definition |
| `src/value_store.rs` | Arena for List/Object/Symbol handles |

### FiniteF64 (value.rs lines 38–84)

Custom newtype wrapping `f64`. **Invariant**: value must be finite at construction.

```rust
pub struct FiniteF64(f64);

impl FiniteF64 {
    pub fn new(value: f64) -> CoreResult<Self> {
        if value.is_finite() { Ok(Self(value)) } else { Err(CoreError::NonFiniteNumber) }
    }
    pub const fn get(self) -> f64 { self.0 }
}
```

**Accepted**: all finite f64 including subnormals, ±0, ±f64::MIN, ±f64::MAX
**Rejected**: NaN (all bit patterns), +Inf, -Inf

### ExprOp Enum (expressions.rs lines 40–98)

All ops that can appear in compiled bytecode. F64-relevant:
- `LoadConst(ConstIdx)` — pushes `ConstValue::F64(FiniteF64)` onto stack
- `LoadSlot(SlotIdx)` — pushes slot value (can be `SlotValue::F64`)
- `Add`, `Sub`, `Mul`, `Div` — binary arithmetic
- `Not`, `Neg` — unary ops (Neg applies to F64)
- `Gt`, `Gte`, `Lt`, `Lte`, `Eq`, `NotEq` — comparisons (work on F64)
- Helper ops: `Contains`, `StartsWith`, `EndsWith`, `Has`, `Exists`, `Length`, `Empty`, `Append`, `AppendIf`, `Merge`, `Sum`, `Count`, `Unique`

---

## Test Coverage

### vb_expr/bytecode/tests.rs (339 lines)
| Test | Coverage |
|------|----------|
| `compiles_float_literal_to_f64_constant` | Parses `3.14` → ConstValue::F64 |
| `compiles_float_literal_with_leading_zero` | Parses `0.5` |
| `constant_folds_float_literal` | AST folding for `2.5` |
| `f64_literal_roundtrips_through_eval` | compile + eval for `3.14` |
| `f64_arithmetic_roundtrips_through_eval` | compile + eval for `1.5 + 2.5` |

### vb_expr/eval/tests/integration.rs (1272 lines)
**NOTE**: No F64-specific tests found in eval integration suite. F64 eval path is exercised only via bytecode roundtrip tests above.

### vb_core/value.rs (1115 lines, exhaustive adversarial tests)
- `finite_f64_rejects_nan_returns_non_finite_number`
- `finite_f64_rejects_positive_infinity_returns_non_finite_number`
- `finite_f64_rejects_negative_infinity_returns_non_finite_number`
- `finite_f64_accepts_zero`, `finite_f64_accepts_negative_one`
- `finite_f64_accepts_max_finite`, `finite_f64_accepts_min_positive_normal`
- `finite_f64_negative_zero_is_accepted_and_preserves_sign_bit`
- `finite_f64_accepts_smallest_positive_subnormal`, `finite_f64_accepts_largest_subnormal`
- `finite_f64_rejects_signaling_nan`, `finite_f64_rejects_nan_payload_variants`

---

## Risk Tags

| Risk | Category | Evidence |
|------|----------|----------|
| F64 non-finite results (NaN, Inf) during arithmetic | **user-visible behavior** | `FiniteF64::new` construction in eval.rs ops; tests in value.rs |
| F64 comparison with NaN (always false) | **user-visible behavior** | Raw f64 comparisons in eval_gt_op/lte/etc. |
| F64 division by zero → ±Inf (not error) | **user-visible behavior / parser/codec** | eval_div_op F64 path uses raw `/` |
| F64 constant folding not implemented | **missing functionality** | fold.rs returns None for F64 |
| I64 overflow during mixed-type fallback | **persistence** | `checked_add/sub/mul` in eval_i64_values_ |
| Division by zero for I64 → error | **user-visible behavior** | eval_div_values_ checks `right == 0` |
| Stack overflow (>64 entries) | **performance** | ArrayVec capacity check |
| Type confusion (F64 vs I64 in helpers) | **parser/codec** | expect_i64, expect_bool in helper paths |
| Subnormal F64 preservation | **user-visible behavior** | Sign bit preserved per value.rs test |
| Expression too long (>256 ops) | **performance** | MAX_OPS check in bytecode/mod.rs |

---

## Excluded from Scope

- **vb_runtime** — pre-existing build failure (missing `chunk_001.rs`). Classified as `DEFERRED_GLOBAL`. Does NOT block vb-qi37.9.2.
- F64 helper functions (e.g., `sum` on F64 list) — not in bead scope
- Typecheck F64 inference — exists (`ExprType::F64`) but not bead target
- Kani/Verus/Flux proofs — none currently exist for F64 eval path
- proptest harness for F64 eval — not in bead scope

---

## Downstream Owners

| Owner | Artifact |
|-------|----------|
| rust-contract | `contract.md` — F64 semantics, non-finite policy |
| test-planner | `test-plan.md` — F64 eval path tests |
| holzman-rust | `implementation.md` — if any F64 implementation changes needed |
| formal-verifier | `proof-strategy.md` — if proof obligations generated |

---

## DEFERRED_GLOBAL

| Item | Evidence | Owner |
|------|----------|-------|
| vb_runtime build failure: missing `crates/vb_runtime/src/runtime/chunk_001.rs` | baseline-report.md | vb-qi37.9 (parent bead) |

vb_expr (the crate this bead operates on) builds successfully. This deferred item does NOT block vb-qi37.9.2.
