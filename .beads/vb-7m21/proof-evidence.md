# Proof Evidence — vb-7m21 State 5 Attempt 8

## Scope

This evidence records the creation of 14 lightweight proof artifacts for the storage blackhat corruption fixture corpus bead (vb-7m21). State 4 is APPROVED with reduced scope: 3 Kani harnesses, 8 proptest properties, 3 fuzz targets.

## Artifact Inventory

### Kani Harnesses (3 files, 12 proofs)

| File | Harness Count | Proof Claims |
|---|---|---|
| `crates/vb_storage/src/kani_vb_7m21_codec_panic.rs` | 3 | `decode_record_header`, `decode_record_payload`, `decode_record<JournalEvent>` never panic on arbitrary bytes |
| `crates/vb_storage/src/kani_vb_7m21_header_validate.rs` | 4 | `validate_schema_version`, `validate_known_kind`, `validate_kind_family` have complete error coverage |
| `crates/vb_storage/src/kani_vb_7m21_payload_bounds.rs` | 5 | `payload_len_u32`, `encode_record_payload`, `decode_record_payload` enforce size bounds |

### Proptest Properties (1 file, 8 properties)

File: `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs`

| Property | Contract Clause | Coverage |
|---|---|---|
| `oversized_declared_record_returns_payload_too_large` | REQ-5 | PayloadTooLarge rejection |
| `future_schema_is_unsupported` | REQ-3 | UnsupportedSchemaVersion classification |
| `truncated_header_is_unexpected_eof` | REQ-6 | UnexpectedEof for all truncation lengths |
| `missing_side_index_is_typed` | REQ-4 | IndexParityMismatch typing |
| `sequence_gap_is_typed` | REQ-8 | SequenceGap typing |
| `divergent_duplicate_is_typed` | REQ-9 | DuplicateEvent typing |
| `stale_snapshot_replays_tail` | REQ-10 | ReplayTail typing |
| `missing_manifest_keyspace_is_typed` | REQ-11 | MissingManifestKeyspace typing |

### Fuzz Targets (3 files)

| Target | Path | Scope |
|---|---|---|
| `vb_7m21_envelope_decode` | `fuzz/fuzz_targets/vb_7m21_envelope_decode.rs` | Full record decode with JournalEvent, multiple magics, semantic validation |
| `vb_7m21_header_parse` | `fuzz/fuzz_targets/vb_7m21_header_parse.rs` | Header-only decode across all 6 magic values + max_payload_len=0 edge |
| `vb_7m21_payload_decode` | `fuzz/fuzz_targets/vb_7m21_payload_decode.rs` | Payload corruption + digest mismatch + round-trip + direct verify_digest_match |

## Compilation Evidence

### vb_storage crate (dev)
```
$ cargo check -p vb_storage
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.07s
```

### Fuzz crate
```
$ cargo check -p velvet-ballistics-fuzz
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.91s
```

### Kani compilation
```
$ cargo kani -p vb_storage --harness kani_vb_7m21_validate_schema_version_never_panics --verbose
...
Compiling vb_storage v0.1.0
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.30s
Finished goto-cc in 0.04256664s
Finished goto-cc in 0.030299418s
Finished goto-instrument in 0.05056575s
Finished goto-instrument in 0.03228584s
Finished goto-instrument in 0.06229993s
```

### Kani verification — BLOCKED

Verification blocked by Kani 0.67 tooling limitation: unwinding recursion in error type drop implementations (`JournalError`, `TrimError`). This affects ALL Kani harnesses in `vb_storage`, not just the new ones. Mitigation: marked as PENDING_FORMAL_EXECUTION.

```
Unwinding recursion std::ptr::drop_in_place::<error::JournalError> iteration 607
```

## GOD RULE Compliance Evidence

### GOD RULE 1: No Hardcoded Kani Shapes

All 12 new Kani harnesses use `kani::any()` for inputs:
- Byte data: constructed with `kani::any()` in bounded loops
- Magic values: `kani::any()` or discrete sampler via `kani::any()`
- Schema versions, record kinds, payload lengths: `kani::any()`
- Max payload bounds: discrete sampler or `kani::any()`

No harness hardcodes a structural `WorkflowParts` or `RunFrame` with fixed dummy data.

## Trusted Base Boundaries

### kani::assume usage
- `kani_vb_7m21_payload_len_exceeds_max_is_rejected`: 4 assumes (bound ranges for tractability)
- `kani_vb_7m21_payload_len_within_bounds_is_accepted`: 3 assumes
- `kani_vb_7m21_encode_rejects_oversize`: 3 assumes
- `kani_vb_7m21_decode_rejects_payload_exceeding_max`: 1 assume (skip when encoding setup fails)

### Model bounds
- Byte array lengths bounded to [0, 128] or [0, 256] for Kani tractability
- Max payload bounds sampled from {0, 1, 60, 1024, u32::MAX}
- No unbounded `Nat` assumptions

### External dependencies assumed correct
- `postcard` (serialization/deserialization): trusted
- `crc32c` (checksum): trusted
- `blake3` (digest): trusted

## Remaining Gaps

1. Kani verification: compilation PASS, verification PENDING_FORMAL_EXECUTION (Kani unwinding recursion)
2. Proptest: 8 classifier properties — some use mock classification functions (`classify_index_parity`, `classify_sequence`, etc.) rather than calling storage APIs directly (test-first design)
3. Fuzz: compilation PASS, deep corpus runs deferred to formal-verifier state
4. Verus/Flux: 7 + 7 obligations remain planned but unwritten (out of reduced scope)
