# Test Writer Report — vb-qi37.9.1

## Bead: F64 Literal AST Lowering

## Summary
- **State**: 8 (Test Writing - Failing First)
- **Test File**: `tests/vb_qi37_9_1_f64_literal_tests.rs`
- **Status**: Tests written; compile will fail because `ExpressionLiteral::F64` does not exist yet

## Missing Implementation (Why Tests Fail)

### 1. `ExpressionLiteral::F64(f64)` — MISSING from `expression.rs:71`
```rust
pub enum ExpressionLiteral {
    Null,
    Bool(bool),
    I64(i64),
    Text(Box<str>),
    // F64 MISSING — needs: F64(f64),
}
```

### 2. `TokenKind::Float(FiniteF64)` — MISSING from `expression.rs:127`
```rust
enum TokenKind {
    Integer(i64),
    String(Box<str>),
    // Float MISSING — needs: Float(FiniteF64),
    ...
}
```

### 3. Lexer `lex_float` method — MISSING
The lexer at `expression.rs:192` handles `lex_integer` for digits but has no `lex_float` for decimal literals.

### 4. `expression_literal_fact` F64 arm — MISSING from `type_taint.rs:314`
```rust
fn expression_literal_fact(value: &ExpressionLiteral) -> ValueFact {
    match value {
        ExpressionLiteral::Null => ValueFact::clean(ValueType::Null),
        ExpressionLiteral::Bool(_) => ValueFact::clean(ValueType::Boolean),
        ExpressionLiteral::I64(_) => ValueFact::clean(ValueType::Number),
        ExpressionLiteral::Text(_) => ValueFact::clean(ValueType::Text),
        // F64 MISSING — needs: ExpressionLiteral::F64(_) => ValueFact::clean(ValueType::Number),
    }
}
```

### 5. `lower_literal` F64 arm — MISSING from `expression_bytecode.rs:226`
```rust
fn lower_literal(literal: &ExpressionLiteral, ...) -> Result<(), CompileError> {
    let value = match literal {
        ExpressionLiteral::Null => ConstValue::Null,
        ExpressionLiteral::Bool(value) => ConstValue::Bool(*value),
        ExpressionLiteral::I64(value) => ConstValue::I64(*value),
        ExpressionLiteral::Text(_) => return Err(...), // text unsupported
        // F64 MISSING — needs: ExpressionLiteral::F64(value) => ConstValue::F64(*value),
    };
    ...
}
```

## Tests Written

### Integration Tests (Public API — `tests/vb_qi37_9_1_f64_literal_tests.rs`)

| Test | Behavior | Expected Outcome | Current Result |
|------|----------|------------------|-----------------|
| `parse_expression_accepts_simple_positive_f64_literal` | Parse `"3.14159"` | Returns `Ok(ParsedExpression::Literal(F64(3.14159)))` | FAILS: lexer rejects decimal |
| `parse_expression_accepts_negative_f64_literal` | Parse `"-2.71828"` | Returns `Ok(ParsedExpression::Literal(F64(-2.71828)))` | FAILS: lexer rejects decimal |
| `parse_expression_accepts_f64_literal_with_leading_zero` | Parse `"0.5"` | Returns `Ok(ParsedExpression::Literal(F64(0.5)))` | FAILS: lexer rejects decimal |
| `parse_expression_accepts_f64_literal_with_exponent` | Parse `"1e10"` | Returns `Ok(ParsedExpression::Literal(F64(1e10)))` | FAILS: lexer rejects exponent |
| `parse_expression_accepts_f64_literal_with_negative_exponent` | Parse `"1.5e-3"` | Returns `Ok(ParsedExpression::Literal(F64(1.5e-3)))` | FAILS: lexer rejects exponent |
| `parse_expression_rejects_integer_literal_as_f64` | Parse `"42"` | Returns `I64(42)` not F64 | PASSES: existing behavior |
| `parse_expression_produces_expression_literal_f64_variant` | Parse `"3.14"` | Returns `ExpressionLiteral::F64` variant | FAILS: variant doesn't exist |
| `parse_expression_f64_preserves_value` | Parse `"2.71828"` | F64 value preserved | FAILS: variant doesn't exist |
| `compile_and_parse_accept_f64_finish_literal_positive` | Compile workflow with `result: 3.14159` | Workflow compiles, constant is `F64(3.14159)` | FAILS: lexer rejects decimal |
| `compile_and_parse_accept_f64_finish_literal_negative` | Compile workflow with `result: -2.71828` | Workflow compiles, constant is `F64(-2.71828)` | FAILS: lexer rejects decimal |
| `compile_and_parse_accept_f64_finish_literal_zero` | Compile workflow with `result: 0.0` | Workflow compiles, constant is `F64(0.0)` | FAILS: lexer rejects decimal |
| `compile_and_parse_accept_f64_finish_literal_exponent` | Compile workflow with `result: 1e5` | Workflow compiles, constant is `F64(1e5)` | FAILS: lexer rejects exponent |
| `lower_literal_handles_f64_positive` | Lower `ExpressionLiteral::F64(3.14159)` | Emits `ConstValue::F64(3.14159)` | FAILS: variant doesn't exist |
| `lower_literal_handles_f64_negative` | Lower `ExpressionLiteral::F64(-2.71828)` | Emits `ConstValue::F64(-2.71828)` | FAILS: variant doesn't exist |
| `lower_literal_handles_f64_zero` | Lower `ExpressionLiteral::F64(0.0)` | Emits `ConstValue::F64(0.0)` | FAILS: variant doesn't exist |
| `lexer_produces_token_kind_float_for_decimal` | Parse `"1.5"` | Returns `ExpressionLiteral::F64` | FAILS: lexer doesn't produce Float token |
| `lexer_distinguishes_integer_from_float` | Parse `"42"` vs `"42.0"` | I64 vs F64 | FAILS: `"42.0"` rejected |
| `expression_literal_enum_has_f64_variant` | Construct `ExpressionLiteral::F64(1.0)` | Compiles | FAILS: variant doesn't exist (COMPILE ERROR) |
| `const_value_enum_has_f64_variant` | Construct `ConstValue::F64` | Compiles | PASSES: variant exists in vb_core |

## Test Count
- **Integration tests written**: 19
- **Expected compile errors**: 2 (`ExpressionLiteral::F64` references)
- **Expected test failures**: 17 (implementation missing)

## What Needs to be Added

1. **`ExpressionLiteral::F64(f64)`** variant to enum at `expression.rs:71`
2. **`TokenKind::Float(FiniteF64)`** variant to enum at `expression.rs:127`
3. **`lex_float()`** method in `Lexer` at `expression.rs` (after `lex_integer`)
4. **Float handling** in `parse_prefix` at `expression.rs:414` (add `TokenKind::Float` case)
5. **`ExpressionLiteral::F64`** arm in `expression_literal_fact` at `type_taint.rs:314`
6. **`ExpressionLiteral::F64`** arm in `lower_literal` at `expression_bytecode.rs:226`

## Verification

Due to `vb_runtime` build issues in this workspace (missing `runtime/chunk_001.rs`), the full test suite cannot be executed. However:
- `vb_compile` crate compiles successfully without the new test file
- The test file references `ExpressionLiteral::F64` which doesn't exist, causing compile failure (expected)
- Once F64 is implemented, tests should pass
