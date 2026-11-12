# Lean Contract: vb-9ret

## Lean-Owned Clauses
None.

## Rationale
This bead verifies adapter preservation during deduplication and compile workflow integrity. These are behavioral/integration concerns that do not map to pure deterministic kernels suitable for Lean projection. The pure Rust implementation is validated via unit tests, integration tests, and proptest.

## Layer Assignment
- vb_compile adapter behavior: Rust unit + integration
- vb_validate adapter behavior: Rust unit + integration
- Workflow compilation: Rust integration tests

## Waived Clauses
- moon ci: WAIVE-INCLUDE-STR-PATH-ORIGIN-MAIN (see verification-layers.md)
