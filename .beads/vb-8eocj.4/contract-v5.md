# vb-8eocj.4 — Source-Map Local Input Resolution (Contract v5)

Status: proposed, not implemented.

## What Changed Since v4

v4 named three non-existent symbols: `DeclaredInputAllocator` (type),
`resolve_slot()` (method), and `UnknownInputSlot` (error variant).
This contract replaces them with the actual free functions and variants
present in the codebase. No new types are invented.

## Actual Existing Symbols (Verified)

| Role | Actual Symbol | Location |
|------|---------------|----------|
| Input slot allocation | `allocate_input_slots(inputs, builder)` free function | `part_01.rs:95` |
| Reference resolution | `resolve_slot_reference(ref, outputs, step_outputs)` free function | `canonical_references.rs:47` |
| Bytecode resolver adapter | `StepReferenceResolver` (wraps above) | `canonical_references.rs:25` |
| Unknown input error | `CompileError::UnknownReferenceName { kind: "input", reference, name }` | `kind.rs:138` |
| Schema error | `CompileError::InvalidInputSchema { field, expected }` | `kind.rs:82` |
| Type error | `CompileError::TypeMismatch { field, expected, found }` | `kind.rs:148` |
| Input kind mapping | `kind_from_input_schema(value)` free function | `input_schema.rs:9` |
| Do input resolution | `resolve_do_input(expr, outputs, step_outputs, index)` | `canonical_do.rs:57` |
| Finish input prep | `prepare_finish_input_slots(...)` | `finish_inputs.rs:35` |

## Binding Order

1. **Step lowering first.** All `Set`/`Do`/`ForEach`/... steps are lowered
   before input slots are allocated. This establishes the baseline slot
   range for step outputs.

2. **Source slot reservation.** Numeric source operands (`"1"`, `"2"`, ...)
   and legacy `"done"` are resolved through the same
   `resolve_slot_reference` mechanism. No separate "reserve" phase;
   numeric references simply resolve to their raw slot indices.

3. **Input allocation exactly once.**
   - For non-finish workflows: `allocate_input_slots(source.inputs(), builder)`
     runs after all steps (part_01.rs:70).
   - For finish workflows with `$input.` references:
     `prepare_finish_input_slots` runs `allocate_input_slots` inside the
     finish-lowering path (part_01.rs:47, finish_inputs.rs:47).
   - One allocation pass. No parallel allocator, no private counter,
     no `DeclaredInputAllocator` type.

4. **Input name registration.** After allocation, `prepare_finish_input_slots`
   inserts `$input.<name>` entries into the `outputs` HashMap
   (finish_inputs.rs:52-56). The SAME `resolve_slot_reference` resolver
   then handles both step outputs and input references — there is no
   separate input resolver.

## Type Mapping (Actual)

`kind_from_input_schema` in `input_schema.rs` maps declared schemas to
`InputSlotKind` (not `ExprType`):

| Declared `is:` | InputSlotKind |
|---------------|---------------|
| `text` | `Symbol` |
| `number` | `I64` |
| `boolean` | `Bool` |
| `object` | `Object` |
| `any` | `Any` |
| `list` (shorthand) | `List` |
| `list<any>` / `list<text>` / `list<number>` / `list<boolean>` | `List` |

`ExprType` is used separately by the vb_expr type checker for Set expressions.
The mapping from `InputSlotKind` to runtime `ExprType` is not in this crate.

## Input Reference Forms

| YAML Syntax | Resolver Behavior |
|-------------|-------------------|
| `$input.payload` | Resolves through `outputs` HashMap after allocation |
| `input.payload` (no `$`) | NOT currently a reference; literal string |
| `$steps.do_process` | Step output resolution (existing) |
| `1` (numeric) | Raw slot index via `resolve_numeric_slot` |
| `$slot.1` | Compile-specific slot reference (references.rs) |

Direct Do input resolution: `resolve_do_input` calls `resolve_slot_reference`
directly (canonical_do.rs:57). No wrapper, no separate Do resolver.

## Error Contract (Actual Variants)

**Unknown input name:**
```
CompileError::UnknownReferenceName {
    kind: "input",
    reference: Box::from("$input.unknownfield"),
    name: Box::from("unknownfield"),
}
```
This is produced by `references.rs` via `map_validation_error` when the
`input` namespace name is not declared, OR by the compile-time resolver
when `$input.name` is not in the `outputs` HashMap after allocation.

**Malformed input schema:**
```
CompileError::InvalidInputSchema {
    field: "inputs",
    expected: "an allowed input schema",
}
```

**Boolean arithmetic type error:**
```
CompileError::TypeMismatch {
    field: "expression",
    expected: "numeric",
    found: "boolean",
}
```
Produced by vb_expr type checker, mapped via `canonical_types.rs`.

## Acceptance Cases (Source Map Local)

Given workflow:
```yaml
inputs:
  payload: { is: number }
steps:
  - id: increment
    set: { value: "$input.payload + 1", output: "incremented" }
  - id: do_process
    do: { action: "1", input: "$input.payload" }
  - id: report
    finish: "$steps.do_process"
```

1. **Set reads input:** `increment` evaluates `$input.payload + 1` -> `11`
   when payload is bound to `10` at runtime.
2. **Do uses input directly:** `do_process` receives slot for `$input.payload`
   (value `10`). Produces result `99`.
3. **Finish uses step reference:** `report` finishes with `$steps.do_process`
   result (`99`).
4. **Unknown input name:** `$input.missing` -> `UnknownReferenceName { kind: "input", ... }`
5. **Malformed schema:** `is: widget` -> `InvalidInputSchema`
6. **Boolean arithmetic:** `value: "$input.flag + 1"` where flag is boolean
   -> `TypeMismatch` from vb_expr type checker.
7. **Legacy numeric sources:** `input: "1"` still resolves as raw slot index.
8. **One SlotIdx per input name:** `payload` allocates exactly one slot.

## What Is NOT Changed

- Step output resolution (`$steps.<id>`, bare `$name`) — unchanged.
- Numeric slot references — unchanged.
- `$slot.<n>` compile-specific references — unchanged.
- `vb_expr` evaluation engine — unchanged.
- `vb_runtime` execution — unchanged.
- IR structure (CompiledWorkflow, ExprProgram, etc.) — unchanged.

## Open Semantics (Not Contracted Here)

- How `ExprType::Symbol` maps from `InputSlotKind::Symbol` at runtime.
- How `ExprType::Unknown` (dynamic action outputs) interacts with input
  slot types during Set evaluation.
- Whether `list<T>` parametric types are representable in the current
  expression type system (currently only `List` kind, no element type).
