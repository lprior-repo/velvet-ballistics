# Error Taxonomy: vb-vzcuf

## Error Families

| Family | Existing/proposed error | Meaning | Mutates? | Aborts? |
| --- | --- | --- | --- | --- |
| Key construction | existing key errors such as `KeyCapacity` | Cannot build event key | no | no |
| Durable duplicate | `DuplicateEvent { run, seq }` | Event already committed in durable keyspace | no | yes |
| Count capacity | `QueueFull` | Batch operation count reached `MAX_BATCH_COUNT` | no | no |
| Per-record payload | `PayloadTooLarge { len, max }` | One event payload exceeds `MAX_JOURNAL_EVENT_PAYLOAD_BYTES` before/while encoding | no | no |
| Accumulated bytes | proposed `JournalBatchBytesExceeded { attempted, limit }` | Encoded journal event batch total would exceed limit | no | no |
| Accumulated arithmetic | proposed same family or explicit overflow variant | Checked add/conversion failed | no | no |
| Fjall commit | `Fjall` | Storage backend failure during commit or lookup | no append mutation on lookup failure | no new behavior |

## Required Accumulated-Budget Error

The storage-visible error must support exact assertions:

- attempted total or enough fields to reconstruct attempted total;
- configured limit;
- no conflation with `QueueFull`;
- no conflation with `PayloadTooLarge`;
- stable display text identifying batch byte pressure, not queue count or single payload pressure.

Recommended shape:

```text
JournalError::JournalBatchBytesExceeded { attempted: u64, limit: u64 }
```

If overflow prevents an exact attempted total, recommended shape:

```text
JournalError::JournalBatchByteAccountingOverflow { staged: u64, event_len: u64, limit: u64 }
```

or a documented saturated `attempted` plus a separate source enum. The proof/test lanes need a non-panicking observable branch.

## C6 Error Separation Contract

To prove/test C6 error separation, downstream lanes must control unrelated guards:

- To observe accumulated byte rejection, use a fresh durable key, `inner.len() < MAX_BATCH_COUNT`, and an event whose individual payload is within `MAX_JOURNAL_EVENT_PAYLOAD_BYTES`.
- To observe `PayloadTooLarge`, use a large single payload with enough accumulated budget so batch budget is not the limiting guard.
- To observe `QueueFull`, fill count capacity while byte limit is large enough or candidate is otherwise controlled.
- To observe `DuplicateEvent`, precommit the exact event key; byte/count limits must not mask it.

## API Compatibility Notes

`JournalError` is already `#[non_exhaustive]`, allowing addition of a new variant without exhaustive matching compatibility guarantees. However, public behavior still changes and must be reflected in integration tests and release notes if this crate is consumed externally.
