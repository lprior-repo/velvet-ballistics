# Type Contracts: `together` Primitive

## Boundary: YAML Parser → IR (`vb_yaml` crate)

### `vb_yaml::ast::parse_steps::is_primitive`

**Signature**: `fn is_primitive(field: &str) -> bool`

**Contract**:
- Returns `true` for canonical key `"together"`
- Returns `true` for deprecated alias `"parallel"` (during transition)
- Returns `false` for any other string
- Pure function: no I/O, no mutation

**Current defect**: Only checks `"parallel"`, does NOT check `"together"` (line 95 in `parse_steps.rs`)

### `vb_yaml::ast::parse_steps::parse_step`

**Contract**:
- Input: parsed `saphyr::Yaml` node
- Output: `YamlResult<StepAst>` 
- Exactly one primitive key must be present
- Multiple primitives → `YamlError::FieldShape { field: "step", expected: "exactly one primitive" }`
- Zero primitives → `YamlError::MissingField { field: "step primitive" }`

### `vb_yaml::ast::parse_steps::parse_parallel`

**Contract**:
- Input: `saphyr::Yaml` node with `branches` field
- Output: `YamlResult<StepPrimitive::Together>`
- `branches` field is required
- Each branch must have non-empty `label` and non-empty `steps`
- Empty `branches` array → `YamlError::FieldShape`
- Unknown fields → `YamlError::UnknownField`

### `vb_yaml::ast::StepPrimitive::Together`

**Smart constructor**: `parse_parallel()` is the sole constructor

**Typestate**: Once constructed, the `Together` variant is immutable and structurally valid

## Boundary: YAML → Validate (`vb_validate` crate)

### `STEP_PRIMITIVES` arrays (3 locations)

| Location | Current value | Required value |
|----------|--------------|----------------|
| `vb_validate/src/schema.rs:38-50` | `"parallel"` | `"together"` |
| `vb_validate/src/schema_fields.rs:34-46` | `"parallel"` | `"together"` |
| `vb_validate/src/schema/validation.rs:36-39` | `"parallel"` | `"together"` |

**Contract**: The array defines which YAML keys are recognized as valid step primitives. If `"together"` is not in the list, the YAML validator will reject it as an unknown field.

## Boundary: Compile (`vb_compile` crate)

### `StepPrimitive::from_field` (part_09.rs)

**Current**:
```rust
"parallel" => Some(Self::Parallel),
```

**Required**: Add `"together" => Some(Self::Parallel)` as alias during transition, or rename to `Self::Together`

### `StepPrimitive::as_str` (part_09.rs)

**Current**: `Self::Parallel => "parallel"`

**Required**: `Self::Parallel => "together"` (or rename variant to `Together`)

### `compile/mod.rs` line 210

**Current**: `vb_yaml::ast::StepPrimitive::Together { .. } => "parallel"`

**Required**: `vb_yaml::ast::StepPrimitive::Together { .. } => "together"`

## Error Scenarios

| Input | Expected Behavior |
|-------|------------------|
| `together: { branches: [...] }` | Parse as `StepPrimitive::Together` ✓ |
| `parallel: { branches: [...] }` | Reject as deprecated OR accept as alias |
| `together: {}` (empty) | `YamlError::FieldShape { field: "together.branches" }` |
| `together: { branches: [] }` (zero branches) | `YamlError::FieldShape` at validate step |
| `together: { branches: [{ label: "" }] }` | `YamlError::EmptyString` for label |
| `together: { unknown_field: x }` | `YamlError::UnknownField` |
