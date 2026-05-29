# Contract — vb-7m21 State 3 Domain/Type Contract

## Bead Intent

Add a deterministic blackhat corruption fixture corpus for `vb_storage` that proves known-good storage records are accepted and corrupt/invariant-breaking records map to exact typed outcomes.

## Normative Requirements

- REQ-1: The corpus includes a known-good minimal journal event fixture that succeeds.
- REQ-2: The corpus includes a known-good snapshot envelope fixture that succeeds.
- REQ-3: An unknown/future schema version fixture returns `UnsupportedSchemaVersion`.
- REQ-4: A missing side-index fixture returns `IndexParityMismatch` or an explicit typed corpus-local equivalent if public storage has no such variant.
- REQ-5: An oversized declared record fixture returns `PayloadTooLarge` before payload allocation.
- REQ-6: A truncated header fixture returns `UnexpectedEof`.
- REQ-7: Corrupt envelope/payload fixtures return exact checksum/digest/decode errors according to intended mutation.
- REQ-8: A journal gap fixture returns `SequenceGap`.
- REQ-9: A duplicate event/idempotency-substitute fixture returns `DuplicateEvent` or documents idempotent identical duplicate success as a separate legal outcome.
- REQ-10: A stale/corrupt snapshot fixture returns a typed recovery/storage error or proves deterministic replay cannot be hidden by stale state.
- REQ-11: A missing manifest fixture has a precise declared-keyspace/manifest invariant and exact typed outcome.
- REQ-12: Each fixture maps to exactly one expected typed outcome.
- REQ-13: The corpus covers every storage error family required by bead scope.
- REQ-14: No fixture uses random bytes without an explicit seed.
- REQ-15: Corruption tests operate only on isolated temporary storage and never mutate production data.
- REQ-16: Fixture generation uses VB public APIs/constants and does not copy Restate code, bytes, APIs, storage layout, or wire format.

## Acceptance Boundary

This contract authorizes proof planning and test planning. It does not authorize implementation, tests, proof artifacts, or proof obligations.

## Remaining Illegal-State Risks

- `IndexParityMismatch` remains representable only as an acceptance phrase until downstream selects public error variant vs corpus-local typed classification.
- `duplicate idempotency key` remains ambiguous for `vb_storage`; storage duplicate event keys are the contracted substitute.
- `missing manifest` remains bound to declared Fjall keyspace/manifest parity, pending implementation inspection of existing manifest tests.
