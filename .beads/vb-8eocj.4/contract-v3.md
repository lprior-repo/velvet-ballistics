# Rust Contract: Declared Source Input Lexical References in Set/Do Expressions

**Bead:** vb-8eocj.4
**Crate:** vb_compile (mod_compile_lowering)
**IR:** Unchanged (CompiledWorkflow structure stable)
**Allocator:** Existing free function `allocate_input_slots` in part_01.rs
## Domain Problem

Declared source inputs (`$input.user_id`) resolve in Finish steps only because
`prepare_finish_input_slots` runs before Finish lowering. For Set expressions and
Do step inputs, `$input.*` references fail because input slots are allocated at the
END of `compile_source` (line 70 of part_01.rs), after all step lowering.

## Current State (v18)

**Allocation path:** `allocate_input_slots` is a free function in part_01.rs called:
1. At end of `compile_source` for normal workflows (line 70)
2. During Finish step via `prepare_finish_input_slots` when Finish references `$input.*`

**Resolver mismatch (existing):** `prepare_finish_input_slots` inserts `"$input.<name>"`
(with `$`) into `outputs`, but `resolve_slot_reference` strips `$` and looks up
`"input.<name>"`. The Finish path works only because it uses the special
`resolve_finish_input_slot` function that looks up the name with the `$` prefix intact.

**Type map:** `slot_types: HashMap<SlotIdx, ExprType>` (not `prior_types`).
Input slots are not currently added to `slot_types`.

## Proposed State (vb-8eocj.4)

**Allocation path:** Call `allocate_input_slots` BEFORE step lowering begins, then
insert names into `outputs` WITHOUT the `$` prefix so `resolve_slot_reference` can
find them via the standard name-resolution path.

**Type map:** Populate `slot_types` with each input slot's `ExprType` derived from
its schema via `kind_from_input_schema` → `InputSlotKind` → `ExprType` mapping.

## Contract

### Precondition
`compile_source` allocates input slots and populates `outputs`/`slot_types` before
lowering any steps.

### Obligation
Each declared input `<name>` with allocated slot `<slot>` must:

1. Insert `"input.<name>"` (without `$`) -> `<slot>` into `outputs` map.
2. Be resolvable by `resolve_slot_reference` in Set expressions.
3. Be resolvable by `resolve_slot_reference` in Do inputs (already supported).
4. Contribute `<slot>` -> `<ExprType>` to `slot_types` for static type checking.
5. Not conflict with existing step output names (collision = error).

### Name Resolution Rules
- `$input.<name>` → resolver strips `$` → looks up `"input.<name>"` in `outputs` → slot
- If `"input.<name>"` also exists as a step output name → `ExpressionLoweringUnsupported`
- If `"input.<name>"` not in `outputs` → `UnknownReferenceName { kind: "output", ... }`

### Error Cases

| Scenario | Error Variant |
|----------|---------------|
| Unknown input ref: `$input.missing` | `UnknownReferenceName { kind: "output", reference: "$input.missing", name: "input.missing" }` |
| Ambiguous: step and input share `name` | `ExpressionLoweringUnsupported` |
| Invalid input schema type | `InvalidInputSchema { field: "inputs" }` |
| Non-string input schema value | `InvalidInputSchema` |

### Input Schema Types (valid values)

Shorthand: `text`, `number`, `boolean`, `object`, `any`, `list<any>`, `list<text>`,
`list<number>`, `list<boolean>`

Longform: `{ is: "<type>" }` where `<type>` is any shorthand above.

## Resource Bounds

- One `SlotIdx` per declared input (u16, bounded by workflow input count).
- One `"input.<name>"` -> `SlotIdx` entry in `outputs` HashMap per input.
- One `<slot>` -> `ExprType` entry in `slot_types` HashMap per input.
- No runtime string lookup; slot indices resolved at compile time.
- Memory: O(input_count) additional HashMap entries, released after compilation.

## Acceptance Fixture

```yaml
version: velvet-ballistics/v1
name: source_input_ref
when:
  manual: {}
inputs:
  user_id: text
  multiplier: number
steps:
  - id: s1
    set:
      output: uid
      value: "$input.user_id"
  - id: s2
    set:
      output: product
      value: "$input.multiplier * 2"
  - id: s3
    do:
      action: "1"
  - id: finish
    finish:
      result: "$s3"
```

**Acceptance:**
1. `$input.user_id` resolves to input slot in Set expression at step s1.
2. `$input.multiplier` resolves in Set expression at step s2.
3. Do step s3 resolves `uid` via existing step output resolution.
4. Compilation succeeds; input slots allocated before step lowering.
5. `$input.missing` rejected at compile time with `UnknownReferenceName`.

## Bounded Scope

This contract covers declared source input resolution in Set expressions, Do inputs,
and Finish steps. It does not cover runtime value binding, input validation, or
runtime `$input` lookup. It reuses existing compilation infrastructure without
introducing new maps, allocators, or runtime string lookup.
