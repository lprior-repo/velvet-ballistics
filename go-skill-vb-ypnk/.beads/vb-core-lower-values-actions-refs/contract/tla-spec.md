# TLA+ Temporal Model Plan — vb-core-lower-values-actions-refs

## Boundary

- **Temporal/workflow behavior**: None. The lowering phase (`WorkflowAst → WorkflowParts`) is a pure, stateless, order-preserving transformation with no loops, concurrency, retries, event-sourcing, or stateful persistence. It is a function, not a protocol.
- **Rust/core behavior excluded from TLA+ and handled by Verus/Kani/tests**: Slot index bounds, expression bytecode stack safety, constant pool overflow, accessor path numeric-only enforcement
- **External systems abstracted**: None — YAML parser, expression lexer, and type/taint validator all run before lowering
- **Non-applicability rationale**: This bead is a pure compiler transformation. There are no temporal properties (liveness, fairness, eventual consistency, deadlock, retry loops, concurrent branches, or state machines with nondeterminism) to verify in the lowering itself. The runtime scheduling of `TogetherStart`/`TogetherJoin` is a runtime concern, not a lowering concern.

## TLA+-Owned Clauses

**None.**

Rationale: The lowering functions (`lower_slot_reference`, `lower_accessor_reference`, `compile_expr_to_bytecode`, `SlotCompiler::build_parts`) are total pure functions from `(&str, &mut Vec<AccessorProgram>) → Result<ExprOp, CompileError>` and `(SlotCompiler, &str, WorkflowDigest) → Result<WorkflowParts, CompileError>`. There are no:

- Temporal operators (`[]`, `<>`, `~>`, `WF`, `SF`)
- Liveness properties
- Fairness assumptions
- State machines with nondeterminism
- Concurrent processes or message passing
- Retry loops or backoff
- Distributed coordination

The invariants (INV-001 through INV-007) are all data-structure correctness properties:

| Invariant | Nature | Proof Method |
|---|---|---|
| INV-001: max_slot tracking | Integer max function | Verus `max_slot` spec/proof |
| INV-002: record_slot called per slot | Structural/deterministic | Unit test + Kani |
| INV-003: StepIdx in bounds | Bounded index | Unit test |
| INV-004: Expression bytecode stack safety | Integer stack effect | Verus + Kani |
| INV-005: Numeric-only accessor paths | Parse predicate | Unit test |
| INV-006: Order-preserving lowering | Deterministic order | Unit test |
| INV-007: Unique CompiledNode.id | No duplicate StepIdx | Unit test + Kani |

## Model Shape

Not applicable — no TLA+ model required for this bead.

## Properties

Not applicable.

## Evidence Command

Not applicable.

## Waivers

| Clause | Waiver Rationale |
|---|---|
| Any TLA+ temporal model | Lowering is a pure function; no temporal properties to model. Runtime step execution is handled by the runtime bead `vb-core-execution`, not this lowering bead. |
| Together/ForEach/Collect/Reduce loop semantics | These are emitted as IR nodes; their runtime execution semantics are verified in `vb-core-execution` and `vb-core-lower-control-primitives`. Lowering only produces the correct IR shape. |
