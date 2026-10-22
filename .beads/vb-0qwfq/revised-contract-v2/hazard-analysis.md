# Hazard Analysis — Run Sequence Number Allocation

## Critical Hazards (Must Address)

| ID | Hazard | Severity | Current State |
|----|--------|----------|---------------|
| H1 | **Lost update via batch commit failure** | Critical | Handled: Fjall `OwnedWriteBatch` rolls back on failure; retry with same seq succeeds. |
| H2 | **Visible-but-not-durable event** | Critical | Handled for strict path: `append_strict` batch stages AND SyncAlls atomically. For journaled path, this is accepted risk. |
| H3 | **Duplicate event from retry** | High | Handled: `append_strict` pre-checks durable key AND same-batch duplicate detection. |
| H4 | **Sequence overflow at u64::MAX** | Low | Handled: `checked_add` with `JournalError::SequenceOverflow` on overflow. |
| H5 | **Envelope committed without events** | Medium | Handled: `RunSubmission.commit()` writes envelope + events in same batch. |
| H6 | **Events without envelope** | Medium | Caller responsibility: shard must submit envelope before appending events. |

## Subtle Hazards (Design Notes)

| ID | Hazard | Mitigation |
|----|--------|------------|
| H7 | **Caller supplies wrong seq** | Storage validates key uniqueness; caller must track last issued seq. |
| H8 | **Cross-run seq collision** | Not a hazard: seq is per-run, keys include run_id. |
| H9 | **write_lock provides false serialization sense** | Documented: lock is poison-detection only, not a write mutex. |
| H10 | **Fjall crash during batch commit** | Handled: Fjall WAL ensures atomic commit visibility. |

## Durability Window Analysis

| Path | Window | Risk |
|------|--------|------|
| `append_unfsynced` | Event visible → power loss | Event lost (memtable only) |
| `append_journaled` (queue) | Event queued → writer drains | Event lost if power loss before write |
| `append_strict` | None (atomic batch + SyncAll) | Event durable on return |
| `append_strict_batch` | None (single SyncAll for all) | All events durable on return |
| `commit_run_submission` | None (same batch for envelope + events) | Envelope and events durable atomically |

## Sequence Allocation Race

Two concurrent callers both allocate seq N for the same run:
- First caller commits successfully.
- Second caller's batch commit fails (Fjall key conflict or pre-check).
- Second caller retries with seq N+1.
- No data loss; one caller gets seq N, other gets seq N+1.

This is acceptable; sequence gaps are allowed (unused seq values are valid).