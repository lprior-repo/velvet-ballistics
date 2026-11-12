# Lean Theorem Kernel Projection

## Boundary
- **TLA+-owned temporal model**: ControlLowering.tla — structural well-formedness of step chains
- **Verus-owned Rust core**: All pure lowering functions (`lower_*`) — slot recording, node construction, `WaitKind`, overflow checks
- **Theorem-owned kernel**: None — the domain is fully expressible in Verus
- **Rust/runtime shell**: `WorkflowBuilder`, `SlotCompiler::new()`, `builder.record_slot()` calls — these are the procedural shell around the pure lowering
- **External systems excluded from theorem proof**: None

## Theorem-Owned Clauses
- None — Verus can cover all Rust-local pure behavior

## Verus Scope
- **Rust target**: `crates/vb_compile/src/lib.rs` — `lower_repeat`, `lower_ask`, `lower_wait`, `lower_for_each`, `lower_together`, `lower_collect`, `lower_reduce`
- **Spec/proof function**: For each `lower_*` function, prove:
  - `lower_repeat`: `attempt_slot.id = id + 1` (no overflow)
  - `lower_ask`: `resume.id = id + 1` (no overflow)
  - All `CompiledNode` vectors have the exact expected length
  - All `kind` discriminants match the primitive type
- **Invariants**: `SlotIdx::new(u16)` is only called on values that fit in `u16`
- **Trusted boundary**: `StepIdx::new(u16)` and `SlotIdx::new(u16)` constructors are trusted; overflow is prevented by `checked_add` before construction
- **Shell exclusions**: `WorkflowBuilder`, mutable slot recording, I/O

## Waivers
- Theorem kernel not needed — Verus is sufficient for all pure lowering properties
