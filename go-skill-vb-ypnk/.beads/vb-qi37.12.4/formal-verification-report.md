# Formal Verification Report: vb-qi37.12.4

STATUS: APPROVED

## Executed Obligations

- `GATE-IGNORED-FALLIBLE-RESULTS`: PASS via `scripts/check-ignored-fallible-results.sh` exit 0.
- `GATE-MOON-001`: PASS via `moon run :verify-standard` exit 0.
- `STATIC-LINT-001`: PASS inside `moon run :verify-standard`.
- `UNIT-EXPR-BYTESTACK-001`, `UNIT-SLOT-COMPILER-001`, `UNIT-LOWER-DO-001`, `POST-009-VALIDATE-001`: PASS inside `moon run :verify-standard`.
- `KANI-EXPR-BYTECODE-001`, `KANI-SLOT-REF-001/001b`, `KANI-CONSTANT-POOL-001/001b/001c`, `KANI-ACCESSOR-REF-001/001b/001c`: PASS inside `moon run :verify-standard`.

## Waivers

- TLA+, Verus, and Lean waivers remain as approved by contract-verification review because this bead is a shell/static-gate repair, not a Rust-local theorem or temporal state-machine change.

## Decision

State 11 is approved for the bead scope.
