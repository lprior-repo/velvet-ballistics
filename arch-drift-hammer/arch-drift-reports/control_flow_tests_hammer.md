# Architectural Drift Report: `control_flow/tests.rs`

**File**: `crates/vb_compile/src/control_flow/tests.rs`
**Total Lines**: 444 (VIOLATION: <300 rule — 148% of limit)
**Report Type**: Architectural Drift Hammer
**Date**: 2026-05-29

---

## EXECUTIVE SUMMARY

| Category | Severity | Count |
|----------|----------|-------|
| Line Count Violation | CRITICAL | 1 (444 > 300) |
| Primitive Obsession | CRITICAL | 7 |
| DDD Cohesion Violation | HIGH | 3 |
| Test Duplication | HIGH | 6 pairs |
| Stringly-Typed Domain | HIGH | 4 |

---

## 1. LINE COUNT VIOLATION

**Rule**: All source files must be ≤300 lines.
**Status**: ❌ FAIL — 444 lines detected (148% over limit)

The file MUST be split. Suggested decomposition:

| Slice | Est. Lines | Contains |
|-------|------------|----------|
| `control_flow/test_helpers.rs` | ~60 | `parse_error`, `ensure`, `ensure_*` helpers |
| `control_flow/tests/preemption.rs` | ~120 | Preemption test cases |
| `control_flow/tests/branch_targets.rs` | ~100 | Backward/self-cycle test cases |
| `control_flow/tests/unreachable.rs` | ~80 | Unreachable step test cases |
| `control_flow/tests/schema_errors.rs` | ~80 | Input/reference schema error tests |

---

## 2. PRIMITIVE OBSESSION VIOLATIONS (CRITICAL)

### 2.1 Raw `usize` Step Indices in Pattern Matches
**Location**: Lines 22-91 (all `ensure_*` functions)

```rust
// VIOLATION: Raw usize for domain concept "step index"
fn ensure_unknown_target(error: CompileError) -> Result<(), String> {
    ensure(
        matches!(
            error,
            CompileError::UnknownStepTarget { step: 1, target: 3 }  // ← raw 1, 3
        ),
        "unknown target did not use public typed diagnostic",
    )
}
```

**Domain Concept**: `StepIndex` — a bounded, non-negative step identifier.
**Fix**: Create `StepIndex(u16)` newtype in domain model. Tests should construct `StepIndex::new(1)` and match against that.

### 2.2 Raw `usize` Branch Target Indices
**Location**: Lines 26, 70, 80, 88

```rust
CompileError::UnknownStepTarget { step: 1, target: 3 }  // ← raw target: 3
CompileError::BackwardBranchTarget { step: 1, target: 0 } // ← raw target: 0
CompileError::BackwardBranchTarget { step: 1, target: 1 } // ← self-cycle
CompileError::UnreachableStep { step: 2 }                // ← raw step: 2
```

**Domain Concept**: `BranchTarget` — a forward edge in the control flow graph.
**Fix**: `BranchTarget(StepIndex)` wrapper.

### 2.3 Stringly-Typed Field Names
**Location**: Lines 51, 108

```rust
CompileError::StepFieldShape { field: "finish", .. }  // ← stringly typed
CompileError::FieldShape { field: "inputs", .. }      // ← stringly typed
```

**Domain Concept**: `FieldName` — a known workflow field.
**Fix**: `FieldName` enum with variants `Finish`, `Inputs`, `Then`, `Choose`, etc.

### 2.4 Stringly-Typed Reference Kind
**Location**: Line 97

```rust
CompileError::UnknownReferenceName { kind: "input", .. }  // ← stringly typed
```

**Domain Concept**: `ReferenceKind` — input, output, var, secret, etc.
**Fix**: `ReferenceKind` enum with variants `Input`, `Output`, `Var`, `Secret`.

### 2.5 Raw `&[u8]` for YAML Source
**Location**: Lines 3, 116-118, 122-138, 306-329

```rust
// VIOLATION: Raw bytes instead of domain type
fn parse_error(source: &[u8]) -> Result<CompileError, String> {
    match YamlCompiler::default().parse_ast(source) {
```

**Domain Concept**: `WorkflowSource` — the YAML document being compiled.
**Fix**: `WorkflowSource(Vec<u8>)` or `WorkflowSource(Cow<'static, [u8]>)` newtype.

### 2.6 Mixed `condition` Types in YAML Test Data
**Location**: Lines 132, 172, 228, 256, 283, 319, 341, 364, 388, 409

```yaml
condition: 0   # ← integer at lines 132, 172, 228, 256, 283, 319, 341
condition: true  # ← boolean at lines 364, 388, 409
```

**Domain Concept**: `Condition` — a value that determines branch choice.
**Fix**: Unified `ConditionValue` type with explicit variants or coercion.

### 2.7 Raw `bool` Result in Test Helper
**Location**: Lines 14-20

```rust
fn ensure(condition: bool, message: &'static str) -> Result<(), String> {
```

**Domain Concept**: `TestAssertion` — a testable claim about domain behavior.
**Fix**: Use a proper assertion combinator library or typed assertions.

---

## 3. DDD COHESION VIOLATIONS

### 3.1 Test Utilities Mixed with Test Cases
**Location**: Lines 3-118 (utilities) + Lines 120-444 (tests)

The `parse_error`, `ensure`, and all `ensure_*` functions are defined in the same file as the test cases. Per DDD, test utilities should be in a separate module.

**Required Structure**:
```
control_flow/
├── mod.rs           (~5 lines - re-exports)
├── test_helpers.rs  (~60 lines - parse_error, ensure, ensure_*)
└── tests/
    ├── mod.rs       (~10 lines)
    ├── preemption.rs    (~120 lines)
    ├── branch_targets.rs (~100 lines)
    └── unreachable.rs   (~80 lines)
```

### 3.2 Pattern Matching Against Raw Values Instead of Domain Types
**Location**: Lines 22-91

The `ensure_*` functions validate behavior by pattern matching on `CompileError` with hardcoded integer values:

```rust
fn ensure_unknown_target(error: CompileError) -> Result<(), String> {
    ensure(
        matches!(
            error,
            CompileError::UnknownStepTarget { step: 1, target: 3 }  // ← hardcoded
        ),
        "unknown target did not use public typed diagnostic",
    )
}
```

**DDD Principle Violated**: "Parse, don't validate" — the tests should use constructed domain values, not raw pattern matching.

**Fix**: Create typed domain fixtures:
```rust
fn unknown_target_error() -> CompileError {
    CompileError::UnknownStepTarget {
        step: StepIndex::new(1),
        target: StepTarget::new(3),
    }
}
```

### 3.3 Test Assertion Language is Imperative, Not Declarative
**Location**: Lines 14-20, 22-114

```rust
fn ensure(condition: bool, message: &'static str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(message.to_owned()) }
}
```

This is an imperative assertion style. The tests should use declarative BDD-style assertions with domain types.

---

## 4. TEST DUPLICATION (HIGH)

### 4.1 Duplicate Backward Branch Tests
**Location**: Lines 353-374 (`parse_ast_rejects_backward_step_targets`) and Lines 376-397 (`parse_ast_rejects_backward_step_targets_again`)

Both tests use identical YAML:
```yaml
- id: first
  save:
    value: 1
- id: route
  choose:
    condition: true
    on_true: 0    # ← backward to step 0
    on_false: 2
```

**Fix**: Parameterize the test with a helper function or use a test factory pattern.

### 4.2 Duplicate Self-Cycle Tests
**Location**: Lines 399-421 (`parse_ast_rejects_self_cycles_again`) and Lines 423-444 (`parse_ast_rejects_self_cycles`)

Both tests use identical YAML:
```yaml
- id: first
  save:
    value: 1
- id: route
  choose:
    condition: true
    on_true: 1    # ← self-cycle
    on_false: 2
```

### 4.3 Duplicate Unreachable Step Tests
**Location**: Lines 218-242 (`parse_ast_rejects_unreachable_steps_after_reference_validation`) and Lines 244-269 (`parse_ast_rejects_unreachable_steps_after_reference_validation_again`)

Both tests use identical YAML.

**Fix**: Consolidate into parameterized tests:
```rust
#[test]
fn parse_ast_rejects_unreachable_steps() {
    for (name, source) in unreachable_test_cases() {
        let error = parse_error(source).unwrap();
        ensure_unreachable(error);
    }
}
```

---

## 5. STRINGLY-TYPED DOMAIN VALUES

### 5.1 Field Names as Strings
**Location**: Lines 51, 108

The tests reference `"finish"` and `"inputs"` as string literals, not as `FieldName` enum variants.

### 5.2 Reference Kind as String
**Location**: Line 97

The test checks `kind: "input"` as a string literal, not as a `ReferenceKind` enum.

### 5.3 Trigger/When Shape as Stringly-Typed
**Location**: Lines 306-329

```yaml
when:
  manual: {}
```

The test doesn't validate trigger kinds through typed structures.

---

## 6. SUMMARY OF REQUIRED REFACTORS

| # | Action | Priority |
|---|--------|----------|
| 1 | Split file into `test_helpers.rs` + `tests/*.rs` | CRITICAL |
| 2 | Create `StepIndex(u16)` newtype, replace raw `usize` in tests | CRITICAL |
| 3 | Create `BranchTarget(StepIndex)` newtype | CRITICAL |
| 4 | Create `FieldName` enum (`Finish`, `Inputs`, `Then`, etc.) | HIGH |
| 5 | Create `ReferenceKind` enum (`Input`, `Output`, `Var`, `Secret`) | HIGH |
| 6 | Create `WorkflowSource(Vec<u8>)` wrapper | MEDIUM |
| 7 | Create `Condition` domain type unifying integer/boolean | MEDIUM |
| 8 | Consolidate duplicate test cases into parameterized tests | MEDIUM |
| 9 | Replace `ensure()` helper with typed BDD assertions | MEDIUM |
| 10 | Use constructed domain error types, not raw pattern matching | HIGH |

---

## 7. VERDICT

**ARCHITECTURAL DRIFT STATUS**: ❌ FAIL

This test file is a repository of **primitive obsession** and **test duplication**. The tests validate complex domain behavior (control flow validation, preemption ordering, unreachable step detection) using raw integers, string literals, and pattern matching against `CompileError` variants with hardcoded values.

The file exceeds the 300-line limit at 444 lines and contains 6 pairs of duplicate test cases that should be consolidated into parameterized tests.

**Estimated Refactor Effort**: 2 beads

**First Action**: Create `crates/vb_compile/src/control_flow/test_helpers.rs` and extract `parse_error`, `ensure`, and `ensure_*` functions. Then split tests into `tests/preemption.rs`, `tests/branch_targets.rs`, and `tests/unreachable.rs`.

---

## 8. DOMAIN MAP

```
CompileError (domain)
├── UnknownStepTarget { step: StepIndex, target: BranchTarget }
├── BackwardBranchTarget { step: StepIndex, target: StepIndex }
├── UnreachableStep { step: StepIndex }
├── UnknownReferenceName { kind: ReferenceKind, ... }
├── FieldShape { field: FieldName, ... }
├── StepFieldShape { field: FieldName, ... }
└── LastStepMustFinish

ReferenceKind (MISSING - use string "input")
FieldName (MISSING - use string "finish", "inputs")
StepIndex (MISSING - use raw usize)
BranchTarget (MISSING - use raw usize)
WorkflowSource (MISSING - use raw &[u8])
Condition (MISSING - inconsistent usize/bool)
```
