---
section: 30
title: "Mandatory Function Surface: `vb_runtime`"
parent: velvet-ballistics-MASTER.md
---

## 30. Mandatory Function Surface: `vb_runtime`


**Source of truth:** `crates/vb_runtime/src/`.

Required coverage areas:

| Area | Required public surface |
|------|------------------------|
| Runtime | `Runtime::new`, `new_with_journal`, `submit_direct`, `submit_compiled`, `submit_compiled_with_inputs`, `cancel_run`, `inspect_run`, `tick_all`, `tick_shard`, `complete_action_with_output`, `fail_action`, `timer_fired`, `shutdown_graceful`, `drain_trace`, `take_inspect_response`, `counters_snapshot`. |
| Shard | `Shard::new`, `new_with_journal`, `enqueue`, `tick`, internal drive/action/timer handlers, `drain_for_shutdown`, `counters`, `snapshot_run`. |
| Engine | `execute_node_full` (all node kinds), `drive_deterministic_full`, `drive_with_actions`, `resume_action_outcome`. |
| Primitives | Per-primitive handlers in `primitives/`: for_each, together, collect, reduce, repeat, wait_ask. |
| Frame pool | `FramePool::take`, `release`, `available`, `capacity`. |
| Action dispatch | `ActionRegistry::register`, `dispatch`. |
| Trace | `TraceRing` with SPSC ring, drain, and history. |
| Journal adapters | `NoopRuntimeJournal`, `VolatileRuntimeJournal`, `StorageRuntimeJournal`, `QueuedStorageRuntimeJournal`. |

---
