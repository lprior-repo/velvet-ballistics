# Contract Specification — vb-qi37.5.3

## Context

- **Feature**: Carry idempotency evidence from `VerificationProof` into `RunAdmission`
- **Bead**: runtime: Carry idempotency evidence into admission
- **Gap**: `RunAdmission` does NOT carry `idempotency_keyed` or `idempotency_attested` fields from `VerificationProof`
- **Domain terms**: RunAdmission, VerificationProof, IdempotencyTracker, AcceptedArtifact, StorageArtifactStore, ArtifactEnvelopeError
- **Assumptions**:
  - `VerificationProof.idempotency_keyed: Box<[ActionId]>` and `idempotency_attested: Box<[ActionId]>` already exist in vb_storage
  - `RunAdmission` is constructed via `admit_artifact_run` which loads an `AcceptedArtifact` from `StorageArtifactStore`
  - `IdempotencyTracker` already exists in vb_runtime and uses HashMap; thread-safety is unverified
- **Open questions**:
  - Should idempotency evidence be embedded directly in `RunAdmission` or referenced by index?
  - Should `IdempotencyTracker` be pre-populated at admission time or lazily populated?
  - Does `IdempotencyTracker` need `Send + Sync` for multi-shard use?
  - Should `RunFrame` embed a reference to `RunAdmission` or its idempotency subset?

---

## Preconditions

- PRE-01: `admit_artifact_run` receives a valid `ArtifactEnvelope` that passes `ArtifactEnvelopeError` validation
- PRE-02: `StorageArtifactStore::load_accepted_artifact` must successfully load an `AcceptedArtifact` with a valid `VerificationProof`
- PRE-03: The `AcceptedArtifact`'s `VerificationProof` must have non-null `idempotency_keyed` and `idempotency_attested` fields (may be empty slices, not null)

## Postconditions

- POST-01: `RunAdmission` returned from `admit_artifact_run` MUST contain the `idempotency_keyed` and `idempotency_attested` fields copied from the loaded `AcceptedArtifact.VerificationProof`
- POST-02: The fields MUST be stored as `Box<[ActionId]>` matching the type used in `VerificationProof`
- POST-03: All existing `RunAdmission` fields (`artifact_digest`, `run_id`, `granted_capabilities`, `policy`, `budget`) MUST remain unchanged
- POST-04: All existing callers of `admit_run`, `admit_artifact_run`, `admit_run_with_budget` that construct `RunAdmission` MUST provide idempotency evidence or a default
- POST-05: `IdempotencyTracker` MUST correctly track `idempotency_keyed` actions and `is_completed_for_policy` MUST return accurate results
- POST-06: No new panics or errors may be introduced in the admission path

## Invariants

- INV-01: `RunAdmission.idempotency_keyed.len() == VerificationProof.idempotency_keyed.len()` at construction time
- INV-02: `RunAdmission.idempotency_attested.len() == VerificationProof.idempotency_attested.len()` at construction time
- INV-03: `IdempotencyTracker` entries never exceed `DEFAULT_CAPACITY` (1024) after eviction; oldest entry evicted on overflow
- INV-04: `IdempotencyTracker` is safe for concurrent access from multiple shards (Send + Sync) OR access is serialized through a mutex
- INV-05: If `VerificationProof.durable && VerificationProof.bounded && VerificationProof.taint_safe && VerificationProof.retry_safe && VerificationProof.replayable`, then `idempotency_keyed` actions in `RunAdmission` have deterministic replay semantics

## Error Taxonomy

- `ArtifactEnvelopeError::MalformedEnvelope` — envelope fails structure validation
- `ArtifactEnvelopeError::MissingVerificationProof` — no verification proof present
- `ArtifactEnvelopeError::IdempotencyKeyMismatch` — key mismatch during replay (future)
- `AdmissionError::StoreError(ArtifactEnvelopeError)` — propagated store errors
- `AdmissionError::IdempotencyViolation(IdempotencyViolation)` — key validation failure

## Contract Signatures

```rust
// vb_runtime::admission

pub struct RunAdmission {
    pub run_id: RunId,
    pub artifact_digest: Digest,
    pub granted_capabilities: CapabilitySet,
    pub policy: BudgetPolicy,
    pub budget: Option<Budget>,
    pub idempotency_keyed: Box<[ActionId]>,   // NEW: from VerificationProof
    pub idempotency_attested: Box<[ActionId]>, // NEW: from VerificationProof
}

pub fn admit_artifact_run(
    artifact_store: &dyn AcceptedArtifactStore,
    envelope: ArtifactEnvelope,
) -> Result<RunAdmission, AdmissionError>;

pub struct IdempotencyTracker {
    // existing fields...
}

impl IdempotencyTracker {
    pub fn track_for_policy(&mut self, key: u128, ticket: ActionTicket);
    pub fn is_completed_for_policy(&self, key: u128) -> bool;
}
```

## Verus-Owned Clauses

- INV-01, INV-02: Pure field-copy property at RunAdmission construction — expressible in Verus
- INV-03: IdempotencyTracker capacity bound — expressible in Verus with decreases clause

## TLA+-Owned Clauses

- None — this is a data-flow and type-propagation change, not a temporal/workflow behavior change. No workflow state machine, scheduler, queue, retry loop, claim/lease, lifecycle transition, distributed coordination, or eventuality property is being introduced. TLA+ does not apply.

## Non-goals

- TLA+ temporal modeling (no new workflows)
- Lean/Aeneas/Hax theorem projection (no algebraic kernel beyond Verus)
- Formal verification of `chunk_001.rs` (pre-existing DEFERRED_GLOBAL)
- Changes to `VerificationProof` structure itself (already has the fields)
- Changes to the journal or persistence layer

## Scope Exclusions (pre-existing gaps — not in scope for vb-qi37.5.3)

The following files are excluded from coverage gates for this bead because they are
pre-existing infrastructure not touched by idempotency evidence propagation:

| File | Reason |
|------|--------|
| `crates/vb_storage/src/error/warnings.rs` | Defines `error::warnings::VerificationWarning` (SchemaVersionMismatch variant) — a **different type** from `admission::VerificationWarning` (code/message/gate fields). NOT in delivery scope. |
| `crates/vb_storage/src/error/mod.rs` | Error module plumbing |
| `crates/vb_storage/src/error/codes.rs` | Diagnostic code definitions |
| `crates/vb_storage/src/events.rs` | Event emission infrastructure |
| `crates/vb_storage/src/records.rs` | Record storage |
| `crates/vb_storage/src/recovery/**/*.rs` | Recovery/hydration infrastructure (pre-existing gaps) |
| `crates/vb_storage/src/process_lock.rs` | Process locking (pre-existing gap) |
| `crates/vb_storage/src/journal/core.rs` | Journal core (pre-existing gap) |
| `crates/vb_storage/src/trimming/mod.rs` | Trimming module (pre-existing gap) |

**Coverage gate**: 90% threshold applies to scoped files only — `crates/vb_storage/src/admission.rs` (92.78% PASS) and `crates/vb_runtime/src/admission.rs`.
