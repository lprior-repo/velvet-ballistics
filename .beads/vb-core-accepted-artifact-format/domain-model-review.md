# Domain Model Review — AcceptedArtifact Format

## Model Shape

### Core Types

| Type | File | Purpose |
|------|------|---------|
| `AcceptedArtifact` | `vb_storage/src/admission.rs:104` | Persistent artifact envelope |
| `VerificationProof` | `vb_storage/src/admission.rs:60` | Proof of verification passage |
| `VerificationWarning` | `vb_storage/src/admission.rs:17` | Soft failure during admission |
| `WorkflowParts` | `vb_core/src/workflow/mod.rs:255` | Pre-gate compiled representation |
| `CompiledWorkflow` | `vb_core/src/compiled_workflow.rs:12` | Post-gate immutable workflow |
| `ResourceContract` | `vb_core/src/budget.rs` | Resource bounds for artifact |
| `ArtifactEnvelopeError` | `vb_runtime/src/admission.rs:23` | Runtime admission rejection |

### Type Hierarchy

```
CompiledWorkflow (gate-checked, not Serialize)
    ↑
WorkflowParts (Serialize, intermediate)
    ↑
[CI runtime path: YAML → WorkflowParts → CompiledWorkflow]

AcceptedArtifact (persistent envelope)
├── digest: WorkflowDigest
├── ir: Vec<u8> (postcard WorkflowParts)
├── verification: VerificationProof
├── accepted_at_seq: EventSeq
└── required_capabilities: Box<[Capability]>

VerificationProof
├── digest: WorkflowDigest
├── gate_count: u8  ← MISMATCH: storage=2, runtime=15
├── durable: bool
├── bounded: bool
├── taint_safe: bool
├── retry_safe: bool
├── replayable: bool
├── idempotency_keyed: Box<[ActionId]>
├── idempotency_attested: Box<[ActionId]>
└── warnings: Vec<VerificationWarning>
```

## Gate Count Mismatch Analysis

### Current State

- **vb_storage** admission produces `VerificationProof { gate_count: 2, ... all flags = true }` (see `admission.rs:118` and `VerificationProof::new` at line 86)
- **vb_runtime** admission requires `gate_count == 15` (see `admission.rs:16`)
- Result: stored artifacts rejected under Strict/Journaled policy

### Structural Observation

`VerificationProof::new` (line 86-99) hardcodes all proof flags to `true`:

```rust
pub fn new(digest: WorkflowDigest, gate_count: u8, durable: bool) -> Self {
    Self {
        digest,
        gate_count,
        durable,
        bounded: true,      // hardcoded
        taint_safe: true,   // hardcoded
        retry_safe: true,   // hardcoded
        replayable: true,   // hardcoded
        idempotency_keyed: Box::new([]),
        idempotency_attested: Box::new([]),
        warnings: Vec::new(),
    }
}
```

This means the current 2-gate proof is not derived from actual verification — it is a trust placeholder.

## Invariant Review

| ID | Invariant | Status |
|----|-----------|--------|
| INV-001 | digest == sha256(ir) | Maintained by `submit_artifact` checksum validation |
| INV-002 | gate_count >= 1 | Maintained — minimum 1 gate always passes |
| INV-003 | Proof flags derived from gates | **VIOLATED** — currently hardcoded true |
| INV-004 | CompiledWorkflow sole constructor | Maintained — `try_from_parts` is only impl |
| INV-005 | Atomic persistence | Needs Fjall transaction confirmation |

## Critical Path Review

### Happy Path: submit_artifact

1. `submit_artifact(journal, workflow, policy)` called
2. `submit_artifact_with_contracts` invoked
3. Policy check: Relaxed skips gate validation
4. Structure re-validation: re-parse IR from `workflow.to_parts()`
5. Checksum: `digest == sha256(ir_bytes)`
6. Gate validation: `gate_count == 2 && all_flags`
7. Persistence: `journal.put_compiled_ir(record)`
8. Durability: if Strict, `SyncAll` before return
9. Returns `AcceptedArtifact { gate_count: 2, ... }`

### Failure Path: runtime admission

1. `StorageArtifactStore::load_accepted_artifact` called
2. Artifact loaded from Fjall
3. Postcard decode of envelope
4. **Gate count check: `gate_count == 15`** ← FAILS (found=2)
5. Returns `ArtifactEnvelopeError::InvalidGateCount { found: 2, required: 15 }`

## Model Ambiguities

1. **accepted_at_seq**: Set to `EventSeq::new(0)` in storage admission — not a real journal sequence
2. **required_capabilities**: Extracted from action contracts but never validated against runtime grants
3. **idempotency_keyed/attested**: Always empty — no actual idempotency validation
4. **warnings**: Always empty — no actual gate warnings emitted

## Ownership

| Component | Owner |
|-----------|-------|
| `AcceptedArtifact` format | vb_storage |
| `ArtifactEnvelopeError` | vb_runtime |
| `CompiledWorkflow` gate | vb_core |
| Gate count alignment | vb-core-proof-15-gate (deferred) |
