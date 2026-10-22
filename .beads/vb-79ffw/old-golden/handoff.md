# Old Golden Export Handoff (vb-79ffw)

## Completed Work
Generated valid golden journal records for `JournalEvent::ActionScheduledTicket` using the actual old production serializer at commit `production423d8ae8b667d82b4d36e57d29d7ef2aede08bdf`. Proved the actual old public decoder (`parse_event`) accepts all generated records.

## Test Location
`crates/vb_storage/tests/old_ticket_golden_export.rs`

## Fixtures Generated
All fixtures are in `.beads/vb-79ffw/old-golden/`:

| Case | File | Length | Description |
|------|------|--------|-------------|
| small-canonical | small-canonical.bin | 122 bytes | run=1, seq=1, step=0, action=0, attempt=1, capacity=3 |
| high-values | high-values.bin | 135 bytes | run=65535, seq=65535, step=65535, action=65535, attempt=10, capacity=15 |
| max-attempt-capacity | max-attempt-capacity.bin | 128 bytes | run=123, seq=456, step=7, action=42, attempt=65535, capacity=65535 |

## Verification
Each fixture:
1. Was serialized via the old `encode_record` API with magic=0x5642_4A45, kind=34, and real CRC/BLAKE3
2. Was parsed back via the old `parse_event` API
3. Was compared for exact equality with the original event
4. Had its envelope record kind verified as 34

All three cases passed verification and are labeled `valid: true` in the manifest.

## Commands to Reproduce
```bash
# Run the export test
cargo test -p vb_storage --test old_ticket_golden_export -- --nocapture

# Verify a specific fixture decodes correctly
cargo test -p vb_storage --test old_ticket_golden_export -- --nocapture 2>&1 | grep -E "\[OK\]|\[FAIL\]"
```

## Note for Current Decoder Testing
These fixtures prove provenance only (old serializer + old decoder). The current decoder step should be assigned separately and tested against these exact binary files. Do not infer current compatibility from old-decoder success.
