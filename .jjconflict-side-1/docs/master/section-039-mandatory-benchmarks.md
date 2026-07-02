---
section: 39
title: "Mandatory Benchmarks"
parent: velvet-ballistics-MASTER.md
---

## 39. Mandatory Benchmarks


**Benchmark naming:** Exact benchmark names are not mandated. Benchmarks must exist covering the following areas. The authoritative benchmark list is `benches/velvet_ballistics.rs`.

Required coverage areas:

| Area | Required benchmarks |
|------|-------------------|
| YAML parsing | Small workflow, large (1 MiB) workflow |
| Validation | Minimal workflow, 1000-step workflow |
| Compilation | Minimal workflow, 1000-step workflow |
| Expression | Symbol equality, number comparison, boolean chain, arithmetic |
| Slot operations | Read, write, copy |
| Core transitions | SetConst, EvalExpr, Choose (2-branch, 100-branch), Finish |
| Run chains | 1-step, 10-step, 1000-step save chains |
| Iteration | for_each, together, collect, reduce, repeat |
| Storage | Fjall append (no-persist, journaled, strict), Fjall read 1000 events |
| IPC | Frame encode, frame decode |
| Queues | ArrayQueue push/pop, rtrb push/pop |
| Trace | Trace event push, ring full policy |
| Writer queue | Journal writer queue push, group commit (batch 1, 64, 1024) |
| Scheduler | Shard submit-to-start, submit-to-finish |
| Direct API | Submit-to-finish |
| Async primitives | Ask answer resume, action complete resume, wait timer resume |

Every benchmark result must include metadata:

```text
git commit
rustc version
nightly date
CPU model
CPU governor
kernel version
build profile
RUSTFLAGS
benchmark tool and version
sample count or instruction count
input fixture digest
durability profile
execution mode (`ir-interpreter` for the current milestone)
p50/p95/p99 latency
instruction counts
allocation count
bytes allocated
Fjall write latency
direct API latency
IPC latency
```

Acceptance rule: no speed claim without benchmark numbers. No optimization PR without before/after benchmark output and correctness evidence. Compileable Criterion scaffold benchmarks are placeholders only; no-op scaffolds such as `black_box(())` prove the harness builds, not that the implementation is faster, lower allocation, lower latency, or production ready.

---
