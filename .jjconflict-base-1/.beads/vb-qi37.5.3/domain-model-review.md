# Domain Model Review - vb-qi37.5.3

## Model

- `VerificationProof` is the accepted-artifact proof envelope.
- `idempotency_verified` is the gate status bit proving the idempotency verifier ran and passed.
- `idempotency_keyed` is the set of actions requiring key-based retry/replay safety metadata.
- `idempotency_attested` is the set of actions whose idempotency evidence is accepted.
- `RunAdmission` is the runtime admission token consumed by dispatch/runtime components.

## Illegal States

- Artifact accepted under Strict/Journaled with `idempotency_verified == false`.
- Artifact accepted under Strict/Journaled with keyed actions absent from attested evidence.
- Run admitted successfully while losing the attested idempotency action set.

## Decision

- Model is sufficient for this bead. No extra enum is necessary because the persisted proof envelope already separates status, keyed requirements, and attested action IDs.
