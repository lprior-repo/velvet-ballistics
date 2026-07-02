---
section: 29
title: "Mandatory Function Surface: `vb_storage`"
parent: velvet-ballistics-MASTER.md
---

## 29. Mandatory Function Surface: `vb_storage`


**Source of truth:** `crates/vb_storage/src/`.

Required coverage areas:

| Area | Required public surface |
|------|------------------------|
| Database | `FjallJournal::open` (creates/opens Fjall with 9 keyspaces). |
| Write path | `append_journaled`, `append_strict`, `append_strict_batch`, `persist_strict`. Write lock for ordering. |
| Keyspaces | Per-keyspace put/get: `put_workflow_source`, `put_compiled_ir`, `put_run_header`, `put_snapshot`, `put_blob`, index puts. |
| Read path | `workflow_source`, `compiled_ir`, `run_header`, `run_headers`, `snapshot`, `blob`, `events_for_run`. |
| Record encoding | `encode_record`, `decode_record` (BLAKE3 digest + CRC32C envelope). |
| Key construction | `workflow_source_key`, `compiled_ir_key`, `run_header_key`, `run_event_key`, `run_snapshot_key`, `blob_key`, index key constructors. |
| Recovery | `recover_full_journal`, `recover_snapshot_plus_tail`, `recover_runtime_summary`, `recover_runtime_frame_seed`, `recover_all_incomplete_runs`, `replay_events`, `is_terminal_event`, `extract_terminal`. |
| Digest verification | `verify_digests`, `check_workflow_source_digest`, `check_compiled_ir_digest`. |
| Writer queue | `JournalWriterQueue` for bounded group commit. |

---
