# vb-h7rex Binary Compatibility V4 Handoff

## Summary

Focused repair of version-aware checkpoint decoder tests and prose. Production version-aware decoder is unchanged and already wired to Runtime recovery. This iteration corrects misleading test names, comments, and constructs that claimed historical checkpoint migration or compatibility where none exists.

## Changed Files

1. `crates/vb_runtime/src/run_checkpoint.rs` — Corrected decoder doc comment (VARINT encoding, not "first two bytes")
2. `crates/vb_runtime/src/run_checkpoint_tests.rs` — Rewrote test suite for binary compatibility verification

## Production Changes

**Doc comment correction only** in `decode_checkpoint_payload`:
- Old: "Peeks at the first two bytes to determine the checkpoint format version"
- New: "Reads the VARINT-encoded version field first to determine the checkpoint format version"

Postcard encodes u16 as VARINT, not fixed little-endian two bytes. The doc comment was misleading.

## Test Changes

### Removed unchecked indexing
All tests now use literal encoded arrays (`let payload = [1, 0, 0, 0]`) instead of unchecked `payload[0] = 1` writes. Literal arrays have compile-time size checks.

### Renamed and re-described version 1 test
- Old: `versioned_decoder_rejects_historical_version_1_payload`
- New: `versioned_decoder_rejects_version_1_header_before_layout_decode`
- Old description: "Construct a minimal payload with version 1 bytes (little-endian u16)"
- New description: "Construct a payload with VARINT-encoded version 1 followed by garbage bytes. The version-aware decoder must reject the legacy version header before attempting to decode the checkpoint layout. This is an explicit version boundary test, not a historical checkpoint migration or compatibility test."

### Fixed truncated payload test
- Old: Used single byte `[0]` (VARINT version 0), claimed "truncated"
- New: `versioned_decoder_rejects_truncated_valid_checkpoint` constructs a real captured version 2 checkpoint, serializes it, verifies full decode success, then truncates the valid bytes by one and asserts `DecodeError`

### Added empty payload test
- `versioned_decoder_rejects_empty_payload` — asserts `UnsupportedVersion` for empty slice

### Added truncated VARINT version test
- `versioned_decoder_rejects_truncated_varint_version` — byte 255 (VARINT continuation) with no follow-up byte; asserts `UnsupportedVersion`

### Fixed malformed payload test
- Uses literal 64-byte array with VARINT version 2 header and all-zero garbage body
- Removed unchecked `payload[1] = 0` write

### Roundtrip test already correct
- `versioned_checkpoint_roundtrips_version_2_through_decoder` already asserts full checkpoint equality (`assert_eq!(decoded, checkpoint)`)

### Admission test naming already correct
- `versioned_decoder_preserves_pinned_admission_through_roundtrip` tests admission preservation, not checkpoint compatibility

## Verification Commands

```bash
cargo test -p vb_runtime checkpoint
# Result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 1967 filtered out

cargo clippy -p vb_runtime --lib -- -D warnings
# Result: ok. 0 warnings.
```

Evidence: `.beads/vb-h7rex/evidence/BINARY_V4_01.*` and `BINARY_V4_02.*`

## Remaining Public Runtime/Fjall Recovery Gap

The version-aware decoder (`decode_checkpoint_payload`) is tested and wired to Runtime's `recover_retry`. However, **public Fjall crash recovery remains unverified** for the new policy_pin field in RunCheckpointV1. Specifically:

1. Fjall's `recovery/full/admission.rs::validate_recovered_admission_evidence` reads the policy_pin from the recovered RunAdmission and validates it against the accepted artifact's policy_digest
2. This validation path is not exercised by the current test suite
3. The child contract explicitly permits "explicit version1 rejection" as the boundary, which is now correctly tested
4. Full Fjall crash recovery with the new binary layout requires a separate end-to-end test in `crates/workspace_tests/` that exercises the Runtime→Fjall handoff

This gap is documented in the bead contract and remains open for future work.

## No Cargo Changes

No dependency or configuration changes. Production version guard unchanged. No lint allowances added.

## Bead Status

Beacon NOT closed. Ready for coordinator acceptance.
