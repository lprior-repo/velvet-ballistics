# Proof Strategy - vb-qi37.5.3

## Required Lanes

- Unit tests for runtime admission predicates and storage metadata derivation.
- Existing Kani all-45 parity harness for compile/validate idempotency decision-table consistency.
- Source clippy for touched source crates.

## Deferred Lanes

- TLA+/Lean/Verus not required: no concurrent temporal protocol, arithmetic model, or ghost/spec binding changed.
- Fuzz admission target remains blocked by known local musl/sanitizer tooling from related idempotency evidence.

## Acceptance

- All planned unit tests pass.
- `rtk cargo kani -p vb_compile --harness idempotency_gate_parity` reports `VERIFICATION:- SUCCESSFUL`.
- Source clippy passes for `vb_runtime` and `vb_storage` libraries.
