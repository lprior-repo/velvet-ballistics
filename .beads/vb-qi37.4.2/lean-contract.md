# Lean/Aeneas Theorem Kernel Projection: vb-qi37.4.2

## Boundary

- **TLA+-owned temporal model**: None (waived — single atomic step function, no temporal behavior)
- **Verus-owned Rust core**: INV-001 (run never inserted when admission fails) — verifiable via code inspection and integration tests
- **Theorem-owned kernel**: None — no algebraic state transitions, protocol lattices, arithmetic bounds, or parser/codec invariants beyond what integration tests can verify
- **Rust/runtime shell**: The `?` short-circuit in `handle_submit_with_inputs_contracts_and_header_mode` is purely structural Rust control flow
- **External systems**: None — single-shard in-process execution

## Theorem-Owned Clauses

None. The admission gate sequencing is a deterministic Rust control flow property that does not require a theorem prover.

## Verus Scope

No Verus proof is required for this bead because:
1. The sequencing is deterministic linear Rust code, not a proof-obligating pure function
2. The integration test with `NeverPresentArtifactStore` + Strict policy provides behavioral verification
3. The `?` propagation is mechanically checkable by inspection and Miri

If a future bead requires Verus proofs for this area, the proof surface would be:
- Target: `handle_submit_with_inputs_contracts_and_header_mode`
- Property: `ensures result.is_err() ==> runs.is_empty()` (run not inserted on rejection)
- This would require modeling the shard's internal state in Verus, which is out of scope for this bead

## Explicit Waiver

**Clause**: All formal proof obligations  
**Owner**: vb-qi37.4.2  
**Reason**: Deterministic Rust control flow; behavioral verification via integration tests is sufficient  
**Expiry**: N/A — this bead does not require theorem proving  
**Compensating Evidence**: `cargo test admission_rejection_does_not_insert_run_state_strict` + Miri execution

## Non-goals

- Verus proofs for `handle_submit_with_inputs_contracts_and_header_mode`
- Algebraic state machine refinement
- Lean/Aeneas/Hax kernel extraction
