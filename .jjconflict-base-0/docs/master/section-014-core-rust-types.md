---
section: 14
title: "Core Rust Types"
parent: velvet-ballistics-MASTER.md
---

## 14. Core Rust Types


**Source of truth:** `crates/vb_core/src/`. The authoritative type layout is the code. This section states behavioral contracts that the code must satisfy. If code and doc disagree on field layout, the code wins. If code and doc disagree on behavior, the doc wins.

### `ids.rs` — Numeric ID Types

Required ID types (all `#[repr(transparent)]`, `Copy`, `Serialize`, `Deserialize`):

| Type | Inner | Purpose |
|------|-------|---------|
| `WorkflowId` | `u32` | Workflow identity |
| `RunId` | `u64` | Run identity |
| `StepIdx` | `u16` | Step/node index into `CompiledWorkflow.nodes` |
| `SlotIdx` | `u16` | Slot index into `RunFrame.slots` |
| `ExprIdx` | `u16` | Expression program index |
| `ActionId` | `u16` | Action identity |
| `AccessorIdx` | `u16` | Accessor program index |
| `ConstIdx` | `u16` | Constant pool index |
| `SymbolId` | `u32` | Interned string handle |
| `ListId` | `u32` | List arena handle |
| `ObjectId` | `u32` | Object arena handle |
| `BlobId` | `u64` | Blob arena handle |
| `SeqNo` | `u64` | Monotonic event sequence |
| `WorkflowDigest` | `[u8; 32]` | BLAKE3 digest |

Behavioral contracts:
- Table-index types (`StepIdx`, `SlotIdx`, `ExprIdx`, `AccessorIdx`, `ConstIdx`) must provide checked access to slices (via `CheckedIndex` trait or equivalent).
- No ID type may be constructed from unchecked arithmetic or unchecked casts.
- `WorkflowDigest` must provide `from_bytes` and `as_bytes` for storage interop.

### `value.rs` — Slot Value Model

Required types:

| Type | Contract |
|------|----------|
| `Taint` | Three-level lattice: `Clean < DerivedFromSecret < Secret`. `#[repr(u8)]` with explicit discriminants. Propagation rules in Section 47. |
| `FiniteF64` | Newtype over `f64`. Rejects NaN, +inf, -inf in both debug and release builds. Manual `Serialize`/`Deserialize` (not derive) to enforce rejection on decode. |
| `SlotValue` | Handle-only `Copy` enum: `Null`, `Bool(bool)`, `I64(i64)`, `F64(FiniteF64)`, `Symbol(SymbolId)`, `List(ListId)`, `Object(ObjectId)`, `Blob(BlobId)`. Must provide `type_name()` and `is_true()`. |
| `ConstValue` | Compile-time constant: `Null`, `Bool(bool)`, `I64(i64)`, `F64(FiniteF64)`, `Symbol(SymbolId)`. Must convert to `SlotValue` via `to_slot_value()` with no silent `Null` fallback. |

Behavioral contracts:
- `SlotValue` is handle-based; text and large payloads are referenced by handles, never stored inline.
- `FiniteF64::new` returns `CoreError::NonFiniteNumber` for non-finite inputs. No panic path.
- `ConstValue::to_slot_value` must map every variant; no default/fallback.

### `error.rs` — Core Error Types

Required error variants (the authoritative list is in the code; this lists the minimum):

```text
InvalidCompiledWorkflow { reason }
InvalidProgramCounter { step }
MissingNextStep { step }
MissingOutputSlot { step }
SlotOutOfBounds { slot }
ConstOutOfBounds { index }
ExprOutOfBounds { expr }
StepStateOutOfBounds { step }
ExpressionStackOverflow { max }
ExpressionStackUnderflow
UnsupportedPrimitive { primitive }
TypeMismatch { expected, found }
DivisionByZero
NonFiniteNumber
QueueFull
ResourceLimitExceeded { resource }
AllocationFailed
InternalInvariantViolation { reason }
```

All errors must be typed (no stringly errors), must carry diagnostic codes (Section 16), and must never require heap allocation in the hot path.

### `workflow.rs` — Compiled IR Types

Required types (authoritative layout in code):

| Type | Contract |
|------|----------|
| `CompiledWorkflow` | Immutable compiled artifact. Holds `nodes`, `expressions`, `accessors`, `constants`, `slot_count`, `entry: StepIdx`, `digest: WorkflowDigest`, `name`, `resource_contract`. Fields are private with getter methods. Constructed via `try_from_parts()` which validates all bounds. |
| `CompiledNode` | Single IR node: `id: StepIdx`, `output: Option<SlotIdx>`, `next: Option<StepIdx>`, `kind: CompiledNodeKind`. |
| `CompiledNodeKind` | 34+ variants covering all primitives (Section 15 lists them). The authoritative variant list is in the code. |
| `ExprProgram` | Postfix bytecode: `ops: Box<[ExprOp]>`, `max_stack: u8`. Stack effects validated by `check_expr_stack_bound`. |
| `ExprOp` | 30 opcodes: `LoadSlot`, `LoadConst`, `LoadAccessor`, comparison, logical, arithmetic, and helper ops (Section 46). |
| `AccessorProgram` | Path traversal: `root: SlotIdx`, `path: Box<[PathSegment]>` where `PathSegment = Field(SymbolId) \| Index(u32)`. |
| `ConstValue` | See `value.rs` above. |
| `ResourceContract` | 16 fields controlling hard limits (Section 13). |

Compiler rule: high-level YAML primitives may lower to multiple IR nodes. Runtime executes IR only in the current milestone. Final choose IR has exactly two checked forms: `Choose` evaluates expression-branch conditions from `ExprIdx`, and `ChooseSlot` reads pre-materialized boolean conditions from `SlotIdx` values produced by earlier IR. Raw YAML condition strings and untyped choose nodes are forbidden in final IR.

### `frame.rs` — Run Frame

`RunFrame` holds mutable execution state for a single run:

| Field | Type | Contract |
|-------|------|----------|
| `run_id` | `RunId` | Immutable after construction |
| `pc` | `StepIdx` | Program counter; set by `set_pc()` |
| `executed` | `u64` | Transition counter; incremented by deterministic steps |
| `states` | `Box<[StepState]>` | Per-step state machine; transitions validated (Section 45) |
| `slots` | `Box<[Option<SlotValue>]>` | Slot values; checked access only |
| `taint` | `Box<[Taint]>` | Per-slot taint; parallel to `slots` |

Behavioral contracts:
- `RunFrame::new` is the only constructor. Allocates exactly three boxed arrays. Rejects `step_count == 0` and out-of-range `first_step`. No arena/blob/symbol/journal allocation.
- `read_slot`/`write_slot`/`read_taint`/`write_taint` return `SlotOutOfBounds` for invalid indices.
- `mark_*` methods return `StepStateOutOfBounds` for invalid steps.
- Step-state transitions follow the contract in Section 45. Invalid transitions return `InternalInvariantViolation`.

### `engine.rs` — Execution Engine

Required types and functions:

| Type/Function | Contract |
|---------------|----------|
| `EngineSignal` | `Continue`, `Finished(SlotValue, Taint)`, `StepBudgetExhausted`, `AwaitingAction`, `AwaitingWait`, `AwaitingAsk` |
| `StepBudget` | Bounded step counter. `try_take() -> CoreResult<bool>`. Budget 0 returns `StepBudgetExhausted` immediately. |
| `step_once` | Execute single node dispatch. Returns `EngineSignal`. |
| `drive_deterministic` | Loop calling `step_once` until blocked by budget, suspension, or finish. |

`StepBudget` uses `remaining: u64`; `try_take() -> CoreResult<bool>`. Budget `0` executes zero transitions and returns `StepBudgetExhausted`. Budget `1` executes exactly one transition.

`EngineSignal::Finished(SlotValue, Taint)` carries taint from the result slot. The Finish node reads slot taint and propagates it to the signal. Validation does not reject `Secret` or `DerivedFromSecret` finish results; runtime preserves the result-slot taint in the signal.

---
