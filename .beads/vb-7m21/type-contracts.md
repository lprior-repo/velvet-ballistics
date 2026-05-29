# Type Contracts — vb-7m21

## Core Type Shape

The following are contracts for implementation/test-planning, not production code.

```text
StorageBlackhatCorpus = NonEmptyVec<StorageFixture>
StorageFixture = ValidFixture | InvalidFixture
ValidFixture = KnownGoodJournal | KnownGoodSnapshot
InvalidFixture = HeaderCorruption | PayloadCorruption | PersistenceInvariantCorruption
```

## Value Objects

### FixtureId
- Newtype over a short static string or interned test identifier.
- Constructor rejects empty identifiers and duplicate IDs within a corpus.
- IDs are diagnostic/cold-path only; never part of runtime storage layout.

### FixtureFamily
- Closed enum, no boolean flags.
- Illegal state: `is_valid: bool` plus optional error. Use variants instead.

### BoundedFixtureBytes
- Newtype over bytes with declared maximum tied to fixture family.
- Constructor records declared length separately from actual length where header corruption requires mismatch.
- Oversize fixture may declare `max + 1` in the header but must not allocate `max + 1` payload bytes.

### DeterministicMutation
- Closed enum over exact mutations: `SetSchemaVersion`, `TruncateAt`, `SetPayloadLen`, `FlipHeaderCrcBit`, `FlipPayloadByte`, `RemoveIndexMarker`, `SkipSequence`, `DuplicateEventKey`, `RemoveDeclaredKeyspace`, `StaleSnapshotSeq`.
- If a mutation needs entropy, it carries an explicit seed. Unseeded randomness is invalid.

### ExpectedTypedOutcome
- Closed enum mirroring exact public storage outcomes required by this bead:
  - `SuccessJournalEvent`
  - `SuccessSnapshot`
  - `UnsupportedSchemaVersion`
  - `IndexParityMismatch` or `CorpusIndexParityMismatch`
  - `PayloadTooLarge`
  - `UnexpectedEof`
  - `HeaderChecksumMismatch`
  - `PayloadDigestMismatch`
  - `PostcardDecodeFailed`
  - `SequenceGap`
  - `DuplicateEvent`
  - `CorruptSnapshot`
  - `MissingManifest` or `CorpusMissingManifest`

## Smart Constructors / Parsers at Boundaries

- `StorageBlackhatCorpus::try_from_fixtures(fixtures)` rejects empty corpus, duplicate IDs, fixtures without expected outcomes, and missing required families.
- `StorageFixture::known_good_journal(event)` accepts only semantically valid `JournalEvent` values.
- `StorageFixture::known_good_snapshot(snapshot)` accepts only snapshot records satisfying snapshot sequence/run invariants.
- `StorageFixture::mutated_from_valid(base, mutation, expected)` requires mutation and expected outcome compatibility.
- `ExpectedTypedOutcome::from_journal_error(error)` must be total for all storage errors asserted by the corpus and must reject stringly/unclassified errors.

## Illegal States to Make Unrepresentable

- Fixture with no expected outcome.
- Fixture with both success and error expected.
- Fixture whose bytes are random but unseeded.
- Fixture whose corruption family does not match its expected typed error.
- Missing-index fixture built with string keys instead of `vb_storage::keys` encoders.
- Oversize fixture that allocates the oversized payload to prove rejection.
- Restate-derived raw wire bytes in the corpus.
- Manifest fixture that cannot state which manifest/keyspace invariant is absent.

## Error Variant Contract Impact

If downstream adds `JournalError::IndexParityMismatch`, it must also update diagnostic code mapping and existing error-code tests. If downstream uses a fixture-runner-local corpus error, acceptance tests must prove the missing-index observation maps to exactly that typed corpus error and not to a string diagnostic.
