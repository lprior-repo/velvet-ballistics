# Codebase Map — vb-core-lower-control-primitives

## Bead
- **id**: vb-core-lower-control-primitives
- **title**: compiler: Lower v1 control primitives from YAML AST

## Pipeline Overview

The compiler lowering pipeline has 2 layers:

1. **vb_yaml** (`crates/vb_yaml/src/ast/`) — YAML AST parsing layer.
   - Input: raw YAML bytes
   - Output: `WorkflowSource` AST (vb_yaml AST types in `types.rs`)
   - Control primitives recognized as `StepPrimitive` enum variants (for_each, together, collect, reduce, repeat, wait, ask)

2. **vb_compile** (`crates/vb_compile/src/`) — Cold compiler layer.
   - Input: YAML AST from vb_yaml
   - Output: `CompiledWorkflow` (numeric IR via `CompiledNode` array)
   - Control primitives recognized as `StepKindAst` enum variants

3. **vb_validate** (`crates/vb_validate/src/`) — Validation layer.
   - Validates CompiledWorkflow IR via gates 7-15
   - Shared validation via `vb_validate::shared::validate()`

---

## Control Primitives — AST Types

### vb_yaml AST (`crates/vb_yaml/src/ast/types.rs`)

```rust
pub enum StepPrimitive {
    ForEach { variable, input, at_once, body },   // lines 104-113
    Together { branches },                         // lines 115-118
    Collect { variable, source, pages, items, body }, // lines 120-131
    Reduce { variable, input, initial, body },     // lines 133-142
    Repeat { max_attempts, body },                // lines 144-149
    Wait { event, timeout },                       // lines 151-156
    Ask { prompt, timeout },                       // lines 158-163
    ...
}
```

### vb_compile AST (`crates/vb_compile/src/ast/types.rs`)

```rust
pub enum StepKindAst {
    ForEach { input: SlotIdx, item: SlotIdx, limit: u32 },  // lines 149-154
    Together { branches: Vec<StepIdx> },                     // line 156
    Collect { source: SlotIdx, limit: u32, page_size: u32 }, // lines 158-162
    Reduce { input: SlotIdx, accumulator: SlotIdx, initial: AstValue }, // lines 164-168
    Repeat { max_attempts: u16 },                           // line 170
    Wait { slot, timeout, is_event },                       // lines 172-179
    Ask { prompt: SlotIdx, answer: SlotIdx, timeout },       // lines 181-188
    ...
}
```

---

## Lowering Functions — vb_compile (`crates/vb_compile/src/lib.rs`)

### `lower_for_each` (lines 354-394)
```rust
pub fn lower_for_each(
    id: StepIdx,
    input: SlotIdx,
    item_slot: SlotIdx,
    limit: u32,
    body: StepIdx,
    done: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError>
```
- Emits: `ForEachStart`, `ForEachNext` nodes

### `lower_together` (lines 397-435)
```rust
pub fn lower_together(
    id: StepIdx,
    branches: Vec<StepIdx>,
    join: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError>
```
- Emits: `TogetherStart`, `TogetherJoin` nodes
- Allocates accumulator slot via `alloc_accumulator_slot()` (lines 438-443)

### `lower_collect` (lines 446-493)
```rust
pub fn lower_collect(
    id: StepIdx,
    source: SlotIdx,
    limit: u32,
    page_size: u32,
    body: StepIdx,
    done: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError>
```
- Emits: `CollectStart`, `CollectPage`, `CollectFinish` nodes

### `lower_reduce` (lines 496-545)
```rust
pub fn lower_reduce(
    id: StepIdx,
    input: SlotIdx,
    accumulator: SlotIdx,
    initial: ConstIdx,
    body: StepIdx,
    done: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError>
```
- Emits: `ReduceStart`, `ReduceNext`, `ReduceFinish` nodes

### `lower_repeat` (lines 548-597)
```rust
pub fn lower_repeat(
    id: StepIdx,
    max_attempts: u16,
    body: StepIdx,
    done: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError>
```
- Emits: `RepeatStart`, `RepeatAttempt`, `RepeatFinish` nodes
- Note: computes `attempt_slot = id + 1` (line 556) — THIS IS THE "id-plus-one" body assumption

### `lower_wait` (lines 615-642)
```rust
pub fn lower_wait(id: StepIdx, kind: WaitKind, builder: &mut SlotCompiler) -> CompiledNode
```
- Emits: `WaitUntil` or `WaitEvent` node

### `lower_ask` (lines 645-683)
```rust
pub fn lower_ask(
    id: StepIdx,
    prompt: SlotIdx,
    answer: SlotIdx,
    timeout_slot: Option<SlotIdx>,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError>
```
- Emits: `Ask`, `AskResume` nodes
- Note: computes `resume = id.checked_add(1)` (line 654) — ANOTHER "id-plus-one" assumption

---

## YAML Parsing — vb_yaml (`crates/vb_yaml/src/ast/parse_steps.rs`)

### `parse_step_primitive` (lines 46-132)
Dispatches to specific parsers:
- `parse_foreach` (lines 156-167)
- `parse_together` (lines 169-185)
- `parse_collect` (lines 187-200)
- `parse_reduce` (lines 202-213)
- `parse_repeat` (lines 215-219)
- Wait/Ask: inline in `parse_step_primitive`

---

## vb_compile AST Parsing — `crates/vb_compile/src/ast/parse.rs`

### `parse_step_kind` (lines 262-288)
Maps YAML field names to `StepPrimitiveAst` and `StepKindAst`:
- `for_each` → `parse_for_each` (lines 347-353)
- `together` → `parse_together` (lines 355-359)
- `collect` → `parse_collect` (lines 361-367)
- `reduce` → `parse_reduce` (lines 369-379)
- `repeat` → `parse_repeat` (lines 381-385)
- `wait` → `parse_wait` (lines 387-407)
- `ask` → `parse_ask` (lines 409-415)

---

## CompiledNodeKind IR Types — vb_core (`crates/vb_core/src/nodes.rs`)

```rust
pub enum CompiledNodeKind {
    ForEachStart { input, item_slot, limit, body, done },
    ForEachNext { iterator_slot, body, done },
    TogetherStart { branches, join },
    TogetherJoin { branch_count, accumulator },
    CollectStart { source, limit, page_size, body, done },
    CollectPage { collector_slot, body, done },
    CollectFinish { collector_slot },
    ReduceStart { input, accumulator, initial, body, done },
    ReduceNext { iterator_slot, accumulator, body, done },
    ReduceFinish { accumulator },
    RepeatStart { max_attempts, body, done },
    RepeatAttempt { attempt_slot, body, done },
    RepeatFinish { result },
    WaitUntil { deadline_slot },
    WaitEvent { event, timeout_slot },
    Ask { prompt, timeout_slot },
    AskResume { answer },
}
```

---

## Error Types — `crates/vb_compile/src/lib.rs`

Key error variants relevant to control primitives:
- `CompileError::SlotIndexOutOfRange { value }` (line 1237)
- `CompileError::PrimitiveLoweringLimitExceeded { primitive, field, value, limit }` (line 1257)
- `CompileError::BackwardBranchTarget { step, target }` (line 1249)
- `CompileError::UnknownStepTarget { step, target }` (line 1312)
- `CompileError::UnreachableStep { step }` (line 1320)

---

## Validation — vb_validate (`crates/vb_validate/src/control_flow.rs`)

- `validate_control_flow()` — validates forward targets and reachability
- `validate_forward_only_then()` — validates then targets
- `push_successors()` — handles Together branches (lines 94+)

---

## Key Risks / Assumptions

1. **`repeat` lower** (line 556): `attempt_slot = id + 1` — hardcoded offset assumption. If body steps don't follow this pattern, the slot is wrong.

2. **`ask` lower** (line 654): `resume = id.checked_add(1)` — hardcoded offset assumption.

3. **Dense index requirement**: The pipeline assumes step indices are dense (no gaps). Any "synthetic id-plus-one body assumptions" would break this.

4. **`together` accumulator slot**: Allocated via `alloc_accumulator_slot()` which uses `builder.slot_count()`. The slot index must not conflict with other slot allocations.

5. **SlotCompiler state**: Lower functions use a fresh `SlotCompiler::new()` rather than shared state, which may cause subtle differences in slot allocation across multiple lowering calls.

---

## Existing Tests

### vb_yaml AST parsing tests:
- `crates/vb_yaml/src/ast/tests/tests_steps_foreach_together.rs` — for_each, together parsing
- `crates/vb_yaml/src/ast/tests/tests_steps_collect_reduce_repeat.rs` — collect, reduce, repeat parsing
- `crates/vb_yaml/src/ast/tests/tests_errors_adversarial_steps.rs` — invalid shape diagnostics

### vb_compile tests (in `lib.rs` tests module):
- `compile_for_each` (line 3114) — internal compile function, tested via integration
- `compile_together` (line 3142) — internal compile function
- `compile_collect` (line 3193) — internal compile function
- `compile_reduce` (line 3225) — internal compile function
- `compile_repeat` (line 3259) — internal compile function
- `compile_wait` (line 3289) — internal compile function
- `compile_ask` (line 3333) — internal compile function

---

## Required Verifier Modes

| Primitive | Risk Tag | Verifier Mode |
|-----------|----------|---------------|
| for_each | parser/codec, dense-index | proptest + miri |
| together | parser/codec, dense-index | proptest + miri |
| collect | parser/codec, dense-index | proptest + miri |
| reduce | parser/codec, dense-index | proptest + miri |
| repeat | parser/codec, id-plus-one assumption | proptest + miri + kani |
| wait | parser/codec | unit test |
| ask | parser/codec, id-plus-one assumption | proptest + miri + kani |

---

## Files TOUCHED by this bead (excludes generated Rust)

| Crate | File | Role |
|-------|------|------|
| vb_yaml | `src/ast/parse_steps.rs` | YAML parsing for control primitives |
| vb_yaml | `src/ast/types.rs` | AST types for StepPrimitive |
| vb_yaml | `src/ast/tests/tests_steps_foreach_together.rs` | for_each/together parsing tests |
| vb_yaml | `src/ast/tests/tests_steps_collect_reduce_repeat.rs` | collect/reduce/repeat parsing tests |
| vb_yaml | `src/ast/tests/tests_errors_adversarial_steps.rs` | invalid shape tests |
| vb_compile | `src/ast/parse.rs` | AST parsing for StepKindAst |
| vb_compile | `src/ast/types.rs` | AST types for StepKindAst |
| vb_compile | `src/lib.rs` | lower_* functions |
| vb_compile | `src/control_flow.rs` | Control flow validation |
| vb_compile | `src/lower/mod.rs` | lower module re-exports |
| vb_core | `src/nodes.rs` | CompiledNodeKind IR types |
| vb_validate | `src/control_flow.rs` | WorkflowFlow validation |

---

## Open Questions / BLOCKERS

1. **KNOWN ISSUE**: `lower_repeat` (line 556) uses `id.checked_add(1)` for `attempt_slot`. This is the "synthetic id-plus-one body assumption" that the bead explicitly excludes. Need to understand if this needs fixing.

2. **KNOWN ISSUE**: `lower_ask` (line 654) uses `id.checked_add(1)` for `resume`. Same id-plus-one assumption.

3. Need to verify: are there tests that cover the positive lowering path for each control primitive?

4. Need to verify: are there tests that cover invalid shape diagnostics for each control primitive?
