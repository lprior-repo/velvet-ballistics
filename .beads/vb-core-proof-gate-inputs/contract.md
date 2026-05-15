# Contract Specification — VerificationProof Gate Inputs

## Context

- **Feature**: Derive all VerificationProof gate inputs for artifact admission
- **Bead**: vb-core-proof-gate-inputs
- **Domain terms**:
  - `VerificationProof`: proof struct produced when artifact passes admission gates
  - `gate_count`: number of gates passed (0 for Relaxed, 2 for Journaled/Strict)
  - `ADMISSION_GATE_COUNT`: constant = 2
  - `VerificationWarning`: soft failure (does not block admission)
  - `ProofFlag`: bounded, taint_safe, retry_safe, replayable
- **Assumptions**:
  - Gate 1 is structure validation via `CompiledWorkflow::try_from_parts`
  - Gate 2 is checksum validation via blake3 hash of postcard-serialized parts
  - Proof flags default to `true` in `VerificationProof::new`
- **Open questions**:
  - Whether idempotency_keyed/attested lists are derived inside submit_artifact or elsewhere
  - Whether bounded flag should be conditionally set based on budget validation result
  - Whether taint_safe/retry_safe/replayable derivation chain from ActionContract is implemented

---

## Preconditions

- PRE-001: `submit_artifact_with_contracts` preconditions:
  - `journal` is a valid `&FjallJournal` reference
  - `workflow` is a valid `&vb_core::CompiledWorkflow` reference
  - `policy` is one of `Relaxed`, `Journaled`, `Strict`
  - `action_contracts` is a valid slice of `ActionContract` (may be empty)

---

## Postconditions

- POST-001: `VerificationProof::new(digest, gate_count, durable)` produces a proof where:
  - `digest == digest`
  - `gate_count == gate_count`
  - `durable == durable`
  - `bounded == true`
  - `taint_safe == true`
  - `retry_safe == true`
  - `replayable == true`
  - `idempotency_keyed == Box::new([])`
  - `idempotency_attested == Box::new([])`
  - `warnings == Vec::new()`

- POST-002: `submit_artifact(journal, workflow, Relaxed)` postconditions:
  - Returns `AcceptedArtifact` with `verification.gate_count == 0`
  - Returns `AcceptedArtifact` with `verification.durable == false`
  - Both gates are skipped (no structure/checksum validation)

- POST-003: `submit_artifact(journal, workflow, Journaled)` postconditions:
  - Returns `AcceptedArtifact` with `verification.gate_count == 2`
  - Returns `AcceptedArtifact` with `verification.durable == false`
  - Gate 1 (structure) and Gate 2 (checksum) both pass

- POST-004: `submit_artifact(journal, workflow, Strict)` postconditions:
  - Returns `AcceptedArtifact` with `verification.gate_count == 2`
  - Returns `AcceptedArtifact` with `verification.durable == true`
  - Gate 1 and Gate 2 both pass; SyncAll is called after persistence

---

## Invariants

- INV-001: `VerificationProof` is well-formed when returned from admission:
  - `digest` is a valid 32-byte `WorkflowDigest`
  - `gate_count ∈ {0, 2}` (Relaxed=0, Journaled/Strict=2)
  - `bounded == true` (default, structure gate sufficient)
  - `taint_safe == true` (default, taint propagation not yet derived)
  - `retry_safe == true` (default, retry safety not yet derived)
  - `replayable == true` (default, replay not yet derived)
  - `idempotency_keyed.len() >= 0` (empty in current impl)
  - `idempotency_attested.len() >= 0` (empty in current impl)
  - `warnings.len() >= 0`

- INV-002: `VerificationWarning::is_valid()` holds for all warnings:
  - `gate >= VerificationWarning::MIN_GATE (1)`
  - `gate <= VerificationWarning::MAX_GATE (2)`

---

## Error Taxonomy

- `JournalError::ArtifactMalformed` — structure validation failed (Gate 1) or postcard serialization failed
- `JournalError::ArtifactChecksumMismatch` — checksum validation failed (Gate 2)
- `JournalError::InvalidGateCount` — gate_count is not 0 or 2 (proof validation failure)
- `JournalError::MissingRequiredProofFlag` — a required proof flag is false

---

## Gate Derivation

### Gate 1 — Structure Validation
- **Source**: `CompiledWorkflow::try_from_parts(parts.clone())` at admission.rs:174
- **validate_parts** calls:
  - `validate_resource_contract(parts)` — resource contract limits
  - `validate_entry(entry, nodes.len())` — entry step in bounds
  - `validate_expressions(expressions, accessors.len())` — expression bytecode valid
  - `validate_accessors(accessors, slot_count)` — accessor programs valid
  - `validate_node_id(node, index)` — node id matches array position
  - `nodes::validate_node(node, parts)` — node-specific validation
  - `validate_accessor_path_symbols(accessors)` — symbol references valid
  - `graph::validate_reachability(parts)` — all nodes reachable from entry
  - `graph::validate_forward_edges(parts)` — no backward edges except loops
- **validate_budget** calls:
  - `WholeWorkflowBudget::compute(nodes, entry, resource_contract)?`
  - `BoundednessPolicy::DEFAULT.validate(&budget)` → sets `bounded = true` on success

### Gate 2 — Checksum Validation
- **Source**: admission.rs:177-184
- Algorithm: BLAKE3 hash of postcard-serialized `WorkflowParts` with digest zeroed
- Compares computed hash against `workflow.digest()`
- On mismatch → `JournalError::ArtifactChecksumMismatch`

---

## Proof Flag Derivation

| Flag | Source | Current Default |
|------|--------|----------------|
| `bounded` | `validate_budget` passes → `BoundednessPolicy::DEFAULT.validate(&budget)` | `true` |
| `taint_safe` | Derived from `ActionContract.idempotency` taint propagation rules | `true` |
| `retry_safe` | Derived from `ActionContract.retry_safety` | `true` |
| `replayable` | Derived from `ActionContract.idempotency` replay rules | `true` |
| `idempotency_keyed` | Actions with `Idempotency != DeterministicPure` | empty |
| `idempotency_attested` | Actions with explicit idempotency key | empty |

**Note**: Current implementation defaults all flags to `true` in `VerificationProof::new`. The action-contract-based derivation is not yet wired.

---

## Non-goals

- Proof flag derivation from `ActionContract` (future work)
- Idempotency list population from action contract analysis (future work)
- Taint propagation tracking through workflow execution (future work)
