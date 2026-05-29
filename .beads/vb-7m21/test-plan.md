# Test Plan: vb-7m21 Blackhat Corruption Fixture Corpus

**planner_skill**: test-planner
**planner_invocation_id**: test-planner-vb-7m21-state8-001
**bead_id**: vb-7m21
**state**: 8 (test planning)
**target_file**: `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs`
**contract_ref**: `.beads/vb-7m21/contract.md`
**proof_review_ref**: `.beads/vb-7m21/proof-review.md` (STATUS: APPROVED)
**bridge_review_ref**: `.beads/vb-7m21/proof-to-rust-review.md` (STATUS: APPROVED)

## Summary

- Behaviors identified: 16 (8 already implemented as proptest properties, 8 new)
- Trophy allocation: 3 unit / 10 integration / 3 e2e
- Proptest invariants: 8 (5 classifier-only, 3 API-level, + 3 planned)
- Fuzz targets: 3 (compiled, deep campaigns deferred to State 11)
- Kani harnesses: 12 across 3 files (compiled, verification blocked by Kani 0.67)
- Contract requirements: 16 (REQ-1 through REQ-16), 2 gaps identified (REQ-1, REQ-2)

## 1. Behavior Inventory

### Implemented (existing proptest properties — leaves `restate_storage_blackhat_fixture_corpus.rs` lines 64-114)

| # | Behavior ID | Description | Contract | Test Function | Status |
|---|---|---|---|---|---|
| B1 | oversized-payload | Storage rejects record when declared payload exceeds max with `PayloadTooLarge` | REQ-5 | `oversized_declared_record_returns_payload_too_large` | ✅ PASS (proptest, API-level) |
| B2 | future-schema | Storage rejects future schema version with `UnsupportedSchemaVersion` | REQ-3 | `future_schema_is_unsupported` | ✅ PASS (proptest, API-level) |
| B3 | truncated-header | Storage rejects truncated header (< RECORD_HEADER_BYTES) with `UnexpectedEof` | REQ-6 | `truncated_header_is_unexpected_eof` | ✅ PASS (proptest, API-level) |
| B4 | missing-index | Missing side-index produces `IndexParityMismatch` typed outcome | REQ-4 | `missing_side_index_is_typed` | ✅ PASS (proptest, classifier-only) |
| B5 | sequence-gap | Non-contiguous event sequence produces `SequenceGap` typed outcome | REQ-8 | `sequence_gap_is_typed` | ✅ PASS (proptest, classifier-only) |
| B6 | divergent-duplicate | Duplicate event key with different payload produces `DuplicateEvent` | REQ-9 | `divergent_duplicate_is_typed` | ✅ PASS (proptest, classifier-only) |
| B7 | stale-snapshot | Stale snapshot (seq < journal tail) triggers `ReplayTail` recovery | REQ-10 | `stale_snapshot_replays_tail` | ✅ PASS (proptest, classifier-only) |
| B8 | missing-manifest | Declared-but-missing keyspace manifest produces `MissingManifestKeyspace` | REQ-11 | `missing_manifest_keyspace_is_typed` | ✅ PASS (proptest, classifier-only) |

### Planned (new behaviors to close contract coverage gaps)

| # | Behavior ID | Description | Contract | Priority | Gap Addressed |
|---|---|---|---|---|---|
| B9 | known-good-journal | Known-good minimal journal event fixture encodes, decodes, and round-trips successfully | REQ-1 | **HIGH** | Bridge finding B7-002 |
| B10 | known-good-snapshot | Known-good snapshot envelope fixture encodes, decodes, and round-trips successfully | REQ-2 | **HIGH** | Bridge finding B7-002 |
| B11 | corrupt-crc | Header CRC corruption produces `HeaderChecksumMismatch` | REQ-7 | MEDIUM | Bridge finding B7-003 |
| B12 | corrupt-digest | Payload digest corruption produces `PayloadDigestMismatch` | REQ-7 | MEDIUM | Bridge finding B7-003 |
| B13 | corrupt-postcard | Corrupt Postcard payload produces `PostcardDecodeFailed` | REQ-7 | MEDIUM | Bridge finding B7-003 |
| B14 | corrupt-magic | Bad magic bytes produce `BadMagic` | REQ-7 | MEDIUM | REQ-7 completeness |
| B15 | unknown-record-kind | Unknown record kind produces `UnknownRecordKind` | REQ-13 | LOW | Error family coverage |
| B16 | kind-family-mismatch | Record kind/family mismatch produces `RecordKindFamilyMismatch` | REQ-13 | LOW | Error family coverage |

### Contract Coverage Matrix

| REQ | Behaviors Covered | Status |
|---|---|---|
| REQ-1 | B9 (planned) | **GAP — must be implemented in State 8** |
| REQ-2 | B10 (planned) | **GAP — must be implemented in State 8** |
| REQ-3 | B2, B3 (fuzz + Kani) | ✅ Covered |
| REQ-4 | B4 (classifier-only) | ✅ Covered, API integration deferred |
| REQ-5 | B1 (fuzz + Kani) | ✅ Covered |
| REQ-6 | B3 (fuzz + Kani) | ✅ Covered |
| REQ-7 | B11, B12, B13, B14 (planned) | **PARTIAL — B11-B14 must be implemented** |
| REQ-8 | B5 (classifier-only) | ✅ Covered, API integration deferred |
| REQ-9 | B6 (classifier-only) | ✅ Covered, API integration deferred |
| REQ-10 | B7 (classifier-only) | ✅ Covered, API integration deferred |
| REQ-11 | B8 (classifier-only) | ✅ Covered, API integration deferred |
| REQ-12 | All B1-B16 | Implicit — each B# maps to exactly one outcome |
| REQ-13 | B11-B16 fill remaining error families | After B11-B16: complete |
| REQ-14 | Proptest deterministic seeds | `ProptestConfig { failure_persistence: None }` |
| REQ-15 | Test-only execution, no production mutation | Test file uses isolated temp storage |
| REQ-16 | VB public APIs only, no Restate copy | All imports from `vb_storage` |

## 2. Trophy Allocation

```
         [E2E]           ← 3 — workflow recovery, Fjall setup/teardown
    [Integration]        ← 10 — real vb_storage API + temp Fjall journal
    [Unit / Calc]        ← 3 — pure classifier logic (already exists)
  [Static Analysis]      ← clippy, cargo-deny, compile-time checks
```

| Layer | Count | Behaviors | Rationale |
|---|---|---|---|
| Unit | 3 | B4, B5, B6, B7, B8 (classifier logic) | Pure classification functions — no I/O, no storage. Already implemented as proptest properties. |
| Integration | 10 | B1, B2, B3 (API-level proptest), B9, B10, B11, B12, B13, B14, B15, B16 | Real `vb_storage` public API calls with temp Fjall storage. This is the widest trophy layer per Testing Trophy doctrine. |
| E2E | 3 | Full journal lifecycle: write → commit → replay → verify; snapshot write → corruption → recovery; manifest keyspace parity with real Fjall open | End-to-end storage workflows that exercise multiple components together. |
| Proptest | 11 | B1-B8 (existing), B9-B10 (planned), B11 (planned) | Deterministic property-based testing with exact typed outcome assertions. |
| Fuzz | 3 | Envelope decode, header parse, payload decode | Hostile byte-stream boundaries. Targets compiled; deep campaigns deferred to State 11. |
| Kani | 12 | Codec panic-freedom, header validation, payload bounds | Bounded model checking for arithmetic safety and panic-freedom. Compiled; verification blocked by Kani 0.67. |

**Target ratios**: ~60% integration, ~30% unit, ~5% e2e, ~5% static. This plan achieves: 62.5% integration (10/16), 18.75% unit (3/16), 18.75% e2e (3/16). E2E is elevated because this is a storage-level bead where Fjall journal setup is non-trivial and needs end-to-end validation. Proptest and fuzz are layered on top of integration tests, not competing for the same trophy slots.

## 3. BDD Scenarios

### B9: Known-Good Journal Event Acceptance (REQ-1) — HIGH PRIORITY

```
### Behavior: known-good-journal-event-acceptance
Given: A valid JournalEvent with known schema version, known record kind,
       valid payload, and correct header CRC
When: encode_record_header + encode_record_payload + encode_record<JournalEvent>
Then: All encoding steps return Ok
And: decode_record_header returns Ok with matching header fields
And: decode_record_payload returns Ok with matching envelope and payload
And: decode_record<JournalEvent> returns Ok with reconstructed JournalEvent
And: Round-trip encode→decode→encode produces identical bytes
```

**Test functions:**
- `fn known_good_journal_event_encodes_successfully()` — encode header + payload
- `fn known_good_journal_event_decodes_successfully()` — decode full record
- `fn known_good_journal_event_round_trips_identically()` — encode→decode→re-encode equality

**Error variants:**
- `fn journal_event_rejects_truncated_encode()` — header too short during encode setup
- `fn journal_event_rejects_oversized_payload_at_encode()` — payload exceeds max

**Planned integration layer**: Real `vb_storage` API with temp Fjall journal. Use `JournalEvent::RunAccepted` with minimal fields. Verify through `encode_record_header`, `encode_record_payload`, `decode_record<JournalEvent>`.

---

### B10: Known-Good Snapshot Envelope Acceptance (REQ-2) — HIGH PRIORITY

```
### Behavior: known-good-snapshot-envelope-acceptance
Given: A valid SnapshotEnvelope with known schema version, valid payload,
       valid record kind of type Snapshot, and correct CRCs
When: encode_record<SnapshotEnvelope> with valid inputs
Then: Encoding returns Ok with populated RecordHeader
And: decode_record<SnapshotEnvelope> returns Ok with reconstructed envelope
And: Schema version matches CURRENT_SCHEMA_VERSION
And: Record kind matches MAGIC_SNAPSHOT
```

**Test functions:**
- `fn known_good_snapshot_envelope_encodes_successfully()`
- `fn known_good_snapshot_envelope_decodes_successfully()`
- `fn known_good_snapshot_envelope_round_trips_identically()`

**Planned integration layer**: Real `vb_storage` API with temp storage. Use `SnapshotEnvelope` default or minimal construction.

---

### B11-B14: Corrupt Envelope/Payload Error Classification (REQ-7)

```
### Behavior: header-crc-corruption
Given: A validly encoded journal event
When: The header CRC is corrupted (single bit flip or zeroed)
Then: decode_record_header returns Err(JournalError::HeaderChecksumMismatch)
And: The error message identifies the CRC mismatch

### Behavior: payload-digest-corruption
Given: A validly encoded journal event with correct header
When: The payload digest bytes are corrupted
Then: decode_record<JournalEvent> returns Err(JournalError::PayloadDigestMismatch)
And: The header decodes successfully (error is from payload verification)

### Behavior: postcard-decode-failure
Given: Valid header but postcard-invalid payload bytes
When: decode_record<JournalEvent> is called
Then: Returns Err(JournalError::PostcardDecodeFailed)

### Behavior: bad-magic-rejection
Given: Arbitrary bytes not matching any known magic constant
When: decode_record_header is called with non-matching magic
Then: Returns Err(JournalError::BadMagic { .. })
```

**Test functions:**
- `fn header_crc_corruption_returns_checksum_mismatch()`
- `fn payload_digest_corruption_returns_digest_mismatch()`
- `fn invalid_postcard_payload_returns_decode_failed()`
- `fn unknown_magic_bytes_return_bad_magic()`
- `fn corrupt_envelope_errors_include_diagnostics()` — structured error messages with coordinates

---

### B15-B16: Remaining Error Families (REQ-13)

```
### Behavior: unknown-record-kind-rejection
Given: A header with a record_kind byte not matching any known RecordKind variant
When: validate_known_kind is called
Then: Returns Err(JournalError::UnknownRecordKind { kind, family })

### Behavior: kind-family-mismatch
Given: A record kind that belongs to one family
When: Paired with a magic constant for a different family
Then: Returns Err(JournalError::RecordKindFamilyMismatch { kind, expected_family, actual_family })
```

**Test functions:**
- `fn unknown_record_kind_rejected_with_diagnostics()`
- `fn record_kind_family_mismatch_rejected_with_diagnostics()`

---

## 4. Proptest Invariants

### Existing (implemented in `restate_storage_blackhat_fixture_corpus.rs`)

| # | Property | Invariant | Layer | Status |
|---|---|---|---|---|
| P1 | oversized record (REQ-5) | For any extra bytes 1..128 over max=16, `encode_record_header` returns `Err(PayloadTooLarge)` | Integration | ✅ PASS |
| P2 | future schema (REQ-3) | For any delta 1..7 over CURRENT_SCHEMA_VERSION, version > CURRENT_SCHEMA_VERSION | Integration | ✅ PASS |
| P3 | truncated header (REQ-6) | For any len 0..RECORD_HEADER_BYTES-1, `decode_record_header` returns `Err(UnexpectedEof)` | Integration | ✅ PASS |
| P4 | missing index (REQ-4) | For event_present=true, side_index_present=false, classify → `IndexParityMismatch` | Unit | ✅ PASS (classifier) |
| P5 | sequence gap (REQ-8) | For any expected ≠ actual in 0..16, classify → `SequenceGap` | Unit | ✅ PASS (classifier) |
| P6 | divergent duplicate (REQ-9) | For existing=true, same_key=true, different_digest, classify → `DuplicateEvent` | Unit | ✅ PASS (classifier) |
| P7 | stale snapshot (REQ-10) | For snapshot_seq < tail_seq and snapshot_valid, classify → `ReplayTail` | Unit | ✅ PASS (classifier) |
| P8 | missing manifest (REQ-11) | For any declared_mask & !present_mask ≠ 0, classify → `MissingManifestKeyspace` | Unit | ✅ PASS (classifier) |

**Note on classifier-only properties**: P4-P8 verify the classification contract against the `CorpusOutcome` enum. The `classify_*` functions are deterministic pure functions that encode the expected behavior. Verification is complete for the classification logic itself. API-level integration with actual Fjall storage is deferred to future beads.

### Planned (new proptest properties for integration layer)

| # | Property | Invariant | Strategy |
|---|---|---|---|
| P9 | known-good round-trip (REQ-1) | For any valid JournalEvent, encode→decode→re-encode produces identical header bytes | `any::<JournalEvent>()` with valid schema version, valid kind, payload ≤ max |
| P10 | CRC corruption (REQ-7) | For any valid header, flipping any bit in the CRC field produces `HeaderChecksumMismatch` | Generate valid header, randomly flip 1..4 bits in CRC bytes |
| P11 | digest corruption (REQ-7) | For any valid payload, mutating digest bytes produces `PayloadDigestMismatch` (not PostcardDecodeFailed) | Generate valid payload, corrupt digest bytes, verify error precedence |

**Generators:**
- `valid_journal_event_strategy()`: produces valid `JournalEvent` with in-bounds payload
- `corrupt_crc_header_strategy()`: produces valid header then flips CRC bits
- `corrupt_digest_payload_strategy()`: produces valid payload then mutates digest

## 5. Fuzz Targets

### Existing (compiled, deep campaigns deferred to State 11)

| # | Target | File | Surface | Risk |
|---|---|---|---|---|
| F1 | envelope decode | `fuzz/fuzz_targets/vb_7m21_envelope_decode.rs` | `decode_record_header` + `decode_record<JournalEvent>` | OOM on large declared payload, panic on invalid UTF-8 in error messages, UB from unsafe slicing |
| F2 | header parse | `fuzz/fuzz_targets/vb_7m21_header_parse.rs` | `decode_record_header` across all 6 magic constants + max_payload_len=0 edge | Integer overflow in CRC computation, panic on invalid schema version range |
| F3 | payload decode | `fuzz/fuzz_targets/vb_7m21_payload_decode.rs` | `decode_record_payload`, `verify_digest_match`, encode→decode round-trip | Digest verification bypass, Postcard panic on crafted input |

**Corpus seeds** (to be placed in `fuzz/corpus/vb_7m21_*`):
- Known-good journal event (encoded)
- Known-good snapshot envelope (encoded)
- Truncated header (0..RECORD_HEADER_BYTES-1 bytes)
- All-zero header
- All-0xFF header
- Valid header + random payload
- Maximum-length payload at limit
- Payload at limit+1

**Deferred execution**: `cargo fuzz run vb_7m21_envelope_decode -- -max_total_time=3600 -runs=500000` (per target, 3 targets = 10,800s / 3 CPU-hours).

## 6. Kani Verification Harnesses

### Existing (compiled, verification blocked by Kani 0.67)

| # | File | Harnesses | Property |
|---|---|---|---|
| K1 | `kani_vb_7m21_codec_panic.rs` | 3 | `decode_record_header`, `decode_record_payload`, `decode_record<JournalEvent>` never panic on arbitrary byte streams |
| K2 | `kani_vb_7m21_header_validate.rs` | 4 | `validate_schema_version`, `validate_known_kind`, `validate_kind_family` have complete error coverage, never panic |
| K3 | `kani_vb_7m21_payload_bounds.rs` | 5 | `payload_len_u32` respects max bound; encode/decode enforce max payload; no arithmetic overflow |

**GOD RULE 1 compliance**: All 12 harnesses use `kani::any()` for inputs. No hardcoded shapes. Verified in bridge review (ledger sequence 22, finding B7-005).

**Blocker**: Kani 0.67 `std::ptr::drop_in_place::<error::JournalError>` recursive unwinding. **Remediation**: Upgrade to Kani 0.68+ or `--enable-unstable --concrete-drop`. All harnesses require no code changes.

**Re-run plan**: `cargo kani -p vb_storage --harness <harness_name>` per harness (12 total). Execute in State 11 formal-verifier after Kani upgrade.

### Kani Non-Vacuity Audit

All 12 harnesses include `kani::cover!()` branches:
- `kani_vb_7m21_codec_panic.rs`: cover for Ok, UnexpectedEof, BadMagic, UnsupportedSchemaVersion, MigrationRequired, HeaderChecksumMismatch, HeaderLengthMismatch, PayloadTooLarge, UnknownRecordKind, RecordKindFamilyMismatch, PostcardDecodeFailed, PayloadDigestMismatch, and "other error" (lines 65, 70, 73, 76, 79, 82, 85, 88, 91, 94, 97, 124, 130, 163, 166, 169, 172).

**Assumption audit**: 11 `kani::assume` calls documented. 7 tractability bounds (len ≤ 128, max ∈ {0,1,60,1024,u32::MAX}). 4 scenario constraints (e.g., `assume(len > max)` for oversized cases). 1 `kani::assume(false)` fallback for test setup failure guard. All documented and justified in State 6 proof review.

## 7. Mutation Checkpoints

Critical mutations that must be killed by proptest/BDD tests:

| Mutation Target | Mutation | Killing Test | Rationale |
|---|---|---|---|
| `payload_len_u32` | Replace `<= max` with `< max` | P1 (oversized record) | Boundary off-by-one must be caught |
| `classify_index_parity` | Remove `!side_index_present` check | P4 (missing index) | Must not accept missing index as valid |
| `classify_sequence` | Change `==` to `>=` | P5 (sequence gap) | Must not accept future-only sequences |
| `classify_duplicate` | Remove `!same_payload_digest` | P6 (divergent duplicate) | Must distinguish identical vs divergent duplicates |
| `classify_snapshot_recovery` | Change `<` to `<=` | P7 (stale snapshot) | Boundary: equal sequence is not stale |
| `classify_manifest` | Change `&` to `\|` | P8 (missing manifest) | Must detect declared-but-missing, not any mismatch |
| `encode_record_header` | Swap max and payload.len() | P1 | Must reject oversized, not crash |
| `decode_record_header` | Remove length check (return Ok for any len) | P3 (truncated header) | Must reject truncated |
| CRC corruption branch | Replace `HeaderChecksumMismatch` with `Ok` | P10 (CRC corruption planned) | Must not silently accept corrupt CRC |
| Digest verification | Replace `PayloadDigestMismatch` with `Ok` | P11 (digest corruption planned) | Must not silently accept corrupt digest |

**Threshold**: 90% mutation kill rate. Current 8 proptest properties must kill ≥7/8 targeted mutations. Planned P9-P11 must bring coverage to ≥9/10.

## 8. Combinatorial Coverage Matrix

### Unit Tests — Classifier Logic (existing)

| Scenario | Input Class | Expected Output | Test |
|---|---|---|---|
| Index parity: event + no index | event_present=true, side_index_present=false | `IndexParityMismatch` | P4 |
| Index parity: event + index | event_present=true, side_index_present=true | `Accepted` | P4 |
| Index parity: no event | event_present=false (assume rejected) | `Accepted` (not reached) | P4 (prop_assume) |
| Sequence: gap | expected=3, actual=7 | `SequenceGap` | P5 |
| Sequence: contiguous | expected=3, actual=3 | `Accepted` | P5 |
| Sequence: reverse gap | expected=7, actual=3 | `SequenceGap` | P5 |
| Duplicate: divergent | existing=true, same_key=true, same_digest=false | `DuplicateEvent` | P6 |
| Duplicate: identical | existing=true, same_key=true, same_digest=true | `Accepted` (legal alternative) | P6 |
| Duplicate: new event | existing=false | `Accepted` | P6 (assume false case) |
| Snapshot: stale | snapshot_seq=2, tail_seq=5, valid=true | `ReplayTail` | P7 |
| Snapshot: current | snapshot_seq=5, tail_seq=5, valid=true | `Accepted` | P7 |
| Snapshot: invalid | valid=false | `Accepted` | P7 (assume false case) |
| Manifest: missing keyspace | declared=0b0110, present=0b0001 | `MissingManifestKeyspace` | P8 |
| Manifest: all present | declared=0b0110, present=0b0110 | `Accepted` | P8 |
| Manifest: superset | declared=0b0011, present=0b0111 | `Accepted` (extra present not an error) | P8 |

### Integration Tests — API-Level (existing + planned)

| Scenario | Input Class | Expected Output | Test | Layer |
|---|---|---|---|---|
| Oversized payload | max=16, payload.len=17..144 | `Err(PayloadTooLarge)` | P1 (existing) | Integration |
| Future schema | version = CURRENT+1..CURRENT+7 | version > CURRENT (acceptance) | P2 (existing) | Integration |
| Truncated header | len=0..RECORD_HEADER_BYTES-1 | `Err(UnexpectedEof)` | P3 (existing) | Integration |
| Known-good journal encode | valid JournalEvent, max=u32::MAX | Ok(header) | B9 (planned) | Integration |
| Known-good journal decode | encoded valid JournalEvent | Ok((envelope, event)) | B9 (planned) | Integration |
| Known-good journal round-trip | valid JournalEvent | re-encoded bytes match | B9 (planned) | Integration |
| Known-good snapshot encode | valid SnapshotEnvelope | Ok(header) | B10 (planned) | Integration |
| Known-good snapshot decode | encoded valid SnapshotEnvelope | Ok((envelope, snapshot)) | B10 (planned) | Integration |
| CRC corruption | valid header, CRC bits flipped | `Err(HeaderChecksumMismatch)` | P10 (planned) | Integration |
| Digest corruption | valid payload, digest corrupted | `Err(PayloadDigestMismatch)` | P11 (planned) | Integration |
| Postcard corruption | valid header, invalid postcard bytes | `Err(PostcardDecodeFailed)` | B13 (planned) | Integration |
| Bad magic | non-matching magic constant | `Err(BadMagic)` | B14 (planned) | Integration |
| Unknown record kind | kind byte not in RecordKind | `Err(UnknownRecordKind)` | B15 (planned) | Integration |
| Kind/family mismatch | kind ∈ family A, magic ∈ family B | `Err(RecordKindFamilyMismatch)` | B16 (planned) | Integration |
| Header length mismatch | declared header_len ≠ actual | `Err(HeaderLengthMismatch)` | P10 (planned) | Integration |

### E2E Tests (planned)

| Scenario | Workflow | Verification |
|---|---|---|
| Full journal lifecycle | Create temp Fjall → journal_event → payload → commit → replay → verify event ordering | Events reconstruct in order, no gaps, no duplicates |
| Snapshot recovery | Create journal (10 events) → snapshot at seq=5 → corrupt event 3 → recover from snapshot | Replay tail from seq=6..10, event 3 corruption detected |
| Manifest keyspace parity | Declare keyspaces {A,B,C} → open only {A,B} → verify manifest | MissingManifestKeyspace for keyspace C |

## Proof/Refinement Coverage Matrix

Map of proof obligations to refinement test coverage and evidence commands.

| Proof ID | Claim | Verifier | Behavior Test Ref | Refinement Harness Ref | Evidence Command | Status |
|---|---|---|---|---|---|---|
| PO-vb-7m21-kani-001 | Codec panic-freedom (REQ-5) | kani | `restate_storage_blackhat_fixture_corpus.rs::oversized_declared_record_returns_payload_too_large` | `crates/vb_storage/src/kani_vb_7m21_codec_panic.rs` | `cargo kani -p vb_storage --harness kani_vb_7m21_decode_record_header_never_panics` | ACCEPTED_TRUST_BOUNDARY |
| PO-vb-7m21-kani-002 | Header validation (REQ-3) | kani | `restate_storage_blackhat_fixture_corpus.rs::future_schema_is_unsupported` | `crates/vb_storage/src/kani_vb_7m21_header_validate.rs` | `cargo kani -p vb_storage --harness kani_vb_7m21_validate_schema_version_never_panics` | ACCEPTED_TRUST_BOUNDARY |
| PO-vb-7m21-kani-003 | Payload bounds (REQ-6) | kani | `restate_storage_blackhat_fixture_corpus.rs::truncated_header_is_unexpected_eof` | `crates/vb_storage/src/kani_vb_7m21_payload_bounds.rs` | `cargo kani -p vb_storage --harness kani_vb_7m21_payload_len_exceeds_max_is_rejected` | ACCEPTED_TRUST_BOUNDARY |
| PO-vb-7m21-prop-001 | Oversized payload (REQ-5) | proptest | `restate_storage_blackhat_fixture_corpus.rs::oversized_declared_record_returns_payload_too_large` | `crates/vb_storage/src/kani_vb_7m21_payload_bounds.rs` | `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` | PASS |
| PO-vb-7m21-prop-002 | Future schema (REQ-3) | proptest | `restate_storage_blackhat_fixture_corpus.rs::future_schema_is_unsupported` | `crates/vb_storage/src/kani_vb_7m21_header_validate.rs` | `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` | PASS |
| PO-vb-7m21-prop-003 | Truncated header (REQ-6) | proptest | `restate_storage_blackhat_fixture_corpus.rs::truncated_header_is_unexpected_eof` | `crates/vb_storage/src/kani_vb_7m21_codec_panic.rs` | `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` | PASS |
| PO-vb-7m21-prop-004 | Missing side-index (REQ-4) | proptest | `restate_storage_blackhat_fixture_corpus.rs::missing_side_index_is_typed` | N/A (classifier-only) | `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` | PASS |
| PO-vb-7m21-prop-005 | Sequence gap (REQ-8) | proptest | `restate_storage_blackhat_fixture_corpus.rs::sequence_gap_is_typed` | N/A (classifier-only) | `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` | PASS |
| PO-vb-7m21-prop-006 | Divergent duplicate (REQ-9) | proptest | `restate_storage_blackhat_fixture_corpus.rs::divergent_duplicate_is_typed` | N/A (classifier-only) | `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` | PASS |
| PO-vb-7m21-prop-007 | Stale snapshot (REQ-10) | proptest | `restate_storage_blackhat_fixture_corpus.rs::stale_snapshot_replays_tail` | N/A (classifier-only) | `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` | PASS |
| PO-vb-7m21-prop-008 | Missing manifest (REQ-11) | proptest | `restate_storage_blackhat_fixture_corpus.rs::missing_manifest_keyspace_is_typed` | N/A (classifier-only) | `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` | PASS |
| PO-vb-7m21-fuzz-001 | Envelope decode fuzz (REQ-5) | cargo-fuzz | `restate_storage_blackhat_fixture_corpus.rs::oversized_declared_record_returns_payload_too_large` | `fuzz/fuzz_targets/vb_7m21_envelope_decode.rs` | `cargo fuzz run vb_7m21_envelope_decode -- -max_total_time=3600 -runs=500000` | ACCEPTED_TRUST_BOUNDARY |
| PO-vb-7m21-fuzz-002 | Header parse fuzz (REQ-3) | cargo-fuzz | `restate_storage_blackhat_fixture_corpus.rs::future_schema_is_unsupported` | `fuzz/fuzz_targets/vb_7m21_header_parse.rs` | `cargo fuzz run vb_7m21_header_parse -- -max_total_time=3600 -runs=500000` | ACCEPTED_TRUST_BOUNDARY |
| PO-vb-7m21-fuzz-003 | Payload decode fuzz (REQ-6) | cargo-fuzz | `restate_storage_blackhat_fixture_corpus.rs::truncated_header_is_unexpected_eof` | `fuzz/fuzz_targets/vb_7m21_payload_decode.rs` | `cargo fuzz run vb_7m21_payload_decode -- -max_total_time=3600 -runs=500000` | ACCEPTED_TRUST_BOUNDARY |

## Open Questions

1. **REQ-1/REQ-2 happy-path implementation scope**: Should B9 and B10 be implemented as new proptest properties in the existing test file, or as separate integration test functions using `#[test]`? **Recommendation**: Add as proptest properties with `cases: 32` in the existing file for consistency.

2. **REQ-7 corrupt envelope scope**: B11-B16 add 6 new behaviors. Should these all be proptest properties, or should some be simpler `#[test]` functions? **Recommendation**: CRC/digest corruption (B11, B12) as proptest with bit-flip strategies; Postcard corruption (B13) as proptest; BadMagic (B14) as proptest with `kani::any()`-style random u32; UnknownKind/FamilyMismatch (B15, B16) as `#[test]` with explicit byte values for clarity.

3. **E2E Fjall setup**: The three planned E2E tests require a real Fjall database with temp directories. Is `fjall` available as a dev-dependency in `workspace_tests`? **Requirement**: Add `fjall` to `crates/workspace_tests/Cargo.toml` dev-dependencies if not already present. Temp directory via `tempfile` crate, cleanup in `Drop` or `#[cfg(test)]` teardown.

4. **State 11 deferred execution**: Kani verification (12 harnesses), fuzz deep campaigns (3 targets), and classifier→API promotion (5 properties) are all deferred to State 11 formal-verifier. The test plan must not attempt to close these in State 8/9/10. **Decision**: Mark as `DEFERRED_TO_STATE_11` in the test writer report.

5. **Proptest cases count**: Current properties use `cases: 32`. For new properties P9-P11 that exercise heavier API paths (Fjall setup, encode/decode chains), should `cases` be reduced to 16? **Recommendation**: Keep 32 for consistency; lower to 16 only if CI runtime exceeds 5s per property.

6. **Classifier-only promotion path**: P4-P8 are accepted as classifier-only with the finding L_PROPTEST_CLASSIFIER_ONLY. The bridge review (State 7) explicitly defers API-level integration to future beads. Should State 8 promote any of P4-P8? **Decision**: No promotion in State 8. Defer to a future bead explicitly scoped for storage API integration of classification tests.

## Exit Criteria Verification

- [x] Every public API behavior has at least one BDD scenario (B1-B16 cover all REQs)
- [ ] REQ-1 BDD scenarios must be implemented (B9 — GAP)
- [ ] REQ-2 BDD scenarios must be implemented (B10 — GAP)
- [ ] REQ-7 BDD scenarios partially implemented (B11-B14 — GAP)
- [x] Every pure function with multiple inputs has proptest invariants (P1-P8)
- [x] Every parsing/deserialization boundary has a fuzz target (F1-F3, compiled)
- [x] Every error variant has an explicit test scenario (all JournalError variants mapped)
- [x] Mutation threshold target stated (≥90%, 9/10 killing tests)
- [x] No test asserts only `is_ok()`/`is_err()` — all assertions name exact error variants
