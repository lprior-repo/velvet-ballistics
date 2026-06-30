# Domain Model: `together` Primitive Acceptance

## Ubiquitous Language

| Term | Canonical Name | Aliases | Definition |
|------|---------------|---------|------------|
| Parallel fan-out primitive | `together` | `parallel` (deprecated) | Concurrent branch execution construct |
| Branch list | `TogetherBranch` | — | Named sequence of steps executed concurrently |
| Fan-out | `branches` | — | Field containing `TogetherBranch` array |
| IR variant | `StepPrimitive::Together` | — | AST node for the together construct |

## Domain Entities

### StepPrimitive::Together
- **Type**: `enum StepPrimitive` variant in `vb_yaml/src/ast/types.rs`
- **Fields**: `branches: Vec<TogetherBranch>`
- **Invariant**: `branches.len() >= 1` (enforced at parse time)
- **Forbidden states**:
  - Empty `branches` vector
  - `branches` containing empty `steps` vector
  - Duplicate branch `label` values

### TogetherBranch
- **Type**: `struct TogetherBranch` in `vb_yaml/src/ast/types.rs`
- **Fields**: `label: String`, `steps: Vec<StepAst>`
- **Invariant**: `!label.is_empty()`, `!steps.is_empty()`

## Value Objects

| Value Object | Type | Invariant |
|-------------|------|-----------|
| BranchLabel | `String` | Non-empty, alphanumeric + underscore |
| BranchCount | `usize` | 1..=u16::MAX |

## Relationships

```
WorkflowDoc
  └── steps: Vec<StepAst>
        └── primitive: StepPrimitive::Together { branches: Vec<TogetherBranch> }
                                                   └── TogetherBranch { label, steps }
```

## Key Domain Decision

**"together" is the canonical YAML key name.** The IR uses `StepPrimitive::Together`. The string "parallel" is a legacy alias that must be rejected after the fix is deployed, or accepted as a backward-compatible alias during a transition window.

Current defect: YAML parser `is_primitive()` in `vb_yaml/src/ast/parse_steps.rs:85-102` and `STEP_PRIMITIVES` arrays in `vb_validate/src/schema.rs`, `vb_validate/src/schema_fields.rs`, `vb_validate/src/schema/validation.rs` only list `"parallel"` — they do not recognize the canonical `"together"` key.

## Open Domain Questions

1. Should `"parallel"` be accepted as a backward-compatible alias for `"together"` during a transition period, or rejected immediately as a breaking change?
2. Should the compile layer's `StepPrimitive::Parallel` variant be renamed to `StepPrimitive::Together` for consistency with the IR, or kept as-is for diff minimization?
