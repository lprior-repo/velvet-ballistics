# Test Plan: LETHAL-3 — F64 Arithmetic Contradiction

## Summary

- **Bead**: LETHAL-3
- **Behaviors identified**: 4
- **Trophy allocation**: 6 unit / 4 integration / 1 e2e / 2 static
- **Proptest invariants**: 3
- **Fuzz targets**: 2
- **Kani harnesses**: 1

---

## 1. Behavior Inventory

1. **Helper functions reject F64 inputs** — `eval_helper_*` functions in `vb_expr` must return `Err(ExprError::F64NotSupported)` when any argument is `SlotValue::F64`
2. **Codegen arithmetic does NOT use F64 paths** — The `ExprOp::Add/Sub/Mul/Div` emitted code must NOT produce `SlotValue::F64`; F64 inputs must be lossy-cast to i64 or return an error
3. **Constant folding eliminates F64 at compile time** — The typecheck/compile pipeline must reject or transform expressions that would produce F64 arithmetic results before codegen
4. **Comparison operators handle F64 safely** — F64 comparisons in codegen emit correct IEEE754 semantics (not lossy i64 casts)

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| Unit / Calc | 6 | `eval.rs` helpers (9 helpers × F64 rejection), `typecheck/mod.rs` F64 coercion, `codegen/mod.rs` arithmetic emission |
| Integration | 4 | End-to-end compile→codegen→eval pipeline for F64 rejection at each boundary |
| E2E | 1 | Full workflow with F64 constant → compile → codegen → eval rejection |
| Static Analysis | 2 | `clippy::float_arithmetic` lint must fire zero times; `no-F64-in-helpers` custom lint check |

**Deviation rationale**: Unit-heavy because the contradiction lives in the typechecker→codegen→eval boundary — each piece must be verified in isolation before integration.

---

## 3. BDD Scenarios

### Behavior: `eval_helper_*` rejects F64 SlotValue inputs

**Scenario**: `Length` helper receives F64 SlotValue
```
Given: eval_helper(ExprHelper::Length, &[SlotValue::F64(finite_f64)])
When: eval_helper evaluates
Then: Returns Err(ExprError::F64NotSupported { helper: "length", found: "f64" })
And: No type_name inspection is performed on the F64 value
```

**Scenario**: `Empty` helper receives F64 SlotValue
```
Given: eval_helper(ExprHelper::Empty, &[SlotValue::F64(finite_f64)])
When: eval_helper evaluates
Then: Returns Err(ExprError::F64NotSupported { helper: "empty", found: "f64" })
```

**Scenario**: `Unique` helper receives F64 SlotValue
```
Given: eval_helper(ExprHelper::Unique, &[SlotValue::F64(finite_f64)])
When: eval_helper evaluates
Then: Returns Err(ExprError::F64NotSupported { helper: "unique", found: "f64" })
```

**Scenario**: `Contains` helper receives F64 in first position
```
Given: eval_helper(ExprHelper::Contains, &[SlotValue::F64(finite_f64), SlotValue::Symbol(sym)])
When: eval_helper evaluates
Then: Returns Err(ExprError::F64NotSupported { helper: "contains", found: "f64" })
```

**Scenario**: `Contains` helper receives F64 in second position
```
Given: eval_helper(ExprHelper::Contains, &[SlotValue::Symbol(sym), SlotValue::F64(finite_f64)])
When: eval_helper evaluates
Then: Returns Err(ExprError::F64NotSupported { helper: "contains", found: "f64" })
```

**Scenario**: `Sum` helper receives list containing F64
```
Given: A list stored in ValueStore containing at least one F64 element
And: eval_helper_sum_with_store is called on that list
When: Sum evaluates
Then: Returns Err(ExprError::F64NotSupported { helper: "sum", found: "f64" })
```

**Error variant — arity mismatch does NOT shadow F64 rejection**:
```
Given: eval_helper(ExprHelper::Length, &[SlotValue::F64(finite_f64), SlotValue::F64(finite_f64)])
When: eval_helper evaluates
Then: Returns Err(ExprError::F64NotSupported { helper: "length", found: "f64" })
And: NOT Err(ExprError::HelperArityMismatch { ... })
```

---

### Behavior: `eval_helper_with_store` rejects F64 SlotValue inputs

All store-aware helpers (`Length`, `Empty`, `Contains`, `StartsWith`, `EndsWith`, `Has`, `Append`, `AppendIf`, `Merge`, `Sum`, `Count`, `Unique`) must mirror the F64 rejection behavior of their non-store counterparts.

```
Given: eval_helper_with_store(ExprHelper::Length, &[SlotValue::F64(finite_f64)], &mut store)
When: eval_helper_with_store evaluates
Then: Returns Err(ExprError::F64NotSupported { helper: "length", found: "f64" })
```

---

### Behavior: Codegen arithmetic emission does NOT preserve F64 paths

**Scenario**: `ExprOp::Add` codegen emits lossy cast for F64
```
Given: An expression Add(left, right) where left::F64 or right::F64
When: emit_expr_function generates Rust code
Then: The generated match arm for (SlotValue::F64(a), SlotValue::F64(b)) does NOT appear
And: Instead, the arm returns Err(ExprError::F64NotSupported { ... })
OR: The generated code explicitly truncates: SlotValue::I64(a as i64 + b as i64) with a comment explaining lossy semantics
```

**Scenario**: `ExprOp::Div` codegen emits F64 division (preserved semantic)
```
Given: An expression Div(left, right) where left::F64 and right::F64
When: emit_expr_function generates Rust code
Then: The generated match arm for (SlotValue::F64(a), SlotValue::F64(b)) emits SlotValue::F64(a / b)
And: The result is wrapped in FiniteF64::new check
```

**Scenario**: Comparison codegen handles F64 with IEEE754 semantics
```
Given: An expression Lt(left, right) where left::F64 and right::F64
When: emit_expr_function generates Rust code
Then: The generated code produces SlotValue::Bool(a < b) using f64 total ordering
And: NaN ordering follows IEEE754 (NaN < -Inf is false, NaN > Inf is false)
```

---

### Behavior: Typecheck constant folding eliminates F64 from helper arguments

**Scenario**: F64 literal in helper argument is rejected at compile time
```
Given: Expression: length(3.14)
When: parse_expr → typecheck_expr is called
Then: Returns Err(ExprError::F64NotSupported { helper: "length", found: "f64" })
And: The error is caught BEFORE bytecode compilation
```

**Scenario**: F64 constant in arithmetic is rejected at compile time
```
Given: Expression: empty(1.0 + 2.0)
When: parse_expr → typecheck_expr is called
Then: Returns Err(ExprError::F64NotSupported { helper: "empty", found: "f64" })
And: No bytecode is produced for this expression
```

**Scenario**: Mixed F64/I64 arithmetic produces F64 which is then rejected
```
Given: Expression: contains([1.0], 1)
When: parse_expr → typecheck_expr is called
Then: The binary op (1.0, 1) has type F64
And: typecheck_expr returns Err(ExprError::F64NotSupported { helper: "contains", found: "f64" })
```

**Scenario**: F64 constant propagates through nested helper call
```
Given: Expression: length(unique([1.5, 2.5]))
When: parse_expr → typecheck_expr is called
Then: unique([1.5, 2.5]) has type List<F64>
And: length(...) returns Err(ExprError::F64NotSupported { ... })
```

---

## 4. Proptest Invariants

### Proptest: `eval_helper` F64 rejection
- **Invariant**: For all helpers H and all finite f64 values v, `eval_helper(H, &[SlotValue::F64(v)])` returns `Err(ExprError::F64NotSupported)`
- **Strategy**: `any::<FiniteF64>()` → `SlotValue::F64(f)`, helpers from `ExprHelper` enum
- **Anti-invariant**: Any helper receiving `SlotValue::F64` must NOT return `Ok(...)` or `Err(TypeMismatch)`

### Proptest: `typecheck_expr` F64 rejection for helpers
- **Invariant**: For all helper expressions H with F64-typed arguments, `typecheck_expr(H, ctx)` returns `Err(ExprError::F64NotSupported)` and NOT `Ok(ExprType::F64)`
- **Strategy**: `arb_typed_expr(ExprType::F64)` applied to all helper argument positions
- **Anti-invariant**: F64 type must NOT appear in the type inference result for any helper

### Proptest: Codegen F64 arithmetic is lossy-cast
- **Invariant**: For all generated `Add/Sub/Mul` arithmetic on `(SlotValue::F64, SlotValue::F64)`, the generated code produces `SlotValue::I64` (lossy cast) and NOT `SlotValue::F64`
- **Strategy**: Generate all four arithmetic binary ops with F64 constant pool entries
- **Note**: `Div` is the exception — it preserves F64 semantics in codegen per IEEE754

---

## 5. Fuzz Targets

### Fuzz Target: `typecheck_expr` with F64 literals
- **Input**: Arbitrary `ExprAst` trees with `ExprLiteral::F64` values at random positions
- **Risk**: F64 type propagating through type inference into helper arguments without error — logic error
- **Corpus seeds**:
  - `length(3.14)`
  - `empty(1.0 + 2.0)`
  - `contains([1.5], x)`
  - `sum([f64::NAN, f64::INFINITY, f64::MAX])`

### Fuzz Target: `eval_expr_program` with F64 SlotValue inputs
- **Input**: Arbitrary `ExprProgram` with F64 constants loaded via `LoadConst`
- **Risk**: F64 values reaching helper ops without error — panic or wrong result
- **Corpus seeds**:
  - Program that loads F64 constant and calls `Length`
  - Program that loads F64 constant and calls `Empty`
  - Program that loads F64 constant and calls `Contains`

---

## 6. Kani Harnesses

### Kani Harness: `eval_helper_all_helpers_f64_rejection`
- **Property**: For every helper H in `ExprHelper` enum, and for every finite f64 value v, calling `eval_helper(H, &[SlotValue::F64(v)])` returns `Err(ExprError::F64NotSupported)`
- **Bound**: 9 helpers × 3 f64 edge values (0.0, MIN_POSITIVE, MAX) = 27 paths
- **Rationale**: Exhaustive verification needed because F64 rejection is a MUST contract — a single helper that accepts F64 would break the "no F64 evaluation" mandate

### Kani Harness: `typecheck_f64_constant_not_in_helper_result`
- **Property**: For all helper calls H(args) where any arg has inferred type F64, `typecheck_expr(H(args))` returns `Err(ExprError::F64NotSupported)`
- **Bound**: 9 helpers × 3 arg positions × 2 type combinations = 54 paths
- **Rationale**: Type inference is the compile-time gate; if F64 slips through here, it reaches codegen

---

## 7. Mutation Checkpoints

Critical mutations that MUST be caught:

| Mutation | Test That Catches It |
|----------|----------------------|
| `eval_helper_length` removes F64 check → returns Ok | `eval_helper_length_rejects_f64` |
| `eval_helper_empty` removes F64 check → returns Ok | `eval_helper_empty_rejects_f64` |
| `eval_helper_contains` removes F64 check → returns Ok | `eval_helper_contains_rejects_f64` |
| `eval_helper_sum` removes F64 check in loop | `eval_helper_sum_rejects_f64_in_list` |
| `codegen/mod.rs` `ExprOp::Add` emits `SlotValue::F64` directly | `codegen_add_emits_i64_for_f64_inputs` |
| `typecheck/mod.rs` `coerce_numeric` returns `ExprType::F64` for mixed | `typecheck_rejects_f64_in_helper_args` |
| Constant folding passes F64 through without error | `constant_folding_eliminates_f64_before_codegen` |
| `eval_helper_with_store` removes F64 check | `eval_helper_with_store_rejects_f64` |

**Threshold**: ≥90% mutation kill rate

---

## 8. Combinatorial Coverage Matrix

### vb_expr Helper F64 Rejection (unit tests)

| Helper | F64 arg | Null arg | I64 arg | Symbol arg | List arg |
|--------|---------|----------|---------|------------|----------|
| Length | Err(F64NotSupported) | Err(TypeMismatch) | Err(TypeMismatch) | Err(TypeMismatch) | Err(TypeMismatch) |
| Empty | Err(F64NotSupported) | Ok(true) | Err(TypeMismatch) | Err(TypeMismatch) | Err(TypeMismatch) |
| Unique | Err(F64NotSupported) | Err(TypeMismatch) | Err(TypeMismatch) | Err(TypeMismatch) | Err(TypeMismatch) |
| Contains (F64 first) | Err(F64NotSupported) | — | — | — | — |
| Contains (F64 second) | Err(F64NotSupported) | — | — | — | — |
| StartsWith | Err(F64NotSupported) | — | — | Err(TypeMismatch) | — |
| EndsWith | Err(F64NotSupported) | — | — | Err(TypeMismatch) | — |
| Has | Err(F64NotSupported) | — | — | Err(TypeMismatch) | — |
| Append | Err(F64NotSupported) | — | Err(TypeMismatch) | — | Err(TypeMismatch) |
| AppendIf | Err(F64NotSupported) | — | Err(TypeMismatch) | — | Err(TypeMismatch) |
| Merge | Err(F64NotSupported) | — | — | — | — |
| Sum | Err(F64NotSupported) | Err(TypeMismatch) | Ok(i64) | Err(TypeMismatch) | Err(TypeMismatch) |
| Count | Err(F64NotSupported) | Err(TypeMismatch) | Err(TypeMismatch) | Err(TypeMismatch) | Err(TypeMismatch) |
| Exists | Err(F64NotSupported) | Ok(false) | Err(TypeMismatch) | Err(TypeMismatch) | Err(TypeMismatch) |

### Codegen F64 Arithmetic (integration tests)

| Op | F64+F64 | F64+I64 | I64+F64 | I64+I64 |
|----|---------|---------|---------|---------|
| Add | i64 loss-cast (commented) | i64 loss-cast | i64 loss-cast | i64 checked |
| Sub | i64 loss-cast (commented) | i64 loss-cast | i64 loss-cast | i64 checked |
| Mul | i64 loss-cast (commented) | i64 loss-cast | i64 loss-cast | i64 checked |
| Div | F64 preserved (IEEE) | F64 preserved | F64 preserved | i64 checked |

---

## Open Questions

1. **Error variant name**: The task references `Error::F64NotSupported` — does this map to `ExprError::F64NotSupported` in `vb_expr`? Or should it be a new `CodegenError::F64NotSupported` in `vb_codegen`? Both error enums need alignment.

2. **Divergence between eval and codegen**: `eval.rs` uses `FiniteF64::new` to check overflow and returns `NonFiniteFloat`. Codegen emits raw `a / b` with no overflow check. Are these intentionally different? Should codegen also use `FiniteF64`?

3. **Comparison operators in codegen**: Lines 1374-1384 of `codegen/mod.rs` emit raw f64 comparisons with no NaN handling. Should NaN inputs produce a defined error (`Err(F64NotSupported)`) or follow IEEE754 NaN semantics?

4. **Constant folding boundary**: Is the F64 elimination meant to happen in `typecheck_expr` (type inference) or in a separate constant folding pass? Currently `coerce_numeric` in typecheck allows F64 through.

5. **Lossy cast vs error**: The codegen `Add/Sub/Mul` ops currently emit `a as i64 + b as i64` for F64. Should this be changed to return an error? The task says "helpers must NOT use F64 evaluation" — codegen arithmetic is not a helper, so the constraint may not apply there.
