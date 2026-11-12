# Railway Error Taxonomy for Body Length Violations

## Error Taxonomy (Railway Pattern)

### Success Path
```
AST Step (source index N)
  → lower_canonical_* (index = N, id = compiled_id)
  → emit_single_body_set(body, ...)
  → body.len() == 1 && body[0].primitive == Set
  → CompiledNode emitted
  → Ok(())
```

### Failure Paths

#### F1: Empty Body
```
AST Step (source index N) declares collect/for_each/aggregate/repeat with body: []
  → emit_single_body_set called with empty body slice
  → body.len() != 1
  → CompileError::StepFieldShape {
        step: <SHOULD_BE_N>,   // BUG: currently reports synthetic body_step
        field: "steps",
        expected: "exactly one set step",
    }
```

#### F2: Multiple Body Steps
```
AST Step (source index N) declares collect with body: [set, set]
  → emit_single_body_set called with 2-element body slice
  → body.len() != 1
  → CompileError::StepFieldShape {
        step: <SHOULD_BE_N>,   // BUG: currently reports synthetic body_step
        field: "steps",
        expected: "exactly one set step",
    }
```

#### F3: Non-Set Body Primitive
```
AST Step (source index N) declares collect with body: [finish]
  → emit_single_body_set called with 1-element body slice
  → body.len() == 1 (passes first check)
  → body[0].primitive != Set
  → CompileError::UnsupportedStepPrimitive {
        step: <SHOULD_BE_N>,   // BUG: currently reports synthetic body_step
        primitive: "finish",
    }
```

#### F4: Synthetic Step Overflow
```
AST Step (source index N) near u16::MAX
  → checked_step_offset(id, 1, ...) fails
  → CompileError::PrimitiveLoweringLimitExceeded { primitive, field, value, limit }
  → OK: this error correctly does not use a step index field
```

## Error Variant Mapping

| User Mistake | Expected Error | Current (Bug) Error | Field Affected |
|--------------|---------------|---------------------|----------------|
| Empty body | `StepFieldShape { step: N, field: "steps", ... }` | `StepFieldShape { step: synthetic, ... }` | `step` |
| Multiple body steps | `StepFieldShape { step: N, field: "steps", ... }` | `StepFieldShape { step: synthetic, ... }` | `step` |
| Body is not `Set` | `UnsupportedStepPrimitive { step: N, ... }` | `UnsupportedStepPrimitive { step: synthetic, ... }` | `step` |

## Severity

- **User impact**: Medium. Users see a step number that does not exist in their YAML, causing confusion and hindering debugging.
- **Compiler correctness**: Low. The compilation still fails; no unsound IR is produced.
- **Trust impact**: Medium. Wrong diagnostics erode user trust in the compiler.

## Error Recovery

There is no recovery path for body length violations. The compilation halts and returns the error. The only remediation is for the user to fix their YAML. Therefore, the diagnostic **must** be accurate.
