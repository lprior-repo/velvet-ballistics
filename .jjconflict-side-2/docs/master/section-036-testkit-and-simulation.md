---
section: 36
title: "Testkit and Simulation"
parent: velvet-ballistics-MASTER.md
---

## 36. Testkit and Simulation

Simulation is a first-class product surface.

`cargo velvet simulate` runs accepted workflows with mocked actions and deterministic inputs. It emits a `SimulationReport` that includes:

```text
accepted artifact digest
policy digest
action ABI digest
input digest
mock manifest digest
event sequence
slot summaries
action schedules/completions
result summary
taint summary
resource usage
replayability result
```

Testkit crash points must map to real durability boundaries:

```rust
pub enum CrashPoint {
    BeforeRunAcceptedWrite,
    AfterRunAcceptedWriteBeforeAck,
    BeforeFrameCommit,
    AfterFrameCommitBeforeAck,
    AfterActionScheduledBeforeDispatch,
    AfterActionDispatchBeforeCompletion,
    AfterActionCompletionBeforeFrameMutation,
    AfterFrameMutationBeforeCompletionAck,
    AfterSnapshotBeforeTrim,
    DuringJournalTailReplay,
}
```

A toy interpreter is forbidden. Simulation and crash labs use production code paths.

---

