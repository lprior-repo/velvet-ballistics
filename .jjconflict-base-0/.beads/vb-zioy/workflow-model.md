# Workflow Model: Parse → Lower → Validate → Compile

## State Machine

```
[YAML Source] --(parse)--> [AST: Vec<StepAst>] --(lower)--> [CompiledWorkflow IR] --(validate)--> [Validated IR]
```

### Phase 1: Parse
- Input: UTF-8 YAML bytes
- Output: `Vec<StepAst>` + source spans
- Errors: `Parse`, `CanonicalYaml`, `NonStringKey`, `DuplicateKey`, etc.
- **Diagnostic step indices are born here** (0-based AST ordinals)

### Phase 2: Lower (this bead's focus)
- Input: `Vec<StepAst>`, `index: usize` per step
- Output: `CompiledWorkflow` graph (`Vec<CompiledNode>`)
- Sub-states:
  1. **Primitive dispatch**: Match `StepPrimitive` to lowering function
  2. **Synthetic step allocation**: `checked_step_offset(id, offset, ...)`
  3. **Body lowering**: `emit_single_body_set(body, synthetic_id, ...)` ← **defect here**
  4. **Node emission**: `builder.push_node(...)`
- Errors: `StepFieldShape`, `UnsupportedStepPrimitive`, `PrimitiveLoweringLimitExceeded`, etc.

### Phase 3: Validate
- Input: `CompiledWorkflow` IR
- Output: Validated IR with reachability/checks
- Errors: `WorkflowError`, `ValidationError`

### Phase 4: Compile (Byte-code emission)
- Input: Validated IR
- Output: Bytecode / execution plan
- No user-facing diagnostics at this stage

## Lowering Workflow for Collect

```
lower_canonical_collect(index, id, collect, builder)
  ├─→ source = slot_from_text(collect.source, index, "collect.source")
  ├─→ body_step = checked_step_offset(id, 1, "collect", "body")   // synthetic
  ├─→ page      = checked_step_offset(id, 2, "collect", "page")   // synthetic
  ├─→ done      = checked_step_offset(id, 3, "collect", "done")   // synthetic
  ├─→ push CollectStart { source, limit, page_size, body: body_step, done }
  ├─→ emit_single_body_set(collect.body, body_step, slot, Some(page), builder, false)
  │     └─→ [BUG] if body.len() != 1, reports step = body_step.as_usize()
  │           instead of the original `index`
  ├─→ push CollectPage { collector_slot, body: body_step, done }
  └─→ push CollectFinish { collector_slot }
```

## Legal Transitions

| From | Event | Guard | To |
|------|-------|-------|-----|
| Parsed AST | Lower primitive | `body.len() == 1` | Compiled nodes emitted |
| Parsed AST | Lower primitive | `body.len() != 1` | `CompileError::StepFieldShape` |
| Parsed AST | Lower primitive | `body[0].primitive != Set` | `CompileError::UnsupportedStepPrimitive` |
| Parsed AST | Lower primitive | `id + offset > u16::MAX` | `CompileError::PrimitiveLoweringLimitExceeded` |

## Terminal States

- **Success**: All steps lowered, IR graph complete
- **Diagnostic Failure**: One or more `CompileError`s collected; user sees step numbers mapping to source YAML
- **Internal Failure**: Overflow, invariant violation (should never reach user)

## Hazard: Cross-Phase Index Drift

The `index: usize` from the parser must survive through lowering to validation. Any transformation that reorders or inserts steps must maintain a mapping back to source indices for diagnostics. The current bug is a **local instance** of this hazard: synthetic steps are inserted, and the diagnostic index is lost at the `emit_single_body_set` boundary.
