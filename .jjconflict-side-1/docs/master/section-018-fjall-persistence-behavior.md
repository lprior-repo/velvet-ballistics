---
section: 18
title: "Fjall Persistence Behavior"
parent: velvet-ballistics-MASTER.md
---

## 18. Fjall Persistence Behavior


Fjall is required. Recovery from Fjall is a product requirement, not an optional persistence layer.

Keyspaces:

```text
workflow_source   immutable YAML source by digest
compiled_ir       compiled workflow IR by digest
run_header        run metadata and status
run_event         compact binary event journal
run_snapshot      compact binary run snapshots
blob              large input/output/action payload blobs
index_status      status/time indexes
index_workflow    workflow/run indexes
index_action      pending action indexes
```

Binary key format uses prefix bytes plus big-endian numeric IDs. String keys are forbidden on hot paths.

```text
[0x01][workflow_digest_32]                         -> workflow_source
[0x02][compiled_digest_32]                         -> compiled_ir
[0x10][run_id_u64_be]                              -> run_header
[0x11][run_id_u64_be][seq_u64_be]                  -> run_event
[0x12][run_id_u64_be][seq_u64_be]                  -> run_snapshot
[0x20][blob_digest_32]                             -> blob
[0x30][state_u8][timestamp_u64_be][run_id_u64_be]  -> index_status
[0x31][workflow_id_u32_be][run_id_u64_be]          -> index_workflow
[0x32][action_id_u16_be][run_id_u64_be][step_u16]  -> index_action
```

Durability profiles:

| Profile | Behavior |
|---------|----------|
| `volatile` | No Fjall writes during run; only valid for explicit benchmark/test mode; restart loses accepted volatile runs. |
| `journaled` | Accepted runs append compact events to a bounded Fjall writer queue with bounded group commit; acknowledgement may occur after queue admission according to policy. |
| `strict` | Critical records are synchronously persisted and flushed before acknowledgement; blocking is allowed only at strict durability boundaries. |

Persistence invariants:

1. Accepted run binds immutably to one compiled workflow digest.
2. Journal sequence numbers are monotonic per run.
3. Recovery replays snapshots plus tail journal or full journal deterministically.
4. Replay never re-executes external side effects unless the action ABI declares the operation idempotent and replay-safe.
5. Corrupt records fail with typed storage/replay errors.
6. Storage writes obey durability profile and bounded batch contracts.
7. Recovery never reparses YAML for existing runs; it loads compiled artifacts, snapshots, and journal records by digest.
8. Replay checks workflow source digest, compiled workflow digest, action ABI digest, and policy digest. Mismatch returns typed replay failure and must not silently continue.

Every binary file, IPC frame, compiled artifact, snapshot, and journal record uses this envelope before payload decode. Multi-byte envelope fields are little-endian. Fjall keys remain big-endian as specified above for lexicographic ordering; record bodies are little-endian through this envelope and Postcard payloads.

```text
offset  bytes  field
0       4      magic_u32
4       2      schema_version_u16
6       2      record_kind_u16
8       4      header_len_u32 = 60
12      4      payload_len_u32
16      8      sequence_u64
24      32     payload_digest_blake3_256
56      4      header_crc32c
60      N      postcard payload, where N == payload_len_u32
```

Magic values:

| Family | Magic u32 | ASCII |
|--------|-----------|-------|
| Compiled artifact | `0x56424952` | `VBIR` |
| Journal event | `0x56424A45` | `VBJE` |
| Snapshot | `0x5642534E` | `VBSN` |
| Blob record | `0x5642424C` | `VBBL` |
| IPC frame | `0x56424C54` | `VBLT` |
| Workflow source record | `0x56425352` | `VBSR` |
| Index record | `0x56424958` | `VBIX` |

Required `record_kind_u16` IDs:

| ID | Kind |
|----|------|
| 1 | `WorkflowSource` |
| 2 | `CompiledIr` |
| 3 | `RunHeader` |
| 10 | `RunAccepted` |
| 11 | `StepStarted` |
| 12 | `SlotWritten` |
| 13 | `ActionScheduled` |
| 14 | `ActionCompleted` |
| 15 | `ActionFailed` |
| 16 | `WaitScheduled` |
| 17 | `AskScheduled` |
| 18 | `AskAnswered` |
| 19 | `RetryScheduled` |
| 20 | `StepFailed` |
| 21 | `RunCancelled` |
| 22 | `RunFinished` |
| 23 | `RunFailed` |
| 24 | `RunAdmission` |
| 25 | `RunResumed` |
| 26 | `RunRetried` |
| 27 | `RunAnswered` |
| 28 | `RunKilled` |
| 29 | `AskTimedOut` |
| 30 | `Snapshot` |
| 40 | `Blob` |
| 50 | `IndexUpdate` |

The current v1 storage contract includes `AskTimedOut = 29` so ask timeout
replay is distinguishable from `AskAnswered = 18`. This repository is still at
workspace crate version `0.1.0`; the table above is the authoritative v1 wire
contract before a stable compatibility release, so `CURRENT_SCHEMA_VERSION`
remains `1`. Implementations using an older in-repo draft table that rejected
kind `29` are not considered compatible v1 decoders. After a stable v1 storage
compatibility release, adding or repurposing any `record_kind_u16` requires a
schema-version bump or an explicit named migration with evidence.

Decode order is mandatory: read 60-byte header, validate `magic_u32`, validate supported `schema_version_u16`, validate `record_kind_u16` is allowed for that family, validate `header_len_u32 == 60`, validate `payload_len_u32 <= ResourceContract.max_journal_batch_bytes` for journal batches or the configured family-specific maximum for compiled artifacts, snapshots, blobs, and IPC payloads, verify `header_crc32c` over bytes `0..56`, then read exactly `payload_len_u32` bytes, verify `payload_digest_blake3_256`, then Postcard-decode into the typed payload for the record kind. Payload allocation before length validation is forbidden.

Typed storage/decode errors must include `BadMagic { found: u32 }`, `UnsupportedSchemaVersion { version: u16 }`, `UnknownRecordKind { kind: u16 }`, `RecordKindFamilyMismatch { magic: u32, kind: u16 }`, `HeaderLengthMismatch { found: u32 }`, `PayloadTooLarge { len: u32, max: u32 }`, `HeaderChecksumMismatch`, `PayloadDigestMismatch`, `UnexpectedEof`, `PostcardDecodeFailed`, and `MigrationRequired { from: u16, to: u16 }`. Schema version migration is never implicit: an older supported version must pass through a named migration function that emits the current version and records migration evidence; unsupported versions fail with `MigrationRequired` or `UnsupportedSchemaVersion` and must not be replayed.

---
