# Old Golden Export - Cleanup Results

**Source SHA**: 423d8ae8b667d82b4d36e57d29d7ef2aede08bdf (exact, no prefix)
**Test Command**: `cargo test -p vb_storage --test old_ticket_golden_export -- --nocapture`
**Exit Code**: 0 (success)
**Runner**: python3 /tmp/vb-run-gate.py 120 .beads/vb-79ffw/evidence/old-export-final

## Defects Fixed
1. OLD_SHA prefix "production" removed; now exact Git SHA
2. All `expect()` calls on filesystem I/O replaced with match/Result handling
3. Unchecked `cases.len()-1` arithmetic replaced with safe iteration
4. Test now requires all 3 named cases valid; any decode mismatch/error fails test
5. Exact valid count (3) and case names asserted

## Exported Golden Records
All three cases valid with matching bytes (old encode_record/parse_event equality verified):

| Case | Size (bytes) | Blake3 Digest |
|------|--------------|---------------|
| small-canonical | 122 | 7b3b30032c8174b8d0c29898c90895524d375c51610dada1792dc3fdb26ded54 |
| high-values | 135 | 49b5d3967957648a2b7cc54a6fdf8490fb9014e1445eed0a883ab4684f0ca64d |
| max-attempt-capacity | 128 | 23d289042d3e810f76af2963e908b54f07371f8e8890ed562134c8578c7a5ca6 |

## Golden Content
- Record kind: 34 (ActionScheduledTicket) for all cases
- Magic: MAGIC_JOURNAL_EVENT
- CRC and BLAKE3 computed correctly by old production encoder
- Decoded events match originals exactly (equality verified)
- Old public decoder (parse_event) accepts all three binary fixtures

## Evidence Files
- `.beads/vb-79ffw/old-golden/manifest.json` - updated with exact SHA and all 3 valid cases
- `.beads/vb-79ffw/old-golden/*.bin` - unchanged binary golden fixtures
- `.beads/vb-79ffw/evidence/old-export-final.{json,stdout,stderr}` - runner output