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
max_steps: 10_000
max_slots: 1_024
max_constants: 65_535
max_accessors: 8_192
max_expressions: 4_096
max_expr_stack: 64
max_step_budget_per_tick: 10_000
max_transitions_per_tick: 10_000
max_input_bytes: 1 MiB
max_output_bytes: 256 KiB
max_blob_bytes: 16 MiB
max_ipc_payload_bytes: 1 MiB
max_retry_attempts: 3
max_fanout: 64
max_collect_items: 1_024
max_queue_depth: 1_024
max_journal_batch_bytes: 1 MiB
allows_secret_results: Deny
```

### Named Profiles

| Profile | Persistence | Allocation | Code path |
|---------|-------------|------------|-----------|
| `dev` | Volatile | On-demand | IR interpreter |
| `test` | Volatile + deterministic tracing | On-demand | IR interpreter |
| `turbo` | Journaled | Preallocated frames, bounded queues | IR interpreter |

`maxperf` is removed and is not a current runtime profile requirement.

---
