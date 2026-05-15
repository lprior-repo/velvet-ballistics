# Codebase Map — vb-core-proof-gate-inputs

## Bead
- **bead_id**: vb-core-proof-gate-inputs
- **title**: artifact: Derive all VerificationProof gate inputs
- **phase**: 2 (Explore and Scope)
- **source**: vb_storage admission.rs, vb_core validation.rs

---

## VerificationProof Type

**Location**: `crates/vb_storage/src/admission.rs` lines 58-100

```rust
pub struct VerificationProof {
    pub digest: vb_core::WorkflowDigest,       // Confirmed digest of verified artifact
    pub gate_count: u8,                        // Number of verification gates passed
    pub durable: bool,                        // SyncAll durability
    pub bounded: bool,                         // IR is size-bounded
    pub taint_safe: bool,                      // Does not propagate taint
    pub retry_safe: bool,                      // Actions safe to retry
    pub replayable: bool,                      // Can be replayed
    pub idempotency_keyed: Box<[vb_core::ActionId]>,   // Actions keyed by idempotency key
    pub idempotency_attested: Box<[vb_core::ActionId]>,  // Actions with idempotency attested
    pub warnings: Vec<VerificationWarning>,    // Soft verification failures
}
```

**Derived from**: `VerificationProof::new(digest, gate_count, durable)` (lines 83-99)
- All proof flags (bounded, taint_safe, retry_safe, replayable) default to `true`
- idempotency lists default to empty
- warnings default to empty

---

## Gate Inputs (ADMISSION_GATE_COUNT = 2)

### Gate 1: Structure Validation
**File**: `crates/vb_core/src/compiled_workflow.rs` lines 27-42

```rust
pub fn try_from_parts(parts: WorkflowParts) -> Result<Self, WorkflowError> {
    validate_parts(&parts)?;    // Validates nodes, expressions, accessors, resource contract, reachability, forward edges
    validate_budget(&parts)?;  // Validates whole-workflow budget against BoundednessPolicy
    ...
}
```

**validate_parts** (vb_core/src/validation.rs lines 113-129):
- `validate_resource_contract(parts)` - resource contract limits
- `validate_entry(entry, nodes.len())` - entry step in bounds
- `validate_expressions(expressions, accessors.len())` - expression bytecode valid
- `validate_accessors(accessors, slot_count)` - accessor programs valid
- `validate_node_id(node, index)` - node id matches array position
- `nodes::validate_node(node, parts)` - node-specific validation
- `validate_accessor_path_symbols(accessors)` - symbol references valid
- `graph::validate_reachability(parts)` - all nodes reachable from entry
- `graph::validate_forward_edges(parts)` - no backward edges except loops

**validate_budget** (vb_core/src/validation.rs lines 131-168):
- `WholeWorkflowBudget::compute(nodes, entry, resource_contract)?`
- `BoundednessPolicy::DEFAULT.validate(&budget)` → BudgetError mapping

### Gate 2: Checksum Validation
**File**: `crates/vb_storage/src/admission.rs` lines 177-183

```rust
let mut parts_for_hash = parts.clone();
parts_for_hash.digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
let hash_bytes = postcard::to_allocvec(&parts_for_hash)
    .map_err(|_| JournalError::ArtifactMalformed)?;
let computed = blake3::hash(&hash_bytes);
if computed.as_bytes() != &workflow.digest().as_bytes() {
    return Err(JournalError::ArtifactChecksumMismatch);
}
```

---

## Policy Behavior

| Policy | gate_count | durable | bypasses_gates |
|--------|-----------|---------|----------------|
| Relaxed | 0 | false | yes (both gates skipped) |
| Journaled | 2 | false | no |
| Strict | 2 | true | no (plus SyncAll) |

---

## Proof Flag Derivation

### bounded
- **Source**: `validate_budget()` passes → `bounded = true`
- **Default**: `true` in `VerificationProof::new`

### taint_safe, retry_safe, replayable
- **Source**: Default to `true` in `VerificationProof::new`
- **Runtime**: Action contracts may modify these flags via `submit_artifact_with_contracts`
- **Evidence**: `crates/vb_storage/src/admission.rs` line 188 creates proof with all flags true

### idempotency_keyed, idempotency_attested
- **Source**: Derived from `action_contracts` parameter in `submit_artifact_with_contracts`
- **Default**: Empty in `VerificationProof::new`

### warnings
- **Source**: `Vec<VerificationWarning>` populated during admission
- **MAX_GATE**: 2 (defined in `VerificationWarning::MAX_GATE`)
- **MIN_GATE**: 1 (defined in `VerificationWarning::MIN_GATE`)

---

## Key Files

| File | Role |
|------|------|
| `crates/vb_storage/src/admission.rs` | VerificationProof type, submit_artifact, admit_compiled_artifact |
| `crates/vb_core/src/compiled_workflow.rs` | CompiledWorkflow::try_from_parts (structure gate) |
| `crates/vb_core/src/validation.rs` | validate_parts, validate_budget |
| `crates/vb_core/src/validation/resource.rs` | Resource contract validation |
| `crates/vb_core/src/validation/nodes.rs` | Node-specific validation |
| `crates/vb_core/src/validation/graph.rs` | Reachability and forward-edge validation |
| `crates/vb_core/src/validation/targets.rs` | Target collection helpers |
| `crates/vb_storage/src/error/mod.rs` | JournalError with gate-related variants |
| `crates/vb_storage/src/error/warnings.rs` | VerificationWarning type |
| `crates/vb_storage/src/vb_2bok_durability_gate_tests.rs` | Durability gate BDD tests |
| `crates/vb_storage/tests/accepted_artifact_red_phase.rs` | AcceptedArtifact integration tests |
| `crates/vb_core/src/budget.rs` | WholeWorkflowBudget, BoundednessPolicy |
| `crates/vb_core/src/action.rs` | ActionContract, Idempotency, RetrySafety |

---

## Risk Tags

- **persistence**: FjallJournal storage, SyncAll durability
- **parser/codec**: postcard serialization, blake3 hashing, WorkflowParts roundtrip
- **public_API**: submit_artifact, admit_compiled_artifact, VerificationProof::new
- **concurrency**: SyncAll persistence may involve fsync
- **performance**: BLAKE3 hash computation on full workflow parts

---

## Required Verifier Modes

- **KANI**: `kani_idempotency_gates.rs` (KANI-RUNTIME-001 through KANI-RUNTIME-006)
- **MIRI**: Stacked Borrows on admission.rs pointer handling
- **LOOM**: Concurrency permutation on journal persistence
- **PROPTEST**: Structure validation shrinking on invalid workflow parts

---

## Open Questions

1. **idempotency_keyed/attested derivation**: The bead title says "derive all VerificationProof gate inputs" — the idempotency lists are derived from action_contracts but the exact derivation logic in `required_capabilities_from_contracts` only extracts capabilities, not idempotency keys. Need to confirm if idempotency derivation is a separate pass or part of action contract validation.

2. **bounded flag**: Currently defaults to true in `VerificationProof::new`. Should bounded be conditionally set based on budget validation result, or is the structure gate sufficient to guarantee boundedness?

3. **taint_safe/retry_safe/replayable**: These proof flags default to true but there is no visible derivation chain from action contracts. Need to verify if these are validated elsewhere.
