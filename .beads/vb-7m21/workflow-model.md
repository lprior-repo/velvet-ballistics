# Workflow Model — vb-7m21

## Corpus Lifecycle Typestates

```text
DraftCorpus
  -> CorpusWithFixtures
  -> CoverageCheckedCorpus
  -> ExecutedCorpus
  -> ReportedCorpus
```

## Legal Transitions

1. `DraftCorpus -> CorpusWithFixtures`
   - Guard: at least one fixture exists; every fixture has `FixtureId`, `FixtureFamily`, deterministic construction, and expected typed outcome.
   - Failure: `EmptyCorpus`, `DuplicateFixtureId`, `MissingExpectedOutcome`, `UnseededRandomFixture`.

2. `CorpusWithFixtures -> CoverageCheckedCorpus`
   - Guard: all bead-required families are represented: known-good journal, known-good snapshot, unknown version, missing index, oversized record, truncated header, corrupt envelope/payload, journal gap, duplicate event/idempotency substitute, stale snapshot, missing manifest.
   - Failure: `CoverageFamilyMissing`.

3. `CoverageCheckedCorpus -> ExecutedCorpus`
   - Guard: fixture runner uses only public VB codec/storage APIs, family-specific bounds, and temporary isolated storage roots.
   - Failure: `BoundaryViolation`, `ProductionDataMutationAttempt`, `AllocationBeforeBoundCheck`.

4. `ExecutedCorpus -> ReportedCorpus`
   - Guard: each fixture outcome equals its exact typed expected outcome.
   - Failure: `OutcomeMismatch`, `UnexpectedSuccess`, `UnexpectedErrorVariant`.

## Fixture Execution Outcomes

- `AcceptedValidJournal`: known-good minimal journal event decodes/replays as expected.
- `AcceptedValidSnapshot`: known-good snapshot envelope decodes/loads as expected.
- `RejectedHeader`: header-level corruption maps to the declared header/storage error.
- `RejectedPayload`: payload-level corruption maps to the declared payload/storage error.
- `RejectedInvariant`: cross-keyspace or temporal persistence corruption maps to the declared invariant error.

## Terminal States

- `ReportedCorpus`: every fixture asserted exact typed outcome and coverage is complete.
- `RejectedCorpus`: corpus construction failed before execution.
- `FailedCorpusRun`: at least one fixture produced the wrong typed outcome.

## Temporal Rules

- Header length and declared payload length are validated before payload allocation or Postcard decode.
- Journal replay observes monotonic per-run sequence; gaps fail rather than being silently skipped.
- Snapshot replay must not allow stale snapshot state to hide newer conflicting journal tail state.
- Duplicate event keys must be idempotent only for byte-identical queued events; divergent duplicates fail with a typed error.
- Missing side indexes must not be silently repaired by the fixture runner before assertion.
