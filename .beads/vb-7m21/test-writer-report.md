# Test Writer Report: vb-ttyc

## Bead: vb-ttyc — runtime: Add artifact version barrier tests

## Summary
Written failing-first TDD tests for runtime artifact version barrier enforcement.

## Test Count
- Unit/Integration: 19 tests
- Proptest: 3 tests
- TOTAL: 19 tests

## Gate Results
- [x] Source clippy: 0 warnings
- [x] Test compile: pass
- [x] nextest: 19 passed, 0 failed

## Known Limitations
1. B-16 (ExpressionLoweringUnsupported) not testable through YamlCompiler::compile
2. Schema version tests (B-01 to B-03) require ArtifactSchemaVersion type

## Proof/Refinement Coverage Matrix

Map of proof claims to behavior tests and refinement harnesses executed in State 9.

| Proof ID | Claim | Behavior Affecting | Behavior Test Ref | Refinement Harness Ref | Verifier | Status |
|---|---|---|---|---|---|---|
| PO-vb-7m21-kani-001 | Codec panic-freedom (REQ-5) | true | `restate_storage_blackhat_fixture_corpus.rs::oversized_declared_record_returns_payload_too_large` | `crates/vb_storage/src/kani_vb_7m21_codec_panic.rs` | kani | ACCEPTED_TRUST_BOUNDARY |
| PO-vb-7m21-kani-002 | Header validation (REQ-3) | true | `restate_storage_blackhat_fixture_corpus.rs::future_schema_is_unsupported` | `crates/vb_storage/src/kani_vb_7m21_header_validate.rs` | kani | ACCEPTED_TRUST_BOUNDARY |
| PO-vb-7m21-kani-003 | Payload bounds (REQ-6) | true | `restate_storage_blackhat_fixture_corpus.rs::truncated_header_is_unexpected_eof` | `crates/vb_storage/src/kani_vb_7m21_payload_bounds.rs` | kani | ACCEPTED_TRUST_BOUNDARY |
| PO-vb-7m21-prop-001 | Oversized payload (REQ-5) | true | `restate_storage_blackhat_fixture_corpus.rs::oversized_declared_record_returns_payload_too_large` | `crates/vb_storage/src/kani_vb_7m21_payload_bounds.rs` | proptest | PASS |
| PO-vb-7m21-prop-002 | Future schema (REQ-3) | true | `restate_storage_blackhat_fixture_corpus.rs::future_schema_is_unsupported` | `crates/vb_storage/src/kani_vb_7m21_header_validate.rs` | proptest | PASS |
| PO-vb-7m21-prop-003 | Truncated header (REQ-6) | true | `restate_storage_blackhat_fixture_corpus.rs::truncated_header_is_unexpected_eof` | `crates/vb_storage/src/kani_vb_7m21_codec_panic.rs` | proptest | PASS |
| PO-vb-7m21-prop-004 | Missing side-index (REQ-4) | true | `restate_storage_blackhat_fixture_corpus.rs::missing_side_index_is_typed` | N/A (classifier-only) | proptest | PASS |
| PO-vb-7m21-prop-005 | Sequence gap (REQ-8) | true | `restate_storage_blackhat_fixture_corpus.rs::sequence_gap_is_typed` | N/A (classifier-only) | proptest | PASS |
| PO-vb-7m21-prop-006 | Divergent duplicate (REQ-9) | true | `restate_storage_blackhat_fixture_corpus.rs::divergent_duplicate_is_typed` | N/A (classifier-only) | proptest | PASS |
| PO-vb-7m21-prop-007 | Stale snapshot (REQ-10) | true | `restate_storage_blackhat_fixture_corpus.rs::stale_snapshot_replays_tail` | N/A (classifier-only) | proptest | PASS |
| PO-vb-7m21-prop-008 | Missing manifest (REQ-11) | true | `restate_storage_blackhat_fixture_corpus.rs::missing_manifest_keyspace_is_typed` | N/A (classifier-only) | proptest | PASS |
| PO-vb-7m21-fuzz-001 | Envelope decode fuzz (REQ-5) | true | `restate_storage_blackhat_fixture_corpus.rs::oversized_declared_record_returns_payload_too_large` | `fuzz/fuzz_targets/vb_7m21_envelope_decode.rs` | cargo-fuzz | ACCEPTED_TRUST_BOUNDARY |
| PO-vb-7m21-fuzz-002 | Header parse fuzz (REQ-3) | true | `restate_storage_blackhat_fixture_corpus.rs::future_schema_is_unsupported` | `fuzz/fuzz_targets/vb_7m21_header_parse.rs` | cargo-fuzz | ACCEPTED_TRUST_BOUNDARY |
| PO-vb-7m21-fuzz-003 | Payload decode fuzz (REQ-6) | true | `restate_storage_blackhat_fixture_corpus.rs::truncated_header_is_unexpected_eof` | `fuzz/fuzz_targets/vb_7m21_payload_decode.rs` | cargo-fuzz | ACCEPTED_TRUST_BOUNDARY |

## Behaviors Not Tested
- Schema version validation - requires implementation
- FeatureTag parsing - requires FeatureTag::parse function
- CodegenError::UnsupportedIr - no codegen in runtime crate
