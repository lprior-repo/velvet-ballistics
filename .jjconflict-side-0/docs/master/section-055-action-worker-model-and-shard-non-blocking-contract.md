---
section: 55
title: "Action Worker Model and Shard Non-Blocking Contract"
parent: velvet-ballistics-MASTER.md
---

## 55. Action Worker Model and Shard Non-Blocking Contract


- `DeterministicPure` actions may execute inline only if bounded and non-blocking.
- External actions must not block the shard loop.
- External action dispatch uses explicit `Suspended` ticket path.
- No per-action thread spawning unless through a bounded worker pool.
- Worker pool size is configured and bounded.
- Queue full returns `ActionError::QueueFull`.
- Current implementation: `execute_do_without_contract` always creates an `ActionTicket` and returns `AwaitingAction`. The shard suspends the run. External completion arrives via `ShardCommand::ActionCompleted`.

---
