# Boundary Map — Run Sequence Number Allocation

## Layer Boundaries

```
┌─────────────────────────────────────────────────────────┐
│  Runtime Layer (crates/vb_runtime)                      │
│  - admission.rs: admit_artifact_run_with_certificate_floor│
│  - shard/lifecycle/chunk_002_parts/chunk_002_drive_core.rs│
│  - journal/chunk_001.rs: RuntimeJournal trait           │
└─────────────────────────────────────────────────────────┘
                           │ calls
┌─────────────────────────────────────────────────────────┐
│  Storage Layer (crates/vb_storage)                      │
│  - journal/core.rs: FjallJournal                        │
│  - journal/append.rs: append_strict, append_strict_batch│
│  - batch/types.rs, commit.rs: JournalWriteBatch         │
│  - inputs/submission.rs: RunSubmission                  │
│  - admission/verification.rs: verify_accepted_artifact  │
└─────────────────────────────────────────────────────────┘
                           │ calls
┌─────────────────────────────────────────────────────────┐
│  Fjall Layer (crates/fjall)                             │
│  - Database, OwnedWriteBatch, Keyspace                  │
│  - PersistMode::SyncAll (fsync)                         │
│  - Process lock file                                    │
└─────────────────────────────────────────────────────────┘
                           │ calls
┌─────────────────────────────────────────────────────────┐
│  OS / Disk                                              │
│  - fdatasync / fsync system calls                       │
│  - Page cache                                           │
└─────────────────────────────────────────────────────────┘
```

## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Caller ↔ Runtime | Caller supplies run_id, workflow, capabilities. Runtime validates admission. |
| Runtime ↔ Storage | Runtime calls storage APIs. Storage validates event structure and key uniqueness. |
| Storage ↔ Fjall | Storage uses Fjall batch API. Fjall provides ACID within a single commit. |
| Fjall ↔ OS | Fjall uses fdatasync for SyncAll. OS page cache is the durability risk. |

## Concurrency Boundary

- **Single-process model**: `ProcessLock` ensures only one journal instance per path.
- **write_lock**: Only prevents concurrent unfsynced appends via poison detection. NOT a general serialization lock.
- **Shard-local allocation**: Each shard allocates sequences independently. No cross-shard coordination.
- **Fjall-level safety**: Fjall's WAL provides crash consistency; concurrent readers see either old or new state, never partial.