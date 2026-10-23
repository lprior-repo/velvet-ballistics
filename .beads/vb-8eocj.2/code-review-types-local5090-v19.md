# Independent Local5090 Code Review: vb-8eocj.2 Expression Types
**Date:** 2026-10-23
**Reviewer:** LOCAL5090 (Qwen 3.8 27B)
**Workspace:** ~/src/isoloated/vb-8eocj2-types-code-review-local5090-v19
**Revision:** 0cb351c02ab82c070fe3f1c663b6dfe25da95809 (frozen)
**Scope:** canonical_types.rs (241 lines), canonical_scalar.rs, canonical_references.rs, part_02.rs, expression_bytecode.rs
**Verdict:** STATUS REJECTED — Open findings require follow-up

## Architecture Assessment: CORRECT

### Same Canonical Resolver (Primary Requirement)
PASS. Both type checking and bytecode compilation use the SAME `resolve_slot_reference` function from canonical_references.rs:
- canonical_types.rs line 82: `let slot = resolve_slot_reference(&reference, outputs, step_outputs)?;`
- canonical_references.rs line 130: `let slot = resolve_slot_reference(reference, &self.outputs, &self.step_outputs)?;`

This guarantees SlotIdx->ExprType facts resolve actual expression references through identical lookup logic.

### Original Reference Spelling
PASS. canonical_types.rs line 92 preserves exact spelling:
`context.add_variable(reference.clone().into(), ty);`
The `$` prefix and full name are retained, matching bytecode reference handling.

### Name Validation Before Types
PASS. canonical_scalar.rs lines 89-92:
1. `validate_canonical_references(&parsed, outputs, step_outputs)` runs first
2. `infer_canonical_expr_type(&parsed, slot_types, outputs, step_outputs)` runs second

This ensures unknown name errors precede misleading type errors.

## Producer Step Reference Patterns: CORRECT

| Pattern | Resolution | Type Inference |
|---------|-----------|----------------|
| `$step.id` | step_outputs map | Slot type or Unknown |
| `$steps.id` | step_outputs map | Slot type or Unknown |
| `$output_name` | outputs map | Slot type or Unknown |
| `$step_id` (no prefix) | Both maps, reject ambiguity | Slot type or Unknown |

Ambiguous references (same name in both maps with different slots) are rejected via `ExpressionLoweringUnsupported` error.

## Type System Coverage: CORRECT

### Supported Literal Types
- `true`/`false` → Bool
- `null` → Null
- `-999999` → I64
- `3.14` → F64
- `"text"` → Text (not supported in expressions yet)

### Type Mismatch Errors
`CompileError::TypeMismatch { field: "set.value", expected: "boolean", found: "i64" }`
Example: `true + 2` produces `expected: "number", found: "boolean"`.

### I64SetConst Preservation
PASS. canonical_scalar.rs line 51 checks I64 parse before expression compilation:
```rust
if let Ok(constant) = value.parse::<i64>() {
    lower_set_constant(id, slot, constant, next, builder)?;
    return Ok(ExprType::I64);
}
```
Existing `set slot 0 -999999` continues producing `SetConst` shape.

### Bool/F64/Null Operations
Type checker validates operations at compile time:
- Bool operations (and, or, not) restricted to Bool type
- Arithmetic operations (add, sub, mul, div) restricted to number types
- Null operations allowed (produce null or type error)

## Error Handling Analysis

| Error | Variant | Scenario |
|-------|---------|----------|
| Unknown name | `UnknownReferenceName` | `$nonexistent_slot` |
| Unknown root | `UnknownReferenceRoot` | `$invalid.foo` |
| Type mismatch | `TypeMismatch` | `true + 2`, `null.length()` |
| Helper arity | `ExpressionHelperArity` | `length(x, y)` |
| Unknown helper | `ExpressionUnexpectedToken` | `foo(x)` |

## Open Findings (Blocking)

### F1: Bool/F64 Operation Type Checking Not Fully Verified
**Severity:** Medium
Type checker should reject:
- `true * 2` (Bool arithmetic)
- `2 + null` (Null arithmetic)
- `null.contains("x")` (Null method call)

**Status:** Type inference code is correct, but test coverage for all 7 new type operation cases is pending. The parent F1 test suite passes on 18 new + 20 old fixtures, but the 7 new operation tests are in fixtures being repaired.

### F2: Type Name Limitations in Error Messages
**Severity:** Low
`type_name_static` maps to `'static str`, limiting error expressiveness:
```rust
fn type_name_static(name: &str) -> &'static str {
    match name {
        "null" => "null",
        "boolean" => "boolean",
        // ...
        _ => "unknown",
    }
}
```
Future: Support dynamic type names (e.g., `typeof($slot_name)`) for richer error messages.

### F3: Forward Reference Validation Not Explicitly Tested
**Severity:** Low
Forward references (referencing step outputs not yet defined) should be caught by name validation. Current behavior: `validate_canonical_references` checks `resolve_slot_reference` which returns `UnknownReferenceName` for unresolvable names. No explicit forward-reference test fixture exists.

### F4: Do Action Output Type Inference
**Severity:** Low
Do action outputs are inferred as `ExprType::Unknown` (canonical_scalar.rs line 78). This is correct (action outputs are dynamic), but prevents static type checking of expressions that depend on Do action outputs. Consider: type hints or capability-based inference for known action outputs.

## Test Coverage Assessment

- **Root test suite:** 18 new + 20 old = 38 PASS in 2.1s (native)
- **7 new type operation tests:** Fixtures being repaired, not yet included in root run
- **Parent F1:** Bounded scope, no full source coverage acceptance

## Recommendations

1. **Complete 7 new type operation test fixtures** before accepting this change. These should verify:
   - `true + 2` → TypeMismatch (boolean + i64)
   - `null.length()` → TypeMismatch (null.length undefined)
   - `2 + null` → TypeMismatch (i64 + null)
   - `true * 2` → TypeMismatch (boolean * i64)
   - `"hello".contains("x")` → TypeMismatch (text not supported in expressions)
   - `$bool_slot + 1` → TypeMismatch (referenced bool + i64)
   - `$f64_slot.length()` → TypeMismatch (f64.length undefined)

2. **Add forward reference test** verifying unknown name error precedes type error.

3. **Consider Do action type hints** for improved static checking of action-dependent expressions.

4. **Document I64SetConst preservation** explicitly in change log for backward compatibility.

## Acceptance Criteria for Future Review

- All 7 new type operation tests pass
- Forward reference validation test added
- Full parent F1 suite passes (38 + 7 = 45 tests)
- This change is bounded to expression types; full coverage acceptance requires all F1 tests

