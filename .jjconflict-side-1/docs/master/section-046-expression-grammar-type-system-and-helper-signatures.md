---
section: 46
title: "Expression Grammar, Type System, and Helper Signatures"
parent: velvet-ballistics-MASTER.md
---

## 46. Expression Grammar, Type System, and Helper Signatures


### Precedence Table

Highest to lowest. All binary operators are left-associative.

| Binding power (left/right) | Operators |
|---------------------------|-----------|
| 11 / 12 | Unary `not`, unary `-` (prefix) |
| 11 / 12 | `*`, `/` |
| 9 / 10 | `+`, `-` |
| 7 / 8 | `>`, `>=`, `<`, `<=` |
| 5 / 6 | `==`, `!=` |
| 3 / 4 | `and` |
| 1 / 2 | `or` |

Parenthesized groups reset to minimum binding power. Max nesting depth: 64. Max helper args: 8. Max tokens: 256. Max source bytes: 4096. Stack depth: 64 (`ArrayVec<SlotValue, 64>`).

### Type Rules

| Operator | Accepted types | Return type | Error on mismatch |
|----------|---------------|-------------|-------------------|
| `+`, `-`, `*`, `/` | I64, I64 | I64 | `TypeMismatch { expected: "number" }`. Overflow → `IntegerOverflow`. Div-by-zero → `DivisionByZero`. |
| `>`, `>=`, `<`, `<=` | I64, I64 | Bool | `TypeMismatch { expected: "number" }` |
| `==`, `!=` | Any, Any | Bool | Never (accepts any `SlotValue` pair via `PartialEq`) |
| `and`, `or` | Bool, Bool | Bool | `TypeMismatch { expected: "boolean" }`. **No short-circuit** — both operands evaluated before operator applies. |
| `not` | Bool | Bool | `TypeMismatch { expected: "boolean" }` |
| `-` (unary) | I64 | I64 | `TypeMismatch { expected: "number" }`. `i64::MIN` → `IntegerOverflow`. |

### Null Comparison Rules

`Null == Null` → `true`. `Null == <anything_else>` → `false`. Equality uses `SlotValue::PartialEq` which is derived. `I64(0) != F64(FiniteF64(0.0))` — different types are never equal.

### F64 Status

`ExprType::F64`, `SlotValue::F64(FiniteF64)`, `ConstValue::F64`, `ExprLiteral::F64`, expression float lexing/parsing, bytecode constant lowering, and F64/F64 evaluator arithmetic/comparison arms exist. Strict YAML scalar floats remain forbidden by the YAML profile; float values enter authored workflows through expression strings, runtime slot initialization, or action outputs. Remaining gap: the typechecker still accepts broader numeric coercion than the evaluator. Mixed I64/F64 arithmetic and evaluator/typechecker parity remain current-scope expression evidence gaps. Generated F64 arithmetic semantics and codegen lint parity are removed with `vb_codegen`.

### Helper Signatures

| Helper | Arity | Input types | Return | Implementation status |
|--------|-------|-------------|--------|-----------------------|
| `exists` | 1 | Any | Bool | Implemented: `!matches!(value, Null)` |
| `length` | 1 | List or Null | I64 | Implemented store-aware for symbols/lists/objects/null; no-store helper reports context-required for handles. |
| `count` | 1 | List or Null | I64 | Implemented as count/length over store-aware list values. |
| `empty` | 1 | List or Null | Bool | Implemented store-aware for symbol/list/object/null emptiness. |
| `unique` | 1 | List | List | Implemented store-aware list deduplication preserving first occurrence order. |
| `contains` | 2 | List, T | Bool | Implemented in current evaluators as store-aware Symbol substring search; list-membership/spec parity evidence remains open. |
| `starts_with` | 2 | Symbol, Symbol | Bool | Implemented store-aware text helper; generated-mode behavior is removed with `vb_codegen`. |
| `ends_with` | 2 | Symbol, Symbol | Bool | Implemented store-aware text helper; generated-mode behavior is removed with `vb_codegen`. |
| `has` | 2 | Object, Symbol | Bool | Partially converged: `vb_expr` implements object-field lookup, while the core hot evaluator currently uses list membership semantics; helper parity evidence remains open. |
| `append` | 2 | List, T | List | Implemented store-aware list append. |
| `append_if` | 3 | List, T, Bool | List | Implemented store-aware conditional append. |
| `merge` | 2 | Object, Object | Object | Implemented store-aware object merge; typechecker returns `Object`; interpreter/runtime parity evidence remains open. |
| `sum` | 1 | List | I64 | Implemented store-aware I64 list sum with overflow rejection; arity remains 1. |

### Short-Circuit Policy

`and` and `or` do **not** short-circuit. Both operands are popped from the expression stack and evaluated before the boolean operator applies. A type error in the second operand fires even when the first operand determines the result. The bytecode compiler emits both sub-expression bytecodes before the operator bytecode, so no bytecode-level short-circuit is possible either.

---
