# Contract Specification - vb-m2tbp

## Context
- **Feature**: Strengthen the existing generated NaN error-category oracle for `FiniteF64`
- **Domain terms**: `FiniteF64`, NaN bit pattern, `CoreError::NonFiniteNumber`
- **Scope**: Change only the assertion in the existing `cfg(test)` property `finite_f64_rejects_nan` in `crates/vb_core/src/value.rs`.
- **Assumptions**: The property keeps its existing `nan_bits in 0u64..` generator, conditional NaN check, and case count. This proves only sampled cases.
- **Discovered from**: `vb-eirmk.75.8.25` value-family reconciliation; historical tuple `1b84ccb770125446d3eb646f3688d80aaecacacb`.

## Preconditions
- PRE-001: Generated bits are interpreted by the existing `f64::from_bits` call.
- PRE-002: The exact error assertion runs only when the resulting `f64` is NaN.
- PRE-003: `FiniteF64::new` remains unchanged; current source returns `CoreError::NonFiniteNumber` for all non-finite values.

## Postconditions
- POST-001: For each sampled NaN value, the existing property requires `FiniteF64::new(value) == Err(CoreError::NonFiniteNumber)`.
- POST-002: The generator, property name, strategy, and case count remain unchanged.
- POST-003: Existing unit tests, signed-zero behavior, finite-value acceptance, serialization, and taint behavior remain unchanged.

## Invariants
- INV-001: No production source changes.
- INV-002: No public API, serialized format, dependency, gate, snapshot, or fixture changes.
- INV-003: This is a sampled test-oracle strengthening, not exhaustive NaN-space proof, formal verification, or a production defect report.

## Error Taxonomy
- NaN construction must match `CoreError::NonFiniteNumber`, as specified by MASTER §14.

## Contract Signature
- Existing property only: `finite_f64_rejects_nan(nan_bits in 0u64..)`.

## Non-goals
- New test cases or a separate property.
- Hashing/IndexSet support for `FiniteF64` or `SlotValue`.
- Any production implementation change.
