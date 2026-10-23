# Independent Local3090 Code Review: vb-8eocj.2 Expression Types v22
**Date:** 2026-10-23
**Reviewer:** LOCAL3090 (Qwen 3.8 27B)
**Workspace:** ~/src/isoloated/vb-8eocj2-types-code-review-local5090-v19
**Revision:** 0cb351c02ab82c070fe3f1c663b6dfe25da95809 (frozen)
**Scope:** canonical_types.rs (241L), canonical_scalar.rs, canonical_references.rs, part_02.rs, canonical_do.rs
**Verdict:** NO CONCRETE PRODUCTION CODE DEFECT FOUND IN STATIC REVIEW; TESTS PENDING

## Source Pins

| File | Lines | Fact |
|------|-------|------|
| canonical_types.rs | 82 | `resolve_slot_reference(&reference, outputs, step_outputs)` used by type checker |
| canonical_references.rs | 130 | `resolve_slot_reference(reference, &self.outputs, &self.step_outputs)` used by bytecode compiler |
| canonical_types.rs | 92 | `context.add_variable(reference.clone().into(), ty)` preserves original spelling |
| canonical_scalar.rs | 89-92 | `validate_canonical_references` runs before `infer_canonical_expr_type` |
| canonical_scalar.rs | 51 | `if let Ok(constant) = value.parse::<i64>()` preserves I64 SetConst shape |
| part_02.rs | 58 | `slot_types.insert(slot, inferred)` stores inferred type |
| part_02.rs | 74 | Do action outputs not added to slot_types (intentionally Unknown) |
| canonical_references.rs | 62-66 | `UnknownReferenceRoot` thrown only when `strip_prefix('$')` fails |
| canonical_references.rs | 104-108 | `UnknownReferenceName` thrown for unknown names in valid references |

## Factual Corrections to Prior Report (v20)

### Correction 1: Error Variant for `$invalid.foo` (Material)
**v20 claim:** `$invalid.foo` → `UnknownReferenceRoot`
**Actual:** `$invalid.foo` → `UnknownReferenceName`

Analysis of `resolve_dollar_reference`:
- Reference starts with `$`, so `strip_prefix('$')` succeeds → body = `invalid.foo`
- `split_once('.')` gives root=`invalid`, tail=`foo`
- Root is neither "step" nor "steps", so falls through to `resolve_direct_name`
- `resolve_direct_name` looks for `invalid.foo` (whole string) in both maps
- If not found, throws `UnknownReferenceName { kind: "output", ... }`

`UnknownReferenceRoot` is thrown ONLY when the reference lacks the `$` prefix entirely (line 62-66).
This is a material correction to the error handling table in v20.

### Correction 2: Test Receipt Evidence (Material)
**v20 claim:** "7 new type operation fixtures: Pending, not yet merged"
**Actual:** 7 new type fixtures (5 bool negatives, 2 numeric) PASSED in combined native integration run.

Evidence: `/home/lewis/src/isoloated/velvet-ballistics-vb-8eocj2-source-native-integration-v1/.beads/vb-8eocj/combined-native46-v20/gate.stdout`
- Receipt digest: `a3c7e4f44a8624324a27873f1af626119f23e53b816f665b9ad2e4bb8dcc0b6a` (70 lines)
- Root combined commit: `dfa8901123dd7803e536be03aa56eaa7629a5126`
- Total: 46 tests passed (27 in vb_f1_canonical_scalar_do.rs, 19 in vb_f2_source_slot_preservation.rs)
- 7 new type identity/negative tests: all PASS

### Correction 3: Scope of Proof (Clarification)
The 46-test run proves:
- 27 scalar type tests (18 positive + 9 negative) including 7 new type fixtures
- 19 composite slot preservation tests (for_each, together, reduce)

The run does NOT prove full F1 acceptance. Remaining F1 sub-features:
- `null.length()` type checking (null type not yet supported in expressions)
- Helper call type checking (e.g., `length(x, y)` arity validation in type system)
- Other expression type error paths not yet covered by test fixtures

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

## Error Handling Analysis (Corrected)

| Error | Variant | Scenario |
|-------|---------|----------|
| Unknown name | `UnknownReferenceName` | `$nonexistent_slot`, `$invalid.foo` |
| Unknown root | `UnknownReferenceRoot` | Reference lacks `$` prefix entirely |
| Type mismatch | `TypeMismatch` | `true + 2` |
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

## Test Coverage Status (Corrected)

- **Root combined native run:** 46 PASS / 0 FAIL (receipt a3c7e4f44a8624324a27873f1af626119f23e53b816f665b9ad2e4bb8dcc0b6a)
- **7 new type operation fixtures:** PASS (5 bool negatives + 2 numeric)
- **Remaining F1 sub-features (untested/unproven):**
  - `null.length()` type checking (null type not yet supported in expressions)
  - Helper call type checking (e.g., `length(x, y)` arity validation in type system)
  - Other expression type error paths not yet covered by test fixtures

## Corrected Prior Findings

| Prior Finding | Status | Correction |
|---------------|--------|------------|
| F1: "Type checking not verified" | Partially resolved | Type checker code is correct; 7 new fixtures PASS; null.length and helper call type checking remain unproven |
| F2: "Type name limitations" | Not a defect | `type_name_static` maps all supported types; `_ => "unknown"` is intentional |
| F3: "Forward reference validation" | Resolved | Name validation runs before type checking, catches unknown names first |
| F4: "Do action type inference" | Not a defect | Intentionally `Unknown` for dynamic action outputs |

## Acceptance Criteria for Future Review

This review examines the production code at revision 0cb351c02ab82c070fe3f1c663b6dfe25da95809
and finds no concrete production code defect in the static review. The 7 new type
fixtures are confirmed to PASS via the combined native integration run. Acceptance
of this change as a complete feature requires:
- Full F1 test coverage including `null.length()` and helper call type checking
- Full integration test suite (root) continuing to pass
- No broader code coverage claims until all F1 tests pass
