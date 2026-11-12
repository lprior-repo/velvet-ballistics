# Contract: `together` Primitive Acceptance

## Contract Clause: vb-xi2f.36

> **WHEN** a YAML workflow document containing the key `together` is submitted to the parse/validate/compile pipeline, **THEN** the pipeline MUST recognize `together` as a valid step primitive and produce a valid compiled workflow — **AND** when malformed, MUST return a diagnostic referencing `together` (not `parallel`) as the canonical key name.

## Preconditions

1. YAML bytes contain a map with a `together` key
2. The value of `together` is a map containing a `branches` array
3. Each branch in `branches` has a non-empty `label` and non-empty `steps`

## Postconditions

| # | Condition | Evidence |
|---|-----------|----------|
| P1 | `is_primitive("together")` returns `true` | `vb_yaml/src/ast/parse_steps.rs:85-102` |
| P2 | `parse_step()` matches `"together"` arm to `parse_parallel()` | `vb_yaml/src/ast/parse_steps.rs:68-83` |
| P3 | `parse_parallel()` returns `Ok(StepPrimitive::Together { branches })` | `vb_yaml/src/ast/parse_steps.rs:192-204` |
| P4 | `STEP_PRIMITIVES` arrays contain `"together"` | 3 files in `vb_validate/src/` |
| P5 | `validate_workflow_schema()` does not reject `together` as unknown field | `vb_validate/src/schema.rs` |
| P6 | `StepPrimitive::from_field("together")` returns `Some(Self::Parallel)` | `vb_compile/src/mod_compile_lowering/part_09.rs` |
| P7 | `StepPrimitive::as_str(Self::Parallel)` returns `"together"` | `vb_compile/src/mod_compile_lowering/part_09.rs` |
| P8 | `lower_together()` successfully lowers `StepPrimitive::Together` to `CompiledNode::TogetherStart` | `vb_compile/src/compile/mod.rs` |

## Error Contract

| Input | Expected Error |
|-------|----------------|
| `together: {}` (missing branches) | `YamlError::FieldShape { field: "together.branches" }` |
| `together: { branches: [] }` | `CompileError::InvalidTogetherShape` or budget error |
| `together: { branches: [{ label: "" }] }` | `YamlError::EmptyString` |
| `together: { unknown: x }` | `YamlError::UnknownField { field: "together.unknown" }` |

## Backward Compatibility Clause

> During the transition window, `parallel` MAY be accepted as an alias for `together` in the YAML parse layer, but ALL diagnostic messages and display strings MUST use the canonical name `together`.

## Refinement Obligations

The following Rust functions are refinement obligations that MUST be verified:

1. **`is_primitive("together") == true`** — pure function property (Kani/Verus)
2. **`parse_step(yaml_with_together)` succeeds with `StepPrimitive::Together`** — parse property (Kani)
3. **`validate_workflow_schema(doc_with_together)` returns `Ok`** — validation property (Kani)
4. **`lower_together(to_together) != Err`** — compile lowering property (Kani)
5. **`StepPrimitive::as_str` for `Together` variant returns `"together"`** — pure function property (Verus)

## Domain Invariants

1. `Together { branches }` where `branches.len() >= 1` always holds after successful parse
2. `TogetherBranch { label, steps }` where `!label.is_empty() && !steps.is_empty()` always holds after successful parse
3. No two branches in the same `Together` may have the same `label` (enforced at compile validation)

## Open Issues

- [ ] Whether `"parallel"` is rejected or accepted as alias after transition
- [ ] Whether compile layer's `StepPrimitive::Parallel` variant is renamed to `Together`
