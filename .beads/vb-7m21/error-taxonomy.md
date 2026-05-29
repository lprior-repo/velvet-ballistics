# Error Taxonomy — vb-7m21

## Corpus Construction Errors

- `EmptyCorpus`: no fixtures were supplied.
- `DuplicateFixtureId`: two fixtures share the same identifier.
- `MissingExpectedOutcome`: fixture has no exact typed expected outcome.
- `InvalidFixtureMutation`: mutation does not apply to the selected base record.
- `UnseededRandomFixture`: fixture depends on non-deterministic bytes.
- `NoCopyFenceViolation`: fixture embeds external Restate bytes/layout/API semantics.
- `CoverageFamilyMissing`: required bead family has no fixture.

## Codec / Envelope Expected Errors

- `UnsupportedSchemaVersion`: schema version is greater than current supported version.
- `MigrationRequired`: older schema path, if represented, must not silently decode as current.
- `UnknownRecordKind`: record kind is not recognized.
- `RecordKindFamilyMismatch`: known kind is not valid for the magic/family.
- `HeaderLengthMismatch`: header declares a length other than 60.
- `PayloadTooLarge`: declared payload length exceeds family-specific max and fails before payload allocation.
- `HeaderChecksumMismatch`: header CRC over bytes `0..56` is invalid.
- `PayloadDigestMismatch`: payload digest does not match header digest.
- `UnexpectedEof`: header or payload bytes end before declared size.
- `PostcardDecodeFailed`: bounded payload bytes are not valid for the target payload type.

## Persistence / Invariant Expected Errors

- `SequenceGap`: replay observes non-contiguous per-run sequence numbers.
- `DuplicateEvent`: duplicate divergent persisted event key or duplicate event conflict.
- `IndexParityMismatch` / `CorpusIndexParityMismatch`: event exists but required side-index marker is absent.
- `CorruptSnapshot`: snapshot bytes or snapshot metadata fail recovery invariants.
- `StaleSnapshot`: snapshot sequence/state is older than or inconsistent with required replay tail semantics. May map to an existing typed recovery/storage error if no public stale variant exists.
- `MissingManifest` / `CorpusMissingManifest`: required declared keyspace/manifest entry is absent.

## Railway Result Model

```text
BuildFixture -> Result<StorageFixture, CorpusConstructionError>
BuildCorpus -> Result<CoverageCheckedCorpus, CorpusConstructionError>
RunFixture -> Result<ActualTypedOutcome, FixtureRunnerError>
AssertOutcome -> Result<FixtureEvidence, OutcomeMismatch>
```

## Error Mapping Rules

- Error assertions must pattern-match typed variants, not display strings.
- One fixture must assert one primary typed error.
- If a public `JournalError`/`RecoveryError` variant does not exist, the corpus runner may define a local typed error only for cold-path test classification; that local type must not hide a public API gap.
- Adding a public storage error variant requires diagnostic-code coverage.
