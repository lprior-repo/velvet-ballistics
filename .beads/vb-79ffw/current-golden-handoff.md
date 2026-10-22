# Golden Consumer Test Handoff (vb-79ffw)

## Status
All 5 legacy wire decoder tests pass with exact error variant matching.

## Test File
`crates/workspace_tests/tests/legacy_wire_decoder.rs`

## Golden Fixtures
- 3 real records from commit 423d8ae8b667d82b4d36e57d29d7ef2aede08bdf
- Event kind: ActionScheduledTicket only (kind 34)
- Cases: small-canonical (122 bytes), high-values (135 bytes), max-attempt-capacity (128 bytes)
- Each fixture verified by source SHA, length, and BLAKE3 manifest

## Exact Rejection Type
All 3 old golden records rejected with:
`JournalError::PostcardDecodeFailed(postcard::Error::DeserializeBadVarint)`

Root cause: 7-field old layout vs 8-field current layout causes varint decode to fail when current decoder reads the payload.

## Test Results
```
running 5 tests
test legacy_golden_provenance_and_limitations ... ok
test legacy_golden_fixtures_have_valid_envelopes ... ok
test legacy_high_values_rejected_by_current_decoder ... ok
test legacy_max_attempt_capacity_rejected_by_current_decoder ... ok
test legacy_small_canonical_rejected_by_current_decoder ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Limitations
- Does not cover other JournalEvent variants
- Does not cover pre-423d8ae8 layouts
- Fail-closed behavior established for these 3 specific cases only
- Universal legacy compatibility not claimed

## Cleanup
- Removed rejected draft `crates/workspace_tests/tests/vb_79ffw_ticket_authority_probe.rs`
- Removed corresponding [[test]] entry from Cargo.toml
- Runtime callback probe left for LOCAL5090 worker in separate workspace