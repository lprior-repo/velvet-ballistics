<<<<<<< HEAD
# Domain Model Review: vb-qi37.4.2

## Review Scope

vb_core hot-path core types: Taint lattice, StepState machine, RunFrame, StepBudget, WholeWorkflowBudget, AggregateResourceBudget, EngineSignal, SlotValue, FiniteF64.

---

## Taint Lattice

### Type Definition

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Taint {
    Clean = 0,
    DerivedFromSecret = 1,
    Secret = 2,
}
```

### Join Function

```rust
pub fn join_taint(a: Taint, b: Taint) -> Taint {
    let a_disc: u8 = match a {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
    };
    let b_disc: u8 = match b {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
    };
    if a_disc >= b_disc { a } else { b }
}
```

### Review Findings

- **Correct**: Discrete u8 mapping gives total order 0 < 1 < 2 matching the lattice semantics.
- **Correct**: Join returns the higher discriminant; ties favor `a` (deterministic).
- **Correct**: `#[repr(u8)]` ensures no enum layout uncertainty.
- **Correct**: `#[derive(PartialEq, Eq)]` supports lattice proof.
- **Correct**: `#[derive(Clone, Copy)]` — Taint is trivially copyable (tiny, no Drop).
- **Correct**: `Serialize`/`Deserialize` derived — no custom implementation risk.
- **Issue DRIFT-SECTION-68**: Legacy spec sections claimed `BuildObject`/`BuildList` always produce `Clean` taint. The corrected behavior requires joining from source operands (VB-CORE-TAINT-006). This is a spec contradiction that must be tracked.
- **Issue DRIFT-SIGNAL**: Legacy spec sections claim `Finished(SlotValue)` while corrected spec (and implementation) uses `Finished(SlotValue, Taint)`. Canonical form is `Finished(SlotValue, Taint)` per VB-CORE-SIGNAL-001.

---

## StepState Machine

### Type Definition

```rust
pub enum StepState {
    Pending, Running, Succeeded, Failed,
    Skipped, Waiting, Asking, Cancelled,
}
```

### Valid Transitions

| From | Allowed To |
|------|------------|
| Pending | Running, Succeeded, Failed, Cancelled, Skipped |
| Running | Succeeded, Failed, Waiting, Asking, Cancelled, Skipped |
| Waiting | Running |
| Asking | Running |
| Succeeded | Succeeded (idempotent) |
| Failed | Failed (idempotent) |
| Cancelled | Cancelled (idempotent) |
| Skipped | Skipped (idempotent) |

### Review Findings

- **Correct**: Terminal states (Succeeded, Failed, Cancelled, Skipped) are self-loop only — immutable after reaching terminal.
- **Correct**: Non-terminal states have explicit exit transitions; no implicit fallthrough.
- **Correct**: Waiting/Askingsuspending states only re-enter via Running.
- **Correct**: `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` — all states are value-equal, no identity.
- **Issue**: No Verus spec currently proves the full transition matrix. VB-CORE-STATE-001 is the formal obligation.
- **Issue**: Invalid transitions must return `InternalInvariantViolation { reason: "invalid_state_transition" }` — enforced by tests (VB-CORE-STATE-003) but not yet in Verus.

---

## RunFrame

### Construction Invariants

```rust
pub fn new(run_id: RunId, first_step: StepIdx, step_count: u16, slot_count: u16) -> CoreResult<Self> {
    let states_len = usize::from(step_count);
    if states_len == 0 { return Err(CoreError::InvalidCompiledWorkflow{reason:"step_count_zero"}); }
    if first_step.as_usize() >= states_len { return Err(CoreError::InvalidProgramCounter{step: first_step}); }
    // ...
}
```

### Review Findings

- **Correct**: Precondition `step_count > 0` enforced.
- **Correct**: Precondition `first_step < step_count` enforced.
- **Correct**: Arrays are heap-allocated with exact size: `states: Box<[StepState]>`, `slots: Box<[Option<SlotValue>]>`, `taint: Box<[Taint]>`.
- **Correct**: `parallel_in_flight` starts at 0; `max_parallel_in_flight` defaults to `u16::MAX` (conservative).
- **Correct**: `reinitialize` enforces dimension immutability: `step_count` and `slot_count` must match construction.
- **Correct**: `executed` counter tracks total transitions executed.
- **Issue INV-007**: RunFrame dimension immutability after construction is not yet formally proven in Verus. This is a critical invariant for memory safety (bounds are trusted without rechecking on each access).

---

## StepBudget

### Type and Core Operation

```rust
pub struct StepBudget { remaining: u32 }
impl StepBudget {
    pub fn try_take(&mut self, amount: u32) -> Result<u32, StepBudgetExhausted>
    pub fn is_exhausted(&self) -> bool
}
```

### Review Findings

- **Correct**: Budget is a burn-down counter — monotonically non-increasing.
- **Correct**: `try_take` returns remaining on success (old_remaining - amount).
- **Correct**: `try_take` returns `Err(StepBudgetExhausted)` when amount > remaining — no underflow.
- **Correct**: `is_exhausted` is a cheap check before attempting `try_take`.
- **Issue INV-008**: Monotonicity (remaining never increases) is not yet formally proven in Verus. Kani at L3 covers bounded underflow but not the full inductive proof.
- **Issue**: `StepBudget` is a plain u32 wrapper — no overflow protection on construction. Overflow can only happen via `try_take` abuse (amount > u32::MAX). Since `try_take` checks `amount > remaining`, overflow is impossible in correct usage.

---

## WholeWorkflowBudget

### Type

```rust
pub struct WholeWorkflowBudget {
    pub max_total_steps: u64,
    pub max_total_slots: u64,
    pub max_fanout: u16,
    pub max_nesting_depth: u16,
    pub max_steps_executable: u32,
    pub max_action_tickets: u32,
    pub max_parallel_in_flight: u16,
    pub max_retries_per_action: u16,
    pub max_gather_pages: u32,
    pub max_gather_items: u32,
    pub max_for_each_iterations: u32,
    pub max_together_branches: u16,
    pub max_repeat_attempts: u16,
    pub max_run_time_seconds: u64,
    pub max_result_bytes: u32,
    pub max_total_slots_written: u32,
}
```

### Review Findings

- **Correct**: All fields are concrete primitive types with no interior mutability.
- **Correct**: `max_total_steps` is u64 — accommodates large workflows before narrowing to `max_steps_executable: u32`.
- **Correct**: `max_steps_executable` conversion uses `u32::try_from` with explicit overflow error (not `as` cast).
- **Correct**: `BoundednessPolicy::DEFAULT` provides hard limits for admission control.
- **Issue POST-006**: Budget-to-policy comparison is not yet formally verified. VB-CORE-RESOURCE-004 is the formal obligation.

---

## EngineSignal

### Variants

```rust
pub enum EngineSignal {
    Running,
    Waiting { on: WaitToken },
    Asking { ticket: AskTicket },
    Finished(SlotValue, Taint),  // canonical
    StepBudgetExhausted,
}
```

### Review Findings

- **Correct**: `Finished` carries both `SlotValue` and `Taint` — the corrected canonical form.
- **Issue DRIFT-SIGNAL**: Legacy spec sections reference `Finished(SlotValue)` without Taint. This is a spec contradiction. All new code must produce `Finished(SlotValue, Taint)`. Legacy references are marked as DRIFT.
- **Correct**: Other variants are purely control-flow signals without data payload.

---

## FiniteF64

### Construction

```rust
impl FiniteF64 {
    pub fn new(value: f64) -> CoreResult<Self> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err(CoreError::NonFiniteNumber)
        }
    }
}
```

### Review Findings

- **Correct**: Rejects NaN and infinities in both debug and release builds (unlike `noisy_float`).
- **Correct**: Zero dependencies — avoids `num-traits`, `ordered-float`, `noisy_float`.
- **Correct**: `#[repr(transparent)]` guarantees layout compatibility with `f64`.
- **Correct**: `new` returns `CoreResult<Self>` — caller must handle the error case.
- **Issue**: The `get` accessor is `const fn` returning raw `f64` — callers must re-check finiteness if they pass the value back to `new`.

---

## Numeric ID Types (ids/mod.rs)

### Key Types

```rust
pub struct WorkflowId(pub u32);
pub struct StepIdx(pub u16);
pub struct StepIdx(pub u16);
pub struct ExprIdx(pub u16);
pub struct ActionId(pub u16);
pub struct SlotIdx(pub u16);
pub struct RunId(pub u64);
pub struct EventSeq(pub u64);
pub struct SeqNo(pub u64);
```

### Review Findings

- **Correct**: Newtype wrappers prevent mixing indices across domains.
- **Correct**: `as_usize()` is the checked conversion path; raw `as_usize()` then index is forbidden in hot paths (INV-009, VB-CORE-IDX-002).
- **Issue VB-CORE-IDX-001**: Checked index access is the only sanctioned path. The `forbidden-scan` pattern `as_usize.*index` catches violations in hot-path files.

---

## IPC Frame Decoder (vb_ipc::frame)

### Header Format

- Fixed header length: 60 bytes.
- Fields: magic, version, payload_len, etc.
- Reject-before-allocate policy.

### Review Findings

- **Correct**: `header_len` validation rejects before any buffer allocation.
- **Correct**: `payload_len` upper bound prevents DoS via oversized allocations.
- **Issue**: The exact MAX_PAYLOAD constant must be documented and enforced at the boundary.

---

## Record Decoder (vb_storage::record)

### Decode Pipeline

1. Read magic (4 bytes) — validate before anything else.
2. Read schema (4 bytes) — validate.
3. Read kind (4 bytes) — validate.
4. Read payload_len (8 bytes) — validate range.
5. Read payload (payload_len bytes) — allocate now.
6. Read CRC (4 bytes) — validate against BLAKE3 of payload.

### Review Findings

- **Correct**: Each validation step is a hard error before proceeding to the next.
- **Correct**: CRC uses BLAKE3 (fast, secure) — not MD5 or SHA-1.
- **Correct**: All validation errors are caught before heap allocation.
- **Issue**: The decode pipeline order must be preserved exactly; reordering steps could cause a use-after-free if kind is trusted before magic.

---

## Risk Tags and Critical Invariants

| Risk Tag | Critical Invariant | Status |
|----------|-------------------|--------|
| taint_lattice | INV-001 to INV-006: lattice laws hold for all inputs | Verus L4 pending |
| step_state_machine | INV-007, INV-010: valid transitions, canonical signals | Verus L4 pending |
| budget_arithmetic | INV-008: StepBudget monotonic, no underflow | Verus L4 pending |
| index_safety | INV-009: no unchecked indexing in hot paths | L0 scan + Kani L3 |
| journal_ordering | INV-013: journal-before-dispatch | TLA+ L3 pending |
| replay_safety | VB-REPLAY-001 to 007: journal replay correctness | TLA+ L3 pending |
| concurrency | INV-015: single shard owner, no cross-shard aliasing | TLA+ + Loom L3 pending |
| record_decode | INV-012: validate before allocate | Kani L3 + fuzz |
| ipc_decoder | INV-011: reject before allocate | Kani L3 + fuzz |
| denial_of_service | IPC/Record payloads bounded before allocation | Kani L3 |
=======
# Domain Model Review

Bead: `vb-qi37.4.2`

## Decision

STATUS: READY_FOR_CONTRACT_REVIEW

The contract model separates four concepts that must not be conflated:

1. Artifact existence: a digest has bytes in storage.
2. Accepted envelope validity: bytes decode as accepted-artifact v1 with required gates.
3. Admission authorization: envelope profile exactly matches runtime requirements.
4. Run allocation: runtime state may be created only after 1-3 pass.

## Illegal States Made Explicit

- A raw `WorkflowParts` artifact cannot be an accepted artifact.
- A relaxed artifact with `gate_count == 0` cannot enter strict runtime admission.
- A storage-submitted artifact with `gate_count == 2` cannot enter strict runtime admission while runtime requires `15`.
- A digest-mismatched envelope cannot be admitted even if it decodes.
- A denied admission cannot have `RunAccepted`, runnable state, or allocated frame.
- `AlwaysPresentArtifactStore` cannot prove strict production admission.

## Required Type Boundaries

- `AcceptedArtifact` must be a distinct validated domain type, not bytes plus convention.
- `AdmissionRecord` must be created only from validated accepted artifacts.
- `AdmissionError` must preserve semantic rejection causes.
- Runtime constructors must distinguish relaxed/test admission from strict/journaled production admission.

## Review Risks

- Gate-count disagreement between `vb_storage::ADMISSION_GATE_COUNT == 2` and `vb_runtime::REQUIRED_GATE_COUNT == 15` is a blocking domain ambiguity for implementation unless normalized.
- Current diagnostics may be too coarse if `AdmissionArtifactInvalid` cannot report raw/malformed/stale/digest mismatch distinctly.
- Existing IPC resolver behavior that decodes `record.ir` as `WorkflowParts` conflicts with storage persisting `AcceptedArtifact` bytes.

## Hand-off

- Proof planner must either extend the existing capability Verus/TLA assets or create separate accepted-envelope models.
- Implementation must not use existence-only APIs as strict admission proof.
>>>>>>> origin/go-skill-p0-vb-qi37-4-2
