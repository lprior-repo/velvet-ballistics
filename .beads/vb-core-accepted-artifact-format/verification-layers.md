# Verification Layers — AcceptedArtifact Format

## Boundary

- **Verus-owned kernel**: Pure Rust core invariants for `AcceptedArtifact`, `VerificationProof`, `CompiledWorkflow::try_from_parts`
- **TLA+ temporal model**: Artifact admission lifecycle, gate count validation, Strict/Relaxed policy transitions
- **Theorem projection**: None — no Lean/Aeneas/Hax kernel required
- **Runtime shell**: Postcard codec, Fjall persistence, Arc<dyn AcceptedArtifactStore>, SyncAll
- **External systems excluded from formal proof**: SHA-256 primitive, Fjall transaction engine, journal sequence counter

## Layer Assignment

| Contract Clause | Primary Layer | Secondary Layer | Waiver |
|-----------------|---------------|-----------------|--------|
| INV-001 (digest-IR invariant) | verus | kani (bounded) | — |
| INV-002 (gate_count >= 1) | verus | tla-plus | — |
| INV-003 (proof flags derived) | verus | kani (bounded model check) | — |
| INV-004 (CompiledWorkflow sole constructor) | verus | — | — |
| INV-005 (atomic persistence) | tla-plus | loom (concurrent access) | — |
| PRE-001 (CompiledWorkflow provenance) | verus | kani | — |
| PRE-003 (postcard decode valid) | miri | cargo-fuzz | — |
| PRE-004 (digest checksum) | kani | — | — |
| POST-001 (AcceptedArtifact serde) | api-compat | — | — |
| POST-002 (VerificationProof serde) | api-compat | — | — |
| POST-004 (Relaxed-only admission) | tla-plus | — | — |
| ERR-001 (ArtifactEnvelopeError exhaustive) | verus (exhaustive match) | test-writer | — |
| Gate count mismatch | kani (bounded model check) | tla-plus | — |

## Verus Scope

### Target: `vb_storage::admission::AcceptedArtifact`

- **Spec functions**: `accepted_artifact_digest_matches_ir`, `accepted_artifact_gate_count_valid`
- **Invariants**: Digest-IR invariant, gate count bounds
- **Trusted boundary**: SHA-256 implementation trusted; postcard encode/decode excluded
- **Shell exclusions**: Fjall persistence, SyncAll, Arc<dyn store>

### Target: `vb_storage::admission::VerificationProof`

- **Spec functions**: `proof_flag_derivation`
- **Invariants**: All flags boolean, gate_count in 0..15
- **Trusted boundary**: Gate function outputs are external verification results
- **Shell exclusions**: Postcard codec

### Target: `vb_core::compiled_workflow::CompiledWorkflow::try_from_parts`

- **Proof function**: `proof_try_from_parts_sole_constructor`
- **Invariants**: Structural validity of compiled nodes, expressions, accessors, constants
- **Trusted boundary**: `validate_parts` and `validate_budget` are pure functions
- **Shell exclusions**: None — fully Verus-verifiable

## TLA+ Scope

### Module/model path: `specs/ArtifactAdmission.tla`

**Variables**: `artifactDigest`, `artifactState`, `gateCount`, `proofFlags`, `policy`, `errorMsg`

**Actions**: `Init`, `SubmitArtifact`, `LoadForAdmission`, `AdmitStrict`, `AdmitRelaxed`, `RejectGateCount`, `RejectProofFlag`

**Safety invariants**:
- `ArtifactAdmittedImpliesValidGateCount`: artifactState=Admitted => gateCount ∈ {2, 15} ∧ proofFlags.bounded ∧ proofFlags.durable
- `StrictPolicyRejectsTwoGate`: policy=Strict ∧ artifactState=Admitted => gateCount=15

**Temporal properties**:
- `EventuallyStoredOrRejected`: ◇(artifactState ∈ {Stored, Rejected})
- `NoSpuriousRejection`: a rejected artifact was actually invalid

**Fairness/deadlock stance**: Weak fairness on SubmitArtifact and LoadForAdmission; model is always able to accept a new action

**Refinement boundary**:
- `artifactState = Stored` ↔ `submit_artifact` returned Ok
- `artifactState = Admitted` ↔ `load_accepted_artifact` returned Ok
- `artifactState = Rejected` ↔ `ArtifactEnvelopeError` variant
- `gateCount = 2` ↔ `vb_storage::ADMISSION_GATE_COUNT`
- `gateCount = 15` ↔ `vb_runtime::REQUIRED_GATE_COUNT`

**Evidence command**: `tlc -config specs/ArtifactAdmission.cfg specs/ArtifactAdmission.tla`

## Miri Scope

### Target: `vb_runtime::admission::StorageArtifactStore::load_accepted_artifact`

**Checker**: `cargo miri` with `MIRIFLAGS=-Zmiri-strict-provenance`

**Focus**:
- Postcard decode of untrusted IR bytes from Fjall store
- No use-after-free on artifact envelope
- No invalid enum variant decoding
- No alignment violations on `AcceptedArtifact` struct fields

**Command**: `cargo miri test -p vb_runtime --test accepted_artifact_miri 2>&1 | tee miri-report.txt`

## Kani Scope

### Target: `vb_storage::admission::submit_artifact`

**Claim**: `gate_count` is within valid range (0..16) and `bounded` flag is correctly set

**Command**: `cargo kani --spec "spec_gate_count_bounded" 2>&1 | tee kani-report.txt`

### Target: `vb_core::compiled_workflow::CompiledWorkflow::try_from_parts`

**Claim**: All index accesses within bounds after validation

**Command**: `cargo kani -p vb_core "CompiledWorkflow::try_from_parts" 2>&1 | tee kani-report.txt`

## Loom Scope

### Target: `vb_runtime::shard::types::ShardState::artifact_store`

**Claim**: Concurrent load of `SharedAcceptedArtifactStore` across multiple threads does not produce data races

**Command**: `cargo loom -p vb_runtime --test concurrent_artifact_store 2>&1 | tee loom-report.txt`

**Waiver condition**: If `AlwaysPresentArtifactStore` is test-only and never used in production, concurrent access is read-only and inherently safe

## Performance Scope

**None** — no performance claims in this contract.

## API Compatibility Scope

- `AcceptedArtifact` pub fields: `digest`, `ir`, `verification`, `accepted_at_seq`, `required_capabilities`
- `VerificationProof` pub fields: all fields pub
- Any field addition requires semver bump and `api-compat` evidence

## Waivers

| Clause | Waiver Reason | Compensating Evidence |
|--------|---------------|----------------------|
| TLA+ for Fjall atomicity | Fjall is trusted external store | Integration tests in `vb_2bok_durability_gate_tests.rs` |
| Loom for concurrent store access | `AlwaysPresentArtifactStore` is test-only | `StorageArtifactStore` uses internal Fjall locking |
| Verus for postcard codec | Codec is external crate | `cargo fuzz run decode_accepted_artifact` adversarial inputs |
| Kani for ResourceBudget arithmetic | Deferred to `vb-core-proof-15-gate` | Placeholder for gate derivation proof |
