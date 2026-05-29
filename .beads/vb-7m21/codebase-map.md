# Codebase Map — vb-7m21

## Bead
- **ID:** vb-7m21
- **Title:** storage: Add blackhat corruption fixture corpus
- **State:** go-skill State 2 / explore only
- **Requested outputs:** `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs` test scope, plus likely minimal `vb_storage` typed-error/API support if red tests require it.

## Authority and fences
- `velvet-ballistics-MASTER.md` lines 21-25 require single-server, no-unsafe, no-panic runtime, Fjall persistence, Postcard binary records, and no runtime JSON/YAML/HTTP.
- `velvet-ballistics-MASTER.md` lines 84-105 ban first-party unsafe, unwrap/expect/panic/todo/dbg, unchecked indexing/slicing/casts/arithmetic, ignored fallible results, and unbounded resources.
- `velvet-ballistics-MASTER.md` lines 203-207 require Postcard for journal/snapshot/internal records and Fjall for persistence.
- `to-fix/09-restate-architecture-steal-plan.md` lines 7-13 and 17-21 allow Restate only as failure-mode inspiration. Do **not** copy Restate code, APIs, layouts, wire formats, async architecture, HTTP/gRPC/JSON, RocksDB layout, or distributed assumptions.
- Restate target file `/tmp/opencode/restate/crates/log-server/src/rocksdb_logstore/record_format.rs` is **MISSING** in this environment; exploration did not inspect it. Treat all Restate-derived details as unavailable.

## Existing target test surface
- `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs` is **MISSING**. The bead explicitly names this as the new acceptance test file.
- `crates/workspace_tests/Cargo.toml` package name is `velvet-ballistics-workspace-tests` (line 2). The bead command spells `velvet-ballastics-workspace-tests`; likely command typo. Integration tests are not disabled (`autotests` is absent), so a new `tests/*.rs` file may be auto-discovered, but downstream should verify with the exact nextest command after creation.
- Existing Restate-themed storage tests:
  - `crates/workspace_tests/tests/restate_postcard_envelope_wire_tests.rs`: envelope roundtrip/property tests for all journal `RecordKind` values and edge cases.
  - `crates/workspace_tests/tests/restate_journal_side_index_contracts.rs`: property tests for cross-keyspace event/index atomicity; includes public index query use.
  - `crates/workspace_tests/tests/restate_fjall_keyspace_manifest_tests.rs`: present by directory listing; not deeply inspected in this pass, likely relevant to missing-manifest/keyspace expectations.
  - `crates/workspace_tests/tests/restate_snapshot_sequence_precondition_tests.rs`: present by directory listing; likely relevant to stale snapshot/precondition coverage.
  - `crates/workspace_tests/tests/restate_runtime_version_barrier_tests.rs`: present by directory listing; likely relevant to version barriers.

## Primary crate/API surface: `vb_storage`
- `crates/vb_storage/src/lib.rs`
  - Re-exports `JournalError`, `JournalEvent`, `RecordKind`, `RunSnapshot`, `EventReplayLimit`, `FjallJournal`, `JournalWriteBatch`, `decode_record`, `decode_record_header`, `encode_record`, `encode_record_header`, `verify_digest_match`, and constants.
  - Public wrappers: `open_store`, `init_keyspaces`, `replay_journal`, `append_journal_event`, `write_snapshot`, etc.
- `crates/vb_storage/src/error/mod.rs`
  - Current typed storage errors include `UnsupportedSchemaVersion`, `MigrationRequired`, `UnknownRecordKind`, `RecordKindFamilyMismatch`, `HeaderLengthMismatch`, `PayloadTooLarge`, `HeaderChecksumMismatch`, `PayloadDigestMismatch`, `UnexpectedEof`, `PostcardDecodeFailed`, `InvalidEvent`, `DuplicateEvent`, `SequenceGap`, `TooManyEvents`, `ReplayAllocationFailed`, process-lock errors, and trim errors.
  - **No located `IndexParityMismatch` variant** despite bead acceptance expecting missing-index fixture to return it. Downstream likely needs a minimal new typed variant or a fixture-level wrapper outcome; contract/test planner must decide before implementation.
- `crates/vb_storage/src/error/codes.rs`
  - Diagnostic code mapping for `JournalError`. Any new public error variant probably needs code coverage and code assignment.
- `crates/vb_storage/src/codec/mod.rs`
  - `encode_record<T>` serializes payload with Postcard then creates 60-byte envelope.
  - `decode_record<T>` validates envelope and deserializes payload; `decode_journal_event` adds semantic `JournalEvent::is_valid()` check.
- `crates/vb_storage/src/codec/header.rs`
  - `decode_record_header` checks header length first, then magic, schema version, known kind, kind family, header length, payload bound, and CRC32C.
  - Relevant offsets: schema version bytes 4..6, header len 8..12, payload len 12..16, CRC at `CRC_OFFSET`.
- `crates/vb_storage/src/codec/payload.rs`
  - `payload_len_u32` enforces `u32` conversion and max before allocation; `decode_record_payload` bounds payload slice and checks BLAKE3 digest before Postcard decode.
- `crates/vb_storage/src/codec/validation.rs`
  - Future schema version maps to `JournalError::UnsupportedSchemaVersion`; old schema maps to `MigrationRequired`; unknown kind maps to `UnknownRecordKind`; wrong family maps to `RecordKindFamilyMismatch`.
- `crates/vb_storage/src/constants.rs`
  - Record header is 60 bytes; current schema is 1; max journal event payload is 1 MiB; snapshot max is 64 MiB; keyspaces are declared here.
- `crates/vb_storage/src/records.rs`
  - Defines `RecordKind` ids and durable record structs. `RunHeaderRecord`, `IndexUpdate`, `Snapshot`, and blob/artifact records are in the same file after the inspected range.
- `crates/vb_storage/src/events.rs`
  - `JournalEvent` variants, sequence/run accessors, `record_kind()`, and semantic validity are core fixture payload inputs.
- `crates/vb_storage/src/keys.rs`
  - Fjall key encoders: run events/snapshots use `[prefix][run_id_be][seq_be]`; status/workflow/action indexes have fixed binary keys. Missing-index fixture should use these public key encoders rather than raw ad-hoc string keys.
- `crates/vb_storage/src/indexes.rs`
  - `FjallJournal::put_status_index`, `put_workflow_index`, `put_action_index` insert zero-byte markers into index keyspaces.
- `crates/vb_storage/src/journal/core.rs`
  - `FjallJournal::open` declares all nine keyspaces and exposes public `has_*_index_entry` query APIs for integration tests.
  - `FjallJournal::declared_keyspaces()` is the public manifest-like declared-keyspace surface.
- `crates/vb_storage/src/journal/internal.rs`
  - `decode_optional` decodes stored records from arbitrary keyspaces; `append_unpersisted` rejects already-committed duplicate event keys.
  - `append_queued_unpersisted` treats same duplicate event as idempotent success and divergent duplicate as `DuplicateEvent`.
- `crates/vb_storage/src/journal/replay.rs`
  - `events_for_run`/`events_for_run_bounded` skip durable snapshot prefix if present and validate contiguous event sequences; gaps return `JournalError::SequenceGap`.
  - `get_event_bytes` exposes raw event bytes for tests.
- `crates/vb_storage/src/snapshots.rs`
  - `put_snapshot` and `snapshot` encode/decode `RunSnapshot` records under `MAGIC_SNAPSHOT` and `MAX_SNAPSHOT_BYTES`.
- `crates/vb_storage/src/recovery/types.rs`
  - `RecoveryError::CorruptSnapshot`, `NoRecoveryData`, digest mismatch errors, and `RecoveryRuntimeSummary`/`RunSnapshot` types matter for stale/corrupt snapshot fixtures.
- `crates/vb_storage/src/batch.rs`
  - `JournalWriteBatch` stages cross-keyspace event, index, snapshot, artifact, blob, and header writes into Fjall `OwnedWriteBatch`; relevant for index parity/missing index fixture setup.

## Existing close-neighbor tests/proofs
- `crates/vb_storage/src/security_tests.rs`
  - Already covers oversized declared payload -> `PayloadTooLarge`, header-only/truncated -> `UnexpectedEof`, future schema -> `UnsupportedSchemaVersion`, wrong kind family, cross-run isolation, etc.
- `crates/vb_storage/src/vb_2bok_durability_gate_tests.rs`
  - Blackhat-style cases BH-03/BH-05/BH-07/BH-09/BH-15: all-zero/corrupt payload, truncation, future schema, CRC flip, max-payload rejection.
- `crates/vb_storage/src/tests.rs`
  - Existing sequence gap, duplicate event, schema migration/unsupported schema, unexpected EOF, and diagnostic-code assertions.
- `crates/vb_storage/src/kani_codec.rs`, `kani_record_schema.rs`, `kani_record_payload_len.rs`, `kani_record_crc.rs`, `kani_postcard_envelope_wire.rs`, `kani_storage_invariants.rs`
  - Existing Kani harnesses cover bounded codec/header/schema/payload invariants. New fixture corpus should avoid weakening these; if public error taxonomy changes, Kani/error-code tests may need scoped updates by downstream implementation.
- `crates/vb_storage/src/codec_miri_tests.rs` exists for Miri-only codec checks.

## Fixture families requested and likely mapping
1. **Known-good minimal journal event**: use `JournalEvent::RunAccepted` or `RunCancelled`, `encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, seq, &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)`, expect decode OK.
2. **Known-good snapshot envelope**: use `RunSnapshot` with `MAGIC_SNAPSHOT`, `RecordKind::Snapshot`, `MAX_SNAPSHOT_BYTES`, or `FjallJournal::put_snapshot`/`snapshot`, expect decode OK.
3. **Unknown/future version**: mutate bytes 4..6 to `CURRENT_SCHEMA_VERSION + 1`, recompute CRC32C, expect `JournalError::UnsupportedSchemaVersion { .. }`.
4. **Truncated header/header-only declared payload**: use fewer than 60 bytes or header-only from valid encoded record, expect `UnexpectedEof`.
5. **Oversized declared record**: mutate payload_len at bytes 12..16 above max and recompute CRC; expect `PayloadTooLarge` before payload allocation.
6. **Corrupt envelope/payload**: mutate payload with valid header or mutate CRC; expect `PayloadDigestMismatch` or `HeaderChecksumMismatch` depending fixture intent.
7. **Journal gaps**: append seq 0 and seq 2 for same run, then `events_for_run`, expect `SequenceGap`.
8. **Duplicate idempotency keys**: no storage-level idempotency-key API was located in `vb_storage`; duplicate event key surface is `DuplicateEvent` / queued idempotent same-event success. Runtime idempotency fields exist in other integration tests but not as storage key corpus. Treat as an open contract question.
9. **Missing indexes**: public `has_*_index_entry` can observe missing marker entries, but no typed `IndexParityMismatch` was located. Likely requires a new parity-check function/error or a test-local corpus runner that maps missing index observation to a new typed corpus error.
10. **Stale snapshots**: replay starts after latest durable snapshot seq; stale/corrupt snapshot may be represented via `RunSnapshot` + tail events or recovery errors. Inspect `restate_snapshot_sequence_precondition_tests.rs` before implementation.
11. **Missing manifests**: storage uses `FjallJournal::declared_keyspaces()` and keyspace constants; no file-manifest API found in inspected files. Inspect `restate_fjall_keyspace_manifest_tests.rs` before implementation.

## Risks for downstream lanes
- **Persistence / corruption:** high. Fixtures intentionally feed corrupt records and missing cross-keyspace state.
- **Parser/codec:** high. Envelope decode ordering must be exact and allocation-bounded.
- **Public API:** medium/high if `IndexParityMismatch` or fixture-corpus error types are added.
- **Regression collision:** existing storage security tests already cover several requested errors; new workspace corpus should consolidate/blackhat-assert exact typed mapping without duplicating broad unit coverage unnecessarily.
- **No-copy fence:** Restate source file missing; downstream must not invent/copy any external wire format.
- **Command typo risk:** bead command uses wrong package spelling; actual package is `velvet-ballistics-workspace-tests`.

## Likely scoped commands for downstream evidence
- `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus`
- `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_postcard_envelope_wire_tests --test restate_journal_side_index_contracts`
- If `vb_storage` production API/error changes: `cargo nextest run -p vb_storage` or a narrower lib-test filter around codec/error/recovery.
- Final canonical gate remains `moon ci` per `AGENTS.md` and master contract.

## Open questions / blocked facts
- `IndexParityMismatch` does not exist in located `JournalError` taxonomy. Decide whether acceptance requires adding it to `vb_storage`, adding a fixture-runner-local error enum, or mapping to an existing error.
- `duplicate idempotency key` is not a located storage public concept. Decide whether the fixture should use duplicate event sequence (`DuplicateEvent`) or a runtime/admission idempotency surface outside `vb_storage`.
- `missing manifest` is ambiguous: could mean missing Fjall keyspace declaration, absent external fixture manifest, or missing storage corpus manifest. Existing keyspace manifest tests should be inspected by implementation/test planning.
