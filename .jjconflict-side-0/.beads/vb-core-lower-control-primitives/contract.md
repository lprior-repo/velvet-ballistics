# Contract Specification: vb-core-lower-control-primitives

## Context
- **Feature**: Lower v1 control primitives from YAML AST to compiled IR nodes
- **Domain terms**:
  - `StepIdx` — 16-bit step index into the compiled workflow
  - `SlotIdx` — 16-bit slot index into the slot vector
  - `SlotCompiler` — incremental slot recorder during lowering
  - `CompiledNode` — single step in the compiled IR
  - `CompiledNodeKind` — discriminant of step variants (ForEachStart, TogetherStart, CollectStart, ReduceStart, RepeatStart, WaitUntil, WaitEvent, Ask, AskResume, etc.)
  - `WaitKind` — type-safe discriminator for `wait.until` vs `wait.event`
- **Assumptions**:
  - YAML AST has already been validated (schema, references, types)
  - Step widths are known: Ask/ForEach/Together emit 2 nodes; Collect/Reduce/Repeat emit 3 nodes; Finish emits 1–2
  - `StepIdx` and `SlotIdx` are both `u16::new()` wrapped
- **Open questions**: None

## Preconditions
- PRE-001: `id` is a valid `StepIdx` within `u16::MAX - 1` range when id-plus-one is required by the primitive (repeat, ask)
- PRE-002: All slot indices (`input`, `item_slot`, `accumulator`, `prompt`, `answer`, `deadline`, `event`, etc.) are valid `SlotIdx` values previously recorded via `builder.record_slot`
- PRE-003: `max_attempts` for `repeat` is in range `1..=u16::MAX`
- PRE-004: `limit` for `for_each`, `collect` is in range `0..=u32::MAX`
- PRE-005: `branch_count` for `together` fits in `u16`
- PRE-006: `page_size` for `collect` is non-zero

## Postconditions
- POST-001: `lower_for_each` returns exactly 2 `CompiledNode` values: `[ForEachStart, ForEachNext]` with correct `body` and `done` step offsets
- POST-002: `lower_together` returns exactly 2 `CompiledNode` values: `[TogetherStart, TogetherJoin]` with correct `join` step offset
- POST-003: `lower_collect` returns exactly 3 `CompiledNode` values: `[CollectStart, CollectPage, CollectFinish]` with correct `body` and `done` step offsets
- POST-004: `lower_reduce` returns exactly 3 `CompiledNode` values: `[ReduceStart, ReduceNext, ReduceFinish]` with correct `body` and `done` step offsets
- POST-005: `lower_repeat` returns exactly 3 `CompiledNode` values: `[RepeatStart, RepeatAttempt, RepeatFinish]` with `attempt_slot = id + 1` and correct `body`/`done` offsets
- POST-006: `lower_wait` returns exactly 1 `CompiledNode` with `WaitUntil` or `WaitEvent` kind matching the `WaitKind` discriminant; all referenced slots are recorded
- POST-007: `lower_ask` returns exactly 2 `CompiledNode` values: `[Ask, AskResume]` with `resume_id = id + 1`; `AskResume.output = Some(answer_slot)`
- POST-008: Every `CompiledNode.output` field is `None` unless the node produces a value (TogetherJoin, RepeatAttempt, AskResume, SetConst, Finish)
- POST-009: Every `CompiledNode.next` chain is set by the caller after lowering returns

## Invariants
- INV-001: Step width invariants from `compiled_step_width` are respected (Ask/ForEach/Together → 2; Collect/Reduce/Repeat → 3; Finish → 1 or 2)
- INV-002: All slots used in a primitive are recorded via `builder.record_slot` before the corresponding `CompiledNode` is constructed
- INV-003: `attempt_slot` in `lower_repeat` and `resume` in `lower_ask` must not overflow `u16::MAX`; this is validated by `checked_add` with explicit error

## Error Taxonomy
- `CompileError::PrimitiveLoweringLimitExceeded` — when id-plus-one overflows `u16::MAX` (repeat's `attempt_slot`, ask's `resume_step`)
- `CompileError::SlotIndexOutOfRange` — when a slot index cannot be represented as `u16`
- `CompileError::StepIndexOutOfRange` — when a step index cannot be represented as `u16`
- `CompileError::MissingStepField` — when a required field is absent
- `CompileError::StepFieldShape` — when a field has wrong structural shape

## Contract Signatures
```rust
// All fallible lowering functions return Result
pub fn lower_for_each(id: StepIdx, input: SlotIdx, item_slot: SlotIdx, limit: u32, body: StepIdx, done: StepIdx, builder: &mut SlotCompiler) -> Result<Vec<CompiledNode>, CompileError>
pub fn lower_together(id: StepIdx, branches: Vec<StepIdx>, join: StepIdx, builder: &mut SlotCompiler) -> Result<Vec<CompiledNode>, CompileError>
pub fn lower_collect(id: StepIdx, source: SlotIdx, limit: u32, page_size: u32, body: StepIdx, done: StepIdx, builder: &mut SlotCompiler) -> Result<Vec<CompiledNode>, CompileError>
pub fn lower_reduce(id: StepIdx, input: SlotIdx, accumulator: SlotIdx, initial: ConstIdx, body: StepIdx, done: StepIdx, builder: &mut SlotCompiler) -> Result<Vec<CompiledNode>, CompileError>
pub fn lower_repeat(id: StepIdx, max_attempts: u16, body: StepIdx, done: StepIdx, builder: &mut SlotCompiler) -> Result<Vec<CompiledNode>, CompileError>
pub fn lower_wait(id: StepIdx, kind: WaitKind, builder: &mut SlotCompiler) -> CompiledNode  // infallible
pub fn lower_ask(id: StepIdx, prompt: SlotIdx, answer: SlotIdx, timeout_slot: Option<SlotIdx>, builder: &mut SlotCompiler) -> Result<Vec<CompiledNode>, CompileError>
```

## TLA+-Owned Clauses
- TLA-WF-001: The step chain produced by lowering is structurally well-formed: no step ID appears twice, body/done offsets are positive, and AskResume's id = Ask.id + 1 is enforced
- TLA-WF-002: The slot vector is finite and bounded by u16::MAX; every slot referenced by a node has been recorded

## Verus-Owned Clauses
- VERUS-INV-001: `lower_repeat` computes `attempt_slot = id.checked_add(1)` correctly and does not wrap
- VERUS-INV-002: `lower_ask` computes `resume = id.checked_add(1)` correctly and does not wrap
- VERUS-INV-003: `WaitKind` enum variants are exhaustive and only valid combinations are constructible

## Theorem-Owned Clauses
- None — Rust-local pure logic is fully expressible in Verus

## Non-goals
- Validation of YAML AST (pre-validated before lowering)
- Runtime execution of compiled nodes
- Type-checking of slot values
