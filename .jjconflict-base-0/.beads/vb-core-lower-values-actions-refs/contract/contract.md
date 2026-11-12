# Contract Specification — vb-core-lower-values-actions-refs

## Context

| Field | Value |
|-------|-------|
| Bead | `vb-core-lower-values-actions-refs` |
| Title | compiler: Lower v1 values actions and references |
| Phase | 3 (Contract and type model) |
| Blocker | `vb-f04l` — compiler: Safe v1 primitive source lowering |
| Updated | 2026-05-15 |

## Domain Terms

- **Slot**: `SlotIdx(u16)` — numeric handle for a runtime value slot
- **SlotCompiler**: mutable builder that accumulates `CompiledNode`s, `ConstValue`s, `ExprProgram`s, and `AccessorProgram`s during IR lowering
- **lower_slot_reference**: lowers `$slot.N` references to `ExprOp::LoadSlot(SlotIdx::new(N))`
- **lower_accessor_reference**: lowers `$slots.N.P.Q` (numeric path) to `AccessorProgram { root: SlotIdx::new(N), path: [Index(P), Index(Q)] }` and emits `ExprOp::LoadAccessor(AccessorIdx)`
- **ActionId**: `u16` identifier for an external action; lowered from YAML action name via `vb_validate::references`
- **Taint**: `Taint::Clean | Taint::Secret` — secret metadata propagated through type/taint validation
- **Expression bytecode**: postfix `ExprProgram` with bounded `max_stack` and op count limits
- **WorkflowParts**: intermediate numeric IR with `nodes`, `expressions`, `accessors`, `constants`, `slot_count`

## Assumptions

1. `vb-f04l` provides the safe primitive source lowering that this bead depends on
2. `SlotCompiler` is the sole mutable accumulator during step lowering; no aliasing
3. All slot indices are `u16`; expression op count is bounded by `MAX_EXPRESSION_OPS`
4. Taint metadata is preserved through the type/taint validation phase (`type_taint.rs`) and must not be lost during lowering
5. Accessor paths are restricted to numeric list indices only (no field names) in v1
6. The runtime receives only numeric/handle data — no string-based slot names

## Open Questions

1. How does `symbols_count: 0` in `SlotCompiler::build_parts` affect accessor-based field access in future v2?
2. What is the exact protocol when `vb-f04l` primitive lowering encounters an invalid AST node that passes syntax but fails semantic validation?

---

## Preconditions

- **PRE-001**: `SlotCompiler::new()` produces an empty builder with `max_slot = None`
- **PRE-002**: `lower_slot_reference` is called only with valid `u16` slot indices parsed from `$slot.N` syntax
- **PRE-003**: `lower_accessor_reference` is called only with all-numeric path segments (`$slots.N.P.Q...` where each segment parses as `u32`)
- **PRE-004**: `compile_expr_to_bytecode` receives a `ParsedExpression` that has already passed lexer/parser validation (no invalid tokens)
- **PRE-005**: `SlotCompiler::push_constant` is called with a `ConstValue` that is one of `Null | Bool | I64 | F64 | Text` — no `Reference` variants

## Postconditions

- **POST-001**: `lower_slot_reference("$slot.N")` returns `Ok(ExprOp::LoadSlot(SlotIdx::new(N)))` and does not mutate any `accessors` vector
- **POST-002**: `lower_accessor_reference("$slots.N.P.Q")` pushes exactly one `AccessorProgram { root: SlotIdx::new(N), path: [Index(P), Index(Q)] }` to `accessors` and returns `Ok(ExprOp::LoadAccessor(AccessorIdx::new(K)))` where `K = accessors.len() - 1` before push
- **POST-003**: `compile_expr_to_bytecode` returns an `ExprProgram` where `ops.len() <= MAX_EXPRESSION_OPS` and `max_stack <= MAX_EXPRESSION_STACK`
- **POST-004**: `compile_expr_to_bytecode` returns an `ExprProgram` that leaves exactly one value on the evaluation stack (not empty, not multi-value)
- **POST-005**: `SlotCompiler::push_constant(value)` returns `Ok(ConstIdx::new(K))` where `K = constants.len() - 1` before push; on overflow (> u16::MAX) returns `Err(CompileError::Workflow(WorkflowError::ConstOutOfBounds))`
- **POST-006**: `SlotCompiler::slot_count()` returns the maximum slot index + 1 as `u16`; returns `Ok(0)` for empty builder
- **POST-007**: `SlotCompiler::build_parts` produces a `WorkflowParts` with `slot_count` equal to `slot_count()?`, `symbols_count = 0`, and all accumulated vectors converted to `Box<[...]>`
- **POST-008**: Taint metadata from `vb_validate::type_taint::ValueFact` is preserved in the sense that the type/taint validation phase runs before lowering and rejects `SecretTaintLeak` errors; lowering does not re-validate taint but the pipeline ordering guarantees preservation
- **POST-009**: `lower_steps_to_ir` calls `vb_validate::shared::validate` on the resulting `WorkflowParts` before constructing `CompiledWorkflow`

## Invariants

- **INV-001**: `SlotCompiler` maintains `max_slot = max(existing_max_slot, all_recorded_slot.as_usize())` — no slot index is ever lost
- **INV-002**: `SlotCompiler::record_slot` is called exactly once for every `SlotIdx` that appears in any lowered `CompiledNode` (input, output, condition, accumulator, etc.)
- **INV-003**: All `CompiledNode` indices (`id`, `next`, `body`, `done`, `join`) are within `0..total_nodes` range
- **INV-004**: Expression bytecode programs are structurally valid postfix: for every `N` ops, the stack effect never goes negative and never exceeds `MAX_EXPRESSION_STACK`
- **INV-005**: `AccessorProgram` paths contain only `PathSegment::Index(u32)` segments — no `PathSegment::Field` in v1
- **INV-006**: The lowering pipeline is order-preserving: steps are lowered in source order with monotonically increasing `StepIdx`
- **INV-007**: No two `CompiledNode` entries share the same `StepIdx` as `id` within a single `WorkflowParts::nodes` slice

## Error Taxonomy

| Error Variant | Trigger | Diagnostic Code |
|---|---|---|
| `CompileError::UnknownReferenceName { kind: "slot", ... }` | `$slot.N` where `N` fails `u16` parse | `UNKNOWN_REFERENCE` |
| `CompileError::UnknownReferenceRoot { root: "slot"\|"slots", ... }` | reference missing `$` prefix or wrong root | `UNKNOWN_REFERENCE` |
| `CompileError::UnsupportedAccessorReference { ... }` | non-numeric accessor segment in path | `UNSUPPORTED_ACCESSOR_REFERENCE` |
| `CompileError::ExpressionLoweringUnsupported { feature: "accessor references"\|"text constants", ... }` | reference/text in phase that cannot resolve yet | `INVALID_EXPRESSION` |
| `CompileError::ExpressionHelperArity { helper, expected, actual }` | wrong arg count for helper function | `INVALID_EXPRESSION` |
| `CompileError::ExpressionStackOverflow { max }` | expression exceeds `MAX_EXPRESSION_STACK` | `LIMIT_EXCEEDED` |
| `CompileError::Workflow(WorkflowError::ConstOutOfBounds { ... })` | constant pool overflow (> u16::MAX) | `CONST_OUT_OF_BOUNDS` |
| `CompileError::SlotIndexOutOfRange { value }` | slot index computation overflows `i64` or `u16` | `LIMIT_EXCEEDED` |
| `CompileError::PrimitiveLoweringLimitExceeded { primitive, field, value, limit }` | `TogetherStart` branch count > u16::MAX, etc. | `LIMIT_EXCEEDED` |
| `CompileError::SecretTaintLeak { field }` | secret-tainted value crosses public result boundary | `SECRET_RESULT_LEAK` |
| `CompileError::IdempotencyViolation { action, side_effect, reason }` | side-effecting action declares unsafe retry/idempotency | `IDEMPOTENCY_VIOLATION` |
| `CompileError::Validation(ValidationError::SlotReferenceOutOfRange { ... })` | slot reference exceeds `slot_count` | `TYPE_MISMATCH` |
| `CompileError::Validation(ValidationError::AccessorPathTooDeep { ... })` | accessor path exceeds `MAX_ACCESSOR_DEPTH` | `INVALID_COMPILED_WORKFLOW` |

## Contract Signatures

```rust
// crates/vb_compile/src/lib.rs

/// Lowers a `$slot.N` reference to LoadSlot or `$slots.N.P.Q` to LoadAccessor.
pub fn lower_slot_reference(
    reference: &str,
    accessors: &mut Vec<AccessorProgram>,
) -> Result<ExprOp, CompileError>

/// Lowers a parsed expression to bounded postfix bytecode.
pub fn compile_expr_to_bytecode(
    expression: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
) -> Result<ExprProgram, CompileError>

/// Lowers an expression with slot-rooted accessor support.
pub fn compile_expr_to_bytecode_with_accessors(
    expression: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
    accessors: &mut Vec<AccessorProgram>,
) -> Result<ExprProgram, CompileError>
```

```rust
// crates/vb_compile/src/lib.rs — SlotCompiler

impl SlotCompiler {
    pub fn new() -> Self
    pub fn push_constant(&mut self, value: ConstValue) -> Result<ConstIdx, CompileError>
    pub fn push_expression(&mut self, program: ExprProgram) -> Result<ExprIdx, CompileError>
    pub fn push_accessor(&mut self, program: AccessorProgram) -> Result<AccessorIdx, CompileError>
    pub fn record_slot(&mut self, slot: SlotIdx)
    pub fn push_node(&mut self, node: CompiledNode)
    pub fn slot_count(&self) -> Result<u16, CompileError>
    pub fn build_parts(self, name: &str, digest: WorkflowDigest) -> Result<WorkflowParts, CompileError>
}
```

## Verus-Owned Clauses

- **INV-004** (expression bytecode stack safety): Expressible in Verus via `spec fn stack_effect(ops) -> int` and `proof fn bounded_by(ops, MAX)`. Stack underflow/overflow rules are pure and deterministic.
- **INV-001** (slot max tracking): Expressible via `spec fn max_slot(slots) -> nat` and `proof fn record_slot_preserves_max`.
- **INV-005** (numeric-only accessor paths): Expressible as a `spec fn is_numeric_path(segments) -> bool`.

**Waiver rationale**: Verus is appropriate for these bounds-proof obligations because the properties are pure, total, and expressible as integer inequalities. No I/O, async, or external state required.

## TLA+-Owned Clauses

- **INV-003** (step index ordering/finiteness): TLA+ can model step indices as `0..N` with bounded naturals, but the lowering is deterministic and per-step — not a temporal workflow protocol. No liveness/eventuality properties to verify in lowering itself.

**Non-applicability rationale**: The lowering phase is a pure function `WorkflowAst -> WorkflowParts` with no loops, concurrency, retries, or stateful persistence. The step-index ordering invariant is a data structure correctness property provable by unit tests and Kani. No TLA+ temporal model needed.

## Theorem-Owned Clauses

None. This bead has no algebraic theorem kernels beyond what Verus can handle.

## Non-goals

- Field-name-based accessor paths (v2 feature)
- Symbol table construction (`symbols_count = 0` is intentional for v1)
- Async/concurrent lowering (Together branches are emitted but execution is runtime concern)
- Codegen (bead is `no-codegen` label)
