# Domain Model Review — vb-qi37.5.3

## RunAdmission / VerificationProof Relationship

### Current State (Gap)

```
VerificationProof (vb_storage)
  ├── idempotency_keyed: Box<[ActionId]>
  └── idempotency_attested: Box<[ActionId]>
          │
          │  ← NOT PROPAGATED
          ▼
RunAdmission (vb_runtime)
  ├── run_id: RunId
  ├── artifact_digest: Digest
  ├── granted_capabilities: CapabilitySet
  ├── policy: BudgetPolicy
  └── budget: Option<Budget>
          (missing idempotency_keyed, idempotency_attested)
```

### Target State (After Fix)

```
VerificationProof (vb_storage)
  ├── idempotency_keyed: Box<[ActionId]>
  └── idempotency_attested: Box<[ActionId]>
          │
          │  copied at admit_artifact_run
          ▼
RunAdmission (vb_runtime)
  ├── run_id: RunId
  ├── artifact_digest: Digest
  ├── granted_capabilities: CapabilitySet
  ├── policy: BudgetPolicy
  ├── budget: Option<Budget>
  ├── idempotency_keyed: Box<[ActionId]>     ← NEW
  └── idempotency_attested: Box<[ActionId]>   ← NEW
```

---

## Type Analysis

### RunAdmission

- **Type**: `pub struct RunAdmission`
- **Location**: `vb_runtime/src/admission.rs`
- **Construction**: `admit_artifact_run(store, envelope)` calls `store.load_accepted_artifact(envelope)?` which returns `AcceptedArtifact` containing `VerificationProof`
- **Fields to add**: `idempotency_keyed: Box<[ActionId]>`, `idempotency_attested: Box<[ActionId]>`

### VerificationProof

- **Type**: `pub struct VerificationProof`
- **Location**: `vb_storage/src/admission.rs`
- **Fields**: `digest`, `gate_count`, `durable`, `bounded`, `taint_safe`, `retry_safe`, `replayable`, `idempotency_keyed`, `idempotency_attested`, `warnings`
- **Type of idempotency fields**: `Box<[ActionId]>` (heap-allocated slice)

### AcceptedArtifact

- **Type**: `pub struct AcceptedArtifact`
- **Location**: `vb_storage/src/admission.rs`
- **Field**: `verification: VerificationProof`

### Data Flow

```
ArtifactEnvelope
    │
    ▼
StorageArtifactStore::load_accepted_artifact()
    │
    ▼
AcceptedArtifact { verification: VerificationProof { idempotency_keyed, idempotency_attested } }
    │
    ▼
admit_artifact_run() extracts proof, builds RunAdmission
    │
    ▼
RunAdmission { idempotency_keyed, idempotency_attested }
```

---

## Scott DDD Refactor Analysis

### Primitive Obsession Check

| Field | Current Type | Problem | Fix |
|-------|-------------|---------|-----|
| `idempotency_keyed` | `Box<[ActionId]>` | Acceptable — ActionId is already a newtype | None needed |
| `idempotency_attested` | `Box<[ActionId]>` | Acceptable | None needed |

### Boolean Soup Check

- `RunAdmission` does NOT use boolean flags to encode lifecycle state — PASS
- `VerificationProof` uses boolean flags (`durable`, `bounded`, `taint_safe`, `retry_safe`, `replayable`) but these are persisted proof flags, not runtime state encoding — ACCEPTABLE for this contract

### Option-as-State Check

- `RunAdmission.budget: Option<Budget>` — this is legitimate optionality (budget may or may not be set), not a state machine encoding — ACCEPTABLE
- No field in `RunAdmission` is `Option<T>` for a field that implies lifecycle stage — PASS

### Result Type Check

- `admit_artifact_run` returns `Result<RunAdmission, AdmissionError>` — CORRECT
- `StorageArtifactStore::load_accepted_artifact` returns `Result<AcceptedArtifact, ArtifactEnvelopeError>` — CORRECT
- Error taxonomy is explicit: `ArtifactEnvelopeError` variants, `AdmissionError` with inner error types — PASS

### Domain API Primitives

- `RunId`, `ActionId`, `Digest` are all semantic newtypes — PASS
- `CapabilitySet`, `BudgetPolicy`, `Budget` are domain types — PASS

---

## IdempotencyTracker Analysis

### Current Design

```rust
pub struct IdempotencyTracker {
    completed: HashMap<u128, ActionTicket>,
    pending: HashSet<u128>,
    // policy-aware tracking
}
```

### Thread-Safety Concern

- `HashMap<u128, ActionTicket>` and `HashSet<u128>` are not obviously `Send + Sync`
- Multi-shard concurrent access is UNVERIFIED
- **Risk**: Data races on HashMap operations without interior mutability protection
- **Required**: loom testing + miri checking before multi-shard deployment

### Capacity Bound

- `DEFAULT_CAPACITY = 1024`
- FIFO eviction on overflow
- Journal is authoritative for crash recovery

---

## Conclusion

The type model is sound for the propagation task. `VerificationProof` already has the required `Box<[ActionId]>` fields. The primary work is:

1. Adding those same type fields to `RunAdmission`
2. Copying them in `admit_artifact_run` from the loaded `AcceptedArtifact`
3. Updating all `RunAdmission` construction sites to provide or default the fields
4. Verifying `IdempotencyTracker` thread-safety via loom/miri

No type-state redesign is needed. The change is additive and follows the existing domain model patterns.
