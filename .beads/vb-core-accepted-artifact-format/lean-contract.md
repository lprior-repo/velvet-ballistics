# Theorem Kernel Projection — AcceptedArtifact Format

## Boundary

- **TLA+-owned temporal model**: Artifact admission lifecycle (submitted → stored → admitted/rejected); gate count validation state machine; Strict vs Relaxed policy transitions — see `tla-spec.md`
- **Verus-owned Rust core**: Pure `AcceptedArtifact` construction invariants; `CompiledWorkflow::try_from_parts` gate; digest-IR checksum; proof flag derivation
- **Theorem-owned kernel**: None — no tiny algebraic kernel requires Lean/Aeneas/Hax extraction
- **Rust/runtime shell**: Postcard encode/decode; Fjall persistence; Arc<dyn AcceptedArtifactStore> sharing; SyncAll durability call
- **External systems excluded**: Journal sequence counter; Fjall transaction engine; SHA-256 implementation

## Theorem-Owned Clauses

**None** — no theorem kernel projection required for this bead.

Rationale: The `AcceptedArtifact` format is a data envelope with no complex algebraic state transitions, protocol lattices, or arithmetic bounds that exceed Verus expressiveness. The critical properties are:

1. **Digest-IR invariant**: `digest == sha256(ir)` — Verus can express via spec function
2. **Gate count bounds**: `gate_count >= 1` — simple integer bound, Verus trivially proves
3. **CompiledWorkflow sole constructor**: `try_from_parts` is the only impl — structural, no algebra
4. **Proof flag derivation**: Currently hardcoded; when real gates are added, Verus can verify each gate produces its flag

## Verus Obligations

### VERUS-INV-001: AcceptedArtifact Digest-IR Invariant

**Contract clause**: INV-001

**Target**: `vb_storage::admission::AcceptedArtifact`

**Spec function**:
```verus
spec fn accepted_artifact_digest_matches_ir(artifact: &AcceptedArtifact) -> bool {
    artifact.digest == sha256(&artifact.ir)
}
```

**Proof obligation**: `AcceptedArtifact::new` constructor preserves the invariant when called from `submit_artifact`

**Evidence**: `moon run :verify-proof` targeting `vb_storage::admission`

### VERUS-INV-003: VerificationProof Flags Are Derived

**Contract clause**: INV-003

**Target**: `vb_storage::admission::VerificationProof::new`

**Current issue**: All flags hardcoded to `true` — violates "derived from actual gate outputs" invariant

**Proof obligation**: When 15-gate implementation lands, each flag must be proven from its respective gate output

**Evidence**: `moon run :verify-proof` targeting `vb_storage::admission::VerificationProof`

### VERUS-PRE-001: CompiledWorkflow Provenance Gate

**Contract clause**: PRE-001

**Target**: `vb_core::CompiledWorkflow::try_from_parts`

**Proof obligation**: Only `try_from_parts` can construct a `CompiledWorkflow`; no other constructor or unsafe bypass exists

**Evidence**: `moon run :verify-proof` targeting `vb_core::compiled_workflow`

## Waivers

- **Lean/Aeneas/Hax theorem kernel**: Not required — all critical properties are expressible in Verus or are structural/programmatic
- **Theorem projection for ResourceBudget arithmetic**: Deferred to `vb-core-proof-15-gate` bead when actual gate implementation lands

## Shell Exclusions (All Verus Proofs)

- Postcard encode/decode: external codec, not verified in Verus
- Fjall persistence: external store, not verified in Verus
- SHA-256: trusted primitive, not verified in Verus
- Arc<dyn AcceptedArtifactStore>: dynamic dispatch, not verified in Verus
- SyncAll durability: OS/system call, not verified in Verus
