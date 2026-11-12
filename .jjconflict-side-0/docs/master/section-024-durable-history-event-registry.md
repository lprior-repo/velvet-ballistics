---
section: 24
title: "Durable History Event Registry"
parent: velvet-ballistics-MASTER.md
---

## 24. Durable History Event Registry

Semantic orchestration events are durable truth. Frame deltas are attached where needed.

Required event families:

```text
RunAccepted
FrameAdvanced
SlotWritten
ArenaAppended
ActionScheduled
ActionStarted
ActionCompleted
ActionFailed
TimerScheduled
TimerFired
AskScheduled
AskAnswered
RetryScheduled
RunFinished
RunFailed
RunCancelled
SnapshotCreated
```

Each history event contains:

```rust
pub struct HistoryRecord {
    pub run: RunId,
    pub seq: SeqNo,
    pub attempt: AttemptId,
    pub kind: HistoryKind,
    pub frame_delta: Option<FrameDelta>,
    pub digest: Digest,
}
```

Event registry data must define:

```text
record kind ID
payload schema
valid predecessor state
recovery effect
terminal flag
whether strict durability requires fsync before ack
whether event can dispatch external effect
```

No journal event taxonomy may be maintained manually in multiple prose tables.

---

