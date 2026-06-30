# Workflow Model: vb-vzcuf

## Typestates

```text
Unbuilt
  -> Open { staged_bytes = 0, limit > 0 }
Open
  -> Open { staged_bytes' = staged_bytes + encoded_len } on accepted append
Open
  -> Open { staged_bytes unchanged } on non-aborting rejection
Open
  -> Aborted on durable duplicate or existing aborting integrity failure
Open
  -> Committed on successful commit
Aborted
  -> TerminalNoOp on commit
Committed
  -> Terminal
```

## Append Event Decision Table

| Step | Guard | Accepted transition | Rejected outcome | Mutates bytes? | Aborts? |
| --- | --- | --- | --- | --- | --- |
| 1 | Key can be built | continue | key `JournalError` | no | no |
| 2 | Durable event key absent | continue | `DuplicateEvent { run, seq }` | no | yes, existing behavior |
| 3 | `inner.len() < MAX_BATCH_COUNT` | continue | `QueueFull` | no | no |
| 4 | event payload <= per-record cap | continue | `PayloadTooLarge { len, max }` | no | no |
| 5 | encoded len representable | continue | accumulated accounting error | no | no |
| 6 | checked total succeeds | continue | accumulated accounting error | no | no |
| 7 | `attempted <= limit` | insert + set bytes to attempted | accumulated budget exceeded | no on reject | no |

## Exact-Fit Boundary

- Given `staged_bytes + encoded_len == limit`, append is accepted and `staged_bytes` becomes `limit`.
- Given `staged_bytes + encoded_len > limit`, append is rejected and `staged_bytes` remains unchanged.

## Commit Workflow

- If `Open`, commit writes all staged operations including accepted journal events.
- If `Aborted`, commit returns `Ok(())` without writing, preserving current abort semantics.
- Accumulated budget rejection must not cause `Aborted`; otherwise one oversized candidate would discard prior valid staged writes and change existing non-aborting `QueueFull` semantics.

## Same-Batch Duplicate Workflow

Current source comments say same-batch idempotent inserts are allowed and collapsed at commit time. The byte accounting contract needs one of two implementation choices:

1. **Attempt accounting**: every successful append attempt increases staged bytes even if the same key is later overwritten in `OwnedWriteBatch`. This is simple and conservative but may reject batches whose final durable bytes would fit.
2. **Distinct-key accounting**: repeated same-batch key replacement subtracts the previous value length and adds the new value length. This is precise but requires a map from staged event key to encoded length and verified subtraction.

Default contract recommendation: attempt accounting unless product requires precise final-byte accounting. Do not silently use `HashSet` membership without defining whether duplicates mutate byte totals.

## Terminal Outcomes

- `Accepted`: candidate event is staged and total remains within limit.
- `RejectedNonAborting`: candidate is not staged; previous valid batch remains committable.
- `RejectedAborting`: duplicate durable event preserves existing abort behavior.
- `Committed`: durable Fjall batch committed.
- `TerminalNoOp`: aborted batch commit no-ops.
