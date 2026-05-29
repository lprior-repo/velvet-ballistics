# Proof Writer Report — vb-e5l9n

Scope: Kani-side repair only for vb_core diagnostic-code harness compilation.

## Obligations touched

- PO-001: `SymbolicCode::from_static` validation helper compilation.
- PO-004: `is_supported_code` helper compilation.
- PO-005 / PO-014: diagnostic constructor invariant harness compilation.
- PO-008: diagnostic-code `from_str` compatibility harness compilation.
- PO-009: symbolic-code serde round-trip harness compilation.
- PO-012: reverse lookup harness compilation.
- PO-013: symbolic-code determinism harness compilation.

## Artifacts changed

- `crates/vb_core/src/kani/kani_symbolic_code_validation.rs`
- `crates/vb_core/src/kani/kani_determinism.rs`
- `crates/vb_core/src/kani/kani_diagnostic_constructor.rs`
- `crates/vb_core/src/kani/kani_from_str_compat.rs`
- `crates/vb_core/src/kani/kani_is_supported_code.rs`
- `crates/vb_core/src/kani/kani_registry_bijection.rs`
- `crates/vb_core/src/kani/kani_reverse_lookup.rs`
- `crates/vb_core/src/kani/kani_serde_roundtrip.rs`

## Repair summary

- Centralized Kani diagnostic model helpers in `kani_symbolic_code_validation.rs`:
  - `DiagnosticCode::symbolic_code`
  - `numeric_to_symbolic`
  - `is_supported_code`
  - registry-sourced `SymbolicCode` constructors
- Removed duplicate `DiagnosticCode::symbolic_code` implementations from sibling Kani modules.
- Replaced const `Option::map` / `and_then` usage with explicit `match` chains.
- Fixed serde mirror deserialization to recover the registry-owned `'static` symbolic string before constructing `SymbolicCode`.
- Removed unused imports that were fatal under `RUSTFLAGS=-Dwarnings`.

## Result

The parent-reported diagnostic Kani compilation blockers are repaired. The exact failing command and the canonical Moon Kani task both completed successfully after repair. This is compilation/task evidence only; no claim is made that every individual diagnostic harness has deep CBMC proof evidence in this subagent run.
