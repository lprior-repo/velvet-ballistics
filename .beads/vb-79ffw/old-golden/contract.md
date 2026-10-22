# Old Golden Export Contract (vb-79ffw)

## Purpose
Generate valid golden journal records using the ACTUAL OLD production serializer and prove the ACTUAL OLD public decoder accepts them. This proves fixture provenance only, not current compatibility.

## Scope
- Old workspace commit: production423d8ae8b667d82b4d36e57d29d7ef2aede08bdf
- Event type: `JournalEvent::ActionScheduledTicket`
- Serialization: Old `encode_record` with real CRC/BLAKE3, kind=34, magic=0x5642_4A45
- Deserialization: Old `parse_event` public API

## Old API Contract
- `ActionTicket` has 7 fields: run(RunId), step(StepIdx), seq(SeqNo), action(ActionId), attempt(u16), idempotency_key(u128), capacity(u16)
- `compute_action_idempotency_key(run, seq, action)` takes 3 args, returns u128
- `JournalEvent::ActionScheduledTicket` has 6 fields: run, seq, ticket, input, output, action_abi_digest
- RecordKind::ActionScheduledTicket = 34
- MAGIC_JOURNAL_EVENT = 0x5642_4A45

## Test Cases (3 minimum)
1. **small-canonical**: run=1, seq=1, step=0, action=0, attempt=1, capacity=3 (first-attempt small values)
2. **high-values**: run=65535, seq=65535, step=65535, action=65535, attempt=10, capacity=15 (high u16 values)
3. **max-attempt-capacity**: run=123, seq=456, step=7, action=42, attempt=65535, capacity=65535 (max attempt/capacity)

All idempotency keys computed via `compute_action_idempotency_key`. All run/seq/ticket routing consistent.

## Output
- Golden binary fixtures: `.beads/vb-79ffw/old-golden/<case-name>.bin`
- Manifest: `.beads/vb-79ffw/old-golden/manifest.json` with oldSHA, field values, record kind, length, digest
- Test: `crates/vb_storage/tests/old_ticket_golden_export.rs`

## Verification
Each binary must be accepted by old `parse_event` and match its intended complete event. Invalid cases are recorded but not exported.
