# Type Contracts: ResourceContract Digest Coverage

## Bead

`vb-xi2f.35` — P1: digest covers resource contract semantics

## Type Contract Philosophy

Per Scott Wlaschin DDD: make illegal states unrepresentable. The ResourceContract digest gap is a failure of this principle — the type system allows two workflows with different contracts to have the same `WorkflowDigest`, enabling silent contract substitution.

## Contract 1: ResourceContract Canonical Type

**Location**: `crates/vb_core/src/workflow/mod.rs:191-228`

**Current State**: 17 field value object. Two fields (`max_transitions_per_tick`, `allows_secret_results`) are missing from the duplicate in `compiled_workflow.rs`.

**Contract**:
- All 17 fields are `Copy`, `PartialEq`, `Eq`
- Serializable via serde (Serialize, Deserialize)
- Has a `DEFAULT` const with conservative bounds
- **Must be hashed into canonical digest** (not currently done)

**Primitive Obsession Checklist**:
- [x] `max_steps: u16` → OK, domain-appropriate newtype not needed
- [x] `allows_secret_results: bool` → **REJECTED**: Boolean behavior flag. Should be `enum SecretResultPolicy { Allow, Deny }` for semantic clarity, though `bool` is acceptable for a single-policy toggle.
- [ ] All other fields → OK, numeric limits are domain-appropriate

## Contract 2: WorkflowDigest

**Location**: `crates/vb_core/src/ids/mod.rs`

**Current State**: 32-byte wrapper over blake3 hash.

**Contract**:
- `from_bytes(bytes: [u8; 32]) -> Self`
- `as_bytes(&self) -> &[u8; 32]`
- Must be deterministic given identical semantic inputs
- Must differ given different semantic inputs (INVARIANT VIOLATION)

**Required Behavior**:
```
For all (source, contract_a, contract_b) where contract_a != contract_b:
    canonical_digest(source, contract_a) != canonical_digest(source, contract_b)
```

**Parsing Boundary**: `WorkflowDigest` is created from raw bytes. No validation beyond length.

## Contract 3: canonical_digest() Function Signature

**Current State (part_05.rs:116-138, compile/mod.rs:220-241)**:
```rust
fn canonical_digest(source: &WorkflowSource) -> WorkflowDigest
```

**Required State**:
```rust
fn canonical_digest(source: &WorkflowSource, contract: ResourceContract) -> WorkflowDigest
```

**Pre-conditions**:
- `source` must be a valid parsed `WorkflowSource`
- `contract` must be a valid `ResourceContract` (all fields initialized)

**Post-conditions**:
- Result is deterministic: same `(source, contract)` → same digest
- Result is contract-sensitive: different `contract` → different digest
- Hash includes ALL 17 contract fields in a canonical order

**Contract Encoding for Hashing**:
Each field must be hashed in a stable order, with domain tags to prevent cross-field collisions:

```rust
// Conceptual contract digest encoding:
fn hash_contract_fields(hasher: &mut blake3::Hasher, contract: &ResourceContract) {
    hasher.update(b"resource_contract");
    hasher.update(b"max_steps");            hasher.update(&contract.max_steps.to_le_bytes());
    hasher.update(b"max_slots");            hasher.update(&contract.max_slots.to_le_bytes());
    hasher.update(b"max_constants");        hasher.update(&contract.max_constants.to_le_bytes());
    hasher.update(b"max_accessors");        hasher.update(&contract.max_accessors.to_le_bytes());
    hasher.update(b"max_expressions");      hasher.update(&contract.max_expressions.to_le_bytes());
    hasher.update(b"max_expr_stack");       hasher.update(&[contract.max_expr_stack]);
    hasher.update(b"max_step_budget");      hasher.update(&contract.max_step_budget_per_tick.to_le_bytes());
    hasher.update(b"max_transitions");      hasher.update(&contract.max_transitions_per_tick.to_le_bytes());
    hasher.update(b"max_input_bytes");      hasher.update(&contract.max_input_bytes.to_le_bytes());
    hasher.update(b"max_output_bytes");     hasher.update(&contract.max_output_bytes.to_le_bytes());
    hasher.update(b"max_blob_bytes");       hasher.update(&contract.max_blob_bytes.to_le_bytes());
    hasher.update(b"max_ipc_payload");      hasher.update(&contract.max_ipc_payload_bytes.to_le_bytes());
    hasher.update(b"max_retry_attempts");   hasher.update(&contract.max_retry_attempts.to_le_bytes());
    hasher.update(b"max_fanout");           hasher.update(&contract.max_fanout.to_le_bytes());
    hasher.update(b"max_collect_items");    hasher.update(&contract.max_collect_items.to_le_bytes());
    hasher.update(b"max_queue_depth");      hasher.update(&contract.max_queue_depth.to_le_bytes());
    hasher.update(b"max_journal_batch");    hasher.update(&contract.max_journal_batch_bytes.to_le_bytes());
    hasher.update(b"allows_secret_results");hasher.update(&[contract.allows_secret_results as u8]);
}
```

## Contract 4: Duplicate ResourceContract Type Resolution

**Problem**: Two types with the same name `ResourceContract`:
1. `vb_core::workflow::ResourceContract` — 17 fields (re-exported by `lib.rs`)
2. `vb_core::compiled_workflow::ResourceContract` — 15 fields (used by `validation/resource.rs`)

**Resolution Contract**: Exactly one ResourceContract type must exist:
- Either delete `compiled_workflow::ResourceContract` and route all users to `workflow::ResourceContract`
- Or extend `compiled_workflow::ResourceContract` to 17 fields and delete `workflow::ResourceContract`

**If deleting compiled_workflow::ResourceContract**:
- `crates/vb_core/src/validation/resource.rs` must switch its import from `crate::compiled_workflow::ResourceContract` to `crate::workflow::ResourceContract`
- `crates/vb_core/src/compiled_workflow.rs::CompiledWorkflow` must use `crate::workflow::ResourceContract`
- `crates/vb_core/src/compiled_workflow.rs::WorkflowParts` must use `crate::workflow::ResourceContract`
- The `symbols_count` field divergence in `WorkflowParts` must also be resolved

## Contract 5: Compilation Entry Point Contract

**Current State**: All 6 compilation entry points hardcode `resource_contract: ResourceContract::DEFAULT`:
- `part_01.rs:54` — `compile_source()`
- `part_05.rs:189` — `lower_steps_to_ir()`
- `part_08.rs:103` — `SlotCompiler::build_parts()`
- `compile/mod.rs:105` — `compile_source()` (alt path)
- `compile/mod.rs:308` — `lower_steps_to_ir()` (alt path)
- `compile/mod.rs:854-872` — `SlotCompiler::build_parts()` (alt path)

**Required Contract**: Compilation entry points must accept an optional `ResourceContract` parameter:
```rust
fn compile_source(
    source: &WorkflowSource,
    contract: ResourceContract,  // NOT Option<ResourceContract> — explicit choice
) -> Result<CompiledWorkflow, CompileErrors>
```

**Pre-condition**: `contract` is a valid ResourceContract (validated by caller or at boundary).

**Post-condition**: The resulting `CompiledWorkflow::digest` reflects `contract`.

## Contract 6: YAML Parser Resource Contract Extension

**Location**: `crates/vb_yaml/src/ast/types.rs`, `crates/vb_yaml/src/ast/parse.rs`

**Current State**: `WorkflowSource` has no resource contract fields. Parser whitelist: `["version", "name", "when", "inputs", "vars", "secrets", "steps", "result", "examples"]`.

**Contract**: If resource contracts are sourced from YAML:
- `WorkflowSource` must gain an `Option<ResourceContract>` field (parsed from YAML)
- Parser whitelist must include `"resource_contract"` as an allowed top-level key
- Unknown fields in the resource contract must be rejected
- Missing resource contract → parser provides `ResourceContract::DEFAULT`

**If resource contracts are NOT sourced from YAML** (e.g., API parameter, separate config):
- `WorkflowSource` remains unchanged
- `canonical_digest` and `compile_source` accept the contract as a separate parameter
- The YAML parser contract is unchanged

## Contract 7: Validation Boundary

**Location**: `crates/vb_core/src/validation/resource.rs`

**Current State**: Uses `crate::compiled_workflow::ResourceContract` (15-field). Validates `max_steps`, `max_slots`, `max_constants`, `max_accessors`, `max_expressions`, `max_expr_stack`. Does **not** validate `max_transitions_per_tick` or `allows_secret_results` because the type doesn't have those fields.

**Required Contract**:
- Must use the canonical 17-field ResourceContract
- Must validate `max_transitions_per_tick` against `HARD_MAX_TRANSITIONS_PER_TICK`
- Must validate `allows_secret_results` for consistency (no hard limit, but must be a valid bool)
- Validation failures produce `WorkflowError::ResourceContractExceeded` or `WorkflowError::ResourceContractTooLarge`

## Contract 8: Policy Digest vs Canonical Digest

**Current State**: `vb_storage::admission::compute_policy_digest()` hashes the resource contract separately via postcard serialization. This is a separate digest from the canonical digest.

**Contract**: The policy digest and canonical digest serve different purposes:
- **Canonical digest**: Identifies the workflow's complete semantics including contract. Must change when contract changes.
- **Policy digest**: Identifies the admission policy (contract) independently. May equal the canonical digest's contract portion.
- These are NOT the same thing and do NOT need to be unified. However, the canonical digest MUST also cover the contract.

## Type Contract Violations (Current State)

| Violation | Severity | Detail |
|-----------|----------|--------|
| Duplicate ResourceContract | HIGH | Two types with same name, different fields. `validation/resource.rs` cannot access `max_transitions_per_tick` or `allows_secret_results`. |
| Digest ignores contract | HIGH | `canonical_digest()` signature takes only `WorkflowSource`, not `ResourceContract`. |
| DEFAULT hardcoded everywhere | HIGH | No compilation path accepts a non-default contract. |
| Missing YAML contract parsing | MEDIUM | No path to specify contract in source YAML. |
| `allows_secret_results` as bool | LOW | Boolean flag; acceptable for single-policy toggle but semantically clearer as enum. |

## Smart Constructors (Required)

| Constructor | Pre-conditions | Post-conditions |
|-------------|---------------|-----------------|
| `ResourceContract::try_new(fields...)` | All numeric fields ≤ hard limits; all fields valid | Returns `Ok(Self)` or `Err(ContractValidationError)` |
| `WorkflowDigest::from_canonical_digest(source, contract)` | Valid source, valid contract | Deterministic, contract-sensitive digest |
| `WorkflowParts::new(name, digest, ..., contract)` | All fields validated | Contract flows into CompiledWorkflow |
| `parse_workflow_from_yaml(root, maybe_contract)` | Root is valid YAML mapping | Contract from YAML or DEFAULT used |

## Railway Error Types

| Error Variant | When |
|---------------|------|
| `ContractValidationError::FieldExceedsHardLimit { field, requested, hard_limit }` | Any contract field exceeds hard system limit |
| `ContractValidationError::MissingField { field }` | Required field absent in parsed contract |
| `CompileError::ContractDigestMismatch` | Contract does not match expected digest |
| `YamlError::InvalidResourceContract { field, reason }` | Invalid contract in YAML source |
