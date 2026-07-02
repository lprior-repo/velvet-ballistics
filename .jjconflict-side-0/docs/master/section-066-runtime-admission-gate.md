---
section: 66
title: "Runtime Admission Gate"
parent: velvet-ballistics-MASTER.md
---

## 66. Runtime Admission Gate


### Principle

The runtime only accepts verified artifacts. A run is not durable until `RunAccepted` is recorded.

### Admission Flow

```text
load artifact by digest
  → verify artifact digest matches stored IR
  → validate input against declared input schema
  → bind workflow digest to run
  → check required capabilities are granted
  → check required secrets are available (presence only, not values)
  → allocate run frame from pool
  → record RunAccepted event
  → return run_id
```

If `RunAccepted` is recorded, the run is durable. If any step before it fails, the run was never admitted.

### Admission Record

```rust
pub struct RunAdmission {
    pub run: RunId,
    pub artifact_digest: WorkflowDigest,
    pub input_digest: WorkflowDigest,
    pub capabilities_granted: Box<[Capability]>,
    pub secrets_available: Box<[SymbolId]>,
    pub admitted_at: u64,
}
```

### Secret Availability Check

Admission checks that every secret declared in the workflow is available in the runtime's secret store. Missing secrets cause `SecretUnavailable` rejection. Secret values are never part of the artifact or admission record — only presence is checked.

### Persistence of Admission

`RunAccepted` journal event is recorded durably before the run begins execution. The existing storage layer already defines `JournalEvent::RunAccepted { run, seq, workflow }` (Section 49). Phase 39 extends this event with artifact digest and admission metadata. Under `Strict` durability, this means `SyncAll` before returning `run_id`. Under `Journaled` durability, the event is queued and the run may begin before the write hits disk (acknowledged data-loss window).

### Migration from Existing Submit Flow

The existing `Runtime::submit_direct(run, workflow: CompiledWorkflow)` and `ShardCommand::Submit { run, workflow }` bypass artifact verification — they accept a raw `CompiledWorkflow` with no digest binding, capability check, or secret availability check. These functions remain available for testing and internal use but are gated behind a `RuntimePolicy` flag:

```rust
pub struct RuntimePolicy {
    pub require_accepted_artifact: bool,  // default: false (backward compatible)
    pub strict_admission: bool,           // default: false
}
```

When `require_accepted_artifact` is `true`, `submit_direct` is rejected with `AdmissionRequired`. New admission-aware functions replace it:

```rust
pub fn submit_artifact(&self, run: RunId, artifact_digest: WorkflowDigest, input: &[u8], capabilities: &[Capability]) -> RuntimeResult<()>
```

This migration path allows existing tests and benchmarks to continue using `submit_direct` while production deployments enforce the admission gate. The IPC protocol already defines `SubmitRun` which carries a workflow reference; Phase 39 extends it to carry an artifact digest.

### Capability Model

v1 capabilities are named permissions that actions declare and operators grant:

```rust
pub struct Capability {
    pub name: Box<str>,  // e.g. "network.github", "secrets.read.github_token"
    pub action: ActionId,
}
```

`Capability` appears in two contexts with distinct semantics:
1. **Declared requirement** (in `AcceptedArtifact`): the set of capabilities the artifact's actions require.
2. **Granted permission** (in `RunAdmission`): the set of capabilities the operator has granted for this run.

Admission checks strict set equality between declared requirements and the granted permission set: every declared requirement matches an element of the granted permission set, AND every element of the granted permission set is matched by a declared requirement (cardinality-exact and membership-exact, mirroring `VERUS-CARD-003` / `verification/verus/capability_artifact_model.rs::exact_profile`). Mismatch in either direction — a missing required capability OR an undeclared granted capability — causes `CapabilityDenied` rejection.

Capability checking occurs at admission time (cold path) only. The runtime does not re-check capabilities during execution. `Box<str>` is acceptable because admission is cold-path.

---
