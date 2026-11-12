# Boundary Map: Digest Covers Collect Semantics (vb-xi2f.38)

## Boundary Layers

```
┌─────────────────────────────────────────────────────────────────────┐
│                    BOUNDARY 0: External Input                       │
│  YAML bytes from user/authoring tool                                 │
│  - Raw workflow definition                                           │
│  - Parsed by vb_yaml parser                                         │
└──────────────────────────┬──────────────────────────────────────────┘
                           │ WorkflowSource (AST)
                           ▼
┌─────────────────────────────────────────────────────────────────────┐
│                 BOUNDARY 1: Pure Core (Digest)                       │
│                                                                     │
│  canonical_digest(source) -> WorkflowDigest                         │
│  digest_step_primitive(hasher, primitive)                           │
│                                                                     │
│  NO I/O, NO TIME, NO STORAGE, NO NETWORK, NO RANDOMNESS             │
│                                                                     │
│  vb_yaml::ast::WorkflowSource (input)                               │
│  blake3::Hasher (effect)                                            │
│  vb_core::ids::WorkflowDigest (output)                              │
└──────────────────────────┬──────────────────────────────────────────┘
                           │ WorkflowDigest + WorkflowSource
                           ▼
┌─────────────────────────────────────────────────────────────────────┐
│              BOUNDARY 2: Imperative Shell (Compile)                  │
│                                                                     │
│  compile_workflow(source) -> CompiledWorkflow                      │
│                                                                     │
│  YAML bytes -> AST -> IR nodes                                      │
│  Lowering: Collect -> CollectStart/SetConst/CollectPage/CollectFinish│
│                                                                     │
│  vb_yaml (parsing)                                                  │
│  vb_compile (lowering, compilation)                                 │
│  vb_validate (validation)                                          │
└──────────────────────────┬──────────────────────────────────────────┘
                           │ CompiledWorkflow / WorkflowParts
                           ▼
┌─────────────────────────────────────────────────────────────────────┐
│                BOUNDARY 3: Storage (Artifact)                       │
│                                                                     │
│  compute_compiled_digest(artifact_bytes) -> WorkflowDigest          │
│  Fjall KV store: digest -> artifact_bytes                           │
│                                                                     │
│  vb_storage (persistence)                                           │
│  vb_core (ids, types)                                               │
└──────────────────────────┬──────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────────┐
│                BOUNDARY 4: Runtime (Execution)                      │
│                                                                     │
│  vb_runtime::primitives::collect                                     │
│  CollectStart / CollectPage / CollectNext / CollectFinish          │
│                                                                     │
│  CollectPaginationState (runtime state)                             │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Pure Core: digest_step_primitive

### Location
- `vb_compile/src/mod_compile_lowering/part_05.rs` (lines 140–161)
- `vb_compile/src/compile/mod.rs` (lines 243–261)

### Signature
```rust
fn digest_step_primitive(hasher: &mut blake3::Hasher, primitive: &vb_yaml::ast::StepPrimitive)
```

### What Crosses the Boundary
- **Input**: `blake3::Hasher` (mutable reference), `&StepPrimitive` (reference)
- **Output**: `()` (mutates hasher in place)
- **Effect**: BLAKE3 state updated with serialized primitive content

### Purity Guarantee
No side channels:
- No `unsafe`
- No system clock
- No file system
- No network
- No random number generation
- No `unwrap`/`expect`/`panic`

---

## Imperative Shell: compile_workflow

### Location
- `vb_compile/src/compile/mod.rs` — `compile_workflow`

### What Crosses the Boundary
- YAML bytes → `Result<CompiledWorkflow, CompileErrors>`
- Panics possible on: YAML parse failure, OOM during lowering

### Collect Lowering Path
```
StepPrimitive::Collect
        │
        ▼
lower_canonical_collect (part_03.rs)
        │
        ├─► CollectStart { source_slot, limit, page_size, body, done }
        ├─► SetConst (from body Set step, inserted inline)
        ├─► CollectPage { collector_slot, body, done }
        └─► CollectFinish { collector_slot }
```

### Validation Boundaries
- `vb_validate::shared::validate` — validates `WorkflowParts`
- `vb_validate::gates::validate_node_pairing` — validates collect node pairing
- `WorkflowError` possible: `EmptyNodes`, `StepOutOfBounds`, etc.

---

## Storage Boundary: vb_storage

### Admission Check
```rust
// vb_storage admission pseudocode
fn admit_artifact(digest: WorkflowDigest, bytes: &[u8]) -> Result<(), StorageError> {
    let computed = compute_compiled_digest(bytes);
    if computed != digest {
        return Err(ArtifactDigestMismatch);
    }
    store(digest, bytes)
}
```

### Digest Role at Storage Boundary
- `WorkflowDigest` is the **content-addressed key** for the artifact
- Storage lookup: `load(digest) -> artifact_bytes`
- If `compute_compiled_digest(loaded_bytes) != stored_digest` → fail-closed

---

## Runtime Boundary: vb_runtime

### Collect Execution
```rust
// vb_runtime/src/primitives/collect.rs
pub fn collect_start(...) -> Result<CollectEffect, RuntimeError>
pub fn collect_page(...) -> Result<CollectEffect, RuntimeError>
pub fn collect_next(...) -> Result<CollectEffect, RuntimeError>
pub fn collect_finish(...) -> Result<CollectEffect, RuntimeError>
```

### Runtime State
```rust
pub struct CollectPaginationState {
    pub run_id: RunId,
    pub slot: SlotIdx,
    pub list_id: ListId,
    pub cursor: u32,       // Current page number
    pub limit: u32,        // Max pages
    pub page_size: u32,   // Items per page
}
```

### What Crosses the Boundary
- `CompiledWorkflow` with embedded `WorkflowDigest`
- `CollectPaginationState` for stateful pagination
- `RunFrame` with slot values

---

## Parser Boundary: vb_yaml

### What Crosses the Boundary
- YAML bytes → `WorkflowSource` (AST)
- `parse_collect` creates `StepPrimitive::Collect` with all fields populated
- Parser never sees `WorkflowDigest` — digest is computed after parsing

### No `unsafe` in Parser
All `vb_yaml` parsing is safe Rust:
- `saphyr` YAML parser (pure Rust)
- No `unsafe` blocks
- No `unwrap` on external input paths (errors returned as `YamlResult`)

---

## Boundary Hazards

### H-1: Parser-to-Digest Type Escape
**Description**: `WorkflowSource` is parsed once but must not be mutated between parsing and digest computation.
**Boundary**: Boundary 0 → Boundary 1
**Risk**: If AST is mutated, digest would reflect mutated state.

### H-2: Serialization Non-Determinism
**Description**: `compute_compiled_digest` hashes serialized bytes; if `serde` serialization is non-deterministic (e.g., due to `HashMap` iteration order), same IR could produce different artifact digests.
**Boundary**: Boundary 2 → Boundary 3
**Risk**: Same `CompiledWorkflow` → different artifact digest → content-addressing breaks

### H-3: Lowering IR Drift
**Description**: `canonical_digest` captures YAML AST fields; `compute_compiled_digest` captures serialized IR. If lowering is non-deterministic (same YAML → different IR), the two-stage digest system would show inconsistency.
**Boundary**: Boundary 1 ↔ Boundary 2
**Risk**: Source digest correct but IR changes → artifact digest changes

### H-4: Collect Body Step Hash Ordering
**Description**: `Collect.body: Vec<StepAst>` is hashed in iteration order. If two bodies have same steps in different order, they should (must) produce different digests because step IDs differ. However, if body steps share IDs, ordering matters for the hasher.
**Boundary**: Boundary 1 (pure core)
**Risk**: Low — body steps are hashed in order, step IDs are included

---

## Ownership Boundary

```
vb_yaml        vb_compile         vb_validate        vb_storage         vb_runtime
   │                │                   │                  │                 │
   │ WorkflowSource │                   │                  │                 │
   │───────────────►│                   │                  │                 │
   │                │                   │                  │                 │
   │                │ canonical_digest  │                  │                 │
   │                │──────────────────►│                  │                 │
   │                │                   │                  │                 │
   │                │ CompiledWorkflow  │ WorkflowParts    │                 │
   │                │◄─────────────────│◄─────────────────│                  │
   │                │                   │                  │                 │
   │                │                   │ WorkflowDigest   │ ArtifactDigest  │
   │                │                   │◄─────────────────│◄────────────────│
   │                │                   │                  │                 │
   │                │                   │                  │ CompiledWorkflow│
   │                │                   │                  │────────────────►│
   │                │                   │                  │                 │
   │                │                   │                  │    CollectStart │
   │                │                   │                  │◄────────────────│
```
