# Contract Specification: vb-qi37.4.2

## Context

**Bead**: vb-qi37.4.2 — runtime: Enforce admission gate before run creation
**State**: 3 (Contract and proof planning)
**Scope**: Verify admission gate is evaluated BEFORE frame allocation, journal events, and run state insertion in `handle_submit_with_inputs_contracts_and_header_mode`. The key gap is that the existing test `admission_rejection_does_not_insert_run_state` uses `Relaxed` policy and asserts run IS inserted — it does NOT test rejection.

## Domain Terms

| Term | Definition |
|------|------------|
| `RunAdmission` | Record capturing artifact digest, run ID, granted capabilities, and admission policy after passing the gate |
| `AdmissionError` | Enumeration of rejection reasons: `ArtifactNotFound`, `CapabilityDenied`, `ResourceCapacityExceeded`, `ArtifactEnvelopeDecodeFailed`, `ArtifactInvalidGateCount`, `ArtifactInvalidProofFlag` |
| `ArtifactEnvelopeError` | Validation errors from storage layer: `ArtifactNotFound`, `PostcardDecodeFailed`, `InvalidGateCount`, and 5 proof flag errors |
| `RuntimePolicy` | Enum: `Strict`, `Journaled`, `Relaxed`. Strict/Journaled require valid accepted artifact; Relaxed skips validation |
| `AcceptedArtifactStore` | Trait: `load_accepted_artifact(digest) -> Result<AcceptedArtifact, ArtifactEnvelopeError>` |
| `REQUIRED_GATE_COUNT` | Constant = 15 (runtime). Storage layer uses 2 — known mismatch from vb-qi37.6, out of scope |
| `NeverPresentArtifactStore` | Artifact store (implementing `AcceptedArtifactStore`) that always returns `ArtifactNotFound` — used to trigger rejection under Strict/Journaled |

## Preconditions

- **PRE-001**: Submitter has a `CompiledWorkflow` with a valid digest
- **PRE-002**: Submitter provides a `CapabilitySet` covering the artifact's required capabilities (for Strict/Journaled)
- **PRE-003**: The run ID is not already active in the shard (`!runs.contains_key(&run)`)
- **PRE-004**: The shard has capacity for another active run (`runs.len() < max_active_runs`)

## Postconditions

- **POST-001**: On success: `build_admission` returned `Some(RunAdmission)`, frame was allocated, `RunSubmitted` journaled, `RunAdmission` journaled (if present), run state inserted, and `drive_run` invoked
- **POST-002**: On rejection: `build_admission` returned `Err`, no frame allocated, no journal events written, no run state inserted, `active_run_count` unchanged
- **POST-003**: Error taxonomy is exhaustive: `AdmissionArtifactNotFound`, `AdmissionCapabilityDenied`, `AdmissionArtifactInvalid`, `ActiveRunCapacityExceeded`, `RunAlreadyExists`

## Invariants

- **INV-001**: For Strict/Journaled policy, a run is NEVER inserted into `self.runs` unless `build_admission` returned `Ok`
- **INV-002**: Sequencing: `build_admission` (line 86) → `take_frame_for` (line 87) → `RunSubmitted` journaled (lines 91–100) → `RunAdmission` journaled (lines 102–111) → `runs.insert` (line 125)
- **INV-003**: `ArtifactEnvelopeError` maps 1-to-1 or many-to-1 into `AdmissionError` variants
- **INV-004**: `AdmissionError` maps 1-to-1 into `RuntimeError` variants visible to callers

## Sequencing Contract

```
LINE 86: build_admission(run, digest, caps)?  ← GATE EVALUATED FIRST
LINE 87: take_frame_for(run, &workflow)?        ← Frame allocated AFTER admission
LINE 89: trace_ring.push(RunSubmitted)
LINE 91-100: journal RunSubmitted               ← Journaled AFTER admission
LINE 102-111: journal RunAdmission             ← Journaled AFTER admission
LINE 125: self.runs.insert(run, state)         ← Run created AFTER admission
```

**Critical**: If `build_admission` fails, all subsequent steps (frame allocation, journal, run insertion) are skipped via `?` propagation.

## Error Taxonomy

| `ArtifactEnvelopeError` | `AdmissionError` | `RuntimeError` |
|------------------------|------------------|----------------|
| `ArtifactNotFound { digest }` | `ArtifactNotFound { digest }` | `AdmissionArtifactNotFound { digest }` |
| `PostcardDecodeFailed` | `ArtifactEnvelopeDecodeFailed` | `AdmissionArtifactInvalid { digest: zeroed }` |
| `InvalidGateCount { found, required }` | `ArtifactInvalidGateCount { found, required }` | `AdmissionArtifactInvalid { digest }` |
| `MissingRequiredProofFlagBounded` | `ArtifactInvalidProofFlag { flag: "bounded" }` | `AdmissionArtifactInvalid { digest }` |
| `MissingRequiredProofFlagTaintSafe` | `ArtifactInvalidProofFlag { flag: "taint_safe" }` | `AdmissionArtifactInvalid { digest }` |
| `MissingRequiredProofFlagRetrySafe` | `ArtifactInvalidProofFlag { flag: "retry_safe" }` | `AdmissionArtifactInvalid { digest }` |
| `MissingRequiredProofFlagDurable` | `ArtifactInvalidProofFlag { flag: "durable" }` | `AdmissionArtifactInvalid { digest }` |
| `MissingRequiredProofFlagReplayable` | `ArtifactInvalidProofFlag { flag: "replayable" }` | `AdmissionArtifactInvalid { digest }` |
| (capability mismatch) | `CapabilityDenied { action, required, granted }` | `AdmissionCapabilityDenied { action, required, granted }` |

## Test Gap (Key Finding)

The existing test `admission_rejection_does_not_insert_run_state` (lifecycle_tests/chunk_003.rs:53):
- Uses `Shard::new(small_config())` where `small_config()` sets `policy: RuntimePolicy::Relaxed`
- Asserts `active_run_count() == 1` and `runs_submitted == 1` — confirming run IS inserted
- Does NOT test rejection

**Required fix**: New integration test using:
- `ShardConfig { policy: RuntimePolicy::Strict }` (or `Journaled`)
- `NeverPresentArtifactStore` (implements `AcceptedArtifactStore`, always returns `ArtifactNotFound`)
- Assert `active_run_count() == 0` and `runs_submitted == 0`

## TLA+-Owned Clauses

INV-002 (sequencing) is a state-machine ordering property, not a pure Rust-local invariant. TLA+ is NOT needed since the sequencing is a single linear step function with no branching or concurrency at the shard level.

## Verus-Owned Clauses

- INV-001: Rust-local pure property — `handle_submit_with_inputs_contracts_and_header_mode` never inserts into `self.runs` when `build_admission` returns `Err`
- This is verifiable by inspection of the `?` operator propagation and is better tested via integration tests than proven in Verus

## Theorem-Owned Clauses

None — no algebraic kernel beyond what Verus can handle.

## Non-goals

- Gate count alignment (vb-qi37.6 tracks `REQUIRED_GATE_COUNT=15` vs storage `gate_count=2`)
- Multi-shard routing
- Capability delegation in multi-shard runtime
- Budget admission edge cases
