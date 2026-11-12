# Boundary Map: `together` Primitive Acceptance

## Module Boundaries

```
[YAML bytes]
    │
    ▼  (parser boundary - vb_yaml)
┌──────────────────────────────────────┐
│ vb_yaml::ast::parse_steps            │
│  - is_primitive()  [PURE]            │
│  - parse_step()     [PARSES YAML]   │
│  - parse_parallel() [PARSES YAML]    │
└──────────────────────────────────────┘
    │ YamlResult<StepAst>
    ▼  (validate boundary - vb_validate)
┌──────────────────────────────────────┐
│ vb_validate::schema                 │
│  - STEP_PRIMITIVES    [CONST DATA]  │
│  - validate_workflow_schema()       │
│  - validate_step_fields()           │
└──────────────────────────────────────┘
    │ ValidationResult<()>
    ▼  (compile boundary - vb_compile)
┌──────────────────────────────────────┐
│ vb_compile::compile                  │
│  - StepPrimitive::from_field()       │
│  - lower_together()                  │
│  - compile_together_start()         │
│  - compile_together_branch()         │
│  - compile_together_join()           │
└──────────────────────────────────────┘
    │ CompiledWorkflow
```

## Boundary Characterizations

| Boundary | Character | Risk |
|----------|-----------|------|
| YAML → `vb_yaml` | Parser/Codec | Hostile input, malformed YAML |
| `vb_yaml` → `vb_validate` | Type validation | Illegal state prevention |
| `vb_validate` → `vb_compile` | Semantic validation | Shape validation, budget check |
| `vb_compile` → `vb_core` | IR lowering | Correctness of compiled representation |

## Files Requiring Changes

### `vb_yaml/src/ast/parse_steps.rs`

| Line | Change | Boundary |
|------|--------|----------|
| 74 | Add `"together" => parse_parallel(sub)` match arm | Parser |
| 85-102 | Add `"together"` to `is_primitive()` | Parser |

### `vb_validate/src/schema.rs`

| Line | Change | Boundary |
|------|--------|----------|
| 43 | Replace `"parallel"` with `"together"` in `STEP_PRIMITIVES` | Validation |

### `vb_validate/src/schema_fields.rs`

| Line | Change | Boundary |
|------|--------|----------|
| 39 | Replace `"parallel"` with `"together"` in `STEP_PRIMITIVES` | Validation |

### `vb_validate/src/schema/validation.rs`

| Line | Change | Boundary |
|------|--------|----------|
| 37 | Replace `"parallel"` with `"together"` in `STEP_PRIMITIVES` | Validation |

### `vb_compile/src/mod_compile_lowering/part_09.rs`

| Line | Change | Boundary |
|------|--------|----------|
| 24 | Add `"together" => Some(Self::Parallel)` to `from_field()` | Compile |
| 43 | `Self::Parallel => "together"` in `as_str()` | Compile |

### `vb_compile/src/compile/mod.rs`

| Line | Change | Boundary |
|------|--------|----------|
| 210 | `Together { .. } => "parallel"` → `Together { .. } => "together"` | Compile |

### `vb_compile/src/mod_compile_lowering/part_05.rs`

| Line | Change | Boundary |
|------|--------|----------|
| 105 | `Together { .. } => "parallel"` → `Together { .. } => "together"` | Compile |

## PURE FUNCTIONS (no I/O, no time, no storage)

- `is_primitive()` — purely checks string membership
- `StepPrimitive::as_str()` — purely returns static string
- `StepPrimitive::from_field()` — purely maps string to enum
- `validate_workflow_schema()` — purely validates structure
