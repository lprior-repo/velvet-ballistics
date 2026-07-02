---
section: 23
title: "Durable Orchestration Runtime"
parent: velvet-ballistics-MASTER.md
---

## 23. Durable Orchestration Runtime

The runtime is not a generic VM. It is a history-first durable orchestrator.

Core durable concepts:

```text
HistoryEvent
Decision
Command
Completion
```

A run progresses as:

```text
load accepted artifact
replay durable history into frame projection
execute deterministic decision task over numeric IR
produce commands
persist semantic history events
only then dispatch side effects
persist completions
replay/continue
```

Run status:

```rust
pub enum RunStatus {
    Runnable,
    Blocked(Blocker),
    Finished,
    Failed,
    Cancelled,
}

pub enum Blocker {
    Action(ActionTicket),
    Timer(TimerId),
    Ask(AskId),
    Backpressure,
}
```

Per-step UI state is derived from history. Hot runtime does not rely on authoritative per-node terminal state for loop controllers.

---

