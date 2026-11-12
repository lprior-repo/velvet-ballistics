# Type Contracts

## CompileError Variants (Diagnostic Domain)

### `CompileError::StepFieldShape`
```rust
StepFieldShape {
    step: usize,       // SOURCE step index (0-based YAML ordinal)
    field: &'static str,
    expected: &'static str,
}
```
**Contract**: `step` MUST be an AST source step index, never a synthetic compiled step index. This is the index the user sees in their editor.

### `CompileError::UnsupportedStepPrimitive`
```rust
UnsupportedStepPrimitive {
    step: usize,       // SOURCE step index
    primitive: &'static str,
}
```
**Contract**: Same fidelity requirement as `StepFieldShape`.

### `CompileError::PrimitiveLoweringLimitExceeded`
```rust
PrimitiveLoweringLimitExceeded {
    primitive: &'static str,
    field: &'static str,
    value: usize,
    limit: usize,
}
```
**Contract**: Reports limit violations during synthetic step allocation. Uses field names, not step indices.

### `CompileError::StepIndexOutOfRange`
```rust
StepIndexOutOfRange { value: usize }
```
**Contract**: Overflow of `usize` → `u16` conversion.

## Step Index Types (Two-Namespace Model)

| Type | Namespace | Purpose | Used For Diagnostics? |
|------|-----------|---------|----------------------|
| `usize` (AST ordinal) | Source | Position in parsed YAML step array | **Yes** |
| `StepIdx` (`u16` newtype) | Compiled IR | Node identifier in `CompiledWorkflow` | **No** (internal only) |

**Typestate Rule**: Functions that emit user-facing diagnostics must receive the `usize` source index explicitly. They must not derive it from `StepIdx` when `StepIdx` represents a synthetic step.

## `emit_single_body_set` Signature Contract

### Current (Broken)
```rust
pub(super) fn emit_single_body_set(
    body: &[vb_yaml::ast::StepAst],
    id: StepIdx,          // ← synthetic body step; used for errors
    slot: SlotIdx,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
    reuse_first_constant: bool,
) -> Result<(), CompileErrors>
```
**Failure**: `id.as_usize()` is reported as the diagnostic step. This is the synthetic body step, not the source step.

### Required Contract
The function must accept a **diagnostic step index** separate from the **compiled node id**:
```rust
pub(super) fn emit_single_body_set(
    body: &[vb_yaml::ast::StepAst],
    id: StepIdx,               // compiled IR node id (synthetic)
    diagnostic_step: usize,    // source AST step index for errors
    slot: SlotIdx,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
    reuse_first_constant: bool,
) -> Result<(), CompileErrors>
```
**Invariant**: All `CompileError` variants constructed inside `emit_single_body_set` must use `diagnostic_step`, never `id.as_usize()`.

## Smart Constructors / Parsers

- `StepIdx::checked_add(u16) -> Option<StepIdx>`: Overflow-safe synthetic step allocation.
- `checked_step_offset(id, offset, primitive, field) -> Result<StepIdx, CompileError>`: Maps overflow to `PrimitiveLoweringLimitExceeded`.

## Illegal States to Make Unrepresentable

1. **Synthetic step as diagnostic step**: The type system cannot enforce this today because `StepIdx` and `usize` are both valid numeric types. The contract requires discipline: any function emitting `CompileError` with a `step: usize` field must document whether that index is a source or synthetic index.
2. **Missing diagnostic context**: `emit_single_body_set` lacks a parameter for the source index. The contract fix adds one.

## Ownership Boundaries

- `emit_single_body_set` borrows `body: &[StepAst]` (immutable view of AST)
- `builder: &mut SlotCompiler` is the exclusive mutable output sink
- No internal state persists across calls
