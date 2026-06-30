# Hazard Analysis: vb-vzcuf

## H1 Arithmetic Wraparound

- Hazard: `staged + encoded_len` wraps and admits an oversized batch.
- Contract control: checked addition and checked conversions; overflow returns typed accumulated accounting error.
- Proof seed: Verus/Kani/Flux/proptest around pure admission helper.

## H2 Error Conflation

- Hazard: accumulated byte pressure is reported as `QueueFull` or `PayloadTooLarge`, hiding the failing policy.
- Contract control: distinct `JournalError` variant with attempted/limit fields.
- Proof seed: C6 error separation with controlled unrelated guards.

## H3 Partial Mutation on Rejection

- Hazard: candidate bytes/key inserted before accumulated-byte check, later persisted despite returning error.
- Contract control: permanent mutation only after all guards pass; no byte total increment on rejection.
- Proof seed: Kani/proptest/integration no-mutation model.

## H4 Abort Semantics Drift

- Hazard: accumulated budget rejection sets `aborted`, discarding earlier valid staged events.
- Contract control: accumulated rejection is non-aborting unless an explicit product decision changes it.
- Proof seed: workflow/state property for rejection followed by valid commit.

## H5 Same-Batch Duplicate Ambiguity

- Hazard: `OwnedWriteBatch` key replacement collapses final bytes while accumulator counts attempts, or vice versa.
- Contract control: choose attempt accounting or distinct-key accounting before implementation; document in accessor/tests.
- Proof seed: duplicate same-batch accounting property after domain decision.

## H6 Limit Source Drift

- Hazard: core resource contract has one limit while storage uses another implicit limit, producing inconsistent admission.
- Contract control: typed core-to-storage bridge or explicit default mapping.
- Proof seed: boundary conversion/refinement seed.

## H7 Payload vs Envelope Confusion

- Hazard: accumulated accounting uses postcard payload length instead of full encoded record value length.
- Contract control: construct `EncodedJournalEventBytes` from `encode_record` result length.
- Proof seed: property comparing accepted total to actual staged encoded value lengths.

## H8 Count/Byte Guard Precedence Instability

- Hazard: tests/proofs observe different errors depending on guard order.
- Contract control: specified guard order in `type-contracts.md` and `workflow-model.md`.
- Proof seed: C6 precedence cases.

## H9 Performance Regression

- Hazard: implementation serializes each event twice to compute bytes and then insert.
- Contract control: compute bytes from the single encoded `Vec<u8>` used for insertion. Any speed claim needs benchmarks.
- Proof seed: release/performance hint only; not proof closure.

## H10 Public API Migration

- Hazard: adding constructor parameters breaks callers or tests; preserving old constructor leaves unbounded state.
- Contract control: keep old constructor with safe default and add explicit limited constructor/factory.
- Proof seed: API compatibility traceability.
