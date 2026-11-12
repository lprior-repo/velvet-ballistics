# Error Taxonomy: `together` Primitive

## Parse Errors (`vb_yaml` crate)

All parse errors live in `vb_yaml/src/error.rs` and are typed `YamlError`.

### Malformed Together

| Error Variant | Condition | Code Path |
|--------------|-----------|-----------|
| `UnknownField { field: "together" }` | `is_primitive("together")` returns `false` | `parse_step()` line 81 |
| `FieldShape { field: "together.branches" }` | `branches` field missing or wrong type | `parse_parallel()` |
| `FieldShape { field: "together.branches[].label" }` | label empty or missing | `parse_parallel()` |
| `FieldShape { field: "together.branches[].steps" }` | steps empty or missing | `parse_parallel()` |

**Root cause**: `is_primitive()` at `parse_steps.rs:85-102` does not include `"together"`.

### Related: Parallel Alias

| Error Variant | Condition |
|---------------|-----------|
| `UnknownField { field: "parallel" }` | Currently `is_primitive("parallel")` returns `true`, but `"parallel"` is deprecated |

## Validation Errors (`vb_validate` crate)

All validation errors are `ValidationError` variants.

### Unknown Primitive

| Error | Condition | Code |
|-------|-----------|------|
| `UnknownField { field: "together" }` | `STEP_PRIMITIVES` does not contain `"together"` | `validate_unknown_fields()` |

**Root cause**: `STEP_PRIMITIVES` arrays in `schema.rs`, `schema_fields.rs`, `validation.rs` list `"parallel"` not `"together"`.

### Duplicate Field

| Error | Condition |
|-------|-----------|
| `DuplicateField { field: "together" }` | Two `together` keys in same step map |

## Compile Errors (`vb_compile` crate)

All compile errors are `CompileError` variants in `vb_compile/src/mod_compile_errors/`.

### Fan-out Limit

| Error | Condition | Code |
|-------|-----------|------|
| `TogetherBranchLimitExceeded { limit, actual }` | `branches.len() > budget.max_together_branches` | `lower_together()` |

### Shape Validation

| Error | Condition | Code |
|-------|-----------|------|
| `InvalidTogetherShape` | `branches` is empty | `validate_together_shape()` |

## Error Hierarchy

```
YamlError
├── UnknownField { field: "together" }     ← parse boundary
├── FieldShape { field: "together.branches" }
├── MissingField { field: "together" }
└── ...

ValidationError
├── UnknownField { field: "together" }     ← validate boundary
├── DuplicateField { field: "together" }
└── ...

CompileError
├── TogetherBranchLimitExceeded            ← compile boundary
├── InvalidTogetherShape
└── ...
```

## Symbolic Diagnostics

When `together` is malformed, the error message should reference `"together"` (not `"parallel"`):

- `vb_cli/src/app_impl.rs:4793` currently says: `"The 'together' (parallel) construct is invalid."` — this is a diagnostic output that correctly mentions both names during transition.
