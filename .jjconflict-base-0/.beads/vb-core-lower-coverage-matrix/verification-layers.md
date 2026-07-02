# Verification Layers — vb-core-lower-coverage-matrix

## Boundary
- **Verus-owned kernel**: Pure lowering functions in `vb_compile/src/lower/mod.rs`
- **TLA+ temporal model**: NOT APPLICABLE - static parity checking
- **Theorem projection**: NOT APPLICABLE
- **Runtime shell**: NOT APPLICABLE
- **External systems excluded from formal proof**: NOT APPLICABLE

## Layer Assignment

### INV-001 (Node ID Density)
- **Layer**: unit-test + proptest
- **Evidence**: `v1_primitive_lowering.rs::assert_dense_node_ids` + `proptest_equal_primitive_sources_compile_to_equal_digest_and_ir`
- **Command**: `cargo test -p vb_compile v1_primitive_lowering`

### INV-002 (Slot Reference Bounds)
- **Layer**: unit-test + proptest + kani
- **Evidence**: `v1_primitive_lowering.rs::assert_set_const_node` + `v1_primitive_lowering.rs::proptest_scoped_primitives_never_return_unsupported_step_primitive`
- **Command**: `cargo test -p vb_compile v1_primitive_lowering`

### INV-003 (Target Range)
- **Layer**: unit-test + proptest + kani
- **Evidence**: `v1_primitive_lowering.rs::assert_all_targets_in_range` + `v1_primitive_lowering.rs::proptest_scoped_primitives_never_return_unsupported_step_primitive`
- **Command**: `cargo test -p vb_compile v1_primitive_lowering`

### INV-004 (Primitive Shape Determinism)
- **Layer**: proptest
- **Evidence**: `v1_primitive_lowering.rs::proptest_equal_primitive_sources_compile_to_equal_digest_and_ir`
- **Command**: `cargo test -p vb_compile v1_primitive_lowering`

### POST-001 (Parity Matrix)
- **Layer**: unit-test (exhaustive)
- **Evidence**: `v1_primitive_lowering.rs::compile_workflow_emits_supported_ir_when_each_scoped_primitive_is_valid`
- **Command**: `cargo test -p vb_compile v1_primitive_lowering`

### POST-002 (Unsupported Primitive Rejection)
- **Layer**: unit-test
- **Evidence**: `v1_primitive_lowering.rs::compile_workflow_returns_unsupported_step_primitive_only_for_out_of_scope_primitives`
- **Command**: `cargo test -p vb_compile v1_primitive_lowering`

### POST-003 (Error Variant Taxonomy)
- **Layer**: unit-test
- **Evidence**: `v1_primitive_lowering.rs::compile_source_returns_exact_error_variants_for_contract_taxonomy`
- **Command**: `cargo test -p vb_compile v1_primitive_lowering`

### VERUS Proofs (INV-001 through INV-003 Bounds)
- **Layer**: verus
- **Evidence**: `verification/verus/v1_primitive_lowering.rs`
- **Command**: `verus verification/verus/v1_primitive_lowering.rs`
- **Functions**: `proof_construct_plan_valid`, `proof_lowering_plan_targets_in_range`, `proof_lowering_plan_slot_count_covers_references`, `proof_lowering_plan_checks_bounds_before_casts`

## Known Verification Gaps

| Construct | vb_yaml | vb_validate | vb_compile | Gap |
|-----------|---------|-------------|------------|-----|
| `vars` | parses | UNKNOWN | UNKNOWN | Missing test coverage |
| `secrets` | parses | UNKNOWN | UNKNOWN | Missing test coverage |
| `examples` | parses | UNKNOWN | UNKNOWN | Unknown if validated or ignored |
| `with` | parses | UNKNOWN | UNKNOWN | Missing test coverage |
| `then` | parses | UNKNOWN | UNKNOWN | Missing test coverage |
| `condition` | parses | UNKNOWN | UNKNOWN | Missing test coverage |

## Waivers
- No TLA+ model: Static parity checking, not temporal behavior
- No Kani harness write: Existing unit tests + proptest provide sufficient bounded coverage
- No theorem kernel: Verus proofs cover pure lowering invariants
