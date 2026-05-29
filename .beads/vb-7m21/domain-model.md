# Domain Model — vb-7m21

## Scope

Bead `vb-7m21` adds a blackhat storage corruption fixture corpus for `vb_storage` and its workspace integration surface. The corpus is cold-path test evidence only; it must not introduce runtime JSON/YAML/HTTP, Restate wire formats, copied Restate code, or mutation of production data.

## Ubiquitous Language

- **Fixture corpus**: a deterministic set of named storage examples, each declaring one expected typed outcome.
- **Fixture**: one immutable input scenario. A fixture is either a valid envelope example or a corrupt/persistence-invariant example.
- **Envelope**: the 60-byte binary header plus Postcard payload described by the master storage contract.
- **Header corruption**: a defect fully detected before payload allocation: short header, bad schema, bad record kind family, invalid header length, declared payload over max, or bad header CRC.
- **Payload corruption**: a defect detected after bounded header validation: payload digest mismatch, short payload, or Postcard decode failure.
- **Persistence invariant corruption**: storage state whose bytes may individually decode but whose cross-record relationship is invalid: journal sequence gap, missing side index, duplicate divergent event, stale snapshot, or missing declared keyspace/manifest.
- **Typed outcome**: exact public error variant or exact success family; string matching is not an acceptance criterion.
- **No-copy fixture**: a fixture derived from velvet-ballistics contracts and existing VB APIs, not external Restate bytes, layouts, names, or semantics.

## Entities and Value Objects

- **FixtureId**: stable symbolic identifier used only in the corpus runner; must be unique and non-empty.
- **FixtureFamily**: `ValidJournal`, `ValidSnapshot`, `HeaderCorruption`, `PayloadCorruption`, `JournalInvariant`, `IndexInvariant`, `SnapshotInvariant`, `ManifestInvariant`, `DuplicateInvariant`.
- **FixtureBytes**: bounded byte sequence produced by VB `encode_record` or a deterministic byte mutator over a VB-encoded record.
- **FixtureMutation**: deterministic corruption operation with explicit target field and seed if random-like data is used; unseeded randomness is illegal.
- **ExpectedStorageOutcome**: `OkJournalEvent`, `OkSnapshot`, or exact typed error family.
- **CorpusCoverage**: evidence that every required storage error family named by this bead is represented by at least one fixture.

## Aggregates

- **StorageBlackhatCorpus** owns all fixture definitions and coverage metadata. It is valid only if every fixture has a unique `FixtureId`, deterministic input construction, exact expected typed outcome, and a family classification.
- **FixtureRunner** applies one fixture to the public storage/codec surface and returns a typed outcome. It must not allocate payload memory before the existing header bound checks have accepted the declared payload length.

## Commands

- `BuildKnownGoodJournalFixture`
- `BuildKnownGoodSnapshotFixture`
- `BuildCorruptEnvelopeFixture`
- `BuildUnknownVersionFixture`
- `BuildTruncatedHeaderFixture`
- `BuildOversizedRecordFixture`
- `BuildJournalGapFixture`
- `BuildDuplicateEventFixture`
- `BuildMissingIndexFixture`
- `BuildStaleSnapshotFixture`
- `BuildMissingManifestFixture`
- `RunFixture`
- `AssertCorpusCoverage`

## Domain Events

- `FixtureAcceptedAsValid`
- `FixtureRejectedWithTypedError`
- `FixtureCoverageConfirmed`
- `FixtureConstructionRejected`

## Policies

1. Valid fixture bytes must be generated from VB public storage APIs and constants.
2. Corrupt bytes must be obtained by deterministic mutation of valid VB bytes or by explicit bounded byte literals matching the VB master envelope.
3. Each invalid fixture maps to exactly one typed error or a fixture-runner typed corpus error if the public storage API lacks a direct variant.
4. Oversized declared payload fixtures must fail before payload allocation.
5. Missing-index fixtures must use typed key encoders and public index query APIs, never ad-hoc string keys.
6. Restate artifacts are failure-mode inspiration only; no fixture may embed or imitate Restate record layouts.

## Open Domain Decisions

- `IndexParityMismatch` is required by bead acceptance but was not found in `JournalError`. Downstream must choose either a new public `JournalError::IndexParityMismatch` or a corpus-local typed error that is contractually mapped to missing side-index observation.
- `duplicate idempotency key` is not a located `vb_storage` concept. The storage-level substitute is duplicate event key behavior unless a runtime/action idempotency surface is brought into scope by a later approved bead.
- `missing manifest` must be pinned to either missing declared Fjall keyspace coverage, missing fixture manifest entry, or missing storage corpus manifest. This contract treats it as declared-keyspace/manifest parity until superseded.
