# Contract Specification — AcceptedArtifact Format

## Context

- **Feature**: Stable `AcceptedArtifact` format for strict runtime admission and persistence
- **Bead**: `vb-core-accepted-artifact-format`
- **Gate Count Mismatch**: Storage (`vb_storage`) produces `ADMISSION_GATE_COUNT = 2`, but runtime (`vb_runtime`) requires `REQUIRED_GATE_COUNT = 15`
- **Domain Terms**:
  - `AcceptedArtifact` — envelope containing digest, IR (postcard-encoded WorkflowParts), verification proof, journal seq, and required capabilities
  - `VerificationProof` — proof flags that verification passed at admission time
  - `WorkflowParts` — intermediate compiled workflow representation before gating
  - `CompiledWorkflow` — gate-checked artifact representation produced by `try_from_parts`
  - `ArtifactEnvelopeError` — error taxonomy for runtime admission failures

## Preconditions

- PRE-001: `submit_artifact` caller must provide a valid `CompiledWorkflow` produced by `CompiledWorkflow::try_from_parts`
- PRE-002: `submit_artifact_with_contracts` caller must provide action contracts from validated `ActionContract` extraction
- PRE-003: Artifact IR bytes must postcard-decode to a valid `WorkflowParts` when re-validated
- PRE-004: Artifact digest must match SHA-256 of the submitted IR bytes (checksum validation)

## Postconditions

- POST-001: `AcceptedArtifact` is `Serialize + Deserialize + Clone + PartialEq + Eq + Debug`
- POST-002: `VerificationProof` is `Serialize + Deserialize + Clone + PartialEq + Eq + Debug`
- POST-003: `submit_artifact` returns `Ok(AcceptedArtifact)` with `gate_count = ADMISSION_GATE_COUNT = 2`
- POST-004: Stored artifact passes `StorageArtifactStore::load_accepted_artifact` under Relaxed policy only
- POST-005: `accepted_at_seq` is set to a journal sequence at persistence time
- POST-006: Artifact IR is postcard-encoded `WorkflowParts` stored in `AcceptedArtifact.ir`

## Invariants

- INV-001: `AcceptedArtifact.digest == sha256(AcceptedArtifact.ir)` — artifact content hash is intrinsic
- INV-002: `AcceptedArtifact.verification.gate_count >= 1` — at least one gate passes for any accepted artifact
- INV-003: `VerificationProof` flags (`bounded`, `taint_safe`, `retry_safe`, `replayable`) are derived from actual gate outputs, not hardcoded `true`
- INV-004: `CompiledWorkflow::try_from_parts` is the only constructor that produces a structurally valid `CompiledWorkflow`
- INV-005: Artifact persistence in Fjall `compiled_ir` keyspace is atomic with journal sequence assignment

## Error Taxonomy

- `ArtifactEnvelopeError::ArtifactNotFound` — artifact digest absent from store
- `ArtifactEnvelopeError::PostcardDecodeFailed` — IR bytes fail postcard decode
- `ArtifactEnvelopeError::InvalidGateCount { found, required }` — gate_count mismatch; **CRITICAL**: found=2, required=15 in current mismatch
- `ArtifactEnvelopeError::MissingRequiredProofFlagBounded` — bounded proof flag is false
- `ArtifactEnvelopeError::MissingRequiredProofFlagTaintSafe` — taint_safe proof flag is false
- `ArtifactEnvelopeError::MissingRequiredProofFlagRetrySafe` — retry_safe proof flag is false
- `ArtifactEnvelopeError::MissingRequiredProofFlagDurable` — durable proof flag is false
- `ArtifactEnvelopeError::MissingRequiredProofFlagReplayable` — replayable proof flag is false
- `JournalError` — storage-level persistence failures

## Contract Signatures

```rust
// vb_storage/src/admission.rs
pub fn submit_artifact(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    policy: vb_core::RuntimePolicy,
) -> Result<AcceptedArtifact, JournalError>

pub fn submit_artifact_with_contracts(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    policy: vb_core::RuntimePolicy,
    action_contracts: &[vb_core::action::ActionContract],
) -> Result<AcceptedArtifact, JournalError>

// vb_runtime/src/admission.rs
pub const REQUIRED_GATE_COUNT: u8 = 15;

pub fn load_accepted_artifact(
    artifact_store: &dyn AcceptedArtifactStore,
    digest: &WorkflowDigest,
) -> Result<AcceptedArtifact, ArtifactEnvelopeError>
```

## Gate Count Contract (CRITICAL MISMATCH)

| Location | Constant | Value | Policy |
|----------|----------|-------|--------|
| `vb_storage/src/admission.rs:118` | `ADMISSION_GATE_COUNT` | 2 | Relaxed |
| `vb_runtime/src/admission.rs:16` | `REQUIRED_GATE_COUNT` | 15 | Strict/Journaled |

**Mismatch Consequence**: Artifacts stored via `submit_artifact` with `gate_count=2` will be rejected by `StorageArtifactStore::load_accepted_artifact` under `Strict` or `Journaled` policy with `ArtifactEnvelopeError::InvalidGateCount { found: 2, required: 15 }`.

## Resolution Options

1. **Option A**: Change `ADMISSION_GATE_COUNT` in vb_storage to 15 and emit real 15-gate verification
2. **Option B**: Change `REQUIRED_GATE_COUNT` in vb_runtime to 2 and document 2-gate relaxed admission
3. **Option C**: Implement 15-gate verification in vb_storage and retire the 2-gate path
4. **Option D**: Add a version field to `AcceptedArtifact` and support both 2-gate legacy and 15-gate current formats

## TLA+-Owned Clauses

- INV-002: Temporal model of artifact lifecycle (submitted → stored → admitted → run)
- ERR-001: Admission rejection state machine

## Verus-Owned Clauses

- INV-001: `AcceptedArtifact::new` preserves digest-IR invariant
- INV-004: `CompiledWorkflow::try_from_parts` is the sole constructor — no bypass
- PRE-001: `submit_artifact` validates CompiledWorkflow provenance

## Theorem-Owned Clauses

- None for this bead. Theorem projection applies to tiny algebraic kernels (e.g., `ResourceBudget arithmetic`).

## Non-goals

- Defining the 15 specific verification gates (deferred to `vb-core-proof-15-gate` bead)
- Implementing the 15-gate proof emission in vb_storage
- Runtime artifact loading path changes (deferred to `vb-core-storage-artifact-store` bead)
