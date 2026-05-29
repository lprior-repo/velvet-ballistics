# Hazard Analysis — vb-7m21

## H1 — Allocation Before Length Validation
- Hazard: oversized declared payload fixture accidentally allocates the claimed payload size.
- Impact: denial-of-service path hidden by tests.
- Contract control: construct oversize by mutating header length field only; assert `PayloadTooLarge` before payload decode.

## H2 — Wrong Error Variant Accepted
- Hazard: tests accept any error or string text instead of exact typed error.
- Impact: regression in public storage taxonomy goes unnoticed.
- Contract control: `ExpectedTypedOutcome` must classify exact variants.

## H3 — Index Parity Ambiguity
- Hazard: missing index acceptance requires `IndexParityMismatch`, but no public variant exists.
- Impact: downstream may fake pass with bool assertions or strings.
- Contract control: require either a new typed public variant with diagnostic mapping or a local typed corpus error explicitly documenting API gap.

## H4 — Restate Copy Violation
- Hazard: unavailable external Restate record format is guessed or copied later.
- Impact: violates master no-copy fence and may import incompatible distributed assumptions.
- Contract control: only VB constants/APIs may generate bytes and keys.

## H5 — Duplicate Idempotency Misclassification
- Hazard: storage duplicate event semantics are confused with runtime action idempotency keys.
- Impact: fixture claims coverage of a concept not present in `vb_storage`.
- Contract control: storage lane may cover duplicate event keys; action idempotency requires separate scope.

## H6 — Stale Snapshot Silently Masks Journal Tail
- Hazard: recovery starts from stale snapshot and misses newer journal events.
- Impact: replay divergence or data loss.
- Contract control: stale snapshot fixture must assert typed rejection or deterministic replay from correct tail.

## H7 — Missing Manifest Ambiguity
- Hazard: manifest fixture has no defined storage invariant.
- Impact: meaningless test that checks test fixture metadata only.
- Contract control: define missing manifest as missing declared Fjall keyspace/manifest parity unless a later artifact narrows it.

## H8 — Mutating Production Data
- Hazard: corruption fixtures operate on a developer's real store path.
- Impact: data loss and non-determinism.
- Contract control: fixture runner must allocate isolated temp stores and never use configured production paths.

## H9 — Header CRC Recalculation Mistakes
- Hazard: mutation intended to test schema/length instead yields header checksum mismatch first.
- Impact: fixture covers wrong error family.
- Contract control: mutation types must specify whether CRC is recomputed to reach the intended validation step.

## H10 — Coverage Drift
- Hazard: future storage error families are added but corpus remains stale.
- Impact: blackhat corpus no longer covers every required family.
- Contract control: corpus coverage check must be explicit and fail on missing required families for this bead.
