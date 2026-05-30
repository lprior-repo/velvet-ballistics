# Architectural Drift Report: type_taint/tests.rs

**File:** `crates/vb_compile/src/type_taint/tests.rs`
**Actual Lines:** 854 (VIOLATION: 300-line hard limit)
**Violation Severity:** CRITICAL

---

## Executive Summary

This test file is **554 lines over budget** (854 actual vs 300 max). It exhibits widespread **primitive obsession** and **DDD boundary violations** that make the test suite fragile, poorly structured, and domain-anemic.

---

## Category 1: File Size Violations

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 854 | 300 | **VIOLATION** |
| Test functions | 23 | N/A | Acceptable |
| Helper functions | 27+ | 10 | **EXCESSIVE** |
| Line helpers | 4 | 0 | **EXCESSIVE** |

**Root Cause:** No test fixture module separation. All helpers dumped into single file.

---

## Category 2: Primitive Obsession Violations

### 2.1 Raw Integer Slot Indices (CRITICAL)

Every slot index in this file is a raw `u32`/`usize` instead of a domain type:

```rust
// LINE 105-106: Raw StepIdx construction
.node(StepIdx::new(0))                    // raw 0
.ok_or_else(|| "compiled workflow did not contain step 0".to_owned())?

// LINE 109: Raw StepIdx in pattern
if node.output == Some(SlotIdx::new(0)) && node.next == Some(StepIdx::new(1))

// LINE 249: Magic number 1 embedded in error check
CompileError::UnknownSlotType { field: "choose.condition", slot: 1 }

// LINE 262: Magic number 0 for finish slot
CompileError::UnknownSlotType { field: "finish.result", slot: 0 }

// LINE 271: Magic number 65536 for overflow
CompileError::SlotIndexOutOfRange { value: 65536 }

// LINE 301: 65_536 iterations with raw u32
(0_u32..65_536).map(|index| format!("  - id: pad_{index}\n    save:\n      value: null\n"))

// LINE 568: Raw 65536 in YAML
condition: 65536

// LINE 602: Magic number in YamlLimits struct
max_mapping_entries: 1_024,  // Why 1024? Named constant needed

// LINE 603: Magic number
max_scalar_bytes: 65_536,     // Why 65536? Named constant needed
```

**FIX REQUIRED:** Introduce `SlotIndex(usize)` and `StepIndex(usize)` newtype types with const constructors for known indices (`SLOT_ZERO`, `STEP_ZERO`, etc.).

### 2.2 Raw String Field Names (CRITICAL)

```rust
// LINE 204-207: Field name as raw string
field: "choose.condition",
expected: "boolean",
found: "number"

// LINE 219-222: Field name as raw string
field: "choose.condition",
expected: "boolean",
found,

// LINE 248-249: Field name raw string
field: "choose.condition",
slot: 1

// LINE 261-262: Field name raw string
field: "finish.result",
slot: 0

// LINE 446-449: Field name raw string
field: "finish.result",
slot: 1
```

**FIX REQUIRED:** Replace with `FieldName("choose.condition")` or better yet, `ChooseField::Condition`, `FinishField::Result` domain enums.

### 2.3 Raw Type Name Strings (CRITICAL)

```rust
// LINE 205-206: Type names as raw strings
expected: "boolean",
found: "number"  // "text", "null", "list", "object" all appear as raw strings

// LINE 220-221: More raw type strings
expected: "boolean",
found,
```

**FIX REQUIRED:** `TypeName::Boolean`, `TypeName::Number`, etc. domain enum.

### 2.4 Magic Numbers in YamlLimits (HIGH)

```rust
// LINE 597-604: All magic numbers
YamlLimits {
    max_source_bytes: 4_000_000,      // Why 4M?
    max_depth: 64,                    // Why 64?
    max_nodes: 500_000,               // Why 500K?
    max_sequence_len: 70_000,         // Why 70K?
    max_mapping_entries: 1_024,       // Why 1024?
    max_scalar_bytes: 65_536,         // Why 65536?
}
```

**FIX REQUIRED:** These should come from a `YamlLimits::production()` or `YamlLimits::test_friendly()` constructor with documented rationale.

### 2.5 Version String Primitive (MEDIUM)

```rust
// LINE 309: Raw string repeated across file
version: "velvet-ballistics/v1".into(),

// Also appears as raw bytes:
// LINE 124, 144, 164, 279, 286, 292, 338, 356, 374, 388, 402, 417, 625, 643, 709, 727, 789, 812, 835
```

**FIX REQUIRED:** `Version::VELVET_BALLISTICS_V1` constant.

### 2.6 Raw `$secrets.token`, `$input.user`, `$vars.label` Reference Strings

```rust
// LINE 332: Raw reference
result: AstExpression::Reference("$secrets.token".into()),

// LINE 673: Raw reference string
parsed_reference_expression("$input.user"),

// LINE 677: Raw reference string
parsed_reference_expression("$vars.label"),
```

**FIX REQUIRED:** `SecretRef::new("token")`, `InputRef::new("user")`, `VarRef::new("label")` domain types.

---

## Category 3: DDD Cohesion Violations

### 3.1 Anemic Test Helpers

Functions like `initialized_slot_condition_source()` (line 278) produce raw YAML bytes instead of domain-typed workflow fixtures:

```rust
fn initialized_slot_condition_source(value: &str) -> Vec<u8> {
    // Returns raw bytes - caller must know YAML structure
    format!(r#"version: velvet-ballistics/v1
name: choose_case
when:
  manual: {{}}
steps:
  - id: captured
    save:
      value: {value}
  ...
"#).into_bytes()
}
```

**FIX REQUIRED:** Build `WorkflowAst` directly or use a typed workflow builder.

### 3.2 Inline YAML in 20+ Helper Functions

| Function | Line | Problem |
|----------|------|---------|
| `initialized_boolean_slot_choose_source` | 123 | Inline YAML |
| `literal_boolean_choose_source` | 143 | Inline YAML |
| `finish_literal_source` | 163 | Inline YAML |
| `initialized_slot_condition_source` | 278 | Inline YAML |
| `literal_choose_condition_source` | 285 | Inline YAML |
| `finish_result_fragment_source` | 292 | Inline YAML |
| `large_finish_slot_source` | 299 | Inline YAML |
| `nested_secret_list_finish_source` | 338 | Inline YAML |
| `nested_secret_object_finish_source` | 356 | Inline YAML |
| `clean_input_finish_source` | 374 | Inline YAML |
| `clean_vars_finish_source` | 388 | Inline YAML |
| `forward_finish_slot_source` | 402 | Inline YAML |
| `reference_preempt_source` | 417 | Inline YAML |

**FIX REQUIRED:** Extract to `tests/fixtures/workflow_ast.rs` or similar.

### 3.3 Stringly-Typed Error Matching

```rust
// LINE 199-211: Match on raw CompileError with string comparisons
fn ensure_choose_type_mismatch(error: CompileError) -> Result<(), String> {
    ensure(
        matches!(
            error,
            CompileError::TypeMismatch {
                field: "choose.condition",  // Raw string
                expected: "boolean",        // Raw string
                found: "number"            // Raw string
            }
        ),
        "choose condition did not use boolean type diagnostic",
    )
}
```

**FIX REQUIRED:** Use typed `TypeMismatch::new(ChooseField::Condition, TypeName::Boolean, TypeName::Number)` for comparison.

---

## Category 4: Test Structure Violations

### 4.1 Inline Test Cases (800+ lines in test functions)

Test functions at lines 494-854 are massive inline YAML blobs without helper abstractions:

```rust
#[test]
fn compile_and_parse_ast_reject_non_boolean_choose_condition() -> Result<(), String> {
    let source = br#"version: velvet-ballistics/v1   // 20+ lines of raw YAML
name: type_case
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  ...
```

### 4.2 Duplicate Error-Checking Patterns

Lines 233-267 have near-identical `ensure_*_error` functions that could be unified:

```rust
fn ensure_reference_error(error: CompileError) -> Result<(), String> { ... }
fn ensure_unknown_choose_slot(error: CompileError) -> Result<(), String> { ... }
fn ensure_unknown_finish_slot(error: CompileError) -> Result<(), String> { ... }
fn ensure_slot_index_out_of_range(error: CompileError) -> Result<(), String> { ... }
fn ensure_forward_finish_slot(error: CompileError) -> Result<(), String> { ... }
```

**FIX REQUIRED:** Generic `ensure_compile_error<E: Into<CompileError>>(source, expected_error)` pattern.

---

## Category 5: Boundary Violations

### 5.1 Cross-Crate Type Exposure

Line 8 imports `vb_core` types directly:
```rust
use vb_core::{CompiledNodeKind, CompiledWorkflow, ConstValue, SlotIdx, StepIdx};
```

This exposes internal compilation details to the test. Tests should use public API types only.

### 5.2 Test Depends on Internal `validate_workflow_ast`

Line 1:
```rust
use super::validate_workflow_ast;
```

Tests reach into private module members. This violates encapsulation.

---

## Summary of Violations

| Category | Count | Severity |
|----------|-------|----------|
| File size (>300 lines) | 554 over | CRITICAL |
| Raw slot indices | 20+ occurrences | CRITICAL |
| Raw field name strings | 15+ occurrences | CRITICAL |
| Raw type name strings | 10+ occurrences | CRITICAL |
| Magic numbers in YamlLimits | 6 | HIGH |
| Inline YAML helpers | 13 functions | HIGH |
| Stringly-typed error matching | 5+ functions | HIGH |
| Duplicate error checking patterns | 4+ functions | MEDIUM |
| Cross-crate internal imports | 1 import | MEDIUM |
| Private module access | 1 access | MEDIUM |

---

## Recommended Refactoring

1. **SPLIT** this file into:
   - `tests/type_taint/validators.rs` (validation tests)
   - `tests/type_taint/compilation.rs` (compilation tests)
   - `tests/fixtures/workflow_ast.rs` (shared workflow builders)
   - `tests/fixtures/yaml_sources.rs` (YAML string constants)

2. **INTRODUCE** domain types:
   - `SlotIndex(usize)` with const `ZERO`, `ONE`
   - `StepIndex(usize)` with const `ZERO`, `ONE`
   - `FieldName` enum: `ChooseCondition`, `FinishResult`
   - `TypeName` enum: `Boolean`, `Number`, `Text`, `Null`, `List`, `Object`
   - `Version` const: `VELVET_BALLISTICS_V1`

3. **ELIMINATE** magic numbers by introducing `YamlLimits::test()` with documented values.

4. **REPLACE** inline YAML with `WorkflowAst` builders in test fixture module.
