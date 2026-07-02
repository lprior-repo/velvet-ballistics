---
section: 61
title: "Fjall Storage Contract"
parent: velvet-ballistics-MASTER.md
---

## 61. Fjall Storage Contract


### Keyspace Profiles

| Profile | Keyspaces | Tuning |
|---------|-----------|--------|
| `Hot` | run_event, index_status, index_workflow, index_action, run_header | Bloom filter (10 bits/key), no KV separation |
| `Cold` | workflow_source, compiled_ir, run_snapshot | KV separation at 4096-byte threshold |
| `Blob` | blob | KV separation at 1024-byte threshold |

### Key Format

All keys use prefix byte + big-endian numeric IDs. Fjall keys remain big-endian for lexicographic ordering. Record body envelopes are little-endian. String keys are forbidden on hot paths.

### Write Path

Writes use `Mutex<()>` write lock for ordering. Durability profiles:
- `Volatile`: no Fjall writes during run (test/bench only).
- `Journaled`: bounded group commit via `JournalWriterQueue`.
- `Strict`: synchronous `persist(PersistMode::SyncAll)` after write.

### Recovery

- Full journal replay when no snapshot exists.
- Snapshot + tail journal replay when snapshot exists.
- `ActionReplayTracker` prevents non-idempotent re-execution during recovery.
- Recovery never reparses YAML — loads by digest.

### Atomic Cross-Keyspace Writes

`OwnedWriteBatch` provides single-WAL-fsync atomicity for multi-keyspace writes. Recommended for event + index co-writes. Current implementation uses individual inserts with write lock.

### Single-Writer Enforcement

Fjall v3 acquires an exclusive file lock per database. Only one process may open a database path at a time. Second process receives a typed error on open.

---
