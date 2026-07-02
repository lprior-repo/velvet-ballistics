# TLA Spec - vb-qi37.5.3

TLA+ lane not required for this bead. The state transition is a bounded local admission predicate:

- Inputs: `gate_count`, proof flags, `idempotency_verified`, `idempotency_keyed`, `idempotency_attested`.
- Accept iff all required flags are true and `idempotency_keyed \subseteq idempotency_attested`.
- Reject otherwise before run admission is returned.

Covered by Rust unit tests and the existing all-45 Kani parity harness for the idempotency decision table.
