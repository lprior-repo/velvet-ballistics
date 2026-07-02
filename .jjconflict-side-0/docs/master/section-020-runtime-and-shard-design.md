---
section: 20
title: "Runtime and Shard Design"
parent: velvet-ballistics-MASTER.md
---

## 20. Runtime and Shard Design


Each shard owns:

- Bounded inbound command queue using `crossbeam_queue::ArrayQueue`.
- Run frame pool.
- Timer wheel for `wait`, `ask`, and retry delays.
- Action completion queue.
- Binary trace ring.
- Local counters.
- Fjall writer queue or handle according to durability profile.

No global `Arc<Mutex<RunState>>` is allowed. A run belongs to exactly one shard. Deterministic steps run synchronously inside the shard loop. Suspension boundaries are action, wait, ask, retry delay, fanout join, storage policy boundary, queue backpressure, cancellation, and shutdown.

Shard commands:

```rust
pub enum ShardCommand {
    Submit { run: RunId, workflow: WorkflowId },
    Resume { run: RunId },
    ActionCompleted { run: RunId, step: StepIdx },
    TimerFired { run: RunId },
    Cancel { run: RunId },
    Inspect { run: RunId, correlation: u64 },
    Shutdown,
}
```

---
