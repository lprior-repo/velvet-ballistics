# vb-8eocj.4 — Source-Map Local Variable Resolution (Contract v4)

Status: proposed, not implemented.

## Scope

Lexical resolution of `input.<name>` references through the existing
`DeclaredInputAllocator`, `SlotIdx` resolver, and `ExprType` system.
Covers `Set`, `Do`, and `Finish` input expressions. Bounded; excludes
`var` declaration, secrets, deep nesting, and F1 closure capture.

## Binding Order (F2 Fix)

1. `reserve_source_slots()` runs FIRST: explicit numeric source
   operands (`1`, `2`, ...) and legacy IDs (`done`) map to their
   protected slots before any name resolution.
2. `DeclaredInputAllocator` then maps `$input.<name>` -> new `SlotIdx`
   using `input_slots` from `workflow.input_slots` metadata. One
   allocation per name; no parallel allocator or private counter.
3. `resolve_slot()` reuses the SAME resolver for all expressions;
   finish input selection uses the same resolver, no prefixed map
   key compat bug.

## Runtime Fixture

```yaml
input_slots:
  payload: { is: number }
steps:
  - set: { value: "$input.payload + 1", output: "incremented" }
  - do: { action: "1", input: "$input.payload" }
  - finish: "$action_step"
```

Runtime binds `payload: 10`. `Do` observes `10`. `Set` computes `11`.
Completed `Do` result `99` flows to `Finish`. IR unchanged.

## Type Mapping (`slot_types`)

Uses `SlotIdx` facts and exact schema mapping:
- `is: number` -> `ExprType::I64`
- `is: boolean` -> `ExprType::Bool`
- `is: text` -> `ExprType::Text` (if runtime Symbol mapping supported)
- `is: any` -> `ExprType::Unknown`
- `is: object`/`is: list` -> proper `ExprType` (composite)

List shorthand `list<number>` is valid YAML; not all shorthands are
valid longform `is:` types. Read the actual helper for supported forms.

## Namespace and Collision

`input.user_id` and bare `user_id` are distinct namespaces; not
ambiguous just because leaf names match. Exact duplicate alias
protection is `DuplicateOutputName` for `canonical_set_output_slot`.
Cross-namespace ambiguity only if the actual resolver detects two
different slots for one reference. No false error variant for
leaf-only collision.

## Error Variants

- `UnknownInputSlot` — full kind, output, reference, name
- `TypeMismatch` — e.g., boolean arithmetic
- `InvalidInputSchema` — malformed `input_slots` definition

## Acceptance Cases

1. `Set` reads named input: `$input.payload + 1` evaluates
2. `Do` uses DIRECT named input: `input: "$input.payload"`
3. `Finish` references named input: `finish: "$input.payload"`
4. Unknown slot errors with full kind/output/reference/name
5. Malformed schema errors as `InvalidInputSchema`
6. Boolean arithmetic errors as `TypeMismatch`
7. Legacy numeric sources (`1`, `2`) and `done` still bind
8. No duplicate input allocation; one `SlotIdx` per name

## Out of Scope

`var` declarations, secrets injection, nested expressions, F1
closure capture, cross-workflow references.
