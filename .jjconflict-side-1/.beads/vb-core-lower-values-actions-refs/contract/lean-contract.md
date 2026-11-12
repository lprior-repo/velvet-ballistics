# Theorem Kernel Projection — vb-core-lower-values-actions-refs

## Boundary

| Layer | Owner | Rationale |
|---|---|---|
| TLA+ temporal model | N/A | No temporal properties in lowering; pure function |
| Verus Rust core | **Verus** | Slot index bounds, expression stack effects, constant pool overflow, numeric path enforcement — all pure/integer |
| Theorem kernel | **N/A** | No algebraic theorems, protocol lattices, or arithmetic bounds beyond Verus scope |
| Rust/runtime shell | Unit tests + Kani | Deterministic data structure properties |

## Theorem-Owned Clauses

**None.**

Rationale: This bead operates on concrete Rust types (`SlotIdx(u16)`, `ConstIdx(u16)`, `ExprProgram`, `AccessorProgram`). The critical properties are:

1. **Integer bounds**: `SlotIdx` is `u16` — trivial; no theorem needed
2. **Stack effect**: `check_expr_stack_bound` computes exact stack depth from ops — this is an integer recurrence, expressible in Verus
3. **Constant pool overflow**: `u16::try_from(constants.len())` — trivial bounds check

There is no algebraic state transition, no protocol lattice, no refinement chain beyond what Verus specs can express, and no need for Lean/Aeneas/Hax extraction.

## Lean/Aeneas/Hax Obligations

None.

## Verus Scope

### Target 1: Expression Bytecode Stack Safety (INV-004)

- **Rust target**: `crates/vb_core/src/expressions.rs::ExprProgram::try_from_ops`
- **Spec function**: `spec fn stack_effect(ops: Seq<ExprOp>) -> int` — computes running stack depth
- **Proof function**: `proof fn bounded_by(ops: Seq<ExprOp>, max: u8)` — proves `max(stack_effect(ops)) <= max`
- **Invariant**: `forall ops: ExprProgram . ops.max_stack <= MAX_EXPRESSION_STACK`
- **Trusted boundary**: `ExprProgram::try_from_ops` calls `check_expr_stack_bound` which is trusted Core runtime code
- **Shell exclusions**: No I/O, async, storage, or FFI in `expressions.rs`

### Target 2: SlotCompiler Max Slot Tracking (INV-001)

- **Rust target**: `crates/vb_compile/src/lib.rs::SlotCompiler`
- **Spec function**: `spec fn max_slot_recorded(sc: SlotCompiler) -> int`
- **Proof function**: `proof fn record_slot_preserves_max(sc: SlotCompiler, slot: SlotIdx)`
- **Invariant**: After any sequence of `record_slot` calls, `max_slot == max(all_recorded_slots)`
- **Trusted boundary**: `SlotCompiler` is compile-local; only `vb_compile` constructs it
- **Shell exclusions**: No I/O, async, storage, or FFI in `SlotCompiler`

### Target 3: Numeric-Only Accessor Path Segments (INV-005)

- **Rust target**: `crates/vb_compile/src/expression_bytecode.rs::numeric_path_segments`
- **Spec function**: `spec fn is_numeric_segment(s: string) -> bool`
- **Proof function**: `proof fn all_numeric_segments(path: Vec<PathSegment>)`
- **Invariant**: `forall path . is_numeric_path(path) ==> all_segments_are_u32(path)`
- **Trusted boundary**: Called only from `lower_accessor_reference` after slot index parse
- **Shell exclusions**: No I/O, async, storage, or FFI in `expression_bytecode.rs`

## Waivers

| Clause | Waiver Rationale |
|---|---|
| Any Lean/Aeneas/Hax theorem | No algebraic state transitions, no protocol lattices, no arithmetic beyond Verus scope. Verus specs/proofs are sufficient. |
| TLA+ temporal model | No temporal properties in lowering phase. Runtime scheduling is verified separately. |
