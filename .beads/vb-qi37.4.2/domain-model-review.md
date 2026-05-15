# Domain Model Review: vb-qi37.4.2

## Status: VERIFIED CORRECT

## Admission Gate Sequencing (lifecycle/chunk_001.rs:68-136)

### Sequencing is Correctly Ordered

The current code in `handle_submit_with_inputs_contracts_and_header_mode` already implements the correct ordering:

```
86: let admission = self.build_admission(run, digest, caps)?;
87: let mut frame = self.take_frame_for(run, &workflow)?;
...
91-100: journal RunSubmitted (conditional on persist_header)
102-111: journal RunAdmission (if admission.is_some())
...
125: self.runs.insert(run, state);
```

The `?` operator on line 86 means `build_admission` failures bypass all subsequent lines, including frame allocation, journal events, and run insertion.

### Error Mapping Chain

`ArtifactEnvelopeError` → `AdmissionError` → `RuntimeError` forms a correct 1:1 or N:1 chain with no information loss for the error variants relevant to this bead.

### Policy Behavior

| Policy | `admit_artifact_run` behavior |
|--------|------------------------------|
| `Strict` | Loads artifact, validates gate_count=15 and all proof flags, checks capabilities |
| `Journaled` | Identical to Strict |
| `Relaxed` | Skips all validation, returns empty `RunAdmission` |

This matches the documented contract in `admission.rs` comments.

### Store Trait Hierarchy

```
ArtifactStore (exists check only)
  └─ AlwaysPresentArtifactStore
  └─ StorageArtifactStore

AcceptedArtifactStore (full validation)
  └─ AlwaysPresentArtifactStore (returns dummy artifact with gate_count=15)
  └─ StorageArtifactStore (loads real artifact, validates gate_count=15)
```

The `AlwaysPresentArtifactStore` implements `AcceptedArtifactStore` by returning a dummy artifact that passes Strict/Journaled validation — this is intentional for test scenarios that don't care about artifact content.

### NeverPresentArtifactStore Gap

There is NO existing `NeverPresentArtifactStore` (one that always returns `ArtifactNotFound` under `AcceptedArtifactStore`). The unit tests use local `NeverPresentStore` structs implementing `ArtifactStore`. The integration test needs a `NeverPresentArtifactStore` implementing `AcceptedArtifactStore` that always returns `Err(ArtifactEnvelopeError::ArtifactNotFound { digest })`.

### Verified Findings

1. ✅ Admission gate is evaluated before frame allocation, journal events, and run insertion
2. ✅ `?` propagation correctly short-circuits on rejection
3. ✅ Error mapping is exhaustive for all 8 `ArtifactEnvelopeError` variants
4. ✅ `RuntimeError::AdmissionArtifactNotFound`, `AdmissionCapabilityDenied`, `AdmissionArtifactInvalid` cover all rejection paths
5. ❌ No `NeverPresentArtifactStore` exists — must be created for integration test
6. ❌ Existing test `admission_rejection_does_not_insert_run_state` uses Relaxed policy and does NOT test rejection

### Review Outcome

**APPROVED** for contract artifact generation. The sequencing is correct; the gap is purely in test coverage.
