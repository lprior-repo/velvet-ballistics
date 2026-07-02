---
section: 72
title: "Execution Attempt Tracking"
parent: velvet-ballistics-MASTER.md
---

## 72. Execution Attempt Tracking


When a run fails and is retried, the engine must reject stale events from previous execution attempts. This prevents split-brain between overlapping retries.

### Contract

1. Every run attempt gets a monotonically increasing `attempt: u16` counter.
2. `ActionTicket` carries the `attempt` number.
3. On retry, the attempt counter increments. Any `ActionCompleted`/`ActionFailed` event carrying a stale attempt number is rejected with `StaleAttempt { expected, found }`.
4. Journal events are tagged with the attempt number.
5. Recovery replays events for the latest attempt only. Events from earlier attempts are ignored.
6. The attempt counter is journaled as part of `RunAccepted` and persists across crashes.

This provides invocation execution attempt tracking for single-server synchronous execution.

---
