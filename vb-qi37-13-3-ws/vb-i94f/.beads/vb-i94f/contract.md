# Contract Specification: Taint Propagation Through EvalExpr, BuildObject, BuildList, Choose, and Finish Paths

## Context

- **Feature**: Prove and enforce taint propagation through expression evaluation, object construction, list construction, choice selection, and finish output paths in the velvet-ballastics runtime engine.
- **Domain terms**:
  - `Taint` — three-level lattice: `Clean < DerivedFromSecret < Secret` (`#[repr(u8)]`, defined in `vb_core::value::Taint`)
  - `join_taint(a, Taint)` — lattice join: returns the more restrictive of two taint levels
  - `SlotValue` — handle-based runtime value (`Object(ListId)`, `List(ListId)`, scalar variants)
  - `RunFrame` — hot execution frame; owns `slots: Box<[Option<SlotValue>]>` and `taint: Box<[Taint]>` in parallel
  - `EngineSignal::Finished(SlotValue, Taint)` — terminal signal carrying result value and joined taint
  - `EvalExpr` IR node — evaluates an `ExprProgram` and writes `(SlotValue, Taint)` to an output slot
  - `BuildObject` IR node — reads slot values/taint for each field, joins taint, writes `Object(ListId)` handle and joined taint
  - `BuildList` IR node — reads slot values/taint for each item, joins taint, writes `List(ListId)` handle and joined taint
  - `Choose` / `ChooseSlot` IR nodes — select a branch target; taint is not accumulated (only the selected branch executes)
  - `Finish` IR node — reads result slot value and taint, emits `EngineSignal::Finished(value, taint)`
  - `ValueStore` — cold arena for `Object` field tables and `List` item vectors
  - `ObjectField { key: SymbolId, value: SlotValue, taint: Taint }` — per-field taint annotation
- **Assumptions**:
  - The taint lattice (`Clean < DerivedFromSecret < Secret`) is already implemented and stable in `vb_core::value`
  - `join_taint` is commutative, associative, has `Secret` as top and `Clean` as bottom
  - `RunFrame::read_taint` and `RunFrame::write_taint` require the slot to be initialized (enforced)
  - `EvalExpr` bytecode is already compiled and validated (bounds, stack) before runtime
  - `BuildObject` and `BuildList` use `build_object_with_taint` / `build_list_with_taint` which accumulate taint per field/item
  - `Finish` emits `EngineSignal::Finished(value, taint)` where taint is the result slot's taint (joined from all prior writes)
  - `Choose` / `ChooseSlot` do not accumulate taint because only one branch executes; the branch condition itself is boolean and has no secret leakage implication
- **Open questions**:
  - None — all domain types and propagation paths are located and understood

## Preconditions

- PRE-001: The `RunFrame` passed to `eval_expr_node`, `build_object_node`, `build_list_node`, `finish_run`, `choose_expr_branch`, and `choose_slot_branch` must have been constructed via `RunFrame::new` with slot count ≥ 1 and all slots initialized before use.
- PRE-002: The `CompiledWorkflow` passed to `eval_expr_node` and `choose_expr_branch` must have passed `validate_compiled_workflow` (bounds, node kind constraints, resource contract).
- PRE-003: The `ValueStore` passed to `build_object_node`, `build_list_node`, `choose_expr_branch`, and `eval_expr_node` must be non-null and valid for the duration of the call.
- PRE-004: For any `SlotIdx` passed to `read_slot`, `read_taint`, `write_slot`, or `write_slot_with_taint`, the index must be strictly less than the frame's `slot_count`.

## Postconditions

- POST-001: **EvalExpr taint propagation**: `eval_expr_node` calls `eval_expr_with_store` which returns `(SlotValue, Taint)`. The `Taint` returned is the `join` of all slot taints consumed by `LoadSlot` and `LoadAccessor` ops in the expression. The output slot is written via `write_slot_with_taint(output, value, taint)` where `taint` is exactly the joined taint. No `Clean` value can be produced when any input slot carries `Secret` or `DerivedFromSecret`.
- POST-002: **BuildObject taint propagation**: `build_object_node` calls `build_object_with_taint(store, run, fields)`. This reads every field slot's value and taint, joins all taints into `accumulated_taint`, stores the object, and returns `(ObjectId, accumulated_taint)`. The output slot receives `SlotValue::Object(handle)` and `accumulated_taint`. If any field slot has `Secret`, the object handle carries `Secret` taint.
- POST-003: **BuildList taint propagation**: `build_list_node` calls `build_list_with_taint(store, run, items)`. This reads every item slot's value and taint, joins all taints into `accumulated_taint`, stores the list, and returns `(ListId, accumulated_taint)`. The output slot receives `SlotValue::List(handle)` and `accumulated_taint`. If any item slot has `Secret`, the list handle carries `Secret` taint.
- POST-004: **Choose taint semantics**: `choose_expr_branch` evaluates expression conditions using `eval_expr_with_store` (taint is accumulated for the expression evaluation but the expression result is boolean and does not leak taint). `choose_slot_branch` reads boolean slot values. In both cases, the selected branch PC is set, but no taint is accumulated or emitted by the choose operation itself — only the branch that executes later produces taint.
- POST-005: **Finish taint propagation**: `finish_run` reads the result slot's value and taint via `read_slot` and `read_taint`, and returns `EngineSignal::Finished(value, taint)` with exactly the slot's taint. The taint is preserved through the signal and into `EngineSignal::Finished`.
- POST-006: **Copy slot taint preservation**: `copy_slot` (the `Copy` IR node helper) reads both the source value and source taint, then writes both to the destination slot via `write_slot_with_taint`. The destination slot taint equals the source slot taint.
- POST-007: **Action completion taint**: `resume_action_completion` writes `output_value` and `output_taint` to the designated slot. The taint written equals the `output_taint` argument, which must be at least as restrictive as the action's input taint per the action ABI contract.
- POST-008: **No taint desync**: After every successful `write_slot_with_taint`, `read_taint(slot)` returns exactly the taint that was written. A slot never carries a non-`Clean` taint without a corresponding value.

## Invariants

- INV-001: **Taint monotonicity**: Taint on any slot can only decrease (become more restrictive) if explicitly re-initialized to `Clean` via `reinitialize`. Within a single run, once a slot carries `DerivedFromSecret` or `Secret`, it cannot spontaneously become `Clean` without a `reinitialize` call.
- INV-002: **Taint lattice soundness**: `join_taint` applied across all taint sources always yields the lattice maximum of those sources. For any set of slots S, `accumulated_taint = fold join_taint over S` satisfies `accumulated_taint >= each s in S` in the lattice ordering.
- INV-003: **Slot/taint parallel arrays**: In `RunFrame`, `slots[i]` and `taint[i]` are always written together via `write_slot_with_taint`. Reading `taint[i]` without a corresponding `slots[i] = Some(...)` returns `SlotUninitialized`.
- INV-004: **Object/List field taint**: Each `ObjectField { value, taint, .. }` stored in `ValueStore` carries the taint of the slot that contributed the field value. `ValueStore` does not mutate field taint after insertion.
- INV-005: **Finish signal taint is result slot taint**: `EngineSignal::Finished(v, t)` is only emitted by `finish_run`. The `t` equals `read_taint(result_slot)` at the moment `finish_run` is called.
- INV-006: **DerivedFromSecret is not Secret**: A slot or field marked `DerivedFromSecret` has been computed from a secret (or derived from another `DerivedFromSecret`) but does not directly contain a secret value. Only `Secret` taint blocks finish output; `DerivedFromSecret` is allowed in finish results but triggers redaction at the UI/log/action boundary.
- INV-007: **No untainted wrapper around tainted content**: It is not permitted to construct a `Clean`-tainted object or list whose fields/items include `Secret` or `DerivedFromSecret` values. The `build_object_with_taint` and `build_list_with_taint` APIs make this impossible by construction.

## Error Taxonomy

All fallible operations return `Result<T, EngineError>` where `EngineError` is defined in `vb_core::errors`. The relevant variants for taint propagation paths are:

- `EngineError::SlotOutOfBounds { slot }` — when any slot index exceeds `slot_count`
- `EngineError::SlotUninitialized { slot }` — when reading an uninitialized slot's value or taint
- `EngineError::ExprOutOfBounds { expr }` — when `ExprIdx` does not exist in the workflow
- `EngineError::ConstOutOfBounds { index }` — when `ConstIdx` does not exist in the constant pool
- `EngineError::MissingOutputSlot { step }` — when an IR node declares no output slot but the handler expects one
- `EngineError::MissingNextStep { step }` — when `Choose` falls through with no `otherwise` and no branch matches
- `EngineError::TypeMismatch { expected, found }` — when a boolean condition evaluates to a non-boolean, or a slot read returns a type incompatible with the operation
- `EngineError::InternalInvariantViolation { reason }` — when an internal invariant (e.g., taint array length mismatch) is violated
- `EngineError::AllocationFailed` — when a `try_reserve_exact` fails during object/list construction
- `EngineError::StepCounterOverflow` — when `executed` transitions exceed `u64::MAX`
- `EngineError::InvalidProgramCounter { step }` — when PC is set to an out-of-bounds step
- `EngineError::UnsupportedPrimitive { primitive }` — when an unimplemented IR node kind is dispatched

**Critical security errors** (emit signals, do not panic):
- `EngineError::SecretResultLeak` — detected at compile time (validation gate 15) when a Finish node's result slot is `Secret`-tainted. Runtime enforcement is defense-in-depth.

## Contract Signatures

```rust
// vb_core/src/engine/expr_eval/core.rs
pub fn eval_expr_with_store(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &mut ValueStore,
    expr: ExprIdx,
) -> Result<(SlotValue, Taint), EngineError>

// vb_core/src/engine/object_list.rs
pub(crate) fn build_object_with_taint(
    store: &mut ValueStore,
    run: &crate::RunFrame,
    fields: &[(SymbolId, SlotIdx)],
) -> Result<(ObjectId, Taint), EngineError>

pub(crate) fn build_list_with_taint(
    store: &mut ValueStore,
    run: &crate::RunFrame,
    items: &[SlotIdx],
) -> Result<(ListId, Taint), EngineError>

// vb_core/src/engine/choose.rs
pub(super) fn choose_expr_branch(
    plan: &CompiledWorkflow,
    run: &mut crate::frame::RunFrame,
    store: &mut ValueStore,
    branches: &[ExprBranch],
    otherwise: Option<StepIdx>,
) -> Result<crate::EngineSignal, EngineError>

pub(super) fn choose_slot_branch(
    run: &mut crate::frame::RunFrame,
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
) -> Result<crate::EngineSignal, EngineError>

// vb_core/src/engine/node_helpers.rs
pub(super) fn finish_run(
    run: &mut crate::frame::RunFrame,
    result: SlotIdx,
) -> Result<EngineSignal, EngineError>

pub(super) fn copy_slot(
    run: &mut crate::frame::RunFrame,
    node: &crate::workflow::CompiledNode,
    source: SlotIdx,
) -> Result<EngineSignal, EngineError>

// vb_core/src/engine/step.rs
pub fn resume_action_completion(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    ticket: ActionTicket,
    output_slot: SlotIdx,
    output_value: SlotValue,
    output_taint: Taint,
) -> Result<(EngineSignal, ActionJournalEvent), EngineError>

// vb_core/src/frame.rs
pub fn read_taint(&self, slot: SlotIdx) -> CoreResult<Taint>
pub fn write_taint(&mut self, slot: SlotIdx, taint: Taint) -> CoreResult<()>
pub fn write_slot_with_taint(&mut self, slot: SlotIdx, value: SlotValue, taint: Taint) -> CoreResult<()>
```

## Non-goals

- Proving taint propagation through `Do` (action) nodes — covered by separate action ABI contract
- Proving taint propagation through `ForEach`, `Together`, `Collect`, `Reduce`, `Repeat` — separate compound primitive contracts
- Proving taint propagation through `AccessorProgram` path traversal beyond the existing `taint_accum` accumulation in `eval_load_accessor` (handled in `accessors.rs`)
- Proving compile-time `SecretResultLeak` detection (handled by `vb_validate` type_taint validation)
- Formal proof of the Kani model exhaustiveness for the full `CompiledNodeKind` enum — only targeted nodes are bounded-model-checked
