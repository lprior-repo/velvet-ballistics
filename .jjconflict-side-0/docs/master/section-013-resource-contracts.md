---
section: 13
title: "Resource Contracts"
parent: velvet-ballistics-MASTER.md
---

## 13. Resource Contracts


Every accepted workflow has a compiled `ResourceContract`:

```rust
pub struct ResourceContract {
    pub max_steps: u16,
    pub max_slots: u16,
    pub max_constants: u16,
    pub max_accessors: u16,
    pub max_expressions: u16,
    pub max_expr_stack: u8,
    pub max_step_budget_per_tick: u64,
    pub max_input_bytes: u32,
    pub max_output_bytes: u32,
    pub max_blob_bytes: u64,
    pub max_ipc_payload_bytes: u32,
    pub max_retry_attempts: u16,
    pub max_fanout: u16,
    pub max_collect_items: u32,
    pub max_queue_depth: u32,
    pub max_journal_batch_bytes: u32,
}
```

Compiler, runtime, IPC, and storage must reject or suspend before exceeding bounds. Silent truncation is forbidden.

Compile-time hard limits:

| Resource | Limit |
|----------|-------|
| YAML source bytes | 1 MiB |
| YAML parser depth | 64 |
| Language nesting depth | 8 |
| Steps | 1000 |
| Expressions | 4096 |
| Bytecode ops per expression | 256 |
| Expression stack depth | 64 |
| Constants | 8192 |
| Slots | `u16::MAX`, with a lower runtime default required |
| Accessors | 8192 |
| Path depth | 16 |

Runtime limits must be explicit per profile for active runs, ready queue depth, IPC frame bytes, action input bytes, action output bytes, step output bytes, result bytes, trace ring capacity, journal writer queue capacity, `for_each` item count and `at_once`, `together` branch count, `collect` pages/items/time, `repeat` attempts/time, retry attempts, maximum wait duration, and ask timeout.

---
