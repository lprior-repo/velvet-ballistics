# Error Taxonomy — vb-xi2f.34: Finish Digest Semantics

**Bead**: vb-xi2f.34
**Date**: 2026-05-24
**Rust Contract Agent**: rust-contract

---

## Error Layers

The digest computation and finish lowering span three error domains:

```
YAML Parse Errors  →  Compile Errors  →  Workflow Errors
   (vb_yaml)           (vb_compile)        (vb_core)
```

---

## Layer 1: YAML Parse Errors (`vb_yaml`)

These occur BEFORE digest computation. The digest function never sees unparseable input.

| Error | Trigger | Digest Relevance |
|---|---|---|
| YAML syntax error | Malformed YAML text | None — digest is never computed |
| Schema validation error | Missing required fields, wrong types | None — digest is never computed |
| ScalarValue parse error | Invalid scalar type in YAML | None — ScalarValue is already parsed |

---

## Layer 2: Compile Errors (`vb_compile::CompileError` / `CompileErrors`)

These can occur DURING or AFTER digest computation in the `compile_source()` path.

### Errors that occur AFTER digest computation

Digest is computed at the start of `compile_source()`. These errors happen during lowering:

| Variant | Trigger | Digest is valid? |
|---|---|---|
| `StepFieldShape` | Invalid field type, wrong position (e.g., Finish not last) | Yes — digest already computed from AST |
| `SlotIndexOutOfRange` | Slot index exceeds `u16::MAX` | Yes — digest used `ScalarValue`, not slot index |
| `UnknownOutputName` | Finish references unknown output name | Yes — digest hashed the String name |
| `UnsupportedConstantValue` | Unsupported `ScalarValue` variant in `canonical_finish_slot` | Yes — digest hashed `"unsupported"` |
| `UnsupportedStepPrimitive` | Step primitive not handled by compiler path | Yes — digest used `canonical_primitive_name()` |
| `StepIndexOutOfRange` | Step count exceeds u16 | No — digest already computed successfully |
| `DuplicateOutputName` | Duplicate output in Set steps | Yes — digest already computed |
| `EmptySteps` | Workflow has no steps | No — digest computation not reached |
| `UnsupportedTopLevelDeclaration` | Invalid top-level YAML keys | No — `validate_canonical_compile_scope` fails before digest |

### Key insight: Digest survives compilation failure

The digest is computed from the AST BEFORE any lowering/validation. This means:
- A workflow that fails compilation can still have a deterministic digest.
- The digest reflects what the author *intended*, not what was *validated*.
- Two workflows with identical source that both fail compilation with the same error will have the same digest (assuming identical AST).

### Railway pattern

```
YAML bytes
  → parse → WorkflowSource ──→ canonical_digest() → digest (always succeeds)
  │                              │
  │                              └→ lower/unhandled → CompileErrors
  └→ parse error → YAML error
```

---

## Layer 3: Workflow Errors (`vb_core::WorkflowError`)

These occur after `WorkflowParts` is constructed but before `CompiledWorkflow` is finalized.

| Variant | Trigger | Digest Relevance |
|---|---|---|
| `ValidationError(violations)` | `validate_parts()` finds invalid slot refs, unreachable nodes, etc. | Digest is already set in `WorkflowParts.digest` |
| `EmptyNodes` | No nodes after lowering | Digest is already set |
| Various budget errors | Budget validation fails | Digest is already set |

---

## Error Recovery / Retry

| Scenario | Behavior |
|---|---|
| Invalid YAML → fix source → re-parse → re-digest | Digest changes because source changed |
| Compile error → fix source → re-parse → re-digest | Digest changes because source changed |
| Validation error → this should never happen for a correctly lowered workflow | Indicates a compiler bug; digest is irrelevant |
| Budget violation → increase resource contract → re-compile | Same source → same digest (budget is not digest-covered) |

---

## Error Variants Specific to Finish Digest

### `CompileError::UnknownOutputName`
```
Finish { result: ScalarValue::String("nonexistent") }
```
- **When**: `canonical_finish_slot()` cannot find the output name in the output map.
- **Digest**: Includes `b"finish"` + `"nonexistent".as_bytes()`. The digest is valid and deterministic for this (invalid) source. A different output name would produce a different digest.

### `CompileError::SlotIndexOutOfRange` (from finish)
```
Finish { result: ScalarValue::Integer(99999) }  // exceeds u16::MAX
```
- **When**: `canonical_finish_slot()` tries `u16::try_from(99999)`, which fails.
- **Digest**: Includes `b"finish"` + `99999_i64.to_le_bytes()`. The digest does not depend on whether the slot index is valid.

### `CompileError::UnsupportedConstantValue` (from finish)
```
Finish { result: ScalarValue::SomeFutureVariant { ... } }
```
- **When**: `canonical_finish_slot()` hits the `_` arm for unknown `ScalarValue`.
- **Digest**: `digest_step_primitive()` also hit its `_` arm and wrote `"unsupported"`. The digest does not distinguish WHAT future variant was used — only that it was unsupported.

### `CompileError::StepFieldShape` (finish not last)
```
steps:
  - finish: "result"
  - set: { output: "x", value: "1" }
```
- **When**: `lower_canonical_finish()` checks `index != last`.
- **Digest**: Computed successfully BEFORE the position check. The digest includes the finish step at its actual position (index 0) with its step ID and result value. If the finish step were moved to the last position (index 1), the step order would change and the digest would differ.

---

## Error Classification by Domain

| Error Domain | Variants | Phase |
|---|---|---|
| **Syntax** | YAML parse errors | Pre-compile |
| **Semantic** | `UnknownOutputName`, `DuplicateOutputName`, `UnsupportedStepPrimitive` | Compile (lowering) |
| **Structural** | `StepFieldShape`, `EmptySteps`, `UnsupportedTopLevelDeclaration` | Compile (validation) |
| **Boundedness** | `SlotIndexOutOfRange`, `StepIndexOutOfRange` | Compile (lowering) |
| **Consistency** | `WorkflowError` variants | Post-lowering validation |

---

## Digest Error Surface

The digest function itself has **zero error variants**. It is a pure, infallible function. All errors relating to finish semantics occur in the lowering phase, after the digest is computed. The digest faithfully records what the author wrote, regardless of whether it compiles successfully.
