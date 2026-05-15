# TLA+ Temporal Model Plan — AcceptedArtifact Admission

## Boundary

- **Temporal/workflow behavior**: Artifact submission, storage, and runtime admission lifecycle; gate count validation state machine; Strict vs Relaxed policy transitions
- **Rust/core behavior excluded from TLA+**: Postcard codec internals, Fjall storage implementation, `CompiledWorkflow::try_from_parts` gate logic, SHA-256 implementation
- **External systems abstracted**: Journal sequence numbers as atomic counters; Fjall as transactional key-value store with SyncAll durability
- **Non-applicability rationale**: Not applicable if no temporal/state-over-time behavior needs modeling

## TLA+-Owned Clauses

### TLA-ARTIFACT-001: Artifact Admission State Machine

**Contract clause**: INV-002 (gate_count >= 1 for any accepted artifact)

**Module path**: `specs/ArtifactAdmission.tla`

**Variables**:
- `artifactDigest` — `WorkflowDigest` value held by artifact
- `artifactState` — `Pending | Stored | Admitted | Rejected`
- `gateCount` — integer 0..15 representing gates passed
- `proofFlags` — record `{bounded, taint_safe, retry_safe, replayable, durable}`
- `policy` — `Relaxed | Strict | Journaled`
- `errorMsg` — `""` or error string

**Init action**:
```tla
Init ==
    /\ artifactState = Pending
    /\ gateCount \in 0..15
    /\ proofFlags \in [bounded: {TRUE, FALSE},
                        taint_safe: {TRUE, FALSE},
                        retry_safe: {TRUE, FALSE},
                        replayable: {TRUE, FALSE},
                        durable: {TRUE, FALSE}]
    /\ policy \in {Relaxed, Strict, Journaled}
    /\ errorMsg = ""
```

**Actions**:
- `SubmitArtifact` — transition from any state to Stored with gate_count
- `LoadForAdmission` — load stored artifact
- `AdmitStrict` — if policy = Strict and gateCount = 15 and all proofFlags = TRUE → Admitted
- `AdmitRelaxed` — if policy = Relaxed and gateCount >= 1 → Admitted
- `RejectGateCount` — if gateCount # 15 under Strict → Rejected with error
- `RejectProofFlag` — if any required proof flag = FALSE under Strict → Rejected with error

**Safety invariant**:
```tla
ArtifactAdmittedImpliesValidGateCount ==
    artifactState = Admitted
        => /\ gateCount \in {2, 15}  \* current mismatch allows both
           /\ proofFlags.bounded = TRUE
           /\ proofFlags.durable = TRUE
```

**Temporal property**:
```tla
EventuallyStoredOrRejected ==
    <> (artifactState \in {Stored, Rejected})
```

**Deadlock freedom**: Model is always willing to accept SubmitArtifact or LoadForAdmission

**Refinement to Rust/runtime behavior**:
- `artifactState = Stored` corresponds to `submit_artifact` returning `Ok(AcceptedArtifact)`
- `artifactState = Admitted` under Strict corresponds to `StorageArtifactStore::load_accepted_artifact` returning `Ok(AcceptedArtifact)`
- `artifactState = Rejected` with `errorMsg` corresponds to `ArtifactEnvelopeError` variant
- `gateCount = 2` refinement: `vb_storage::ADMISSION_GATE_COUNT = 2`
- `gateCount = 15` refinement: `vb_runtime::REQUIRED_GATE_COUNT = 15`

### TLA-ARTIFACT-002: Strict Admission Rejects 2-Gate Artifact

**Contract clause**: POST-004 (stored artifact passes runtime admission under Relaxed only)

**Safety invariant**:
```tla
StrictPolicyRejectsTwoGate ==
    policy = Strict /\ artifactState = Admitted
        => gateCount = 15
```

**Model constraint**: BMC unroll depth 20, symmetry set disabled

**Evidence command**: `tlc -config specs/ArtifactAdmissionStrict.cfg specs/ArtifactAdmission.tla`

### TLA-ARTIFACT-003: Artifact Digest Persistence Invariant

**Contract clause**: INV-001 (digest == sha256(ir))

**Variables**:
- `storedDigest` — digest in stored artifact
- `irBytes` — raw IR bytes
- `computedDigest` — sha256(irBytes)

**Safety invariant**:
```tla
DigestMatchesIR ==
    artifactState = Stored
        => storedDigest = computedDigest
```

**Evidence command**: `tlc -config specs/ArtifactDigest.cfg specs/ArtifactDigest.tla`

## Model Shape

### ArtifactAdmission.tla

```
MODULE ArtifactAdmission

VARIABLES
  artifactDigest,
  artifactState,
  gateCount,
  proofFlags,
  policy,
  errorMsg

Init ==
  ...

SubmitArtifact ==
  ...

LoadForAdmission ==
  ...

AdmitStrict ==
  ...

AdmitRelaxed ==
  ...

RejectGateCount ==
  ...

RejectProofFlag ==
  ...

Spec ==
  Init /\ [][Next]_vars

THEOREM Spec => []Safe
```

### ArtifactAdmissionStrict.cfg

```
SPECIFICATION Spec
INVARIANT ArtifactAdmittedImpliesValidGateCount
INVARIANT StrictPolicyRejectsTwoGate
CONSTANTS
  Relaxed = "Relaxed"
  Strict = "Strict"
  Journaled = "Journaled"
```

## Waivers

- **Loom/Shuttle for concurrent runtime access**: Waived — `SharedAcceptedArtifactStore` is Arc<dyn> shared across threads but concurrent load is benign (read-only); write races are prevented by Fjall's internal locking. Formal concurrency testing deferred to `vb-core-storage-artifact-store` bead.
- **TLA+ model for Fjall transaction atomicity**: Waived — Fjall is trusted external store; its transaction semantics are out-of-scope for this contract.
