# Architectural Drift Report: `references/tests.rs`

**File**: `crates/vb_compile/src/references/tests.rs`  
**Line Count**: 1072 lines (EXCEEDS 300-line limit by 357%)  
**Severity**: CRITICAL DRIFT  
**Date**: 2026-05-29

---

## Executive Summary

This file is a **1072-line monolithic test file** that violates:
1. **Hard 300-line limit** (1072 lines = 357% over limit)
2. **Single Responsibility Principle** — 40+ test functions crammed into one file
3. **Primitive Obsession** — raw `&[u8]` byte slices, `String` errors, `bool` assertions
4. **DDD Cohesion** — no value objects, no domain types, no proper test organization

---

## Line Count Breakdown

| Section | Lines | Test Count | Violation |
|---------|-------|------------|-----------|
| Helper functions (parse_error, parse_ok, ensure, etc.) | 1–28 | 4 helpers | DRIFT |
| Unknown reference tests | 29–230 | 6 tests | DRIFT |
| Adversarial reference resolution | 257–434 | 9 tests | DRIFT |
| Security accessor path tests | 436–486 | 2 tests | DRIFT |
| Edge case reference resolution | 488–1072 | 24+ tests | DRIFT |

**Total**: 1072 lines for a test file that should be 300 lines max.

---

## Primitive Obsession Violations

### 1. Raw Byte Slices for YAML Source

```rust
// VIOLATION: `&[u8]` instead of typed YamlSource value object
fn unknown_input_reference_source() -> &'static [u8] {
    br#"version: velvet-ballistics/v1
name: ref_case
..."#
}
```

**Problem**: Raw byte literals `br#"..."#` are primitive `&[u8]`. Should use a domain type like `YamlSource` or `WorkflowSource` value object that encapsulates validation and parsing intent.

**Fix**: Create a `TestFixture` or `WorkflowSource` type that wraps the bytes and provides named constructors.

### 2. String-Based Error Aggregation

```rust
// VIOLATION: Result<..., String> is primitive obsession
fn parse_error(source: &[u8]) -> Result<CompileError, String> {
    match YamlCompiler::default().parse_ast(source) {
        Ok(ast) => Err(format!("parse_ast unexpectedly succeeded: {ast:?}")),
        Err(errors) => errors.0.into_iter().next()
            .ok_or_else(|| "parse_ast failed with no errors".to_string()),
    }
}
```

**Problem**: `String` error messages lose type safety. The `format!` wrappers hide actual domain errors.

**Fix**: Use typed `TestError` enum or at minimum a dedicated `ParseTestError` type.

### 3. Boolean Assertion Helper

```rust
// VIOLATION: bool + &'static str is primitive
fn ensure(condition: bool, message: &'static str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(message.to_owned()) }
}
```

**Problem**: `bool` is the most primitive type. The `message` is a raw `&'static str` instead of a typed assertion error.

**Fix**: Replace with `assert_that!(actual, matches!(expected), "description")` pattern or use a proper test assertion combinator.

### 4. Duplicate Helper Functions

```rust
// Line 3-12: parse_error
fn parse_error(source: &[u8]) -> Result<CompileError, String>

// Line 259-268: adv_ref_parse_error (EXACT DUPLICATE)
fn adv_ref_parse_error(source: &[u8]) -> Result<CompileError, String>
```

**Problem**: Identical function defined twice. This is copy-paste primitive obsession.

### 5. Inline YAML Literals Everywhere

```rust
// VIOLATION: 50+ inline byte string literals scattered across file
parse_ok(
    br#"version: velvet-ballistics/v1
name: ref_case
..."#
)
```

**Problem**: No abstraction over YAML test fixtures. Each test inlines its source, making refactoring impossible and reading difficult.

---

## DDD Cohesion Violations

### Missing Value Objects

| Primitive | Should Be |
|-----------|-----------|
| `&[u8]` YAML source | `YamlSource` or `WorkflowSource` |
| `String` error messages | `TestAssertionError` enum |
| `bool` condition | `Assertion<T>` type |
| `&'static str` messages | `ErrorMessage` newtype |

### Test Organization Failures

The file attempts to group tests by comment header only:

```rust
// ── SECURITY: accessor path traversal tests ──────────────────────────────
```

But these comments are **cosmetic** — the file remains one massive module with 40+ test functions.

### No Test Module Decomposition

According to DDD principles, tests should mirror production module structure:

```
references/
├── mod.rs         (production)
├── tests.rs       (SHOULD BE: integration smoke tests only, <300 lines)
├── reference_validation/    (SHOULD EXIST: unit tests per concept)
│   ├── unknown_ref_tests.rs
│   ├── accessor_tests.rs
│   ├── security_tests.rs
│   └── edge_case_tests.rs
└── fixtures/               (SHOULD EXIST: shared test fixtures)
    └── reference_fixtures.rs
```

---

## Recommended Refactoring

### Phase 1: Extract Value Objects

```rust
// Create: crates/vb_compile/src/references/test_fixtures.rs
pub struct YamlFixture {
    bytes: &'static [u8],
}

impl YamlFixture {
    pub fn new(yaml: &'static str) -> Self {
        Self { bytes: yaml.as_bytes() }
    }
    
    pub fn parse_ast(&self) -> Result<Ast, CompileErrors> {
        YamlCompiler::default().parse_ast(self.bytes)
    }
}
```

### Phase 2: Split Test Modules

Create `references/tests/` directory:
- `tests/smoke.rs` — 5–10 high-level integration tests (<300 lines)
- `tests/unknown_reference.rs` — unknown ref tests
- `tests/accessor_path.rs` — slot accessor tests
- `tests/security.rs` — security tests
- `tests/edge_cases.rs` — edge case tests

### Phase 3: Extract Shared Helpers

```rust
// Create: crates/vb_compile/src/references/testing.rs
pub fn parse_error(source: &YamlFixture) -> TestResult<CompileError>;
pub fn parse_ok(source: &YamlFixture) -> TestResult<()>;
pub fn assert_matches<E: std::fmt::Debug>(actual: E, pattern: ...) -> TestResult<()>;
```

---

## Severity Assessment

| Rule | Status | Evidence |
|------|--------|----------|
| 300-line limit | **FAIL** | 1072 lines (357% over) |
| Primitive obsession | **FAIL** | 5+ distinct violations |
| DDD cohesion | **FAIL** | Single flat module |
| File size | **FAIL** | 1072 lines |

---

## Enforcement Action Required

1. **IMMEDIATELY** split into `<300` line modules
2. Create `references/test_fixtures.rs` value object
3. Create `references/testing.rs` for shared helpers
4. Remove duplicate `adv_ref_parse_error`
5. Replace all `&[u8]` with `YamlFixture` type
6. Replace `Result<(), String>` with typed `TestResult`

**Estimated refactoring**: 4–6 beads

---

*Report generated by arch-drift-hammer v1.0*
