# Domain Model Review — vb-qi37.9.2

## Bead
- **ID**: vb-qi37.9.2
- **Title**: expr: Execute F64 bytecode semantics
- **State**: 3 — Contract

## Type Model Analysis

### Core Types

#### `FiniteF64` (vb_core/src/value.rs, lines 59–84)
```rust
pub struct FiniteF64(f64);

impl FiniteF64 {
    pub fn new(value: f64) -> CoreResult<Self> {
        if value.is_finite() { Ok(Self(value)) } else { Err(CoreError::NonFiniteNumber) }
    }
    pub const fn get(self) -> f64 { self.0 }
}
```
**Design rationale**: Custom newtype preferred over `ordered-float::NotNan<f64>` (allows infinity) and `noisy_float::R64` (debug-assert only, silent in release). Zero dependencies. Validates in both debug and release builds.

**Type-level guarantee**: `FiniteF64` is `Eq` (line 61) despite f64 not being Eq — because all finite f64 values have total ordering and the type can never contain NaN.

**Deserialization boundary**: `FiniteF64::deserialize` (lines 96–103) calls `new(value)` — any malformed F64 JSON/input is rejected at the serde boundary. This is "parse, don't validate" (Scott Wlaschin).

#### `SlotValue::F64(FiniteF64)` (value.rs line 116)
Runtime slot containing a finite f64. The `FiniteF64` newtype is the only constructor — there is no way to construct a `SlotValue::F64` with NaN or infinity without going through `FiniteF64::new`.

#### `ConstValue::F64(FiniteF64)` (value.rs)
Compile-time constant F64. Serde deserialization validates finiteness at load time.

---

## NaN/Inf Handling Analysis

### What produces NaN in F64 arithmetic
| Operation | Example | Result |
|-----------|---------|--------|
| 0.0 / 0.0 | `0.0_f64 / 0.0` | NaN |
| Inf - Inf | `f64::INFINITY - f64::INFINITY` | NaN |
| Inf * 0 | `f64::INFINITY * 0.0` | NaN |
| sqrt(-1) | `(-1.0_f64).sqrt()` | NaN |
| Overflow (e.g., `f64::MAX * 2.0`) | `f64::MAX * 2.0` | Inf (not NaN) |

### What produces ±Inf in F64 arithmetic
| Operation | Example | Result |
|-----------|---------|--------|
| Non-zero / 0 | `1.0 / 0.0` | +Inf |
| Non-zero / -0 | `1.0 / -0.0` | -Inf |
| Overflow | `f64::MAX * 2.0` | +Inf |
| `f64::MAX + f64::MAX` | `f64::MAX + f64::MAX` | +Inf |

### Current enforcement in eval.rs
All five F64 ops (`eval_add_op`, `eval_sub_op`, `eval_mul_op`, `eval_div_op`, `eval_neg_op`) use the pattern:
```rust
let result = l.get() [op] r.get();
let finite = vb_core::value::FiniteF64::new(result)?;
```
This means:
- NaN result → `FiniteF64::new(NaN)` fails → `Err(ExprError::NonFiniteFloat)`
- ±Inf result → `FiniteF64::new(±Inf)` fails → `Err(ExprError::NonFiniteFloat)`

**Critical distinction**: F64 division by zero yields `±Inf` (IEEE 754), which is caught by `FiniteF64::new` → `Err(ExprError::NonFiniteFloat)`. This is intentional and different from I64 division by zero which returns `Err(ExprError::DivisionByZero)`.

---

## Type-State Analysis (Scott Wlaschin DDD)

### `SlotValue` enum — sum type with no Option-as-state
- `SlotValue::Null`, `Bool`, `I64`, `F64`, `Symbol`, `List`, `Object` — exhaustive, no nullable variants hiding state
- Each variant carries only its payload — no `Option<Payload>` for lifecycle
- **Verdict**: Complies with `ban_option_as_state_machine` rule

### `ConstValue` enum
- `ConstValue::Null`, `Bool`, `I64`, `F64`, String` — exhaustive

### `ExprError` — exhaustive error taxonomy
All error variants in `ExprError` are explicit, no stringly errors:
- `NonFiniteFloat` — NaN/Inf from F64 ops
- `DivisionByZero` — I64 division by zero
- `IntegerOverflow` — checked arithmetic overflow
- `TypeMismatch { expected, found }` — type error with context
- `StackOverflow { max }`, `StackUnderflow` — stack discipline errors

### `BinaryOp` and `UnaryOp` enums
- `BinaryOp::Add`, `Sub`, `Mul`, `Div`, `Gt`, `Gte`, `Lt`, `Lte`, `Eq`, `NotEq`, `And`, `Or` — exhaustive
- `UnaryOp::Not`, `Neg` — exhaustive

---

## Gaps and Risks

### Gap 1: No F64 constant folding (fold.rs returns None)
The constant folder returns `None` for F64 arithmetic expressions. This means compile-time arithmetic optimization does not apply to F64. This is classified as `missing functionality` in delivery-scope, not in bead scope.

### Gap 2: No F64-specific proptest for eval path
`vb_core/value.rs` has extensive adversarial tests for `FiniteF64` (NaN, Inf, subnormals, signed zero). However, `vb_expr/eval/tests/integration.rs` has no F64-specific eval tests — F64 is only exercised via bytecode roundtrip tests. This is a coverage gap noted in codebase-map.md.

### Gap 3: NaN comparison semantics
`eval_gt_op` etc. use raw `l.get() > r.get()` — per IEEE 754, any comparison with NaN returns `false`. This is correct behavior but not explicitly documented in tests. NaN-aware test cases should be added.

### Gap 4: F64 division by zero error path
F64/0 → ±Inf → `FiniteF64::new(±Inf)` fails → `ExprError::NonFiniteFloat`. This is the **intentional design** but the error message says "non-finite float" rather than something about division by zero. This could be confusing for users who get `NonFiniteFloat` for what they consider "just dividing by zero". However, this is a deliberate design choice distinguishing F64 from I64 behavior.

---

## Verification of Type Model Invariants

| Invariant | Type-level enforcement | Location |
|-----------|----------------------|----------|
| `SlotValue::F64` always finite | `FiniteF64::new` constructor only | eval.rs:180,196,212,228 |
| `ConstValue::F64` always finite | serde deserialize validates | value.rs:100 |
| No invalid F64 bit patterns in `FiniteF64` | `is_finite()` check | value.rs:72 |
| Negation of finite F64 yields finite F64 | `FiniteF64::new(-f.get())` | eval.rs:316 |
| F64 arithmetic result is finite | `FiniteF64::new(result)?` on all ops | eval.rs lines 180,196,212,228 |
