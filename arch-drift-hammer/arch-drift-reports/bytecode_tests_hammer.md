# Architectural Drift Report: `bytecode/tests.rs`

**File**: `crates/vb_expr/src/bytecode/tests.rs`  
**Total Lines**: 556 (VIOLATION: exceeds 300-line limit)  
**Drift Classification**: CRITICAL — file must be split

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 556 | 300 | ❌ OVER BY 256 LINES |
| Test functions | 34 | — | — |
| Helper functions | 3 | — | — |
| Import block | 15 | — | — |

**Required Action**: File MUST be split into at least 2, ideally 3, smaller modules.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS (Scott Wlaschin DDD)

### 2.1 Raw Index Construction Repeated Everywhere

```rust
// VIOLATION: `ConstIdx::new(0)` scattered across 15+ test sites
ExprOp::LoadConst(ConstIdx::new(0))
ExprOp::LoadConst(ConstIdx::new(1))
SlotIdx::new(0), SlotIdx::new(1)
```

**Problem**: No test fixture abstracts index construction. Every test reconstructs indices manually.

**Fix**: Create `TestSlot` / `TestConst` builder types or a `TestProgram` struct that encapsulates index allocation.

### 2.2 Stringly-Typed Error Matching

```rust
// VIOLATION: Magic string literals in error assertions
Err(crate::ExprError::UnexpectedToken { token: "expected ConstValue::F64".into() })
Err(crate::ExprError::UnexpectedToken { token: "expected ConstValue::F64 from folding".into() })
Err(crate::ExprError::UnexpectedToken { token: "expected SlotValue::F64".into() })
```

**Problem**: `UnexpectedToken` takes a `String` with untyped message content. These are not semantic errors but programmer convenience errors.

### 2.3 Repeated lex/parse/const-fold Pattern (Boilerplate)

```rust
// VIOLATION: Identical 3-line pattern repeated 28+ times
let tokens = lex_expr("...")?;
let ast = parse_expr(&tokens)?;
let folded = const_fold_expr(&ast);
```

**Problem**: No shared `TestExpr` or `ParsedExpr` helper that bundles the lex/parse result.

**Fix**: Add `fn parse_expr_test(source: &str) -> ExprResult<Expr>` in a shared test utility module.

### 2.4 Magic String "$missing" as Test Fixture

```rust
// VIOLATION: Unnamed magic string
compile_expr("$missing + 1", &resolve_test_reference)
assert!(matches!(result, Err(crate::ExprError::InvalidReference { reference }) if reference == "$missing"));
```

**Problem**: `"$missing"` is a magic string not defined as a named constant. The reference resolver returns `None` for this, but the string itself is not extracted to a named value.

### 2.5 Hardcoded Constant Vectors in Every Test

```rust
// VIOLATION: Repeated inline constant vector construction
vec![ConstValue::I64(1), ConstValue::I64(2), ConstValue::I64(3)]
vec![ConstValue::I64(0), ConstValue::I64(1)]
```

**Problem**: No test data builder. Should be `TestConstants::i64s([1, 2, 3])`.

### 2.6 Raw Vec<ExprOp> Comparisons

```rust
// VIOLATION: Comparing raw vectors of domain ops
assert_eq!(program.ops.as_ref(), expected_ops.as_slice());
```

**Problem**: No `assert_ops_equal!` or `ProgramAssertions` struct. The raw `Vec<ExprOp>` is exposed everywhere.

---

## 3. DOMAIN BOUNDARY BLURRING

### 3.1 Mixed Test Concerns in Single File

The file contains tests for **5 distinct bounded contexts**:

1. **Compilation correctness** (ops generation, stack bounds)
2. **Constant folding** (algebraic simplification)
3. **Evaluation/roundtrip** (compile + eval produces expected result)
4. **Reference resolution** (slot lowering)
5. **Error handling** (UnsupportedLiteral, StackUnderflow, InvalidReference)

**Problem**: These are separate behavioral domains. A developer working on constant folding should not need to parse a 556-line file.

### 3.2 No Test Scenario Grouping

Tests are ordered by when they were written (approximately), not by scenario. The `// --- F64 bytecode tests ---` and `// --- BDD bytecode tests ---` section markers are organizational comments but do not create actual module boundaries.

---

## 4. SPECIFIC REFACTORING TARGETS

### 4.1 Extract Test Utilities Module

Create `crates/vb_expr/src/bytecode/tests_common.rs`:

```rust
// Target: ~80 lines extracted
pub fn parse_expr_test(source: &str) -> ExprResult<Expr> { ... }
pub fn compile_test(source: &str) -> ExprResult<ExprProgram> { ... }
pub struct TestConstants(Vec<ConstValue>);
pub struct TestSlots(Vec<SlotIdx>);
```

### 4.2 Split by Bounded Context

```
bytecode/
  tests_compile.rs     (~150 lines) — compilation ops generation
  tests_fold.rs        (~120 lines) — constant folding  
  tests_eval.rs        (~100 lines) — roundtrip eval
  tests_resolve.rs     (~80 lines)  — reference resolution
  tests_error.rs       (~80 lines)  — error cases
```

### 4.3 Index Builder Pattern

```rust
// Instead of: ConstIdx::new(0), ConstIdx::new(1)
struct ConstBuilder { next: usize }
impl ConstBuilder {
    fn next(&mut self) -> ConstIdx { let i = self.next; self.next += 1; ConstIdx::new(i) }
}
```

---

## 5. IMPORTS BLOCK ANALYSIS

```rust
#[allow(unused_imports, dead_code)]
use vb_core::{ConstIdx, ConstValue, ExprOp, SlotIdx};
#[allow(unused_imports)]
use crate::ExprError;
#[allow(unused_imports)]
use crate::bytecode::{ReferenceResolver, check_expr_stack_bound, compile_expr, ...};
use crate::lexer::lex_expr;
use crate::parser::parse_expr;
```

**Issues**:
- `#[allow(unused_imports)]` scattered indicates imports are added as needed and never pruned — sign of organic growth without discipline
- `lex_expr` and `parse_expr` imported separately instead of through a combined `test_helpers` module

---

## 6. ADVERSARIAL MODULE

```rust
mod adversarial;
```

**Note**: This module is present but no tests from it are invoked in this file. Likely an orphaned import or the module is empty/wrong location.

---

## 7. SUMMARY SCORECARD

| Category | Finding | Severity |
|----------|---------|----------|
| Line Count | 556 > 300 | 🔴 CRITICAL |
| Primitive Obsession | Magic indices, strings | 🔴 CRITICAL |
| Test Organization | Single 556-line file | 🔴 CRITICAL |
| DRY Violation | lex/parse repeated 28x | 🟡 HIGH |
| Error Typing | Untyped string errors | 🟡 HIGH |
| Import Hygiene | Unused allow annotations | 🟡 MEDIUM |
| Module Boundary | `mod adversarial` orphan | 🟡 MEDIUM |

---

## 8. MANDATORY REMEDIATION

1. **IMMEDIATE**: Split into ≥2 modules by bounded context (compile, fold, eval)
2. **SHORT TERM**: Extract `tests_common` module for shared parse/compile helpers
3. **SHORT TERM**: Replace `ConstIdx::new(N)` / `SlotIdx::new(N)` with `TestConst::next()` / `TestSlot::next()` builders
4. **MEDIUM TERM**: Replace stringly-typed `UnexpectedToken` errors with domain-typed variants

**Estimated refactor**: 2-3 hours for safe split and fixture extraction.
