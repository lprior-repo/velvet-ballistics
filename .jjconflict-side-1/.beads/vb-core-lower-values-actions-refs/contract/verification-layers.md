# Verification Layers — vb-core-lower-values-actions-refs

## Boundary

| Layer | Scope |
|---|---|
| Verus Rust core | Slot index bounds, expression bytecode stack effects, constant pool overflow, numeric accessor path enforcement |
| TLA+ temporal model | **None** — lowering is a pure function |
| Theorem projection | **None** — no algebraic kernels |
| Runtime shell | `vb_core` hot execution (separate bead) |
| External systems | YAML parser (`vb_yaml`), expression lexer (built into `vb_compile`) |

## Layer Assignment

| Contract Clause | Primary Layer | Secondary Layer | Notes |
|---|---|---|---|
| PRE-001 through PRE-005 | Unit test | Kani | Input validation |
| POST-001 (slot ref → LoadSlot) | Unit test | Kani | Direct lowering |
| POST-002 (accessor ref → LoadAccessor) | Unit test | Kani | Accessor program construction |
| POST-003 (bytecode bounds) | **Verus** | Kani | Stack depth + op count |
| POST-004 (single-stack-result) | **Verus** | Kani | Final depth == 1 |
| POST-005 (constant push) | Unit test | Kani | Constant pool overflow |
| POST-006 (slot_count) | Unit test | Kani | Max slot computation |
| POST-007 (build_parts) | Unit test | Kani | WorkflowParts construction |
| POST-008 (taint preservation) | Pipeline order | **Waiver** | Order-guaranteed by compile pipeline |
| POST-009 (validate before CompiledWorkflow) | Unit test | Kani | Gate 7-15 validation |
| INV-001 (max_slot tracking) | **Verus** | Unit test | Max function |
| INV-002 (record_slot per slot) | Unit test | Kani | Every slot recorded |
| INV-003 (StepIdx in bounds) | Unit test | Kani | Index range |
| INV-004 (bytecode stack safety) | **Verus** | Kani | Stack bounds |
| INV-005 (numeric accessor paths) | Unit test | Kani | Parse predicate |
| INV-006 (order-preserving) | Unit test | — | Deterministic |
| INV-007 (unique node.id) | Unit test | Kani | No duplicate StepIdx |
| ERR-* (error taxonomy) | Unit test | Integration test | Each error variant |
| PERF-* (bytecode perf) | Criterion | **Waiver** | v1 bytecode is bounded; perf non-goal |

## Verus Scope

### Module: `crates/vb_core/src/expressions.rs`

**Target**: `ExprProgram::try_from_ops`

**Spec/Proof surface**:
```verus
spec fn stack_effect(op: ExprOp) -> int  // -1 for pop, +1 for push, 0 for branch
spec fn total_stack_effect(ops: Seq<ExprOp>) -> int  // cumulative sum
proof fn bounded_by(ops: Seq<ExprOp>, max: u8)
  ensures total_stack_effect(ops) <= max
```

**Command**: `verus crates/vb_core/src/expressions.rs`

**Shell exclusions**: No I/O, async, storage, FFI, wall-clock time, randomness.

### Module: `crates/vb_compile/src/lib.rs`

**Target**: `SlotCompiler::record_slot` and `SlotCompiler::slot_count`

**Spec/Proof surface**:
```verus
spec fn max_slot(slots: Set<u16>) -> u16
proof fn record_slot_preserves_max(sc: SlotCompiler, slot: SlotIdx)
  ensures sc.max_slot == max(sc.max_slot@before, slot.as_u16())
```

**Command**: `verus crates/vb_compile/src/lib.rs` (targeting `SlotCompiler` impl block)

**Shell exclusions**: No I/O, async, storage, FFI, wall-clock time, randomness.

## Kani Scope

### Target 1: `lower_slot_reference` + `lower_accessor_reference`

**Claim**: For any valid `$slot.N` reference, returns `Ok(ExprOp::LoadSlot(...))`; for any valid `$slots.N.P...`, returns `Ok(ExprOp::LoadAccessor(...))` with correct `AccessorProgram`.

**Command**: `cargo kani --package vb_compile` (harness in `crates/vb_compile/src/kani_idempotency_parity.rs`)

### Target 2: `compile_expr_to_bytecode` bounds

**Claim**: Returns `Err` on overflow; returns `Ok` with correct `max_stack` otherwise.

**Command**: `cargo kani --package vb_core --harness check_expr_stack_bound`

## Unit Test Scope

### Expression Bytecode Tests

- Happy path: arithmetic, comparison, boolean, helper calls
- Error path: stack overflow, stack underflow, op count overflow, helper arity
- Edge cases: zero, negative numbers, i64::MAX, empty string, deeply nested expressions

### SlotCompiler Tests

- Empty builder → `slot_count() == 0`
- Single slot recorded → `slot_count() == 1`
- Max slot tracked correctly
- Constant pool overflow at u16::MAX + 1

### Accessor Reference Tests

- `$slot.N` → `LoadSlot(N)`
- `$slots.N.P` → `LoadAccessor(0)` with `AccessorProgram { root: N, path: [P] }`
- Reject non-numeric segments
- Reject empty segments (`$slot.1..0`)

## Waiver: Taint Preservation (POST-008)

**Owner**: `type_taint::validate_workflow_ast` (runs before `build_workflow_parts`)

**Reason**: Taint validation is a static analysis pass that runs before lowering. The pipeline order (validate → lower → validate_with_contracts) guarantees no secret-tainted value reaches the `finish.result` field. This is not a runtime property — it is enforced at compile time by `SecretTaintLeak` error.

**Compensating evidence**: `type_taint_tests.rs` (121.6K) covers all taint propagation paths. `kani_taint.rs` provides Kani proofs for taint bounds.

## Waiver: Performance (PERF)

**Reason**: v1 expression bytecode is bounded (`MAX_EXPRESSION_OPS`, `MAX_EXPRESSION_STACK`). No dynamic performance claims — the bytecode is size-bounded at compile time.

**Non-goal**: Micro-optimization of bytecode dispatch in runtime.

## Waiver: TLA+ Temporal Model

**Reason**: Lowering is a pure function `WorkflowAst → WorkflowParts`. No temporal properties, liveness, fairness, or deadlock to verify. The runtime execution of loops/concurrent branches is handled by separate beads.

**Compensating evidence**: Unit tests + Kani cover data-structure correctness. Runtime temporal behavior is verified in `vb-core-lower-control-primitives` and `vb-core-execution`.
