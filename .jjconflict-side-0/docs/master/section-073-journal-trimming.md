---
section: 73
title: "Journal Trimming"
parent: velvet-ballistics-MASTER.md
---

## 73. Journal Trimming


The journal cannot grow indefinitely. After a snapshot is taken, journal events older than the snapshot are eligible for trimming.

### Trimming Contract

1. A snapshot captures the full run state at `SeqNo` N.
2. Once a snapshot at N is confirmed durable (fsynced), all journal events with `SeqNo <= N` for that run are eligible for deletion.
3. Trimming must not delete events for runs that have no snapshot.
4. Terminal runs (finished/failed/cancelled) are eligible for trimming after their final snapshot, subject to a retention policy (default: keep last N terminal runs per workflow).
5. The `doctor` command must report journal size and suggest trimming if the journal exceeds a configured threshold.

This prevents unbounded disk growth in long-running production deployments.

---
