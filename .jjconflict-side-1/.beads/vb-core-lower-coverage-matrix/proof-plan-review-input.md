# Proof Plan Review Input — vb-core-lower-coverage-matrix

## Bead Context
- **Bead ID**: vb-core-lower-coverage-matrix
- **Title**: Prove v1 lowering coverage matrix
- **Planner**: State 4 (Proof Planning)
- **Reviewer**: contract-verification-reviewer, proof-reviewer

## Contract Summary

### Requirements
- Every v1 construct has parser/validator/compiler parity tests
- Unsupported codegen/UI paths are explicitly excluded
- No parser/compiler grammar drift remains

### Key Invariants
- **INV-001**: Node ID density (dense, zero-indexed)
- **INV-002**: Slot reference bounds (all references < slot_count)
- **INV-003**: Target range (all targets within node count)
- **INV-004**: Primitive shape determinism (equal source = equal digest)

### Postconditions
- **POST-001**: Construct classification parity across vb_yaml, vb_validate, vb_compile
- **POST-002**: Supported primitives emit correct IR
- **POST-003**: Unsupported primitives rejected with exact error variants

## Verification Strategy

### Primary Evidence
1. **Unit Tests** (`crates/vb_compile/tests/v1_primitive_lowering.rs`): 1350+ lines of exhaustive tests
2. **Verus Proofs** (`verification/verus/v1_primitive_lowering.rs`): 357 lines of pure lowering invariants

### Execution Commands
```bash
cargo test -p vb_compile v1_primitive_lowering
verus verification/verus/v1_primitive_lowering.rs
```

## Risk Assessment
- **Parser/compiler grammar drift**: Mitigated by exhaustive unit tests
- **Primitive lowering bounds**: Mitigated by Verus proofs + unit tests
- **Unknown coverage gaps**: Documented as waivers with follow-up beads

## Review Questions for Reviewer

1. Is the verification strategy sufficient for the stated risk?
2. Are the gap waivers appropriate (follow-up beads vs. blocking)?
3. Should any gap be elevated to blocking?
4. Is the proof strategy aligned with existing artifacts?
