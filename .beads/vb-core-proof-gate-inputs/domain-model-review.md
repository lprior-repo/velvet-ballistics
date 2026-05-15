# Domain Model Review — VerificationProof Gate Inputs

## Scope

- **Bead**: vb-core-proof-gate-inputs
- **Target types**: `VerificationProof`, `VerificationWarning`, `AcceptedArtifact`, `ProofFlag`
- **Gate functions**: `submit_artifact`, `submit_artifact_with_contracts`
- **Validation functions**: `CompiledWorkflow::try_from_parts`, `validate_parts`, `validate_budget`

---

## Type: VerificationProof

**Location**: `crates/vb_storage/src/admission.rs` lines 58-100

### Fields (10 total)

| # | Field | Type | Contract |
|---|-------|------|----------|
| 1 | `digest` | `vb_core::WorkflowDigest` | Confirmed digest of verified artifact |
| 2 | `gate_count` | `u8` | Number of gates passed; ∈ {0, 2} |
| 3 | `durable` | `bool` | SyncAll durability (true only for Strict) |
| 4 | `bounded` | `bool` | IR is size-bounded (default true) |
| 5 | `taint_safe` | `bool` | Does not propagate taint (default true) |
| 6 | `retry_safe` | `bool` | Actions safe to retry (default true) |
| 7 | `replayable` | `bool` | Can be replayed (default true) |
| 8 | `idempotency_keyed` | `Box<[vb_core::ActionId]>` | Actions keyed by idempotency key |
| 9 | `idempotency_attested` | `Box<[vb_core::ActionId]>` | Actions with idempotency attested |
| 10 | `warnings` | `Vec<VerificationWarning>` | Soft verification failures |

### Constructor

```rust
pub fn new(digest: WorkflowDigest, gate_count: u8, durable: bool) -> Self
```

All proof flags default to `true`; idempotency lists default to empty `Box::new([])`.

### Invariants

- `gate_count ∈ {0, 2}` — Relaxed=0, Journaled/Strict=2
- `durable == true` implies `gate_count == 2`
- `bounded`, `taint_safe`, `retry_safe`, `replayable` are all `true` in default construction

---

## Type: VerificationWarning

**Location**: `crates/vb_storage/src/admission.rs` lines 12-43

### Fields

| Field | Type | Contract |
|-------|------|----------|
| `code` | `u32` | Numeric warning code |
| `message` | `Box<str>` | Human-readable description |
| `gate` | `u8` | Gate that produced this warning; ∈ [1, 2] |

### Constants

- `MIN_GATE: u8 = 1`
- `MAX_GATE: u8 = 2`

### Invariant

- `is_valid()` returns `gate >= 1 && gate <= 2`

---

## Type: AcceptedArtifact

**Location**: `crates/vb_storage/src/admission.rs` lines 102-115

### Fields

| Field | Type | Contract |
|-------|------|----------|
| `digest` | `vb_core::WorkflowDigest` | Content hash |
| `ir` | `Vec<u8>` | Serialized compiled IR (postcard) |
| `verification` | `VerificationProof` | Proof of verification |
| `accepted_at_seq` | `EventSeq` | Journal sequence when accepted |
| `required_capabilities` | `Box<[Capability]>` | Required capabilities for actions |

---

## Type: ProofFlag

**Location**: `crates/vb_storage/src/admission.rs` lines 45-56

Enum variants:
- `Bounded` — IR is size-bounded
- `TaintSafe` — Artifact does not propagate taint
- `RetrySafe` — Artifact actions are safe to retry
- `Replayable` — Artifact can be replayed

---

## Gate 1 — Structure Validation

**File**: `crates/vb_core/src/compiled_workflow.rs` lines 27-42

```rust
pub fn try_from_parts(parts: WorkflowParts) -> Result<Self, WorkflowError>
```

Calls:
- `validate_parts(&parts)?` — nodes, expressions, accessors, resource contract, reachability, forward edges
- `validate_budget(&parts)?` — whole-workflow budget against BoundednessPolicy

### validate_parts sub-validators

| Function | File | Purpose |
|----------|------|---------|
| `validate_resource_contract` | `validation/resource.rs` | Resource contract limits |
| `validate_entry` | `validation.rs` | Entry step in bounds |
| `validate_expressions` | `validation.rs` | Expression bytecode valid |
| `validate_accessors` | `validation.rs` | Accessor programs valid |
| `validate_node_id` | `validation.rs` | Node id matches array position |
| `validate_node` | `validation/nodes.rs` | Node-specific validation |
| `validate_accessor_path_symbols` | `validation.rs` | Symbol references valid |
| `validate_reachability` | `validation/graph.rs` | All nodes reachable from entry |
| `validate_forward_edges` | `validation/graph.rs` | No backward edges except loops |

---

## Gate 2 — Checksum Validation

**File**: `crates/vb_storage/src/admission.rs` lines 177-184

```rust
let mut parts_for_hash = parts.clone();
parts_for_hash.digest = WorkflowDigest::from_bytes([0u8; 32]);
let hash_bytes = postcard::to_allocvec(&parts_for_hash)?;
let computed = blake3::hash(&hash_bytes);
if computed.as_bytes() != &workflow.digest().as_bytes() {
    return Err(ArtifactChecksumMismatch);
}
```

Algorithm: BLAKE3 of postcard-serialized WorkflowParts with digest field zeroed.

---

## ActionContract Flag Derivation (Future)

**File**: `crates/vb_core/src/action.rs` lines 80-102

| ActionContract Field | Derives ProofFlag | Logic |
|---------------------|-------------------|-------|
| `idempotency: Idempotency` | `taint_safe`, `replayable` | `DeterministicPure` → safe; `AtLeastOnceExternal` → not replayable |
| `retry_safety: RetrySafety` | `retry_safe` | `Safe` → true; `Unsafe` → false; `KeyRequired` → depends on key presence |
| `side_effect: SideEffect` | `replayable` | `None` → replayable; others → depends on idempotency |

---

## Policy Behavior Summary

| Policy | gate_count | durable | Gates Skipped? |
|--------|-----------|---------|----------------|
| Relaxed | 0 | false | yes (both) |
| Journaled | 2 | false | no |
| Strict | 2 | true | no (plus SyncAll) |

---

## Review Findings

1. **Flag defaults are sound**: All proof flags default to `true` in `VerificationProof::new`, which is conservative for a proof that has not yet failed any gate.

2. **Gate count is policy-discrete**: Only 0 (Relaxed) or 2 (Journaled/Strict) are valid. The constant `ADMISSION_GATE_COUNT = 2` is the maximum.

3. **Warning gate range matches**: `VerificationWarning::MAX_GATE = 2` matches `ADMISSION_GATE_COUNT`, so warnings can reference any valid gate.

4. **idempotency_keyed/attested are unimplemented**: These fields are always empty in current code; the action-contract-based derivation is not yet wired. This is noted as a future derivation concern in contract.md.

5. **bounded flag**: Currently always `true` because `validate_budget` must pass for Gate 1 to pass. The structure gate implicitly validates boundedness; the explicit budget check confirms resource limits.
