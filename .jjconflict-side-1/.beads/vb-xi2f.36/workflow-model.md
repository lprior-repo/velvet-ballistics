# Workflow Model: `together` Primitive Acceptance

## Workflow: YAML → Validated → Compiled

```
[YAML Source]
     │
     ▼
┌─────────────────────────────┐
│ vb_yaml::ast::parse_steps   │
│ parse_step()                │
│   is_primitive("together") │ ◄── FIX: add "together" to is_primitive()
│   "together" → parse_parallel() │ ◄── FIX: add match arm
│   parse_parallel() → StepPrimitive::Together
└─────────────────────────────┘
     │ YamlResult<StepAst>
     ▼
┌─────────────────────────────┐
│ vb_validate::schema         │
│ validate_workflow_schema()  │
│   STEP_PRIMITIVES.contains("together") │ ◄── FIX: replace "parallel" with "together"
│   validate_step_fields()    │
└─────────────────────────────┘
     │ ValidationResult<()>
     ▼
┌─────────────────────────────┐
│ vb_compile::compile         │
│ lower_together()            │
│   StepPrimitive::Together   │
│   → CompiledNode::TogetherStart │
└─────────────────────────────┘
```

## State Machine: Together Construct

### States

| State | Description |
|-------|-------------|
| `TogetherStart` | Initial state, branches not yet executing |
| `TogetherBranchRunning` | One or more branches actively executing |
| `TogetherJoin` | All branches complete, joining results |
| `TogetherComplete` | Terminal: fan-out/fan-in cycle done |

### Transitions

```
[TogetherStart]
    │
    │ branches assigned to executor slots
    ▼
[TogetherBranchRunning] ──(branch completes)──► [TogetherBranchRunning]
    │                                              (if any remain)
    │ (all branches complete)
    ▼
[TogetherJoin]
    │
    │ join condition satisfied
    ▼
[TogetherComplete] ──► next step
```

## Validation Guards

1. **Non-empty branches**: `branches.len() >= 1` enforced at parse time
2. **Non-empty label**: `!label.is_empty()` enforced at parse time
3. **Non-empty steps**: `!steps.is_empty()` enforced at parse time
4. **No duplicate labels**: enforced at compile validation time
5. **Bounded fan-out**: `branches.len() <= max_together_branches` (from budget) enforced at compile time

## Error Outcomes

| Error | Source | Code |
|-------|--------|------|
| Empty branches | Parse | `YamlError::FieldShape` |
| Missing required field | Parse | `YamlError::MissingField` |
| Unknown field | Parse | `YamlError::UnknownField` |
| Unknown primitive key | Validate | `UNKNOWN_FIELD` |
| Multiple primitives in step | Parse | `MULTIPLE_STEP_PRIMITIVES` |
| Fan-out exceeds budget | Compile | `TOGETHER_BRANCH_LIMIT_EXCEEDED` |

## Terminal States

- `TogetherComplete`: successful join
- Error state: any parse/validate/compile error aborts workflow loading
