# vb-79ffw Code Review

## Primary Task: Legacy Wire Decoder Test

**Status: PASSED**

Executed real legacy wire public decoder test. Old production commit 423d8ae8 defined 7-field `ActionTicket`:
- run: RunId
- step: StepIdx
- seq: SeqNo
- action: ActionId
- attempt: u16
- idempotency_key: u128
- capacity: u16

Current code added `run_attempt: u16` at field position 5 (between `action` and `attempt`), making it an 8-field struct. `CURRENT_SCHEMA_VERSION` remains 1 (unchanged).

**Test: `crates/vb_storage/tests/legacy_wire_decoder.rs`**

Constructed old 7-field `LegacyActionTicket` fixture struct with exact old layout copied read-only from git show 423d8ae8. Serialized with Postcard, wrapped in valid schema-1 envelope with correct magic (0x56424A45), kind (ActionScheduledTicket), sequence, CRC, and BLAKE3 payload digest.

**Result: FAIL-CLOSED**

Current public decoder (`parse_event`) rejected the legacy bytes with:
```
PostcardDecodeFailed(SerdeDeCustom)
```

This is the correct behavior: the decoder recognized the old payload structure as incompatible and returned an error rather than silently misinterpreting the bytes. No fake/invalid digest was used to force rejection — the envelope had a real BLAKE3 digest computed by `encode_record`.

**Migration Limit:**
- Schema-1 journal events with old 7-field ActionTicket layout are NOT compatible with the current decoder.
- Old serialized events will fail to decode with a Postcard deserialization error.
- Any migration path must account for this incompatibility (e.g., data migration or dual-decode path).

## Secondary Task: Strengthened Stale Retry Probe

**Status: PASSED (6/6 tests)**

Strengthened the existing stale retry probe test with full serialized JournalEvent vector capture before/after stale rejection.

**Test: `crates/vb_runtime/tests/vb_79ffw_ticket_authority_probe.rs`**

Key improvements:
1. Captured full event vector before stale rejection (14 events)
2. Captured event vector after stale rejection tick (14 events — no mutation)
3. Captured event vector after current ticket completion (18 events — 4 new)
4. Verified exact tick rejection: "invalid action completion"
5. Verified RunFinished state reached after current ticket completion

**Test Output:**
```
Events before stale rejection: 14
Tick rejected stale failure: invalid action completion
Events after stale rejection: 14
Events after completion: 18
```

All 6 probe tests pass:
- stale_completion_rejected_after_two_retries
- future_run_attempt_ticket_rejected
- zero_run_attempt_ticket_rejected
- cold_recovery_preserves_run_attempt
- idempotency_key_differs_across_run_attempts
- ticket_key_valid_after_serialization