# Red Phase Report: vb-qi37.7.4

## Files changed

- `Cargo.toml` — added `proptest` to root dev-dependencies for test targets that need property coverage.
- `crates/vb_validate/Cargo.toml` — added `proptest` as a crate dev-dependency.
- `crates/vb_validate/src/gate_08_accessor.rs` — added focused Gate 8 red-phase unit/property/Kani tests for accessor field symbol bounds, root boundaries, sentinel index handling, coordinate reporting, and focused/aggregate parity.
- `crates/vb_validate/tests/gate_08_accessor_parity.rs` — added public black-box integration tests for aggregate Gate 8 and `vb_core::workflow::CompiledWorkflow` parity.
- `.beads/vb-qi37.7.4/red-phase.md` — this report.

## Intended failing test commands

Targeted integration red command:

```bash
rtk cargo test --manifest-path "crates/vb_validate/Cargo.toml" --test gate_08_accessor_parity
```

Targeted nextest red command:

```bash
rtk cargo nextest run --manifest-path "crates/vb_validate/Cargo.toml" --test gate_08_accessor_parity
```

Observed red failures before implementation:

- `aggregate_gate_08_rejects_field_equal_to_symbols_count`
- `aggregate_gate_08_rejects_field_above_symbols_count`
- `aggregate_gate_08_reports_invalid_field_segment_coordinates`
- `validate_gate_08_matches_core_workflow_for_invalid_field_boundaries`

Focused/unit/property red command after existing unrelated lib-test compile errors are repaired:

```bash
rtk cargo test --manifest-path "crates/vb_validate/Cargo.toml" --lib gate_08
```

Canonical intended red command after workspace manifest/test compile blockers are repaired:

```bash
cargo nextest run -p vb_validate gate_08
```

## Why failures are expected before implementation

The current Gate 8 implementations accept every `PathSegment::Field(_)` without checking `symbol.get() < parts.symbols_count`. Therefore exact red assertions expecting `Err(ValidationError::AccessorPathInvalid { accessor_index, segment_index })` for equal-bound, above-bound, zero-symbol, and multi-segment invalid fields must fail until production Gate 8 validates field-symbol bounds.

The public `vb_core::workflow::CompiledWorkflow::try_from_parts` path already rejects invalid field symbols with `WorkflowError::SymbolOutOfBounds`, so the parity test intentionally demonstrates current drift: `vb_validate::gates::validate_gate_08_accessor_path_segments` returns `Ok(())` for an invalid field while `vb_core` rejects the same structural boundary.

## Verification notes

- `rtk cargo test --manifest-path "crates/vb_validate/Cargo.toml" --test gate_08_accessor_parity --no-run` compiled the new integration test binary.
- `rtk cargo test --manifest-path "crates/vb_validate/Cargo.toml" --test gate_08_accessor_parity` produced 4 expected red failures and 7 passing baseline/positive-oracle tests.
- `rtk cargo nextest run --manifest-path "crates/vb_validate/Cargo.toml" --test gate_08_accessor_parity` produced the same expected red state: 7 passed, 4 failed.
- Full `rtk cargo test -p vb_validate --no-run` is currently blocked by unrelated existing compile errors in `capability_contract_schema.rs` and `diag_render.rs` plus an unrelated root manifest bench path issue for `aggregate_resource_budget` when invoked through the workspace root.
