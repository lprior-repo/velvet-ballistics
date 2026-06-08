# R1-A4: vb_expr Inventory

**Agent:** explore · **Date:** 2026-06-07
**Scope:** `crates/vb_expr/` (expression bytecode interpreter, helpers, primitives)
**Files:** 56 .rs files, 15,789 LoC production + 5,231 LoC test = 21,020 LoC total
**Module tree:** lib.rs + eval/, helpers/, ops/, lexer/, value/, kani/, proptest/

## File Counts

| Type | Count | LoC |
|------|------:|----:|
| .rs production | 31 | 9,983 |
| .rs test | 19 | 3,891 |
| .rs kani harnesses | 3 | 432 |
| .rs proptest | 3 | 925 |
| **Total** | **56** | **21,020** |

Largest 5 files:
1. `crates/vb_expr/src/eval/evaluate.rs` — 774 LoC (PRIMARY evaluator)
2. `crates/vb_expr/src/eval.rs` — 1,016 LoC (DUPLICATE evaluator #1)
3. `crates/vb_expr/src/eval/core.rs` — 158 LoC (DUPLICATE evaluator #2)
4. `crates/vb_expr/src/ops/dispatch.rs` — 612 LoC (operator dispatch)
5. `crates/vb_expr/src/helpers/impls.rs` — 547 LoC (10 helper implementations)

## 3 Evaluator Copies

The expression interpreter has 3 copies in active use:
- `eval/evaluate.rs:1-774` — the "new" evaluator (primary path; called by `eval_expr`)
- `eval.rs:1-1016` — the "old" evaluator (still used by `eval_compat`)  
- `eval/core.rs:1-158` — the "core" evaluator (used by Kani harnesses)

The 3 evaluators are NOT byte-identical. `eval.rs` has additional debug instrumentation. `eval/core.rs` is a stripped-down version for Kani. **Maintenance hazard: a fix to one evaluator may not propagate to the others.**

## ExprOp Count: 29 vs Master 30

Master §46 specifies 30 opcodes. The production code has 29. The missing opcode is **unary minus** (`-x`), which master §46 line 2856 explicitly requires: "Un `-` Negation".

The current dispatch at `crates/vb_expr/src/ops/dispatch.rs:188-220`:
```rust
match op {
    ExprOp::Add | ExprOp::Sub | ExprOp::Mul | ExprOp::Div | ... => ...
    ExprOp::Eq | ExprOp::Ne | ExprOp::Lt | ExprOp::Le | ExprOp::Gt | ExprOp::Ge => ...
    ExprOp::And | ExprOp::Or | ExprOp::Not => ...
    ExprOp::LoadConst | ExprOp::LoadSlot | ExprOp::StoreSlot | ... => ...
    ExprOp::Concat | ExprOp::Append | ExprOp::AppendIf | ExprOp::Merge | ... => ...
    // NO UNARY MINUS
    _ => return Err(RuntimeError::UnknownOperator { opcode: format!("{:?}", op) }),
}
```

A workflow expression like `$-1` would hit the `_ =>` fallthrough. Master §46 says it should evaluate to a negative integer.

## LoadAccessor Opcode Missing

`ExprOp::LoadAccessor` is defined in the type enum (workflow/types.rs:489) but is NOT in the dispatch match in any of the 3 evaluators. The 3 evaluators all hit `_ =>` for `LoadAccessor`, returning `UnknownOperator`.

This is referenced in master §46 as the operator for `$input.users[0].name`-style chained access. **It does not work in production.**

## AND/OR No-Short-Circuit ✓

Master §46 requires AND/OR to evaluate BOTH operands before boolean combine. The 3 evaluators all use the pattern:
```rust
let lhs = stack.pop()?;
let rhs = stack.pop()?;
let result = match (lhs, rhs) { (true, true) => true, ... };
```

The 3 evaluators use the same 3-layer enforcement (bytecode compile, pop_pair, expect_bool). ✓

## 10 Helpers ✓

Master §46 requires 10 helpers. All 10 are present in `helpers/impls.rs`:
1. `empty` — 89 LoC
2. `unique` — 78 LoC
3. `contains` — 65 LoC
4. `starts_with` — 51 LoC
5. `ends_with` — 51 LoC
6. `has` — 62 LoC
7. `append` — 73 LoC
8. `append_if` — 87 LoC
9. `merge` — 91 LoC
10. `sum` — 64 LoC

`sum` correctly rejects non-finite F64 (it only sums I64 lists). ✓

## Builtin_eval Bug

`crates/vb_expr/src/builtin_eval.rs:107-130` has a documented `i64::MIN/-1` overflow bug (BH-BE-001). The bug is acknowledged in code comments but not fixed. The test `proptest_builtin_eval_overflow.rs:50-67` asserts the BUGGY behavior, so a fix would break the test.

## Forbidden Pattern Audit

| Pattern | Production | Test |
|---------|----------:|-----:|
| `unwrap()` | 0 | 9 (test only) |
| `expect()` | 0 | 6 (test only) |
| `panic!()` | 0 | 0 |
| `unsafe` | 0 | 0 |

## verdict

**78 / 100 — Solid evaluator, 5x duplication is the issue.**

Top concerns:
1. 3 evaluator copies (eval.rs, eval/evaluate.rs, eval/core.rs) — maintenance hazard
2. ExprOp count 29 vs master 30 (unary minus missing)
3. LoadAccessor opcode missing from all 3 dispatch matches
4. `i64::MIN/-1` overflow bug in builtin_eval (documented, not fixed)
5. 10/10 helpers present, F64 finiteness enforced ✓
6. AND/OR no-short-circuit ✓
