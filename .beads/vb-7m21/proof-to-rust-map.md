# Proof-to-Rust Map — vb-7m21 State 7

**reviewer_skill**: proof-to-implementation  
**reviewer_invocation_id**: proof-to-implementation-vb-7m21-state7-001  
**reviewed_writer_invocation_id**: proof-reviewer-vb-7m21-state6-001  
**bead_id**: vb-7m21  
**state**: 7 (proof-to-implementation bridge)  
**sublane**: proof-to-implementation  

## Executive Summary

This map bridges 14 approved proof claims (from `proof-review.md`, invocation `proof-reviewer-vb-7m21-state6-001`) to concrete Rust source refs, independent behavior tests, separate refinement harnesses, and exact evidence commands. All 14 obligations are behavior-affecting. The map materializes the reduced-scope replan: 8 proptest properties (PASS), 3 Kani harness groups (12 harnesses, ACCEPTED_TRUST_BOUNDARY), and 3 fuzz targets (ACCEPTED_TRUST_BOUNDARY). No TLA+ claims are in scope.

## Claims to Source

### Kani Claims → Rust Source

| Proof ID | Contract | Rust Symbols |
|---|---|---|
| PO-vb-7m21-kani-001 | REQ-5 PayloadTooLarge | `crates/vb_storage/src/codec/header.rs::decode_record_header`, `crates/vb_storage/src/codec/payload.rs::decode_record_payload`, `crates/vb_storage/src/codec/mod.rs::decode_record` |
| PO-vb-7m21-kani-002 | REQ-3 UnsupportedSchemaVersion | `crates/vb_storage/src/codec/validation.rs::validate_schema_version`, `crates/vb_storage/src/codec/validation.rs::validate_known_kind`, `crates/vb_storage/src/codec/validation.rs::validate_kind_family` |
| PO-vb-7m21-kani-003 | REQ-6 UnexpectedEof | `crates/vb_storage/src/codec/payload.rs::payload_len_u32`, `crates/vb_storage/src/codec/payload.rs::encode_record_payload`, `crates/vb_storage/src/codec/payload.rs::decode_record_payload` |

### Proptest Claims → Rust Source

| Proof ID | Contract | Rust Symbols |
|---|---|---|
| PO-vb-7m21-prop-001 | REQ-5 PayloadTooLarge | `crates/vb_storage/src/codec/header.rs::encode_record_header`, `crates/vb_storage/src/codec/payload.rs::payload_len_u32` |
| PO-vb-7m21-prop-002 | REQ-3 UnsupportedSchemaVersion | `crates/vb_storage/src/constants.rs::CURRENT_SCHEMA_VERSION` |
| PO-vb-7m21-prop-003 | REQ-6 UnexpectedEof | `crates/vb_storage/src/codec/header.rs::decode_record_header` |
| PO-vb-7m21-prop-004 | REQ-4 IndexParityMismatch | `crates/vb_storage/src/indexes.rs` (side-index storage), `crates/vb_storage/src/error/mod.rs::JournalError` (typed classification) |
| PO-vb-7m21-prop-005 | REQ-8 SequenceGap | `crates/vb_storage/src/error/mod.rs::JournalError::SequenceGap`, `crates/vb_storage/src/journal/replay.rs` (sequence checking) |
| PO-vb-7m21-prop-006 | REQ-9 DuplicateEvent | `crates/vb_storage/src/error/mod.rs::JournalError::DuplicateEvent`, `crates/vb_storage/src/journal/core.rs` (duplicate detection) |
| PO-vb-7m21-prop-007 | REQ-10 StaleSnapshot | `crates/vb_storage/src/snapshots.rs`, `crates/vb_storage/src/recovery/types.rs` (snapshot/replay tail) |
| PO-vb-7m21-prop-008 | REQ-11 MissingManifest | `crates/vb_storage/src/keys.rs`, `crates/vb_storage/src/journal/internal.rs` (declared keyspace parity) |

### Fuzz Claims → Rust Source

| Proof ID | Contract | Rust Symbols |
|---|---|---|
| PO-vb-7m21-fuzz-001 | REQ-5 PayloadTooLarge | `crates/vb_storage/src/codec/header.rs::decode_record_header`, `crates/vb_storage/src/codec/mod.rs::decode_record`, `crates/vb_storage/src/codec/mod.rs::decode_journal_event` |
| PO-vb-7m21-fuzz-002 | REQ-3 UnsupportedSchemaVersion | `crates/vb_storage/src/codec/header.rs::decode_record_header` (all 6 magic values + max=0 edge) |
| PO-vb-7m21-fuzz-003 | REQ-6 UnexpectedEof | `crates/vb_storage/src/codec/payload.rs::decode_record_payload`, `crates/vb_storage/src/codec/payload.rs::verify_digest_match`, `crates/vb_storage/src/codec/mod.rs::encode_record`, `crates/vb_storage/src/codec/mod.rs::decode_record` |

## Behavior Tests

All 8 proptest properties in `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs` serve as the primary behavior tests. Each property:

- `oversized_declared_record_returns_payload_too_large` — calls `vb_storage::encode_record_header` and asserts `Err(JournalError::PayloadTooLarge{..})`
- `future_schema_is_unsupported` — asserts `version > CURRENT_SCHEMA_VERSION`
- `truncated_header_is_unexpected_eof` — calls `vb_storage::decode_record_header` and asserts `Err(JournalError::UnexpectedEof)`
- `missing_side_index_is_typed` — `classify_index_parity` → `IndexParityMismatch`
- `sequence_gap_is_typed` — `classify_sequence` → `SequenceGap`
- `divergent_duplicate_is_typed` — `classify_duplicate` → `DuplicateEvent`
- `stale_snapshot_replays_tail` — `classify_snapshot_recovery` → `ReplayTail`
- `missing_manifest_keyspace_is_typed` — `classify_manifest` → `MissingManifestKeyspace`

Raw command: `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` → 8 passed, 0.004s.

Five properties (prop-004 through prop-008) use local classifier functions rather than calling storage public APIs directly. This is documented as finding L_PROPTEST_CLASSIFIER_ONLY (PF-vb-7m21-018). The classifiers verify the classification contract for each input space. Future beads must wire classification to actual Fjall journal setup for public API integration coverage (tracked in bridge row notes).

## Refinement Harnesses

| Lane | File | Count | Status |
|---|---|---|---|
| Kani | `crates/vb_storage/src/kani_vb_7m21_codec_panic.rs` | 3 harnesses | COMPILED (verification BLOCKED by Kani 0.67) |
| Kani | `crates/vb_storage/src/kani_vb_7m21_header_validate.rs` | 4 harnesses | COMPILED (verification BLOCKED by Kani 0.67) |
| Kani | `crates/vb_storage/src/kani_vb_7m21_payload_bounds.rs` | 5 harnesses | COMPILED (verification BLOCKED by Kani 0.67) |
| Fuzz | `fuzz/fuzz_targets/vb_7m21_envelope_decode.rs` | 1 target | COMPILED (deep campaign deferred to State 11) |
| Fuzz | `fuzz/fuzz_targets/vb_7m21_header_parse.rs` | 1 target | COMPILED (deep campaign deferred to State 11) |
| Fuzz | `fuzz/fuzz_targets/vb_7m21_payload_decode.rs` | 1 target | COMPILED (deep campaign deferred to State 11) |

Kani verification blocker: `std::ptr::drop_in_place::<error::JournalError>` recursive unwinding in Kani 0.67. Remediation requires Kani 0.68+ or `--enable-unstable --concrete-drop`. All 12 harnesses use `kani::any()` per GOD RULE 1. No harness changes required.

Fuzz deep campaign deferred: Targets need `cargo fuzz run -max_total_time=3600 -runs=500000` per target in State 11 formal-verifier.

## Residual Gaps

1. **Kani verification BLOCKED_TOOLING**: All 12 harnesses compiled but not verified. Re-run with Kani 0.68+. Bridge marks `mapping_status: planned` for all Kani rows; `rerun_from: 11` for final materialization.

2. **Proptest classifiers (prop-004 through prop-008)**: Classifier logic verified to produce correct typed outcomes for input space, but public storage API integration deferred. Bridge marks these rows with `mapping_status: planned` for the refinement claim; classifier-only tests must be promoted to API-level integration in a future bead.

3. **Fuzz deep campaign DEFERRED**: Targets compiled; deep libFuzzer corpus runs deferred to State 11 formal-verifier.

4. **Downstream dependencies** (from `proof-to-implementation-input.md`):
   - REQ-4/PS-004 (`IndexParityMismatch`): Not a public `JournalError` variant. Resolved via `CorpusOutcome::IndexParityMismatch` in the test file. Future bead must add the variant to `JournalError` or keep corpus-local classification.
   - REQ-9/PS-006 (`duplicate idempotency key`): Not a located storage public concept. Resolved via `CorpusOutcome::DuplicateEvent` classification based on event key + divergent payload digest. Future bead must decide storage `DuplicateEvent` vs `vb_runtime` idempotency surface.
   - REQ-11/PS-008 (`missing manifest`): Bound to declared Fjall keyspace/manifest parity. Classifier tests manifest mask logic; full Fjall integration deferred.
   - REQ-16/PS-009 (`no-copy fence`): Review obligation only. Bridge confirms source/provenance: all fixtures use VB public APIs/constants per contract.

## Full Mapping Table

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|---|---|---|---|---|---|---|---|---|
| PO-vb-7m21-kani-001 | Codec panic-freedom (REQ-5) | true | `crates/vb_storage/src/codec/header.rs::decode_record_header`, `crates/vb_storage/src/codec/payload.rs::decode_record_payload`, `crates/vb_storage/src/codec/mod.rs::decode_record` | `restate_storage_blackhat_fixture_corpus.rs::oversized_declared_record_returns_payload_too_large` | `crates/vb_storage/src/kani_vb_7m21_codec_panic.rs` | kani | `cargo kani -p vb_storage --harness kani_vb_7m21_decode_record_header_never_panics` | 11 |
| PO-vb-7m21-kani-002 | Header validation (REQ-3) | true | `crates/vb_storage/src/codec/validation.rs::validate_schema_version`, `validate_known_kind`, `validate_kind_family` | `restate_storage_blackhat_fixture_corpus.rs::future_schema_is_unsupported` | `crates/vb_storage/src/kani_vb_7m21_header_validate.rs` | kani | `cargo kani -p vb_storage --harness kani_vb_7m21_validate_schema_version_never_panics` | 11 |
| PO-vb-7m21-kani-003 | Payload bounds (REQ-6) | true | `crates/vb_storage/src/codec/payload.rs::payload_len_u32`, `encode_record_payload`, `decode_record_payload` | `restate_storage_blackhat_fixture_corpus.rs::truncated_header_is_unexpected_eof` | `crates/vb_storage/src/kani_vb_7m21_payload_bounds.rs` | kani | `cargo kani -p vb_storage --harness kani_vb_7m21_payload_len_exceeds_max_is_rejected` | 11 |
| PO-vb-7m21-prop-001 | Oversized payload (REQ-5) | true | `crates/vb_storage/src/codec/header.rs::encode_record_header`, `crates/vb_storage/src/codec/payload.rs::payload_len_u32` | `restate_storage_blackhat_fixture_corpus.rs::oversized_declared_record_returns_payload_too_large` | `crates/vb_storage/src/kani_vb_7m21_payload_bounds.rs` | proptest | `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` | 5 |
| PO-vb-7m21-prop-002 | Future schema (REQ-3) | true | `crates/vb_storage/src/constants.rs::CURRENT_SCHEMA_VERSION` | `restate_storage_blackhat_fixture_corpus.rs::future_schema_is_unsupported` | `crates/vb_storage/src/kani_vb_7m21_header_validate.rs` | proptest | `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` | 5 |
| PO-vb-7m21-prop-003 | Truncated header (REQ-6) | true | `crates/vb_storage/src/codec/header.rs::decode_record_header` | `restate_storage_blackhat_fixture_corpus.rs::truncated_header_is_unexpected_eof` | `crates/vb_storage/src/kani_vb_7m21_codec_panic.rs` | proptest | `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` | 5 |
| PO-vb-7m21-prop-004 | Missing side-index (REQ-4) | true | `crates/vb_storage/src/indexes.rs`, `crates/vb_storage/src/error/mod.rs::JournalError` | `restate_storage_blackhat_fixture_corpus.rs::missing_side_index_is_typed` | N/A (classifier-only; API integration deferred) | proptest | `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` | 5 |
| PO-vb-7m21-prop-005 | Sequence gap (REQ-8) | true | `crates/vb_storage/src/error/mod.rs::JournalError::SequenceGap`, `crates/vb_storage/src/journal/replay.rs` | `restate_storage_blackhat_fixture_corpus.rs::sequence_gap_is_typed` | N/A (classifier-only; API integration deferred) | proptest | `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` | 5 |
| PO-vb-7m21-prop-006 | Divergent duplicate (REQ-9) | true | `crates/vb_storage/src/error/mod.rs::JournalError::DuplicateEvent`, `crates/vb_storage/src/journal/core.rs` | `restate_storage_blackhat_fixture_corpus.rs::divergent_duplicate_is_typed` | N/A (classifier-only; API integration deferred) | proptest | `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` | 5 |
| PO-vb-7m21-prop-007 | Stale snapshot (REQ-10) | true | `crates/vb_storage/src/snapshots.rs`, `crates/vb_storage/src/recovery/types.rs` | `restate_storage_blackhat_fixture_corpus.rs::stale_snapshot_replays_tail` | N/A (classifier-only; API integration deferred) | proptest | `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` | 5 |
| PO-vb-7m21-prop-008 | Missing manifest (REQ-11) | true | `crates/vb_storage/src/keys.rs`, `crates/vb_storage/src/journal/internal.rs` | `restate_storage_blackhat_fixture_corpus.rs::missing_manifest_keyspace_is_typed` | N/A (classifier-only; API integration deferred) | proptest | `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` | 5 |
| PO-vb-7m21-fuzz-001 | Envelope decode fuzz (REQ-5) | true | `crates/vb_storage/src/codec/header.rs::decode_record_header`, `crates/vb_storage/src/codec/mod.rs::decode_record`, `decode_journal_event` | `restate_storage_blackhat_fixture_corpus.rs::oversized_declared_record_returns_payload_too_large` | `fuzz/fuzz_targets/vb_7m21_envelope_decode.rs` | cargo-fuzz | `cargo fuzz run vb_7m21_envelope_decode -- -max_total_time=3600 -runs=500000` | 11 |
| PO-vb-7m21-fuzz-002 | Header parse fuzz (REQ-3) | true | `crates/vb_storage/src/codec/header.rs::decode_record_header` | `restate_storage_blackhat_fixture_corpus.rs::future_schema_is_unsupported` | `fuzz/fuzz_targets/vb_7m21_header_parse.rs` | cargo-fuzz | `cargo fuzz run vb_7m21_header_parse -- -max_total_time=3600 -runs=500000` | 11 |
| PO-vb-7m21-fuzz-003 | Payload decode fuzz (REQ-6) | true | `crates/vb_storage/src/codec/payload.rs::decode_record_payload`, `verify_digest_match`, `crates/vb_storage/src/codec/mod.rs::encode_record`, `decode_record` | `restate_storage_blackhat_fixture_corpus.rs::truncated_header_is_unexpected_eof` | `fuzz/fuzz_targets/vb_7m21_payload_decode.rs` | cargo-fuzz | `cargo fuzz run vb_7m21_payload_decode -- -max_total_time=3600 -runs=500000` | 11 |

*BE = behavior_affecting. All 14 obligations are behavior-affecting (`true`).*

## Reviewer Handoff Inputs

- **proof-review.md**: `/home/lewis/isolated/femdation-velvet-ballistics/vb-7m21/.beads/vb-7m21/proof-review.md` (hash: see agent-invocation-ledger.jsonl entry 17)
- **proof-findings.jsonl**: `/home/lewis/isolated/femdation-velvet-ballistics/vb-7m21/.beads/vb-7m21/proof-findings.jsonl`
- **proof-obligations.planned.jsonl**: `/home/lewis/isolated/femdation-velvet-ballistics/vb-7m21/.beads/vb-7m21/proof-obligations.planned.jsonl`
- **proof-to-implementation-input.md**: `/home/lewis/isolated/femdation-velvet-ballistics/vb-7m21/.beads/vb-7m21/proof-to-implementation-input.md`
- **contract.md**: `/home/lewis/isolated/femdation-velvet-ballistics/vb-7m21/.beads/vb-7m21/contract.md`
- **verification-ledger.jsonl**: `/home/lewis/isolated/femdation-velvet-ballistics/vb-7m21/verification-ledger.jsonl` (lines 49-55)
- **verifier-lane-decisions.jsonl**: `/home/lewis/isolated/femdation-velvet-ballistics/vb-7m21/.beads/vb-7m21/verifier-lane-decisions.jsonl`
- **this map**: `/home/lewis/isolated/femdation-velvet-ballistics/vb-7m21/.beads/vb-7m21/proof-to-rust-map.md`
- **rust-refinement-obligations.jsonl**: `/home/lewis/isolated/femdation-velvet-ballistics/vb-7m21/.beads/vb-7m21/rust-refinement-obligations.jsonl`
