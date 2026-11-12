---
section: 49
title: "Journal Event Payload Schemas and Crash-Consistency Ordering"
parent: velvet-ballistics-MASTER.md
---

## 49. Journal Event Payload Schemas and Crash-Consistency Ordering


### TraceEvent Variants (hot ring)

```text
StepStarted   { run: RunId, step: StepIdx }
StepEnded     { run: RunId, step: StepIdx }
SlotWritten   { run: RunId, slot: SlotIdx }
ActionScheduled { run: RunId, step: StepIdx }
ActionCompleted { run: RunId, step: StepIdx }
ActionFailed  { run: RunId, step: StepIdx, code: ActionFailureCode }
AskAnswered   { run: RunId, step: StepIdx, slot: SlotIdx }
RunSubmitted  { run: RunId }
RunFinished   { run: RunId }
RunFailed     { run: RunId }
RunCancelled  { run: RunId }
```

### Runtime Journal Events (durable)

```text
RunSubmitted     { run: RunId, workflow: WorkflowDigest }
SlotWritten      { run: RunId, slot: SlotIdx }
ActionScheduled  { run: RunId, step: StepIdx, action: ActionId }
ActionCompleted  { run: RunId, step: StepIdx, action: ActionId }
WaitScheduled    { run: RunId, step: StepIdx }
AskScheduled     { run: RunId, step: StepIdx }
WaitResolved     { run: RunId, step: StepIdx }
AskAnswered      { run: RunId, step: StepIdx, slot: SlotIdx }
RunFinished      { run: RunId, result: SlotIdx }
RunFailed        { run: RunId }
RunCancelled     { run: RunId }
```

### Ordering Invariants

1. `RunSubmitted` before any `StepStarted` or `SlotWritten`.
2. `StepStarted` before `SlotWritten` for that step.
3. `ActionScheduled` before external action dispatch.
4. `ActionCompleted` before frame mutation on resume.
5. `RunFinished` after final result slot is persisted.
6. Timer resume: step marked `Running` then `Succeeded` before continuing drive loop.

### Crash-Consistency Rule

External side effects must not be dispatched until `ActionScheduled` is durably recorded under strict durability. For journaled mode, dispatch may occur after queue admission.

### Trace Ring

SPSC ring via `rtrb::RingBuffer`. On full, events are dropped and `dropped` counter incremented. `history: VecDeque` stores all successfully pushed events for snapshot queries. `drain_for_run` consumes non-matching events silently.

---
