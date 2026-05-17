<<<<<<< HEAD
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
=======
# Contract Specification

Bead: `vb-qi37.4.2` - runtime: Enforce admission gate before run creation.

## Context

- Source artifacts read: `baseline-report.md`, `codebase-map.md`, `delivery-scope.jsonl`, and `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-qi37.4.2 --json`.
- Feature: strict runtime admission must accept only durable accepted-artifact envelopes and must reject raw, failed, stale, malformed, digest-mismatched, or under-verified artifacts before runtime state allocation.
- Domain terms:
  - Accepted artifact: postcard-encoded `AcceptedArtifact` with digest, gate evidence, durable flag, capability evidence, and schema/version evidence.
  - Raw artifact: `WorkflowParts`, YAML, JSON, malformed bytes, or any artifact lacking the accepted envelope.
  - Strict/journaled runtime: production admission path that must use storage-backed artifact loading, not dummy existence-only stores.
  - Admission boundary: point before `take_frame_for`, `self.runs.insert`, `drive_run`, `RunAccepted`, or any API/CLI/IPC success acknowledgement.
- Assumptions:
  - Canonical accepted-artifact gate count is `15` until the upstream contract explicitly changes it.
  - `vb-qi37.4.1` owns the envelope definition and is closed; this bead owns runtime enforcement of that envelope before run creation.
  - Dependent beads own atomic Fjall batch durability and production `StorageArtifactStore` wiring, but this bead must not permit a bypass that would make those dependents meaningless.
- Open questions:
  - Whether runtime diagnostics should introduce finer public variants or preserve existing variants with structured detail.
  - Whether inner IR digest validation and envelope digest validation must both be enforced at the same runtime boundary.

## Preconditions

- PRE-001: Strict or journaled run creation input MUST identify a persisted accepted-artifact envelope by digest; raw `WorkflowParts`, YAML, JSON, or opaque bytes are not admissible runtime inputs.
- PRE-002: The loaded envelope MUST decode as accepted-artifact v1 and MUST carry canonical `gate_count == 15` with all required gate proof flags accepted.
- PRE-003: The envelope digest MUST match the requested artifact digest and the persisted compiled-IR record digest; mismatch is a hard admission failure. State3 does not claim an executable Kani harness because `verification/kani/digest_admission_harness.rs` is absent; the contract requires integration/domain tests as compensating evidence until a later proof-writing state creates a bounded harness.
- PRE-004: The envelope MUST be durable and non-stale according to its admission metadata; relaxed artifacts with `gate_count == 0`, `durable == false`, or stale certificate data are rejected.
- PRE-005: Capability grants in the envelope MUST exactly cover the workflow-required capability profile: no missing, excess, prefix, action-mismatched, or duplicate grants.
- PRE-006: Strict/journaled production constructors MUST use a storage-backed accepted-artifact loader or an equivalent verified source; `AlwaysPresentArtifactStore` is permitted only for relaxed/test-only contexts.

## Postconditions

- POST-001: Valid accepted artifacts proceed to run creation without runtime YAML or JSON parsing.
- POST-002: Raw, malformed, failed-gate, stale, non-durable, digest-mismatched, or capability-mismatched artifacts return typed admission diagnostics.
- POST-003: Any admission failure occurs before runtime state allocation: no frame is taken, no run is inserted, no runnable state exists, no `drive_run` occurs, and no `RunAccepted` is emitted.
- POST-004: Rejected diagnostics preserve the rejected digest and semantic cause, including malformed envelope, missing artifact, failed gate, stale certificate, digest mismatch, and capability denial.
- POST-005: Successful admission records the artifact digest, admission certificate/profile, and initial metadata needed by downstream header-persistence work.

## Invariants

- INV-001: There is exactly one canonical accepted-artifact gate-count contract shared by runtime and storage for strict admission.
- INV-002: Existence-only artifact checks cannot satisfy strict admission.
- INV-003: Admission is fail-closed: unknown schema version, missing field, decode failure, stale evidence, or unsupported proof status denies.
- INV-004: Strict/journaled admission never depends on runtime YAML/JSON parsing.
- INV-005: Denied admission cannot allocate or expose runnable state.
- INV-006: Capability checking is exact-cardinality and exact-name/action.
- INV-007: Diagnostics are typed and non-lossy enough for API/CLI/IPC callers to distinguish accepted-envelope failures from storage-not-found and capability denial.

## Error Taxonomy

- ERR-001 `AdmissionError::ArtifactNotFound` - requested digest has no persisted accepted artifact. Expected scenario: `given_missing_artifact_when_strict_run_created_then_artifact_not_found_before_allocation`; diagnostic preserves requested digest and performs no allocation.
- ERR-002 `AdmissionError::ArtifactEnvelopeDecodeFailed` - artifact bytes are raw, malformed, truncated postcard, YAML, JSON, or not accepted-envelope v1. Expected scenario: `given_raw_or_malformed_bytes_when_strict_run_created_then_decode_failed_with_rejected_digest`; diagnostic preserves rejected digest and decode/malformed cause.
- ERR-003 `AdmissionError::ArtifactEnvelopeInvalid` - envelope decodes but lacks required fields, durable marker, schema support, or accepted proof flags. Expected scenario: `given_decoded_envelope_missing_required_acceptance_fields_then_invalid_envelope_denies`; diagnostic names invalid envelope cause and performs no allocation.
- ERR-004 `AdmissionError::ArtifactGateMismatch` - gate count or gate status does not satisfy canonical strict admission. Expected scenario: `given_gate_count_zero_two_or_failed_status_when_strict_run_created_then_gate_mismatch_denies`; diagnostic records observed gate evidence and required canonical gate.
- ERR-005 `AdmissionError::ArtifactDigestMismatch` - requested digest, persisted record digest, or envelope digest disagree. Expected scenario: `given_digest_mismatch_when_strict_run_created_then_digest_mismatch_denies`; diagnostic records requested and observed digest identities without collapsing to invalid envelope.
- ERR-006 `AdmissionError::ArtifactStale` - certificate/evidence is stale for the required runtime profile. Expected scenario: `given_stale_artifact_when_strict_run_created_then_stale_certificate_denies`; diagnostic preserves staleness cause and rejected digest.
- ERR-007 `AdmissionError::CapabilityDenied` - required capability profile is not exactly granted. Expected scenario: `given_missing_excess_prefix_or_action_mismatched_capability_then_capability_denied`; diagnostic preserves required/granted mismatch class.
- ERR-008 `RuntimeError::AdmissionArtifactNotFound`, `RuntimeError::AdmissionArtifactInvalid`, and `RuntimeError::AdmissionCapabilityDenied` mappings MUST preserve the underlying `AdmissionError` category, rejected digest when present, and semantic cause. Expected scenario: `given_cli_ipc_runtime_error_mapping_when_serialized_then_error_category_digest_and_cause_are_preserved`.

## Contract Signatures

- `AcceptedArtifactStore::load_accepted_artifact(digest: ArtifactDigest) -> Result<AcceptedArtifact, AdmissionError>`
- `admit_artifact_run(store: &dyn AcceptedArtifactStore, digest: ArtifactDigest, required: CapabilityProfile, policy: AdmissionPolicy) -> Result<AdmissionRecord, AdmissionError>`
- `build_admission(run_id: RunId, digest: ArtifactDigest, required: CapabilityProfile) -> Result<AdmissionRecord, RuntimeError>`
- `Runtime::new_with_journal_and_artifact_store(journal: Journal, store: StorageArtifactStore) -> Result<Runtime, RuntimeError>`

## Verus-Owned Clauses

- PRE-005, INV-006: exact capability name/action and exact cardinality, using existing `verification/verus/capability_artifact_model.rs`.
- PRE-002, PRE-004, INV-001, INV-003: accepted-envelope gate/status/durable pure predicate uses `verification/verus/accepted_envelope_model.rs`, verified by `verus verification/verus/accepted_envelope_model.rs` in State5 evidence.

## TLA+-Owned Clauses

- POST-003, INV-005: denied admission leaves no run allocation or journaled accepted state.
- PRE-002, INV-001: gate mismatch denies.
- PRE-006, INV-002: legacy/dummy bypass cannot admit protected strict submissions.
- PRE-005, INV-006: capability cardinality mismatch denies before allocation.

## Theorem-Owned Clauses

- None at State 3. Verus is sufficient for Rust-local predicates. Lean/Aeneas/Hax is a non-goal unless proof-review identifies a tiny algebraic kernel that Verus cannot express.

## Non-goals

- Implementing production code, proof code, or tests in State 3.
- Claiming performance improvement; this bead is correctness/admission only.
- Owning atomic Fjall batch persistence after successful admission; that is dependent bead `vb-core-atomic-admission`.
- Claiming State3 executable Kani/fuzz/cargo-mutants/CI passes where no harness, target, diagnostic tests, or CI run exists. The contract ledger keeps these as `status: planned`; any WAIVED/DEFERRED result belongs only in downstream execution evidence artifacts with owner, reason, expiry, limitation, and compensating evidence.
>>>>>>> origin/go-skill-p0-vb-qi37-4-2
