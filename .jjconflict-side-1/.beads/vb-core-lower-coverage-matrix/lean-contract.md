# Theorem Kernel Projection — vb-core-lower-coverage-matrix

## Boundary
- **TLA+-owned temporal model**: NOT APPLICABLE - static parity checking
- **Verus-owned Rust core**: `verification/verus/v1_primitive_lowering.rs` - pure lowering invariants
- **Theorem-owned kernel**: NOT APPLICABLE - Verus suffices for this scope
- **Rust/runtime shell**: NOT APPLICABLE
- **External systems excluded from theorem proof**: NOT APPLICABLE

## Theorem-Owned Clauses
None. Verus proofs in `verification/verus/v1_primitive_lowering.rs` cover all pure lowering invariants:
- `proof_construct_plan_valid` - constructor inputs valid implies plan valid
- `proof_lowering_plan_preserves_dense_node_ids` - node count bounded and positive
- `proof_lowering_plan_targets_in_range` - all targets in valid range
- `proof_lowering_plan_slot_count_covers_references` - slot allocator closed
- `proof_lowering_plan_checks_bounds_before_casts` - primitive bounds checked
- `proof_lowering_plan_deterministic_for_equal_source` - deterministic lowering
- `proof_lowering_plan_preserves_primitive_shapes` - primitive shape invariants

## Theorem Obligations
None - Verus owns the pure Rust core proof obligations.

## Waivers
- No Lean/Aeneas/Hax: Verus can express all required pure lowering invariants
- No theorem kernel extraction: Pure lowering functions are verifiable in Verus directly
