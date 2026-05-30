# ARCHITECTURAL DRIFT REPORT
## Target: `crates/vb_expr/src/parser/tests.rs`
## Severity: CRITICAL — 781 lines (260% of 300-line limit)

---

## 1. LINE COUNT VIOLATION

| Metric | Value |
|--------|-------|
| Actual lines | 781 |
| Limit | 300 |
| Violation | +481 lines (260%) |
| Status | **MANDATORY REFACTOR** |

This file is a monolithic test archive. It MUST be decomposed into multiple smaller test modules.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 Raw String Boxing for Variable References

**Location**: Lines 179-180, 187, 394, 435
```rust
ExprAst::Reference(Box::from("$x"))
ExprAst::Reference(Box::from("$data.field"))
```

**Problem**: `$x` and `$data.field` are raw `&str` literals wrapped in `Box`. A proper `VariableReference` domain type should exist that:
- Validates the `$` prefix
- Parses the field accessor chain
- Provides `Display`/`FromStr` implementations

**Fix**: Introduce `struct VariableReference(String)` or `enum Reference { Variable(&str), FieldAccess(&str, Vec<&str>) }` and have tests use that type.

---

### 2.2 Raw String Boxing for Text Literals

**Location**: Lines 221-224, 230-232
```rust
ExprAst::Literal(ExprLiteral::Text(Box::from("hello world")))
ExprAst::Literal(ExprLiteral::Text(Box::from("")))
```

**Problem**: Text literals use raw `Box<str>` instead of a dedicated `TextLiteral` wrapper that could enforce invariants (e.g., max length, no unescaped control characters).

**Fix**: `struct TextLiteral(String)` with constructor that validates.

---

### 2.3 Hardcoded String Comparisons for Error Tokens

**Location**: Lines 616, 700-702, 746-748
```rust
token.contains("unknown identifier")
token.contains("End")
token.contains("right parenthesis")
```

**Problem**: Error message assertions use fragile string matching on error tokens. This is a classic primitive obsession smell — errors should carry structured payload, not raw strings.

**Fix**: `ExprError` variants should carry structured fields (e.g., `UnexpectedToken { expected: Vec<TokenKind>, actual: TokenKind }`) not raw strings.

---

### 2.4 Raw Usize for Parse Depth

**Location**: Lines 73-74
```rust
usize::from(crate::parser::MAX_DEPTH).saturating_add(2)
```

**Problem**: Depth calculation uses raw `usize` instead of a `ParseDepth` newtype that tracks the limit semantically.

**Fix**: `struct ParseDepth(usize)` with `ParseDepth::max() -> Self` and `ParseDepth::exceeded(&self, actual: usize) -> bool`.

---

### 2.5 Tuple Extraction Instead of Domain Types

**Location**: Lines 754-781
```rust
fn as_binary(expr: &ExprAst) -> crate::ExprResult<(BinaryOp, &ExprAst, &ExprAst)>
fn as_unary(expr: &ExprAst) -> crate::ExprResult<(UnaryOp, &ExprAst)>
fn as_helper(expr: &ExprAst) -> crate::ExprResult<(ExprHelper, &[ExprAst])>
```

**Problem**: Helper functions return raw tuples instead of domain-specific extraction types:
- `BinaryExpr<'a>` instead of `(BinaryOp, &'a ExprAst, &'a ExprAst)`
- `UnaryExpr<'a>` instead of `(UnaryOp, &'a ExprAst)`
- `HelperCall<'a>` instead of `(ExprHelper, &'a [ExprAst])`

**Fix**: Define domain extraction types:
```rust
struct BinaryExpr<'a>(&'a ExprAst);
struct UnaryExpr<'a>(&'a ExprAst);
struct HelperCall<'a>(&'a ExprHelper, &'a [ExprAst]);
```

---

## 3. SINGLE RESPONSIBILITY PRINCIPLE VIOLATIONS

### 3.1 Test File Is Doing Too Much

The file contains tests for:
1. **Operator precedence parsing** (add, sub, mul, div, comparisons)
2. **Unary operator parsing** (neg, not)
3. **Literal parsing** (null, bool, i64, f64, text)
4. **Helper/function call parsing** (exists, length, contains, etc. — 12+ helpers)
5. **Error case handling** (8+ distinct error scenarios)
6. **Precedence chain verification** (full tower tests)
7. **Parenthesized expression parsing**
8. **Variable reference parsing**
9. **F64 special value parsing** (using `vb_core::FiniteF64`)

**Fix**: Split into module files:
- `parser/tests/literals.rs` — null, bool, i64, f64, text tests
- `parser/tests/operators.rs` — binary/unary operator tests
- `parser/tests/helpers.rs` — helper call arity and parsing tests
- `parser/tests/precedence.rs` — precedence chain tests
- `parser/tests/errors.rs` — error case tests

---

## 4. TEST DUPLICATION ANALYSIS

### 4.1 Redundant Helper Arity Tests

Lines 455-507 test arity for 9 different helpers (`exists`, `length`, `empty`, `sum`, `count`, `unique`, `merge`, `append`, `append_if`, `has`, `starts_with`, `ends_with`) with nearly identical structure:

```rust
fn parse_expr_helper_<NAME>_arity_<N>() {
    let expr = parse("<NAME>($x)")?;
    let (name, args) = as_helper(&expr)?;
    assert_eq!(name, ExprHelper::<NAME>);
    assert_eq!(args.len(), N);
    Ok(())
}
```

**Fix**: Parameterize via `proptest!` or a shared test template function.

---

### 4.2 Repeated Binary Operator Assertions

Many tests repeatedly assert `BinaryOp::Add`, `BinaryOp::Mul`, etc. against `ExprAst::Literal(ExprLiteral::I64(...))`.

**Fix**: Introduce test helper:
```rust
fn assert_binary_op(expr: &ExprAst, op: BinaryOp, left_val: i64, right_val: i64)
```

---

## 5. RECOMMENDED REFACTORING PLAN

### Phase 1: Extract Modules
1. Create `parser/tests/` directory
2. Move literal tests → `parser/tests/literals.rs`
3. Move operator tests → `parser/tests/operators.rs`
4. Move helper tests → `parser/tests/helpers.rs`
5. Move precedence tests → `parser/tests/precedence.rs`
6. Move error tests → `parser/tests/errors.rs`
7. Keep shared helpers (`as_binary`, `as_unary`, `as_helper`) in `parser/tests.rs` or move to `parser/tests/common.rs`

### Phase 2: Introduce Domain Types
1. Add `VariableReference` newtype
2. Add `TextLiteral` newtype  
3. Add `ParseDepth` newtype
4. Add extraction types `BinaryExpr`, `UnaryExpr`, `HelperCall`

### Phase 3: Reduce Duplication
1. Parameterize helper arity tests via macro or proptest
2. Add assertion helper functions for common patterns

### Phase 4: Fix Error Primitive Obsession
1. Replace string-based error assertions with structured error type matching

---

## 6. SCOTT WLASCHIN DDD ASSESSMENT

| Principle | Status | Finding |
|-----------|--------|---------|
| Make Illegal States Unrepresentable | ❌ FAIL | Raw `Box<str>` used where `VariableReference` type should enforce `$` prefix validation |
| Primitive Obsession | ❌ FAIL | 5 distinct violations identified (strings, usize, tuples) |
| Single Responsibility | ❌ FAIL | File tests 9+ distinct parsing concerns |
| Small Modules | ❌ FAIL | 781-line monolith |
| Domain Types Over Primitives | ❌ FAIL | Error tokens use `String` instead of structured `TokenKind` enum |

---

## 7. SUMMARY

**ARCH-DRIFT-SEVERITY: CRITICAL**

This file violates 5 major architectural principles:
1. **260% line count over limit** — mandatory decomposition
2. **5 primitive obsession violations** — domain types needed
3. **Single responsibility violation** — 9 concerns in 1 file
4. **Test duplication** — arity tests are copy-paste repeated
5. **Fragile error assertions** — string matching instead of structured types

**IMMEDIATE ACTION REQUIRED**: This file cannot be allowed to grow further. It MUST be refactored before any new parser test cases are added.
