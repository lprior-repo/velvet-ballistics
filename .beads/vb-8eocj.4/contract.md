# Rust Contract: Declared Source Input Lexical References in Set/Do Expressions

**Bead:** vb-8eocj.4
**Crate:** vb_compile (mod_compile_lowering)
**IR:** Unchanged (CompiledWorkflow structure stable)
**Allocator:** Reuses existing SlotCompiler; no new runtime string lookup

## Domain Problem

Declared source inputs (e.g., `$input.user_id`) are currently resolvable only in the
Finish step's result slot via `prepare_finish_input_slots`. Set expressions and Do
step inputs cannot reference declared source inputs because input slots are allocated
after step lowering, leaving the `outputs` map empty for input names during compilation.

## Existing Types and Integration Paths

| Type/Path | Crate | Role |
|-----------|-------|------|
| `WorkflowSource.inputs()` | vb_yaml | Declared `InputField` list |
| `SlotCompiler.allocate_input_slots()` | vb_compile | Maps inputs to `SlotIdx` |
| `StepReferenceResolver` | vb_compile | Resolves `$name` refs during lowering |
| `resolve_slot_reference()` | vb_compile | Core dollar/numeric ref resolution |
| `ParsedExpression::Reference` | vb_compile | AST node for `$input.*` refs |
| `infer_canonical_expr_type()` | vb_compile | Static type check of Set expressions |

## Integration Facts

- Input slots are allocated in `compile_source` (part_01.rs) via `allocate_input_slots`.
- The `outputs` HashMap (name -> SlotIdx) is what Set/Do lowering consults via `StepReferenceResolver`.
- `prepare_finish_input_slots` currently inserts `$input.*` names into `outputs`, but only for Finish.
- `prior_types` HashMap (name -> ExprType) drives static type checking; input types flow from `kind_from_input_schema`.

## Contract: Input Slots Must Be Resolvable in Set/Do

### Precondition
`compile_source` allocates input slots for all declared `InputField` values BEFORE step lowering begins.

### Obligation
Each declared input `<name>` with slot `<slot>` must:

1. Insert `$input.<name>` -> `<slot>` into the `outputs` map passed to `lower_canonical_step`.
2. Be resolvable by `resolve_slot_reference` in Set expressions (`lower_canonical_set_expression`).
3. Be resolvable by `resolve_slot_reference` in Do inputs (`lower_canonical_do` -> `resolve_do_input`).
4. Contribute typed facts to `prior_types` via `kind_from_input_schema` for static type checking.

### Error Cases

| Scenario | Existing Error Variant |
|----------|----------------------|
| Unknown input ref: `$input.missing` | `UnknownReferenceName { kind: "output", name: "input.missing" }` |
| Ambiguous ref: step and input share name | `ExpressionLoweringUnsupported` (existing ambiguity path) |
| Invalid input type in expression | `TypeMismatch { field: "set.value", ... }` |
| Non-string input schema | `InvalidInputSchema` from `kind_from_input_schema` |

## Resource Bounds

- One `SlotIdx` per declared input (u16, bounded by workflow input count).
- One `$input.<name>` -> SlotIdx entry in `outputs` HashMap per input.
- One prior type entry per input in `prior_types`.
- No runtime string lookup; slot indices resolved at compile time.
- Memory: O(input_count) additional HashMap entries, released after compilation.

## Acceptance Fixture

```yaml
# Declared source input
inputs:
  - key: user_id
    value:
      type: i64

steps:
  - id: s1
    set:
      output: uid
      value: "$input.user_id + 1"
  - id: s2
    do:
      action: "1"
      input: "uid"
  - id: finish
    finish:
      result: "$s2"
```

**Acceptance:**
1. `$input.user_id` resolves to input slot in Set expression at step s1.
2. Set output `uid` inferred as `ExprType::I64` from input schema + i64 literal.
3. Do step s2 resolves `uid` via existing step output resolution.
4. Compilation succeeds; `CompiledWorkflow` has input slot allocated before step nodes.
5. `unknown input ref` rejected at compile time with `UnknownReferenceName`.

## Missing Critical Domain Facts

- Input slot type inference relies on `kind_from_input_schema`; schema-less inputs may
  yield `Unknown` type (no false precision). Documented behavior, not a defect.
- Cross-step input references (e.g., using input in two Set expressions) are supported
  by the HashMap-based `outputs` map; no additional state needed.
