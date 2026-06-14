# ADR 021 (v1): Deferred Scope Fence

## Status

Accepted as guardrail.

## Decision

# allow-removed-crate: ADR enumerates the deferred-scope fence for the removed release-crate set
Generated Rust execution, `vb_codegen`, maxperf, PGO, target-cpu native release workflows, native Makepad UI, UI implementation crates, and visual editor work are deferred for the current backend milestone.

## Invariants

- IR interpreter evidence is required for current runtime acceptance.
- Generated Rust compile success is not current runtime evidence.
- UI screenshots, tokens, accessibility, overlap, and animation gates are not backend completion blockers.
- Makepad dependencies do not enter runtime core crates.
- Reactivation requires a dedicated future architecture bead and ADR update.

## Master Anchors

- Section 22: Removed Rust Codegen and Maxperf
# allow-removed-crate: ADR enumerates the deferred-scope fence for the removed release-crate set
- Section 32: Removed Function Surface: `vb_codegen`
- Section 41: Removed PGO and Maxperf Build
- Sections 76 through 83: UI extension material
