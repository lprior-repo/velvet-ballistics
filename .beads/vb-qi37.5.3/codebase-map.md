# Codebase Map — vb-qi37.5.3

**Bead**: runtime: Carry idempotency evidence into admission
**Workspace**: /home/lewis/src/vb-qi37-5-3
**Generated**: State 2 (Explore and scope)

---

## 1. Scope Summary

This bead extends `RunAdmission` (vb_runtime) to carry idempotency evidence from the
accepted artifact's `VerificationProof` forward into the runtime admission record, so that
the runtime engine can make idempotency-aware scheduling decisions without re-loading the artifact.

**Gap**: `RunAdmission` currently stores `artifact_digest`, `run_id`, `granted_capabilities`,
`policy`, and `budget`. The `VerificationProof` carries `idempotency_keyed` and
`idempotency_attested` (action ID lists), but these are NOT transferred to `RunAdmission`.

---

## 2. Touched Crates and Files

### vb_runtime (primary target)
| File | Symbol(s) | Notes |
|------|-----------|-------|
| `crates/vb_runtime/src/admission.rs` | `RunAdmission`, `ArtifactEnvelopeError`, `admit_run`, `admit_artifact_run`, `admit_run_with_budget`, `StorageArtifactStore::load_accepted_artifact` | Add idempotency evidence fields to `RunAdmission`; wire them from `AcceptedArtifact` via `AcceptedArtifactStore` |
| `crates/vb_runtime/src/idempotency.rs` | `IdempotencyTracker` | Already exists; may need to accept evidence from admission rather than recomputing |

### vb_storage (upstream data source)
| File | Symbol(s) | Notes |
|------|-----------|-------|
| `crates/vb_storage/src/admission.rs` | `AcceptedArtifact`, `VerificationProof`, `VerificationWarning`, `ProofFlag` | Source of `idempotency_keyed` and `idempotency_attested`; `submit_artifact` builds this record |
| `crates/vb_storage/src/lib.rs` | re-exports `AcceptedArtifact`, `VerificationProof`, `submit_artifact` | Downstream consumer is `vb_runtime::admission::StorageArtifactStore` |

### vb_core (types)
| File | Symbol(s) | Notes |
|------|-----------|-------|
| `crates/vb_core/src/action.rs` | `Idempotency`, `IdempotencyViolation`, `SideEffect`, `RetrySafety`, `ActionContract`, `ActionTicket` | `Idempotency` enum has `DeterministicPure`, `IdempotentExternal`, `AtLeastOnceExternal` |
| `crates/vb_core/src/ids.rs` | `ActionId`, `RunId`, `WorkflowDigest` | |
| `crates/vb_core/src/frame.rs` | `RunFrame` | Currently does not embed `RunAdmission`; runtime engine attaches admission at frame creation |

---

## 3. Public API Surface — Admission

### vb_runtime::admission (lib public API)
```
pub struct RunAdmission { ... }
pub fn admit_run(...) -> Result<RunAdmission, AdmissionError>
pub fn admit_artifact_run(...) -> Result<RunAdmission, AdmissionError>
pub fn admit_run_with_budget(...) -> Result<RunAdmission, AdmissionError>
pub fn check_capability(...) -> Result<(), AdmissionError>
pub trait ArtifactStore { fn compiled_ir_exists(...) -> bool }
pub trait AcceptedArtifactStore { fn load_accepted_artifact(...) -> Result<AcceptedArtifact, ArtifactEnvelopeError> }
pub type SharedArtifactStore
pub type SharedAcceptedArtifactStore
```

**Issue**: `RunAdmission` has no idempotency evidence fields. The `idempotency_keyed: Box<[ActionId]>`
and `idempotency_attested: Box<[ActionId]>` from `VerificationProof` are not transferred.

### vb_storage::admission (lib public API)
```
pub struct AcceptedArtifact { digest, ir, verification: VerificationProof, accepted_at_seq, required_capabilities }
pub struct VerificationProof { digest, gate_count, durable, bounded, taint_safe, retry_safe, replayable, idempotency_keyed, idempotency_attested, warnings }
pub fn submit_artifact(...) -> Result<AcceptedArtifact, JournalError>
```

---

## 4. Changed Dependencies

- No new dependencies introduced
- `vb_runtime` already depends on `vb_core` and `vb_storage`
- `vb_storage::admission::AcceptedArtifact` is the upstream source of idempotency evidence

---

## 5. Contract Clauses for Idempotency Evidence

From `velvet-ballastics-MASTER.md`:
- Section 38 (idempotency verifier gate): "Key ingredient validation (reject secrets, random, time in keys)"
- Section 65 (idempotency): `idempotency_keyed` and `idempotency_attested` lists
- Section 38: "New `IdempotencyViolation` error type"
- Replay policy must prevent accidental duplicate non-idempotent effects

From `vb_core/src/action.rs`:
- `IdempotencyViolation::MissingKey` — action has side-effect but no idempotency key
- `IdempotencyViolation::SecretInKey` — key contains secret-tainted value
- `IdempotencyViolation::RandomInKey` — key contains random value
- `IdempotencyViolation::TimeInKey` — key contains time-dependent value

From `vb_storage/src/admission.rs`:
- `VerificationProof.idempotency_keyed` — actions that use idempotency keys
- `VerificationProof.idempotency_attested` — actions attested idempotent by contract

---

## 6. Risk Tags

| Tag | Location | Notes |
|-----|----------|-------|
| `persistence` | `vb_storage::admission::AcceptedArtifact` persisted in FjallJournal | Evidence must survive crash |
| `concurrency` | `IdempotencyTracker` uses HashMap without locking | Thread-safety via `Send+Sync` on tracker? |
| `public_api` | `RunAdmission` is returned from `admit_run`, `admit_artifact_run`, `admit_run_with_budget` | Adding fields changes the public struct |
| `verification` | `VerificationProof` gates: `bounded`, `taint_safe`, `retry_safe`, `durable`, `replayable` | These must remain true for admission |
| `migration` | Existing code constructing `RunAdmission::new` / `with_budget` | Callers must pass idempotency evidence |

---

## 7. Required Verifier Modes

| Mode | Trigger | Notes |
|------|---------|-------|
| `miri` | `IdempotencyTracker` HashMap operations | Check for UB on concurrent access patterns |
| `proptest` | `RunAdmission::new` with idempotency evidence | Property-based test for field propagation |
| `kani` | `StorageArtifactStore::load_accepted_artifact` | Bounded model checking on artifact loading path |
| `loom` | `IdempotencyTracker` thread-safety | Concurrency permutation testing |
| `test` | Unit tests in `admission.rs` and `idempotency.rs` | Existing tests must pass and extend |

**NOTE**: `vb_runtime` currently FAILS to build due to missing `crates/vb_runtime/src/runtime/chunk_001.rs`. This is pre-existing (DEFERRED_GLOBAL) and outside this bead's scope. Formal verification cannot proceed until that file exists.

---

## 8. Release / Critical Classification

- **Release**: No — this is an internal refactor of `RunAdmission` to carry additional evidence
- **Critical**: No — does not change external API behavior, only enriches internal record
- **Blocking issue**: The missing `chunk_001.rs` MUST be resolved before formal verification can execute

---

## 9. Pre-existing Build Failure (DEFERRED_GLOBAL)

```
error: couldn't read `crates/vb_runtime/src/runtime/chunk_001.rs`: No such file or directory
 --> crates/vb_runtime/src/runtime.rs:4:1
  |
4 | include!("runtime/chunk_001.rs");
  |
  = couldn't compile `vb_runtime` (lib) due to 1 previous error
```

This is pre-existing at commit `ffbe7f5cd` and is NOT caused by this bead. Filed as DEFERRED_GLOBAL.

---

## 10. Open Questions

1. Should `idempotency_keyed` and `idempotency_attested` be embedded directly in `RunAdmission`
   or referenced via a derived index into the artifact?
2. Should the runtime engine use these lists at schedule time to pre-allocate `IdempotencyTracker`
   entries, or defer to lazy population?
3. Should `RunFrame` embed a reference to `RunAdmission` (or its idempotency subset)?
4. Does `IdempotencyTracker` need to become `Send + Sync` for multi-shard use?
