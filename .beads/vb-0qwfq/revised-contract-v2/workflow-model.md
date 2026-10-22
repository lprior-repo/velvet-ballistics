# Workflow Model — Run Sequence Number Allocation

## Sequence Allocation Flow

```
[Shard] Allocate EventSeq for each event
        │
        ├─► [Pre-flight] Validate workflow/policy/contracts
        │       (before any durable mutation)
        │
        ├─► [Stage] RunSubmission in JournalWriteBatch
        │       (source, artifact, inputs, header)
        │
        ├─► [Stage] Planned events with allocated seq
        │       (RunAccepted at seq=0, RunAdmission at seq=1, ...)
        │
        └─► [Commit] batch.strict().commit()
                (SyncAll durability barrier)
                        │
                        ├─► Success: return Ok(allocated_next_seq)
                        │
                        └─► Failure: retry with same allocated seq
                                (idempotent; no duplicate detection)
```

## Critical Ordering

1. **Validation before mutation**: All pre-flight checks (workflow validity, policy binding, contract verification) MUST occur before any durable state is written. A rejected submission must not persist any events or envelope.

2. **Envelope before events**: The prepared envelope (source, artifact, inputs, header) is committed in the same batch as the initial acceptance/admission events. No events appear without their envelope.

3. **Sequence allocation is caller's responsibility**: Storage does not issue sequences; it validates uniqueness via `(run_id, seq)` key collision detection.

## Per-Run Independence

Each run has an independent sequence counter. The first event in every run is `EventSeq::new(0)`. There is no global sequence or cross-run ordering guarantee from the storage layer.

## Retry Semantics

If `commit()` fails after events are staged but before returning:
- The batch is rolled back (Fjall `OwnedWriteBatch` semantics).
- Events are NOT visible (not staged to memtable).
- A retry with the same `(run, seq)` values succeeds cleanly.
- No `DuplicateEvent` error from a prior failed attempt.

If `commit()` succeeds but `sync()` fails later:
- Events are visible in memtable but may not survive power loss.
- Caller must call `persist_strict()` separately or use `append_strict`.