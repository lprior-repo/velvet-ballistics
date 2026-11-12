---
section: 45
title: "Performance Contract"
parent: velvet-ballistics-MASTER.md
---

## 45. Performance Contract

Performance claims are allowed only with measured evidence.

Current performance goal:

```text
fast IR interpreter over accepted artifacts
single-server shard-owned execution
no runtime YAML/JSON/HTTP
no task per step
bounded queues and arenas
Postcard records
Fjall persistence
```

No generated Rust execution path is active. No `maxperf` profile is active.

Benchmarks must include:

```text
git commit
rustc version
CPU model
kernel version
storage type
build profile
RUSTFLAGS
benchmark tool and version
fixture digest
durability profile
execution mode
p50/p95/p99 latency
instruction counts
allocation counts
bytes allocated
Fjall write latency
IPC latency
```

The largest acceptable speed lever is batching deterministic work into semantic durable boundaries while preserving side-effect safety:

```text
No fsync every instruction by default.
Do persist before external dispatch, wait/ask suspension, completion mutation, terminal run closure, and strict acknowledgements.
```

---

