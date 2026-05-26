# Domain Model: ResourceContract Digest Coverage

## Bead

`vb-xi2f.35` — P1: digest covers resource contract semantics

## Ubiquitous Language

| Term | Definition |
|------|------------|
| **ResourceContract** | A 17-field value object encoding the explicit resource bounds and behavioral flags that a compiled workflow claims at admission. It is the compile-time promise that the runtime validates. |
| **ResourceContract (duplicate)** | A 15-field duplicate in `compiled_workflow.rs` missing `max_transitions_per_tick` and `allows_secret_results`. Used by `validation/resource.rs`. Divergent from canonical. |
| **Canonical Digest** | `canonical_digest(source)` — the blake3 hash computed during compilation that uniquely identifies a workflow's semantics. Currently hashes ONLY source-level properties: version, name, trigger AST, step IDs, step primitive names/values. |
| **Compiled Digest** | `compute_compiled_digest(source)` — blake3 hash of raw source bytes. Distinct from canonical digest. Used for artifact verification. |
| **Policy Digest** | `compute_policy_digest(workflow)` — blake3 hash of the serialized ResourceContract. Computed at admission time by `vb_storage::admission`. |
| **WorkflowParts** | Untrusted compiled workflow parts emitted by a compiler boundary. Contains `resource_contract: ResourceContract`. Exists in two divergent forms (with and without `symbols_count`). |
| **CompiledWorkflow** | Immutable compiled workflow that has passed validation. Contains `resource_contract`. Exists in two divergent forms. |
| **Taint** | Security classification: `Clean`, `Secret`, `DerivedFromSecret`. Tracks data lineage. |
| **allows_secret_results** | Boolean flag on ResourceContract. When `false`, the runtime rejects secret-tainted answer payloads with `RuntimeError::SecretResultNotAllowed`. |
| **max_transitions_per_tick** | u64 limit on transitions per runtime tick. Enforced by `vb_core::budget::CapBudget`. Missing from the 15-field duplicate. |
| **DEFAULT** | `ResourceContract::DEFAULT` — a const with conservative bounds (max_steps=10000, max_slots=1024, allows_secret_results=false, etc.). Hardcoded at every compilation entry point. |

## Entities and Value Objects

### Value Object: ResourceContract (canonical, 17-field)

```rust
// Canonical, in vb_core::workflow::ResourceContract
pub struct ResourceContract {
    pub max_steps: u16,
    pub max_slots: u16,
    pub max_constants: u16,
    pub max_accessors: u16,
    pub max_expressions: u16,
    pub max_expr_stack: u8,
    pub max_step_budget_per_tick: u64,
    pub max_transitions_per_tick: u64,      // MISSING from duplicate
    pub max_input_bytes: u32,
    pub max_output_bytes: u32,
    pub max_blob_bytes: u64,
    pub max_ipc_payload_bytes: u32,
    pub max_retry_attempts: u16,
    pub max_fanout: u16,
    pub max_collect_items: u32,
    pub max_queue_depth: u32,
    pub max_journal_batch_bytes: u32,
    pub allows_secret_results: bool,         // MISSING from duplicate
}
```

**Invariants**: All 17 fields are `Copy` value types. The struct is `PartialEq, Eq, Serialize, Deserialize`. It is **not** hashed into the canonical digest.

### Value Object: ResourceContract (duplicate, 15-field)

```rust
// Duplicate, in vb_core::compiled_workflow::ResourceContract
pub struct ResourceContract {
    pub max_steps: u16,
    // ... 13 more fields ...
    pub max_journal_batch_bytes: u32,
    // MISSING: max_transitions_per_tick
    // MISSING: allows_secret_results
}
```

**Illegal State**: The two types share the name `ResourceContract` but have different field sets. Code using the 15-field type cannot reference `max_transitions_per_tick` or `allows_secret_results`. The `validation/resource.rs` module uses the 15-field type, meaning it cannot validate those two dimensions.

### Value Object: WorkflowDigest

```rust
// vb_core::ids::WorkflowDigest
// 32-byte blake3 hash wrapper
```

**Invariant**: Two workflows with identical semantics should produce identical digests. Two workflows with different semantics (different resource contracts) should produce different digests. **Currently violated**: different resource contracts produce identical digests.

### Aggregate: CompiledWorkflow

Contains `resource_contract: ResourceContract` (the 15-field version in `compiled_workflow.rs`, the 17-field version in `workflow/mod.rs`). The digest (`self.digest`) is computed during compilation and must reflect ALL semantic properties including the resource contract.

### Aggregate: WorkflowParts

Untrusted parts emitted by compilation. Contains `resource_contract: ResourceContract`. The validation layer checks these parts before constructing a trusted `CompiledWorkflow`.

## Commands and Events

| Command | Source | Description |
|---------|--------|-------------|
| `compile_source(source)` | `vb_compile` | Compiles YAML source into `CompiledWorkflow`. Computes digest, hardcodes DEFAULT contract. |
| `canonical_digest(source)` | `vb_compile` | Produces a `WorkflowDigest` from source properties. (Does not hash contract.) |
| `canonical_digest(source, contract)` | NOT YET EXIST | **Desired**: Produces digest from source + contract. |
| `compute_policy_digest(workflow)` | `vb_storage` | Produces policy digest from serialized contract bytes. |
| `compute_compiled_digest(bytes)` | `vb_compile` | Produces digest from raw artifact bytes. |

## Domain Invariants

1. **Digest-Sensitivity Invariant**: If two `CompiledWorkflow` values differ in any ResourceContract field (any of the 17), their `WorkflowDigest` MUST differ.
2. **Taint-Sensitivity Invariant**: If `allows_secret_results` differs between two contracts, digests MUST differ. This is a behavior-affecting field: when `false`, secret-tainted answers are rejected at runtime.
3. **Limit-Sensitivity Invariant**: If any limit field differs (max_steps, max_slots, etc.), digests MUST differ. Runtime behavior depends on these limits.
4. **Determinism Invariant**: `canonical_digest(source, contract)` MUST be deterministic given identical inputs.
5. **Single-Canonical-Type Invariant**: There must be exactly one `ResourceContract` type in the codebase. The 15-field duplicate in `compiled_workflow.rs` must be resolved.

## Forbidden States

- Two contracts with different `allows_secret_results` producing identical digests (CURRENTLY HAPPENS).
- Two contracts with different `max_steps` producing identical digests (CURRENTLY HAPPENS).
- Any compilation path using the 15-field ResourceContract while the canonical type has 17 fields.
- `canonical_digest()` being called without access to the contract.
- `ResourceContract::DEFAULT` being silently used when a user-specified contract should apply.
- Digest collision between workflows with semantically different contracts.

## Open Domain Questions

1. **Source of Contract**: Where does a non-DEFAULT ResourceContract come from? YAML source? Separate config file? API parameter? Currently no path exists for a user to specify contract overrides.
2. **Duplicate Resolution**: Should `compiled_workflow.rs` be deleted and all references use `workflow/mod.rs`? Or should `workflow/mod.rs` be deleted and `compiled_workflow.rs` extended to 17 fields? The codebase-map notes the `workflow/mod.rs` type is the canonical one (re-exported by `lib.rs`).
3. **YAML Path**: If contracts come from YAML, the parser whitelist in `vb_yaml/src/ast/parse.rs` (currently: `["version", "name", "when", "inputs", "vars", "secrets", "steps", "result", "examples"]`) must be updated to include `resource_contract`.
4. **Dual Compilation Paths**: Should `mod_compile_lowering/part_05.rs` and `compile/mod.rs` be unified into one canonical compilation path? Currently both must be kept in sync.
5. **Digest Relationship**: Should the canonical digest be `hash(source) + hash(contract)` or `hash(source || contract_bytes)`? Or should the contract be folded into the source-level digest?
