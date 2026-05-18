# Domain Model Review

## Verdict
STATUS: CONTRACT MODEL READY FOR INDEPENDENT REVIEW

This is not an approval artifact. A separate `contract-verification-review` agent must approve or reject these contracts before test planning or implementation consumes them.

## Domain boundaries
- Storage-owned: run headers, journal append/replay, snapshots, digest checks, recovery summaries, frame seeds, typed storage recovery errors.
- Runtime-owned: converting storage hydration into runnable or rejected `RuntimeRecoveryBoundary`, action ticket semantics, wait/ask primitive continuity.
- Collect-owned: pagination side-table rehydration from `SlotWrittenEvent.extra` and typed collect extra failures.
- Core-owned: `RunFrame`, `StepState`, slot value, taint, pc, executed counts, and frame bounds.

## Illegal states to make unrepresentable or fail-closed
- Runnable recovered frame with no durable events for a non-empty run.
- Secret slot recovered as clean because tail slot taint evidence was absent.
- Snapshot-plus-tail replay accepting a tail event at or before the snapshot watermark.
- Latest attempt recovery mixing stale terminal or slot facts from prior attempts.
- Pending non-idempotent action converted into a runnable frame.
- Collect cursor/page reconstructed from corrupt, missing, or wrong-identity `extra`.

## Type model recommendations
- Prefer validated input wrappers for `RecoveredEventStream`, `SnapshotWatermark`, `ValidatedRunHeader`, `DigestBundle`, and `CollectExtraEnvelope`.
- Separate `RecoverySummary` from `RunnableFrameSeed`; unsupported pending action state should inhabit a rejected/fail-closed branch, not a partial success branch.
- Keep lifecycle diagnostic events separate from ordered recovery events until they carry sequence authority.

## Reviewer focus
- Verify that every acceptance dimension maps to at least one clause and proof obligation.
- Challenge whether the absence of explicit durable taint on `SlotWrittenEvent` is safe.
- Challenge whether crate-level integration evidence is sufficient without a public restart CLI.
