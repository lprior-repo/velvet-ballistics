# ARCHITECTURAL DRIFT REPORT: `typecheck/tests.rs`

**File:** `/home/lewis/src/velvet-ballistics/crates/vb_expr/src/typecheck/tests.rs`
**Line Count:** 561 lines (VIOLATION: exceeds 300-line limit by 87%)
**Report Date:** 2026-05-29
**Enforcer:** architectural-drift

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | 561 | 300 | **FAIL** |
| Over Budget | 261 | 0 | **+87%** |

**Verdict:** This file is a structural violation factory. At 561 lines, it has grown beyond the point where any single module should exist. The file must be decomposed.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 Raw String Literals for Type Assertions

The tests assert against string representations instead of typed `ExprType` values:

```rust
// Lines 107-108: VIOLATION
assert_eq!(expected, "number");   // Should be ExprType::Number or equivalent
assert_eq!(found, "text");        // Should be ExprType::Text
```

**Evidence (extracted):**
- Line 107: `"number"` (instead of typed comparison)
- Line 108: `"text"`
- Line 131: `"boolean"`
- Line 132: `"i64"`
- Line 175: `"number"`
- Line 176: `"boolean"`
- Line 271: `"boolean"`
- Line 272: `"f64"`
- Line 286: `"number"`
- Line 287: `"text"`
- Line 301: `"number"`
- Line 371: `"number"`
- Line 372: `"text"`
- Line 386: `"number"`
- Line 387: `"text"`
- Line 403: `"boolean"`
- Line 404: `"i64"`
- Line 418: `"boolean"`
- Line 419: `"i64"`
- Line 508: `"number"`
- Line 509: `"text"`

**Fix Required:** Introduce a `TypeMismatchAssertion` helper or use `crate::typecheck::type_name()` to convert `ExprType` to string for comparison, or better yet, assert on the typed `ExprType` directly.

### 2.2 Raw String Error Message Assertions

Lines 103-105, 127-129, 171-173, etc. use raw string literals in error assertions:

```rust
// Lines 103-105: VIOLATION
return Err(ExprError::UnexpectedToken {
    token: "expected TypeMismatch".into(),
});
```

**Fix Required:** Use an enum variant or constant instead of raw `"expected TypeMismatch"` strings.

### 2.3 `Box::from("$x")` Primitive Obsession

Every variable reference uses raw `Box::from("$x")` instead of a proper value object:

**Evidence (extracted):**
- Line 78: `ctx.add_variable(Box::from("$x"), ExprType::I64);`
- Line 317: `ctx.add_variable(Box::from("$a"), ExprType::I64);`
- Line 324: `ctx.add_variable(Box::from("$x"), ExprType::I64);`
- Line 325: `ctx.add_variable(Box::from("$x"), ExprType::Text);`
- Line 332-334: Multiple `Box::from("$a")`, `Box::from("$b")`, `Box::from("$c")`

**Fix Required:** Introduce a `VarRef(Box<str>)` newtype or use `&'static str` with `ToOwned()` conversion.

### 2.4 Raw String Test Input Literals

Test expressions use raw `&str` instead of typed expression builders:

```rust
// Lines 21-25: VIOLATION
assert_eq!(check("42")?, ExprType::I64);
assert_eq!(check("true")?, ExprType::Bool);
assert_eq!(check("null")?, ExprType::Null);
assert_eq!(check("\"hello\"")?, ExprType::Text);
assert_eq!(check("3.14")?, ExprType::F64);
```

**Fix Required:** Create test DSL with typed expression builders like `expr().literal_i64(42)` or `pexpr().int(42)`.

---

## 3. TEST ORGANIZATION VIOLATIONS

### 3.1 Flat Test Structure

The file uses a flat structure with 40+ tests in a single module. No BDD scenario organization.

**Test Groups Identified:**
| Group | Lines | Count |
|-------|-------|-------|
| Literal inference | 19-27 | 1 |
| Arithmetic/Comparison/Logical inference | 29-61 | 6 |
| Context resolution | 70-84 | 2 |
| BDD typecheck validation | 88-163 | 12 |
| Coercion tests | 209-257 | 10 |
| Unary type error tests | 259-303 | 5 |
| TypeContext tests | 305-344 | 6 |
| Comparison operator tests | 346-389 | 7 |
| Logical operator edge tests | 391-421 | 4 |
| Equality operator tests | 423-447 | 4 |
| Unknown type passthrough | 449-467 | 3 |
| Helper type inference | 469-520 | 10 |
| Nested expression tests | 521-548 | 4 |
| F64/object/list tests | 549-561 | 3 |

**Fix Required:** Split into submodules by test group, each < 300 lines.

### 3.2 Dual-Abstraction Testing

Some tests use the `check()` helper (lex + parse + typecheck pipeline):
```rust
// Line 21: High-level pipeline
assert_eq!(check("42")?, ExprType::I64);
```

Other tests bypass `check()` and call internals directly:
```rust
// Lines 90-93: Direct internal call
let tokens = lex_expr("1 + 2")?;
let ast = parse_expr(&tokens)?;
let ty = typecheck_expr(&ast, &TypeContext::new())?;
```

**Fix Required:** Standardize on `check()` helper for all happy-path tests; only bypass for sad-path error injection tests.

---

## 4. DDD COHESION VIOLATIONS

### 4.1 Missing Test Value Objects

No `TestExpr` or `ExprFixture` type exists to abstract test expression construction.

**Current state:** Every test re-implements the pipeline:
```rust
let tokens = lex_expr(source)?;
let ast = parse_expr(&tokens)?;
typecheck_expr(&ast, &TypeContext::new())?
```

### 4.2 No TypeMismatch Test Builder

Error assertion logic is copy-pasted ~15 times:
```rust
let Err(ExprError::TypeMismatch { expected, found }) = result else {
    return Err(ExprError::UnexpectedToken { token: "expected TypeMismatch".into() });
};
assert_eq!(expected, "number");
assert_eq!(found, "text");
```

**Fix Required:** Create `assert_type_mismatch(result, expected_type, found_type)` helper.

### 4.3 TypeContext Test API is Leaky

`TypeContext` is tested directly, exposing internal `Vec<(Box<str>, ExprType)>` representation. Tests like `typecontext_shadows_earlier_binding_with_later` (lines 322-327) test implementation, not behavior.

---

## 5. REQUIRED REFACTORS

### 5.1 Mandatory Decomposition (Priority: CRITICAL)

| New Module | Target Lines | Content |
|------------|--------------|---------|
| `tests/literal_inference.rs` | ~60 | Lines 19-61 |
| `tests/binary_ops.rs` | ~80 | Lines 88-163 (ops validation subset) |
| `tests/coercion.rs` | ~80 | Lines 209-257 |
| `tests/type_context.rs` | ~80 | Lines 305-344, 315-327 subset |
| `tests/comparison_ops.rs` | ~80 | Lines 346-389 |
| `tests/logical_ops.rs` | ~80 | Lines 391-421 |
| `tests/equality_ops.rs` | ~60 | Lines 423-447 |
| `tests/helper_inference.rs` | ~80 | Lines 469-520 |
| `tests/nested.rs` | ~60 | Lines 521-561 |
| `tests/adversarial.rs` | ~150 | Already exists, needs integration |

### 5.2 Introduce Test Fixtures (Priority: HIGH)

```rust
// Proposed: tests/common.rs
pub fn check(source: &str) -> ExprResult<ExprType> { ... }

pub fn assert_type_mismatch(
    result: ExprResult<ExprType>,
    expected: &str,
    found: &str,
) -> ExprResult<()> { ... }

pub struct TypeContextBuilder { ... }
impl TypeContextBuilder {
    pub fn var(&mut self, name: &'static str, ty: ExprType) -> &mut Self;
    pub fn build(&self) -> TypeContext { ... }
}
```

### 5.3 Fix Primitive Obsession (Priority: HIGH)

1. Replace all `"number"`, `"text"`, `"boolean"` string literals with typed `ExprType` comparisons
2. Replace `Box::from("$x")` with a `VarRef` newtype
3. Replace error message strings with `assert_eq!(result.is_err(), true)` + `if let Some(...)` pattern

---

## 6. SUMMARY SCORECARD

| Rule | Status | Severity |
|------|--------|----------|
| Line count < 300 | **FAIL** | CRITICAL |
| No primitive obsession | **FAIL** | HIGH |
| DDD cohesion | **FAIL** | MEDIUM |
| Test organization | **FAIL** | HIGH |
| BDD structure | **FAIL** | MEDIUM |

**OVERALL VERDICT:** File requires immediate decomposition. Primitive obsession violations must be fixed before any new tests can be added.

---

## 7. RECOMMENDED DECOMPOSITION ORDER

1. Extract `tests/common.rs` with `check()` helper and type mismatch assertion builder
2. Split `tests.rs` into named submodules matching the test groups identified above
3. Introduce `VarRef` newtype and `TypeContextBuilder`
4. Replace all string literal type assertions with typed `ExprType` comparisons
5. Verify each new module is < 300 lines
6. Run full test suite to confirm no regressions
