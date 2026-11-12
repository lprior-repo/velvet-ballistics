# Consolidated Contract: vb-zioy

## Scope
Fix the diagnostic step index in `emit_single_body_set` so that body validation errors for scoped primitives (collect, for_each, aggregate, repeat, parallel) report the **source YAML step index** instead of the **synthetic compiled step index**.

## Domain Decisions

1. **Two-Index Model**: The compiler maintains two step index namespaces:
   - `usize` — source AST ordinal (user-facing diagnostics)
   - `StepIdx` (`u16`) — compiled IR node id (internal only)
   These must not be conflated in diagnostic contexts.

2. **Synthetic Step Opacity**: Synthetic steps (`id + 1`, `id + 2`, etc.) must never appear in user-facing error messages. They are compiler-internal implementation details.

3. **Shared Dispatcher Contract**: `emit_single_body_set` is a shared pure-logic dispatcher. Because it emits user-facing diagnostics, it must receive the source diagnostic index explicitly, separate from the compiled node id.

## Type Contract

### `emit_single_body_set` Signature Change
```rust
pub(super) fn emit_single_body_set(
    body: &[vb_yaml::ast::StepAst],
    id: StepIdx,               // compiled IR node id for the body step (synthetic)
    diagnostic_step: usize,    // source AST step index for error reporting
    slot: SlotIdx,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
    reuse_first_constant: bool,
) -> Result<(), CompileErrors>
```

### Error Construction Rule
All `CompileError` variants created inside `emit_single_body_set` must use `diagnostic_step` for the `step` field:
- `CompileError::StepFieldShape { step: diagnostic_step, ... }`
- `CompileError::UnsupportedStepPrimitive { step: diagnostic_step, ... }`

### Caller Obligations
Each caller must pass its `index: usize` (or equivalent source index) as `diagnostic_step`:
- `lower_canonical_collect` → `index`
- `lower_canonical_for_each` → `index`
- `lower_canonical_aggregate` → `index`
- `lower_canonical_repeat` → `index`
- `lower_canonical_parallel` (per branch) → `branch_index` or appropriate source index

## Invariants

| # | Invariant | Enforcement |
|---|-----------|-------------|
| I1 | `body.len() == 1` in `emit_single_body_set` | Runtime check with error |
| I2 | `body[0].primitive == Set` | Runtime check with error |
| I3 | Synthetic steps never appear in diagnostics | Contract / code review |
| I4 | `diagnostic_step` is a valid AST ordinal (0 ≤ diagnostic_step < steps.len()) | Caller obligation |

## Workflow Guard

The lowering workflow must preserve the source-to-diagnostic index mapping through the `emit_single_body_set` boundary. The fix adds an explicit parameter to enforce this at the function boundary.

## Hazard Acceptance

- **H4 (Index Namespace Confusion)**: Accepted as residual risk. A future bead may introduce `SourceStepIdx` newtype.
- **H3 (Parallel Branch Ambiguity)**: Deferred to implementation; documented in hazard analysis.

## Non-Goals

- No change to error message text (`field`, `expected` strings remain identical)
- No change to compilation success path behavior
- No change to IR graph structure
- No new error variants introduced
- No newtype introduction for source indices (out of scope)
