# Codebase Map — vb-core-accepted-artifact-format

## Bead Scope

Define one stable `AcceptedArtifact` format for strict runtime admission and persistence. Remove ambiguity between raw `WorkflowParts`, `CompiledWorkflow`, and `AcceptedArtifact` on production paths.

## Key Type Definitions

### 1. `AcceptedArtifact` (vb_storage/admission.rs:104)
```rust
pub struct AcceptedArtifact {
    pub digest: vb_core::WorkflowDigest,           // artifact content hash
    pub ir: Vec<u8>,                                // serialized compiled IR (postcard)
    pub verification: VerificationProof,           // proof that verification passed
    pub accepted_at_seq: EventSeq,                  // journal sequence when accepted
    pub required_capabilities: Box<[vb_core::capability::Capability]>,
}
```

### 2. `VerificationProof` (vb_storage/admission.rs:60)
```rust
pub struct VerificationProof {
    pub digest: vb_core::WorkflowDigest,
    pub gate_count: u8,          // Number of verification gates that passed
    pub durable: bool,           // Whether proof was durably persisted (SyncAll)
    pub bounded: bool,           // Artifact IR is size-bounded
    pub taint_safe: bool,        // Artifact does not propagate taint
    pub retry_safe: bool,        // Artifact actions are safe to retry
    pub replayable: bool,        // Artifact can be replayed
    pub idempotency_keyed: Box<[vb_core::ActionId]>,
    pub idempotency_attested: Box<[vb_core::ActionId]>,
    pub warnings: Vec<VerificationWarning>,
}
```

### 3. `WorkflowParts` (vb_core/workflow/mod.rs:255)
```rust
pub struct WorkflowParts {
    pub name: Box<str>,
    pub digest: WorkflowDigest,
    pub nodes: Box<[CompiledNode]>,
    pub expressions: Box<[ExprProgram]>,
    pub accessors: Box<[AccessorProgram]>,
    pub constants: Box<[ConstValue]>,
    pub slot_count: u16,
    pub symbols_count: u32,
    pub entry: StepIdx,
    pub resource_contract: ResourceContract,
    pub step_names: Box<[Box<str>]>,
}
```

### 4. `CompiledWorkflow` (vb_core/compiled_workflow.rs:12)
```rust
pub struct CompiledWorkflow {
    name: Box<str>,
    digest: WorkflowDigest,
    nodes: Box<[CompiledNode]>,
    expressions: Box<[ExprProgram]>,
    accessors: Box<[AccessorProgram]>,
    constants: Box<[ConstValue]>,
    slot_count: u16,
    entry: StepIdx,
    resource_contract: ResourceContract,
    step_names: Box<[Box<str>]>,
}
```

## Critical Finding: Gate Count Mismatch

**Core ambiguity**: `vb_storage::admission::submit_artifact` creates artifacts with `ADMISSION_GATE_COUNT = 2` (vb_storage/admission.rs:118), but `vb_runtime::admission::StorageArtifactStore::load_accepted_artifact` requires `REQUIRED_GATE_COUNT = 15` (vb_runtime/admission.rs:16).

This means artifacts stored via `submit_artifact` cannot pass runtime admission validation under Strict/Journaled policy — stored artifacts will be rejected with `ArtifactEnvelopeError::InvalidGateCount`.

## Encoding

- **Artifact IR**: Postcard-encoded `WorkflowParts` stored in `AcceptedArtifact.ir`
- **Artifact record**: `CompiledIrRecord { digest, ir }` stored in Fjall `compiled_ir` keyspace
- **Artifact envelope**: Full `AcceptedArtifact` (postcard-encoded) stored via `journal.put_compiled_ir()`

## Stability Guarantees

1. `AcceptedArtifact` is `Serialize + Deserialize + Clone + PartialEq + Eq + Debug`
2. `VerificationProof` is `Serialize + Deserialize + Clone + PartialEq + Eq + Debug`
3. `submit_artifact` enforces checksum validation against claimed digest
4. `CompiledWorkflow::try_from_parts` is the gate for structural validity
5. `accepted_at_seq` set to `EventSeq::new(0)` in current storage admission (NOT a real journal seq)

## Artifact Store Hierarchy

| Store | Location | Behavior |
|-------|----------|----------|
| `AlwaysPresentArtifactStore` | vb_runtime/admission.rs:243 | Returns dummy artifact with `gate_count=15` — test-only |
| `StorageArtifactStore` | vb_runtime/admission.rs:274 | Loads from Fjall journal, validates 15 gates — production |
| `FixedAcceptedStore` | vb_runtime/admission.rs:551 | Test helper with hardcoded artifact |

## Files with AcceptedArtifact Usage

### vb_storage (source of truth for admission)
- `crates/vb_storage/src/admission.rs` — `submit_artifact`, `submit_artifact_with_contracts`, `AcceptedArtifact`, `VerificationProof`, `VerificationWarning`
- `crates/vb_storage/src/lib.rs` — re-exports admission types
- `crates/vb_storage/src/vb_2bok_durability_gate_tests.rs` — durability gate tests
- `crates/vb_storage/tests/accepted_artifact_red_phase.rs` — 31 integration tests for artifact format

### vb_runtime (consumer of accepted artifacts)
- `crates/vb_runtime/src/admission.rs` — `AcceptedArtifactStore` trait, `StorageArtifactStore`, `AlwaysPresentArtifactStore`, `REQUIRED_GATE_COUNT = 15`
- `crates/vb_runtime/src/shard/types.rs` — `artifact_store: SharedAcceptedArtifactStore`
- `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs` — uses `SharedAcceptedArtifactStore`

### vb_core (types only, no admission logic)
- `crates/vb_core/src/compiled_workflow.rs` — `CompiledWorkflow`, `WorkflowParts`, `ResourceContract`
- `crates/vb_core/src/workflow/mod.rs` — `WorkflowParts` (alternate re-export)

### CLI and IPC
- `crates/velvet_ballastics/src/run.rs` — deserializes `WorkflowParts` then converts to `CompiledWorkflow`
- `crates/velvet_ballastics/src/storage.rs` — resolves workflows via `CompiledWorkflow::try_from_parts`

## Risk Tags

- `persistence` — Fjall storage of compiled IR
- `parser/codec` — postcard encoding/decoding of artifacts
- `public_api` — AcceptedArtifactStore trait is exposed to runtime
- `migration` — format change would break stored artifacts
- `concurrency` — Arc<dyn AcceptedArtifactStore> shared across threads
- `verification` — gate count mismatch between storage and runtime

## Required Verifier Modes

1. **Kani** — `vb_core` workflow validation, `CompiledWorkflow::try_from_parts` correctness
2. **Miri** — postcard decode of untrusted IR bytes in `StorageArtifactStore::load_accepted_artifact`
3. **Loom** — concurrent access to `SharedAcceptedArtifactStore`
4. **Formal** — TLA+ model for admission state machine

## Open Questions

1. Should `accepted_at_seq` be set to a real journal sequence at submission time?
2. Should `ADMISSION_GATE_COUNT` in vb_storage be 15 instead of 2?
3. Should `VerificationProof::new` produce derived flags from actual gate outputs rather than all-true?
4. Is `AlwaysPresentArtifactStore` ever used in non-test production paths?
5. Should there be a version field in `AcceptedArtifact` for forward migration?

## Recommended Downstream Owners

- `rust-contract` → artifact schema contract
- `proof-planner` → gate count alignment proof obligations
- `test-planner` → postcard roundtrip tests, mismatch rejection tests
- `holzman-rust` → no-unsafe verification for admission.rs
