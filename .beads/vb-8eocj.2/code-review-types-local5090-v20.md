# Independent Local5090 Code Review: vb-8eocj.2 Expression Types v20
**Date:** 2026-10-23
**Reviewer:** LOCAL5090 (Qwen 3.8 27B)
**Workspace:** ~/src/isoloated/vb-8eocj2-types-code-review-local5090-v19
**Revision:** 0cb351c02ab82c070fe3f1c663b6dfe25da95809 (frozen)
**Scope:** canonical_types.rs (241L), canonical_scalar.rs, canonical_references.rs, part_02.rs
**Verdict:** NO CONCRETE PRODUCTION CODE DEFECT FOUND IN STATIC REVIEW; TESTS PENDING

## Architecture Assessment: CORRECT

### Same Canonical Resolver (Primary Requirement)
PASS. Type checking and bytecode compilation use the SAME `resolve_slot_reference`
function from canonical_references.rs:
- canonical_types.rs:82 — `resolve_slot_reference(&reference, outputs, step_outputs)`
- canonical_references.rs:130 — `resolve_slot_reference(reference, &self.outputs, &self.step_outputs)`

This guarantees SlotIdx→ExprType facts resolve through identical lookup logic.

### Original Reference Spelling
PASS. canonical_types.rs:92 preserves exact spelling:
`context.add_variable(reference.clone().into(), ty);`
The `$` prefix and full name are retained, matching bytecode reference handling.

### Name Validation Before Types
PASS. canonical_scalar.rs lines 89-92:
1. `validate_canonical_references(&parsed, outputs, step_outputs)` runs first
2. `infer_canonical_expr_type(&parsed, slot_types, outputs, step_outputs)` runs second

This ensures unknown name errors precede misleading type errors.

## Type System Design: CORRECT

### Supported Literal Types (vb_expr::ExprType)
- `true`/`false` → Bool
- `null` → Null
- `-999999` → I64
- `3.14` → F64
- `"text"` → Text (supported in expressions)

### Type Mismatch Rejection
`CompileError::TypeMismatch { field: "set.value", expected, found }`
- `true + 2` → `expected: "number", found: "boolean"`
- Bool arithmetic (`true * 2`) rejected at compile time
- Numeric operations (`+`, `-`, `*`, `/`) require numeric operands

### I64 Constant Preservation
PASS. canonical_scalar.rs line 51 checks I64 parse before expression compilation:
```rust
if let Ok(constant) = value.parse::<i64>() {
    lower_set_constant(id, slot, constant, next, builder)?;
    return Ok(ExprType::I64);
}
```
Existing `set slot 0 -999999` continues producing `SetConst` shape.

### Transitive Type Copying
PASS. When `set result: $input` is compiled, `$input` resolves to a slot,
the slot type is retrieved, and the result expression infers that same type.
New slot is stored with the inferred type: part_02.rs line 58 `slot_types.insert(slot, inferred)`.

## Producer Step Reference Patterns: CORRECT

| Pattern | Resolution | Type Inference |
|---------|-----------|----------------|
| `$step.id` | step_outputs map | Slot type or Unknown |
| `$steps.id` | step_outputs map | Slot type or Unknown |
| `$output_name` | outputs map | Slot type or Unknown |
| `$step_id` (no prefix) | Both maps, reject ambiguity | Slot type or Unknown |

Ambiguous references (same name in both maps with different slots) are rejected
via `ExpressionLoweringUnsupported` error.

## Error Handling Analysis

| Error | Variant | Scenario |
|-------|---------|----------|
| Unknown name | `UnknownReferenceName` | `$nonexistent_slot` |
| Unknown root | `UnknownReferenceRoot` | `$invalid.foo` |
| Type mismatch | `TypeMismatch` | `true + 2`, `null.length()` |
| Helper arity | `ExpressionHelperArity` | `length(x, y)` |
| Unknown helper | `ExpressionUnexpectedToken` | `foo(x)` |

## Design Notes (Not Defects)

### Do Action Output Type Inference
Do action outputs are inferred as `ExprType::Unknown` (part_02.rs line 74).
This is intentional — action outputs are dynamic and not statically typable.
The design is correct; it does not prevent type checking of known static types.

### Do Action Input Expressions
Do action inputs use only reference resolution (canonical_do.rs:resolve_do_input),
not full expression type checking. This is intentional — Do inputs are single
slot references, not typed expressions. No defect.

### Cold Function Limit
The 25-logical-function hard limit applies to HOT paths only. No violation.

## Test Coverage Status

- **Root test suite:** 20 PASS / 0 FAIL (2.150s, per actual root run)
- **7 new type operation fixtures:** Pending, not yet merged
  - 5 boolean type negative cases (bare producer, $step/$steps producer,
    transitive bool copy, `true + 2`)
  - 2 numeric runtime cases
- Coverage pending is not a production code defect

## Corrected Prior Findings

| Prior Finding | Status | Correction |
|---------------|--------|------------|
| F1: "Type checking not verified" | Resolved | Type checker code is correct; test coverage is pending |
| F2: "Type name limitations" | Not a defect | `type_name_static` maps all supported types; `_ => "unknown"` is intentional |
| F3: "Forward reference validation" | Resolved | Name validation runs before type checking, catches unknown names first |
| F4: "Do action type inference" | Not a defect | Intentionally `Unknown` for dynamic action outputs |

Prior report's "38 PASS", "18 new fixtures", and "45 tests" were fabricated
inferences, not actual observed results. The actual root run is 20 PASS / 0 FAIL.

## Acceptance Criteria for Future Review

This review examines the production code at revision 0cb351c02ab82c070fe3f1c663b6dfe25da95809
and finds no concrete production code defect in the static review. Acceptance of
this change as a complete feature requires:
- 7 new type operation test fixtures merged and passing
- Full integration test suite (root) continuing to pass
- No broader code coverage claims until all F1 tests pass
