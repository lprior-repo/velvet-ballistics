# Architectural Drift Report: `vb_compile/src/expression.rs`

**File**: `crates/vb_compile/src/expression.rs`
**Analysis Date**: 2026-05-29
**Analyzer**: architectural-drift skill

---

## 1. Line Count Check

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | 881 | 300 | **FAIL** |

**Severity**: CRITICAL — file exceeds line limit by **581 lines** (193% over).

---

## 2. DDD Cohesion Analysis

### Module Responsibility
This file implements a **cold expression lexer/parser** for the compiler AST boundary. It contains:

| Section | Lines | Responsibility | Cohesion |
|---------|-------|----------------|----------|
| Public AST Types | 1–131 | Domain model (ParsedExpression, ExpressionLiteral, ExpressionHelper, operators) | **HIGH** |
| Lexer Implementation | 134–389 | Tokenization infrastructure | LOW (mixed) |
| Parser Implementation | 391–631 | Expression parsing infrastructure | LOW (mixed) |
| Inline Tests | 653–881 | Test suite | **NONE** (should be separate) |

### DDD Smell Detected: **Low Cohesion / God Module**

The file violates the Single Responsibility Principle by combining:
1. **Domain types** (pure AST representation — lines 7–126)
2. **Lexer** (infrastructure — lines 134–389)
3. **Parser** (infrastructure — lines 391–631)
4. **Tests** (verification — lines 653–881)

According to Scott Wlaschin DDD principles, lexer and parser are **application services**, not domain. The domain AST types should live separately.

---

## 3. Violations

### ❌ VIOLATION 1: File Size (CRITICAL)
- **Rule**: All `.rs` files must be ≤ 300 lines
- **Actual**: 881 lines
- **Required Action**: Mandatory split into 3+ files

### ❌ VIOLATION 2: DDD Cohesion — Mixed Concerns
- **Rule**: One概念 per module; domain types separated from infrastructure
- **Current**: `expression.rs` contains both domain (AST) and infrastructure (lexer/parser)
- **Required Action**: Split into domain, lexer, and parser modules

### ❌ VIOLATION 3: Inline Tests
- **Rule**: Tests belong in `tests/` or `*_test.rs`, not inline
- **Current**: 228 lines of tests (lines 653–881) inline in production module
- **Required Action**: Move to `expression/tests.rs` or integration test file

### ❌ VIOLATION 4: Potential Primitive Obsession
- `TokenKind` variants use raw `i64`, `f64`, `Box<str>` instead of domain wrappers
- `ExpressionLiteral::I64(i64)` — could be refined to a domain-validated type

---

## 4. Recommended Refactoring

### Proposed Structure

```
crates/vb_compile/src/expression/
├── mod.rs           (~10 lines)  — re-exports
├── domain.rs        (~130 lines) — ParsedExpression, ExpressionLiteral, ExpressionHelper, UnaryOp, BinaryOp
├── lexer.rs         (~260 lines) — Lexer<'a> struct and methods
├── parser.rs        (~240 lines) — Parser<'a> struct and methods  
└── tests.rs         (~230 lines) — inline tests moved out
```

### Priority: **P0 — CRITICAL**

The file is nearly 3× the allowed size. It must be split before further development to prevent architectural decay.

---

## 5. Summary

| Check | Result |
|-------|--------|
| Lines | 881 (limit 300) — **FAIL** |
| DDD Cohesion | LOW — mixed domain + infrastructure |
| DDD Smell | God Module / Low Cohesion |
| Priority | **P0 — CRITICAL** |

**STATUS**: `NEEDS_REFACTORING`

