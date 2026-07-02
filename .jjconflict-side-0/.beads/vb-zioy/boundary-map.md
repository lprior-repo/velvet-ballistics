# Boundary Map: Functional Core / Imperative Shell

## Core / Shell Split

### Functional Core (Pure)

| Component | Purity | Description |
|-----------|--------|-------------|
| `checked_step_offset` | Pure | Arithmetic on `StepIdx`; no side effects |
| `emit_single_body_set` (logic) | Pure | Validation logic on `&[StepAst]`; pure function of input |
| `body_constant_index` | Pure-mutating | Reads/writes builder state but deterministic given inputs |
| `lower_set` | Pure | Constructs `CompiledNode` value object |
| `slot_from_text` | Pure | String parsing; no I/O |

### Imperative Shell (Mutable / Effectful)

| Component | Effects | Description |
|-----------|---------|-------------|
| `SlotCompiler::push_node` | Mutates builder vec | Appends to compiled node list |
| `SlotCompiler::record_slot` | Mutates slot registry | Tracks live slots |
| `SlotCompiler::push_constant` | Mutates constant pool | May fail if full |
| YAML parse | I/O (source read) | Reads file/bytes from filesystem |

### Boundary Crossing: Lowering Functions

```
lower_canonical_collect  ──┐
lower_canonical_for_each ──┼──> emit_single_body_set ──> SlotCompiler (shell)
lower_canonical_aggregate ─┤         ↑
lower_canonical_repeat ────┘         │
                                     │
                              Pure validation logic
                              (body.len check, primitive match)
```

The **boundary** between core logic and shell effects is at `emit_single_body_set`:
- **Input**: Immutable view of AST (`&[StepAst]`)
- **Logic**: Pure validation (body length, primitive type)
- **Output**: Either pure `CompileErrors` or mutation of `SlotCompiler`

## Parser Boundary

The YAML parser is the outermost shell boundary. It transforms bytes into `StepAst`. Any parse error is caught here before entering the lowering core.

## Storage / Network / Time / FFI / Unsafe

This bead involves **none** of these:
- No storage I/O beyond source YAML read
- No network
- No time-dependent logic
- No FFI
- No `unsafe` code (forbidden by project policy)

## Testability Boundary

`emit_single_body_set` is already tested with property tests (`proptest_body_dispatcher.rs`, `proptest_error_parity.rs`, `proptest_collect.rs`). These tests construct `StepAst` values directly and verify error variants. The contract change (adding `diagnostic_step`) will require test updates but no infrastructure changes.
