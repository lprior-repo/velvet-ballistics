# Contract Specification: vb-qi37.4.2

## Context

- **Feature**: Phase 3 formal contract for hot-path execution core (vb_core, vb_runtime, vb_storage, vb_ipc, vb_expr) and UI model envelope validation.
- **Domain terms**: Taint lattice, StepState machine, RunFrame, StepBudget, WholeWorkflowBudget, AggregateResourceBudget, EngineSignal, SlotValue, FiniteF64, IPC frame, Record decode, Journal replay.
- **Assumptions**:
  - All touched crates compile with `#![forbid(unsafe_code)]` in their root modules.
  - Taint join is total and defined for all Taint × Taint inputs.
  - StepBudget is a burn-down counter: non-negative, monotonically non-increasing.
  - RunFrame dimensions (step_count, slot_count) are fixed at construction and preserved across reinitialize.
  - IPC decoder rejects oversized frames before allocating any buffer.
  - Record decoder validates magic, schema, kind, payload_len, and CRC before allocating.
  - Journal write ordering is the single authoritative ordering for replay.
  - Concurrency is scoped to shard-level parallelism; cross-shard synchronization is forbidden in hot paths.
- **Open questions**:
  - Q1: Does VB-CORE-SIGNAL-001 canonical form (SlotValue, Taint) override legacy spec sections claiming just SlotValue? Resolution: canonical form is Finished(SlotValue, Taint).
  - Q2: Are there legacy spec sections claiming taint Always Clean for BuildObject/BuildList that contradict VB-CORE-TAINT-006? Resolution: yes, DRIFT-SECTION-68 recorded; VB-CORE-TAINT-006 requires join from source operands.
  - Q3: Is budget arithmetic for loop composition (RESOURCE-003) proven free of intermediate overflow in Verus or is Kani bounded? Resolution: the contract requires saturating semantics at the policy maximum; Verus L4 proves no panic/wrap and policy-bounded outputs, not unbounded mathematical no-overflow.

---

## Preconditions

- **PRE-001**: `RunFrame::new(run_id, first_step, step_count, slot_count)` requires `step_count > 0` and `first_step.as_usize() < step_count`.
- **PRE-002**: `WholeWorkflowBudget::compute(nodes, entry, contract)` requires `entry.as_usize() < nodes.len()`.
- **PRE-003**: `FiniteF64::new(value)` requires `value.is_finite()` (not NaN, not ±infinity).
- **PRE-004**: IPC frame decode requires `header_len >= 60` and `payload_len <= MAX_PAYLOAD` before any buffer allocation.
- **PRE-005**: Record decode requires `magic` validation, `schema` validation, `kind` validation, and `payload_len` validation before any deserialization.
- **PRE-006**: `AggregateResourceBudget::try_take(budget, amount)` requires `amount <= budget.remaining`.

---

## Postconditions

- **POST-001**: `RunFrame::new` returns `Ok(frame)` with `states.len() == step_count`, `slots.len() == slot_count`, `taint.len() == slot_count`, all states = Pending, all taint = Clean.
- **POST-002**: `join_taint(a, b)` returns the higher taint level: Clean < DerivedFromSecret < Secret, and satisfies associativity, commutativity, idempotence, and identity laws.
- **POST-003**: `StepBudget::try_take` returns `Ok(remaining)` where `remaining == old_remaining - amount` or `Err(StepBudgetExhausted)` if `amount > remaining`. Remaining is monotonically non-increasing.
- **POST-004**: `EngineSignal::Finished` carries exactly `(SlotValue, Taint)` in canonical form.
- **POST-005**: StepState transitions obey the valid transition map: Pending → {Running, Succeeded, Failed, Cancelled, Skipped}; Running → {Succeeded, Failed, Waiting, Asking, Cancelled, Skipped}; Waiting → {Running}; Asking → {Running}; terminal states (Succeeded, Failed, Cancelled, Skipped) → themselves only.
- **POST-006**: `WholeWorkflowBudget::compute` returns a budget where every field ≤ the corresponding `BoundednessPolicy::DEFAULT` limit.
- **POST-007**: IPC decoder returns `Err` before allocating any buffer when `header_len < 60` or `payload_len > MAX_PAYLOAD`.
- **POST-008**: Record decoder returns `Err` before any heap allocation when any validation (magic, schema, kind, payload_len, CRC) fails.
- **POST-009**: Journal entry sequence numbers are strictly monotonically increasing per shard.
- **POST-010**: `AggregateResourceBudget` sequential composition uses saturating add at the policy maximum, branch composition uses max, and loop composition uses saturating multiply at the policy maximum. No intermediate arithmetic may panic, wrap, or exceed the externally visible `BoundednessPolicy::DEFAULT` limits.

---

## Invariants

- **INV-001**: Taint lattice join is associative: `join(join(a,b),c) == join(a, join(b,c))` for all a,b,c ∈ Taint.
- **INV-002**: Taint lattice join is commutative: `join(a,b) == join(b,a)` for all a,b ∈ Taint.
- **INV-003**: Taint lattice join is idempotent: `join(a,a) == a` for all a ∈ Taint.
- **INV-004**: Taint lattice has identity Clean: `join(Clean, a) == a` and `join(a, Clean) == a` for all a ∈ Taint.
- **INV-005**: Taint lattice has no downward path from Secret: `join(Clean, Secret) == Secret` and `join(Secret, anything) == Secret`.
- **INV-006**: Taint lattice has no downward path from DerivedFromSecret: `join(Clean, DerivedFromSecret) == DerivedFromSecret`.
- **INV-007**: RunFrame dimensions are immutable after construction or reinitialize: `step_count` and `slot_count` never change.
- **INV-008**: StepBudget remaining is always ≥ 0 and never increases.
- **INV-009**: All StepIdx, SlotIdx, ExprIdx, ConstIdx, AccessorIdx accesses use checked conversions; raw `as_usize()` followed by direct indexing is forbidden in hot-path code.
- **INV-010**: `EngineSignal::Finished` always carries a Taint value; no legacy `Finished(SlotValue)` form is produced by the engine.
- **INV-011**: IPC header validation rejects before allocation: `header_len < 60` → `Err`, `payload_len > MAX_PAYLOAD` → `Err`.
- **INV-012**: Record magic, schema, kind, payload_len, and CRC are all validated before any heap allocation or deserialization.
- **INV-013**: Journal entries are written before the corresponding action dispatch (journal-before-dispatch).
- **INV-014**: Idempotency keys are well-formed per `idempotency_key_well_formed`.
- **INV-015**: Each shard has a single owner; no cross-shard mutable aliasing in hot-path frames.

---

## Error Taxonomy

- `CoreError::NonFiniteNumber` — FiniteF64::new receives NaN or infinity.
- `CoreError::InvalidCompiledWorkflow { reason: "step_count_zero" }` — RunFrame::new receives step_count == 0.
- `CoreError::InvalidProgramCounter { step }` — first_step >= step_count.
- `CoreError::InvalidProgramCounter { step }` — reinitialize first_step >= step_count.
- `CoreError::InvalidCompiledWorkflow { reason: "frame_dimension_mismatch" }` — reinitialize dimensions differ from construction.
- `CoreError::InternalInvariantViolation { reason: "invalid_state_transition" }` — rejected StepState transition.
- `EngineError::StepBudgetExhausted` — StepBudget try_take amount > remaining.
- `WorkflowError::EntryOutOfBounds { entry }` — WholeWorkflowBudget::compute entry >= node_count.
- `WorkflowError::StepCountOverflow { actual }` — step count does not fit in u32.
- `IpcError::HeaderTooShort` — header_len < 60.
- `IpcError::PayloadTooLarge` — payload_len > MAX_PAYLOAD.
- `IpcError::MagicMismatch` — IPC magic validation failure.
- `StorageError::RecordMagicInvalid` — record magic validation failure.
- `StorageError::RecordSchemaInvalid` — record schema validation failure.
- `StorageError::RecordKindInvalid` — record kind validation failure.
- `StorageError::RecordPayloadLenInvalid` — record payload_len out of range.
- `StorageError::RecordCrcInvalid` — record CRC mismatch.

---

## Contract Signatures

```rust
// vb_core::value
pub fn join_taint(a: Taint, b: Taint) -> Taint
pub struct FiniteF64(f64);
impl FiniteF64 { pub fn new(value: f64) -> CoreResult<Self> }

// vb_core::frame
pub struct RunFrame { /* ... */ }
impl RunFrame {
    pub fn new(run_id: RunId, first_step: StepIdx, step_count: u16, slot_count: u16) -> CoreResult<Self>
    pub fn reinitialize(&mut self, run_id: RunId, first_step: StepIdx, step_count: u16, slot_count: u16) -> CoreResult<()>
}
pub enum StepState { Pending, Running, Succeeded, Failed, Skipped, Waiting, Asking, Cancelled }

// vb_core::engine
pub struct StepBudget { remaining: u32 }
impl StepBudget {
    pub fn try_take(&mut self, amount: u32) -> Result<u32, StepBudgetExhausted>
    pub fn is_exhausted(&self) -> bool
}

// vb_core::budget
pub struct WholeWorkflowBudget { /* ... */ }
impl WholeWorkflowBudget {
    pub fn compute(nodes: &[CompiledNode], entry: StepIdx, contract: &ResourceContract) -> Result<Self, WorkflowError>
}
pub struct BoundednessPolicy { /* ... */ }
impl BoundednessPolicy { pub const DEFAULT: Self }

// vb_core::signals
pub enum EngineSignal {
    Running,
    Waiting { on: WaitToken },
    Asking { ticket: AskTicket },
    Finished(SlotValue, Taint),  // canonical form per spec
    StepBudgetExhausted,
}

// vb_ipc::frame
pub struct IpcFrameDecoder { /* ... */ }
impl IpcFrameDecoder {
    pub fn decode_header(bytes: &[u8]) -> Result<Header, IpcError>
    pub fn decode_frame(bytes: &[u8]) -> Result<Frame, IpcError>  // rejects before allocation
}

// vb_storage::record
pub struct RecordDecoder { /* ... */ }
impl RecordDecoder {
    pub fn decode(bytes: &[u8]) -> Result<Record, StorageError>  // validates before allocating
}
```

---

## Verus-Owned Clauses

All Rust-local pure/core proof obligations are owned by Verus:

- **INV-001, INV-002, INV-003, INV-004, INV-005, INV-006** (Taint lattice laws) → Verus at L4
- **INV-008** (StepBudget monotonicity) → Verus at L4
- **INV-007** (RunFrame dimension immutability) → Verus at L4
- **INV-010** (EngineSignal Finished canonical form) → Verus at L4
- **VB-CORE-RESOURCE-001, VB-CORE-RESOURCE-002, VB-CORE-RESOURCE-003** (resource budget saturating arithmetic and policy-bounded outputs) → Verus at L4
- **VB-CORE-RUNFRAME-001, VB-CORE-RUNFRAME-002, VB-CORE-RUNFRAME-003** (RunFrame constructor/reinitialize preconditions, postconditions, and dimension immutability) → Verus at L4 + Kani at L3
- **VB-CORE-IDEMPOTENCY-001** (idempotency key well-formedness) → Kani/property evidence at L3/L1
- **VB-CORE-STATE-001** (valid StepState transitions) → Verus at L4 + Kani at L3
- **VB-CORE-BUDGET-003** (try_take never underflows) → Verus at L4

---

## TLA+-Owned Clauses

- **INV-013** (journal-before-dispatch ordering) → TLA+ at L3 via `LifecycleJournal.tla`
- **VB-REPLAY-001 to VB-REPLAY-007** (journal/replay safety) → TLA+ at L3
- **VB-CONC-001 to VB-CONC-005** (concurrency/shard ownership) → TLA+ + Loom at L3

---

## Theorem-Owned Clauses

- None. The taint lattice and resource budget arithmetic are fully expressible in Verus; no Lean/Aeneas theorem kernel required.

---

## Non-goals

- Formal proof of generated Rust code output (vb_codegen) — covered by differential testing.
- UI rendering correctness (makepad) — covered by integration tests.
- Fjall compaction internals — covered by Fjall's own test suite and crash-lab.
- Supply-chain audit beyond L0/L6 gates — handled separately.
