---
section: 56
title: "Runtime Profile Defaults"
parent: velvet-ballistics-MASTER.md
---

## 56. Runtime Profile Defaults


### ShardConfig Defaults

```text
command_queue_capacity: 1024
trace_capacity: 4096
step_budget_per_tick: 1000
max_active_runs: 1024
```

### ResourceContract::DEFAULT

```text
max_steps: 1_000
max_slots: 65_535 (u16::MAX)
max_constants: 65_535
max_accessors: 8_192
max_expressions: 4_096
max_expr_stack: 64
max_step_budget_per_tick: u64::MAX
max_input_bytes: 1 MiB
max_output_bytes: 1 MiB
max_blob_bytes: 16 MiB
max_ipc_payload_bytes: 1 MiB
max_retry_attempts: 65_535
max_fanout: 65_535
max_collect_items: 4_294_967_295 (u32::MAX)
max_queue_depth: 1_024
max_journal_batch_bytes: 1 MiB
```

### Named Profiles

| Profile | Persistence | Allocation | Code path |
|---------|-------------|------------|-----------|
| `dev` | Volatile | On-demand | IR interpreter |
| `test` | Volatile + deterministic tracing | On-demand | IR interpreter |
| `turbo` | Journaled | Preallocated frames, bounded queues | IR interpreter |

`maxperf` is removed and is not a current runtime profile requirement.

---
