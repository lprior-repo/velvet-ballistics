# Architectural Drift Report: `vb_compile/src/references.rs`

**File:** `crates/vb_compile/src/references.rs`  
**Analysis Date:** 2026-05-29  
**Agent:** architectural-drift  

---

## 1. Line Count

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | **360** | 300 | ❌ EXCEEDS LIMIT |

**Violation:** File exceeds 300-line cap by 60 lines (20% over).

---

## 2. DDD Cohesion Analysis

### Domain Concept
Reference validation for compiled workflow ASTs — a focused, coherent domain.

### Cohesion Score: **GOOD**
- Single responsibility: all functions serve reference validation
- Clear entry point: `validate_workflow_ast/1`
- Delegation pattern: correctly outsources non-compile-specific validation to `vb_validate::references`
- Well-structured recursive traversal of AST nodes

### Module Structure
```
validate_workflow_ast (entry)
├── build_ref_tables
│   ├── entry_names_owned
│   ├── secret_names_owned
│   └── step_names_owned
├── collect_references_from_value_entries
├── collect_references_from_expression_entries
├── collect_references_from_values
├── collect_references_from_steps
│   └── collect_references_from_step_kind
├── collect_references_from_expression
│   └── collect_references_from_parsed_expression
├── collect_references_from_value
├── validate_compile_reference  ← compile-specific gate
│   ├── validate_slot_reference
│   ├── numeric_accessor_path
│   └── check_accessor_path
└── map_validation_error
```

---

## 3. Violations

### ❌ VIOLATION 1: Line Count (MANDATORY)
- **Severity:** CRITICAL
- **Rule:** Files must not exceed 300 lines
- **Detail:** 360 lines exceeds cap by 60 lines

### ⚠️ VIOLATION 2: Primitive Obsession — `reference: &str`
- **Severity:** MODERATE
- **Location:** `validate_compile_reference(reference: &str, ...)` (line 213)
- **Problem:** Raw `&str` used for reference paths instead of a NewType
- **Suggestion:** Create `ReferencePath(Box<str>)` or similar NewType to encapsulate parsing semantics and prevent invalid states
- **Quote from Scott Wlaschin:** "Make illegal states unrepresentable"

### ⚠️ VIOLATION 3: Primitive Obsession — `step_index: Option<usize>`
- **Severity:** MINOR
- **Location:** Multiple functions (`collect_references_from_value_entries`, `collect_references_from_expression_entries`, etc.)
- **Problem:** Raw `usize` for step indexing — could be `StepIndex(usize)` or `StepContext { index: usize, ... }`
- **Suggestion:** Consider `StepIndex(u32)` or `StepContext` to bind step index semantics

### ⚠️ VIOLATION 4: Box<str> Repetition in Error Types
- **Severity:** MINOR
- **Location:** `validate_slot_reference`, `map_validation_error`
- **Problem:** Repeated `Box::from(reference)` patterns suggest a `ReferencePath` NewType would eliminate boilerplate

### ℹ️ OBSERVATION: `#[allow(clippy::question_mark]` in `check_accessor_path`
- **Severity:** INFO
- **Location:** Line 292
- **Note:** Acceptable here as `?` on `Option` would obscure the control flow intent

---

## 4. DDD Smell Assessment

| Smell | Present | Severity |
|-------|---------|----------|
| Primitive Obsession | ✅ Yes | MODERATE |
| Feature Envy | ❌ No | — |
| Shotgun Surgery | ❌ No | — |
| Parallel Inheritance | ❌ No | — |
| Long Method | ❌ No | The longest function (`map_validation_error`) is 41 lines and is well-structured |
| Data Class | ❌ No | — |
| Refused Bequest | ❌ No | — |
| Inappropriate Intimacy | ❌ No | Clean delegation to `vb_validate` |

**Overall DDD Smell:** MODERATE (primarily primitive obsession on `&str` references)

---

## 5. Priority Assessment

| Priority | Item | Effort |
|----------|------|--------|
| **P0** | Reduce file from 360 → <300 lines | MEDIUM |
| P1 | Introduce `ReferencePath` NewType for `&str` | LOW |
| P2 | Optionally introduce `StepIndex` NewType | LOW |

---

## 6. Recommended Refactoring

### Split Target (P0 — Required)
The 60-line overage can be absorbed by splitting the test module into a separate file:

```
references.rs    (301 lines currently, would be ~260 after test extraction)
references_tests.rs  (moved from `#[cfg(test)] mod tests` at line 359)
```

### NewType for Reference (P1)
```rust
// In a new `reference_path.rs` or in references.rs
pub struct ReferencePath(Box<str>);

impl ReferencePath {
    pub fn as_str(&self) -> &str { &self.0 }
    pub fn strip_prefix(&self, prefix: &str) -> Option<&str> { ... }
}
```

---

## 7. Conclusion

**STATUS: REFACTOR REQUIRED**

The file is architecturally sound in terms of cohesion and delegation patterns, but violates the mandatory 300-line cap. The primitive obsession on `&str` for references is a moderate DDD smell that should eventually be addressed but is not blocking.

**Immediate action required:** Extract `#[cfg(test)] mod tests` into `references_tests.rs` and add `mod tests;` in its place.
