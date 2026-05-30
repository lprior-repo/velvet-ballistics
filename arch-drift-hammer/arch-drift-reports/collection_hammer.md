# Architectural Drift Report: `collection.rs`

**File:** `crates/vb_compile/src/mod_compile_errors/collection.rs`
**Line Count:** 301 lines (1 line over the 300-line limit)
**Assessment:** CRITICAL — Over limit AND pervasive primitive obsession

---

## 1. LINE COUNT VIOLATION

**Violation:** 301 lines exceeds the 300-line hard cap by exactly 1 line.

**Root Cause:** This file is a dumping ground for error categorization. It mixes:
- `CompileError::code()` method (lines 8-106)
- 9 helper functions for code mapping (lines 114-232)
- `CompileErrors` newtype (lines 235-294)
- `collect()` utility (lines 297-301)

The 300-line violation is a symptom of a deeper cohesion failure: this file should not exist as a monolithic collection of unrelated error-mapping utilities.

---

## 2. RESPONSIBILITY MAPPING

| Responsibility | Lines | Assessment |
|---|---|---|
| `CompileError::code()` mapping | 8-106 | **Mixes 50+ variants** — violates SRP |
| `HasSymbolicCode` impl | 108-112 | Thin delegate, belongs elsewhere |
| `canonical_yaml_code()` | 114-125 | String-based category dispatch |
| `workflow_error_code()` | 127-149 | WorkflowError → SymbolicCode mapping |
| `validation_error_code()` | 151-162 | ValidationError → SymbolicCode mapping |
| `invalid_name_code()` | 164-170 | Reserved name detection |
| `is_reserved_name()` | 172-185 | Hardcoded string matching |
| `primitive_code()` | 187-202 | Primitive name → invalid code |
| `control_field_code()` | 204-211 | Field name → error code |
| `step_field_shape_code()` | 213-224 | Field name → error code |
| `unknown_reference_code()` | 226-232 | Reference kind → error code |
| `CompileErrors` newtype | 235-294 | Throwsaway wrapper, no domain logic |
| `collect()` utility | 297-301 | Free function, should be method |

**Problem:** This file is an **error code categorizer**, but it contains:
1. The enum method for CompileError
2. Cross-crate error mapping helpers (WorkflowError, ValidationError)
3. A throwaway newtype wrapper that adds zero value

These should be split across at least 3 files.

---

## 3. PRIMITIVE OBSESSION VIOLATIONS (Scott Wlaschin DDD)

### 3.1 Field Name Primitive Obsession

Every function that takes a field name uses `&str`:

```rust
// Lines 204, 213 — two separate functions doing nearly identical string matching
pub(super) fn control_field_code(field: &str) -> &'static str { ... }
pub(super) fn step_field_shape_code(field: &str) -> &'static str { ... }
```

**Should be:** `enum StepField` with variants for `Then`, `TryAgain`, `OnError`, `Choose`, `Condition`, `OnTrue`, `OnFalse`, `ForEach`, `Parallel`, `Branches`, `Collect`, `Aggregate`, `Repeat`, `Finish`, `Result`, etc.

### 3.2 Primitive Name Primitive Obsession

```rust
// Line 187 — raw string dispatch
pub(super) fn primitive_code(primitive: &str) -> &'static str {
    match primitive {
        "for_each" => "INVALID_FOR_EACH",
        "parallel" => "INVALID_TOGETHER",
        "collect" | "gather" => "INVALID_COLLECT",
        // ...
    }
}
```

**Should be:** `enum StepPrimitive` with variants `ForEach`, `Parallel`, `Collect`, `Gather`, `Aggregate`, `Summarize`, `Repeat`, `Wait`, `Ask`, `TryAgain`, `OnError`, `Finish`, `Choose`.

### 3.3 Reserved Name Primitive Obsession

```rust
// Lines 172-185 — raw string pattern matching
fn is_reserved_name(value: &str) -> bool {
    matches!(
        value,
        "if" | "then" | "else" | "when" | "steps" | "action" | "result" | "input" | "secret" | "secrets"
    )
}
```

**Should be:** `enum ReservedKeyword` with variants, or at minimum a `ReservedNameSet` type that encapsulates the matching logic.

### 3.4 Reference Kind Primitive Obsession

```rust
// Line 226 — raw string for semantic concept
pub(super) fn unknown_reference_code(kind: &str) -> &'static str {
    if kind == "secret" || kind == "secrets" {
        "SECRET_NOT_DECLARED"
    } else {
        "UNKNOWN_REFERENCE"
    }
}
```

**Should be:** `enum ReferenceKind` with variants `Secret`, `Secrets`, `Other`.

### 3.5 YAML Category Primitive Obsession

```rust
// Line 114 — raw string matching on parser categories
pub(super) fn canonical_yaml_code(category: &str) -> &'static str {
    match category {
        "duplicate_key" => "DUPLICATE_KEY",
        "document_count" => "FORBIDDEN_YAML_FEATURE",
        // ...
    }
}
```

**Should be:** `enum YamlErrorCategory` with typed variants.

### 3.6 `CompileErrors` Newtype is Hollow

```rust
// Lines 236-273 — all delegation, zero domain logic
pub struct CompileErrors(pub Vec<CompileError>);
```

This newtype adds **nothing** over `Vec<CompileError>`. Every method is a direct delegation:
- `first()` → `self.0.first()`
- `as_slice()` → `&self.0`
- `iter()` → `self.0.iter()`
- `len()` → `self.0.len()`
- `is_empty()` → `self.0.is_empty()`

This is not a domain concept — it's a type alias with extra indirection.

---

## 4. COHESION FAILURES

### 4.1 Cross-Crate Import Coupling

Lines 6 imports `vb_core::{ActionId, HasSymbolicCode, SideEffect, SymbolicCode, WorkflowError}` and line 151 uses `vb_validate::ValidationError`. These cross-crate dependencies suggest this file is a **catalog** not a **module**.

### 4.2 SRP Violation in `CompileError::code()`

The match expression at line 12 handles **50+ error variants** grouped by arbitrary categories (e.g., `"FORBIDDEN_YAML_FEATURE"` groups 10 unrelated variants). This is not cohesive — error codes should be defined closer to where the errors are constructed.

### 4.3 Error Code Registry Drift Risk

Lines 90-98 comment states that all symbolic strings must be registered in `CODE_REGISTRY`, but this file has no enforcement mechanism. If a developer adds a new match arm with an unregistered string, `SymbolicCode::INTERNAL_INVARIANT` is silently returned. This is a **hidden invariant** that should be a compile-time guarantee.

---

## 5. PRESCRIBED REMEDIATION

### 5.1 Immediate (resolve 300-line violation)

**Split into 4 files:**

1. **`compile_error_code.rs`** — `CompileError::code()` method only
2. **`error_code_helpers.rs`** — All `*_code()` helper functions (these need refactoring)
3. **`compile_errors.rs`** — `CompileErrors` newtype (or remove entirely if not needed)
4. **`collect.rs`** — The `collect()` function

### 5.2 Type Refactoring (resolve primitive obsession)

Replace all `&str` parameters with **strongly-typed enums**:

```rust
// New types to introduce
enum StepField { Then, TryAgain, OnError, Choose, Condition, OnTrue, OnFalse,
                 ForEach, Parallel, Branches, Collect, Aggregate, Repeat,
                 Finish, Result }
enum StepPrimitive { ForEach, Parallel, Collect, Gather, Aggregate, Summarize,
                      Repeat, Wait, Ask, TryAgain, OnError, Finish, Choose }
enum YamlCategory { DuplicateKey, DocumentCount, LimitExceeded, UnknownField,
                    EmptySource, MissingField, FieldShape, ParseError, ForbiddenFeature }
enum ReferenceKind { Secret, Secrets, Other }
```

### 5.3 `CompileErrors` Decision

Either:
- **Remove it** and use `Vec<CompileError>` directly — the newtype adds no value
- **Enrich it** with actual domain methods (e.g., `has_category()`, `filter_by_code()`, `partition_fatal_vs_recoverable()`)

### 5.4 Code Registry Enforcement

Add a `const fn` assertion or unit test that verifies all strings returned by `*_code()` functions exist in `CODE_REGISTRY`, making the invariant explicit rather than commented.

---

## 6. SUMMARY

| Issue | Severity | Lines Affected |
|---|---|---|
| 300-line violation | **CRITICAL** | 301 |
| Field name primitive obsession | **HIGH** | 164-232 |
| Primitive name primitive obsession | **HIGH** | 187-202 |
| Reserved name primitive obsession | **MEDIUM** | 172-185 |
| Reference kind primitive obsession | **MEDIUM** | 226-232 |
| YAML category primitive obsession | **MEDIUM** | 114-125 |
| Hollow `CompileErrors` newtype | **MEDIUM** | 236-273 |
| Cross-crate import coupling | **LOW** | 6, 151 |
| SRP violation in `code()` | **LOW** | 8-106 |

**Recommended Action:** This file must be refactored before it can pass architectural review. The primitive obsession violations represent significant technical debt that will make future maintenance of error codes error-prone.
